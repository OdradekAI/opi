//! Controlled lifecycle observations, not a live-provider or full parity probe.
//! Named gates control source terminal emission and source EOF.

use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use futures_util::{FutureExt, Stream, StreamExt};
use opi_ai::deferred::{
    DeferredApi, DeferredCancelCall, DeferredError, DeferredFetchCall, DeferredHandle,
};
use opi_ai::message::{AssistantContent, AssistantMessage};
use opi_ai::models::{ApiId, ChatCall, CompatMetadata};
use opi_ai::provider::{
    EventStream, ProviderError, ProviderErrorSummary, Request, extract_diagnostic_error,
};
use opi_ai::stream::{AssistantStreamEvent, CallFailure, ChatStream, StopReason};
use opi_ai::telemetry::{
    AI_FIRST_CHUNK, AI_FIRST_CHUNK_DURATION, AI_POLL_SPAN, AI_REQUEST_DURATION, AI_REQUEST_SPAN,
};
use opi_ai::test_support::{
    CountingAuthResolver, MockProvider, ScriptedDeferredBackend, base_assistant,
    single_route_collection,
};
use opi_ai::{
    AuthProvenanceSource, AuthResolver, AuthScheme, BoxAuthFuture, ChatModelRecord, ModelInfo,
    Models, Provider, ResolvedAuth, StaticAuthResolver,
};
use opi_telemetry::{
    AttributeValue, Attributes, ErrorStatus, InstrumentDefinition, MetricKind, SpanOptions,
    SpanStatus, TelemetryContext, TelemetryError,
    backend::{ContextBackend, InstrumentBackend, MetricsBackend, SpanBackend},
    memory::Recorder,
};
use serde::Serialize;
use serde_json::{Value, json};
use tokio::sync::oneshot;
use tracing::Level;
use tracing_core::field::{Field, Visit};
use tracing_core::span::{Attributes as TracingAttributes, Id, Record};
use tracing_core::{Dispatch, Event, Metadata, Subscriber, dispatcher};

#[derive(Clone, Debug, Default, Serialize)]
struct Observations {
    dispatch_calls: usize,
    #[serde(skip)]
    auth_calls: usize,
    source_started: bool,
    terminal_count: usize,
    source_eof: bool,
    source_dropped: bool,
}

type Shared = Arc<Mutex<Observations>>;

const RESULT_BEFORE_EOF: &str = "models-result-before-source-eof";
const TERMINAL_AND_SPAN_BEFORE_EOF: &str = "models-terminal-and-span-before-source-eof";
const RESULT_ATTRIBUTE_WRITE_FAILURE: &str = "models-result-attribute-write-failure";
const RESULT_ATTRIBUTE_WRITE_ERROR: &str = "controlled-result-attribute-write-failure";
const FIRST_CHUNK_WRITE_FAILURE: &str = "models-first-chunk-event-write-failure";
const FIRST_CHUNK_WRITE_ERROR: &str = "controlled-first-chunk-event-write-failure";
const HISTOGRAM_CREATION_FAILURE: &str = "models-request-duration-histogram-creation-failure";
const HISTOGRAM_CREATION_ERROR: &str = "controlled-request-duration-histogram-creation-failure";
const REQUEST_SPAN_START_FAILURE: &str = "models-request-span-start-failure";
const REQUEST_SPAN_START_ERROR: &str = "controlled-request-span-start-failure";
const FIRST_CHUNK_DURATION_FAILURE: &str = "models-first-chunk-duration-record-failure";
const FIRST_CHUNK_DURATION_ERROR: &str = "controlled-first-chunk-duration-record-failure";
const REQUEST_DURATION_FAILURE: &str = "models-request-duration-record-failure";
const REQUEST_DURATION_ERROR: &str = "controlled-request-duration-record-failure";
const SPAN_FINISH_ERROR: &str = "controlled-span-finish-failure";
const DEFERRED_WAIT0: &str = "models-deferred-wait0-pending-then-ready";

struct ControlledProvider {
    models: Vec<ModelInfo>,
    source: Mutex<Option<ControlledSource>>,
    observations: Shared,
}

impl Provider for ControlledProvider {
    fn id(&self) -> &str {
        "lifecycle-differential"
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    fn stream_prepared(&self, _request: Request, _auth: opi_ai::ResolvedAuth) -> EventStream {
        self.observations.lock().unwrap().dispatch_calls += 1;
        Box::pin(self.source.lock().unwrap().take().expect("one request"))
    }
}

struct ControlledSource {
    observations: Shared,
    terminal_gate: SourceGate,
    eof_gate: Option<SourceGate>,
    dropped: Option<oneshot::Sender<()>>,
    message: AssistantMessage,
    state: SourceState,
}

struct SourceGate {
    reached: Option<oneshot::Sender<()>>,
    repolled: Option<oneshot::Sender<()>>,
    release: oneshot::Receiver<()>,
}

impl SourceGate {
    fn new() -> (Self, oneshot::Receiver<()>, oneshot::Sender<()>) {
        let (reached_tx, reached_rx) = oneshot::channel();
        let (release_tx, release_rx) = oneshot::channel();
        (
            Self {
                reached: Some(reached_tx),
                repolled: None,
                release: release_rx,
            },
            reached_rx,
            release_tx,
        )
    }

    fn report_repoll(&mut self) -> oneshot::Receiver<()> {
        let (tx, rx) = oneshot::channel();
        self.repolled = Some(tx);
        rx
    }

    fn poll_release(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        if let Some(reached) = self.reached.take() {
            reached.send(()).expect("driver waits at source gate");
        } else if let Some(repolled) = self.repolled.take() {
            repolled.send(()).expect("driver waits for the second poll");
        }
        Pin::new(&mut self.release)
            .poll(cx)
            .map(|released| released.expect("driver releases source explicitly"))
    }
}

enum SourceState {
    Start,
    AtGate,
    Tail,
    End,
}

impl Stream for ControlledSource {
    type Item = Result<AssistantStreamEvent, ProviderError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.state {
            SourceState::Start => {
                self.observations.lock().unwrap().source_started = true;
                self.state = SourceState::AtGate;
                let mut partial = self.message.clone();
                partial.stop_reason = StopReason::Pending;
                Poll::Ready(Some(Ok(AssistantStreamEvent::Start { partial })))
            }
            SourceState::AtGate => {
                if self.terminal_gate.reached.is_some() {
                    self.observations.lock().unwrap().source_started = true;
                }
                if self.terminal_gate.poll_release(cx).is_pending() {
                    return Poll::Pending;
                }
                self.observations.lock().unwrap().terminal_count += 1;
                self.state = SourceState::Tail;
                Poll::Ready(Some(Ok(AssistantStreamEvent::Done {
                    reason: StopReason::Stop,
                    message: self.message.clone(),
                })))
            }
            SourceState::Tail => {
                if let Some(gate) = &mut self.eof_gate
                    && gate.poll_release(cx).is_pending()
                {
                    return Poll::Pending;
                }
                self.observations.lock().unwrap().source_eof = true;
                self.state = SourceState::End;
                Poll::Ready(None)
            }
            SourceState::End => Poll::Ready(None),
        }
    }
}

impl Drop for ControlledSource {
    fn drop(&mut self) {
        self.observations.lock().unwrap().source_dropped = true;
        if let Some(dropped) = self.dropped.take() {
            let _ = dropped.send(());
        }
    }
}

async fn bounded<T>(future: impl Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(5), future)
        .await
        .expect("controlled lifecycle operation must make progress")
}

fn request() -> Request {
    Request {
        model: "lifecycle-differential:mock-model".into(),
        context: opi_ai::transcript::normalize_context(Default::default()).unwrap(),
        projection: Default::default(),
        transport: Default::default(),
        max_tokens: None,
        temperature: None,
        sampling_params: None,
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: tokio_util::sync::CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: Default::default(),
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

fn snapshot(observations: &Shared) -> Observations {
    observations.lock().unwrap().clone()
}

fn step(name: &str, observations: &Shared) -> Value {
    json!({"name": name, "observed": snapshot(observations)})
}

fn controlled_call(
    request: Request,
    observations: &Shared,
    terminal_gate: SourceGate,
    eof_gate: Option<SourceGate>,
    dropped: oneshot::Sender<()>,
    emit_start: bool,
) -> ChatCall {
    let provider = ControlledProvider {
        models: MockProvider::new("seed", vec![]).models().to_vec(),
        source: Mutex::new(Some(ControlledSource {
            observations: observations.clone(),
            terminal_gate,
            eof_gate,
            dropped: Some(dropped),
            message: AssistantMessage {
                provider: "lifecycle-differential".into(),
                ..base_assistant()
            },
            state: if emit_start {
                SourceState::Start
            } else {
                SourceState::AtGate
            },
        })),
        observations: observations.clone(),
    };
    let models = single_route_collection(Box::new(provider));
    let model = models
        .get_model("lifecycle-differential", "mock-model")
        .unwrap();
    let record = ChatModelRecord::new("lifecycle-differential", model.clone());
    models.stream_simple(record, request, Default::default())
}

async fn completion(scenario: &str, drop_consumer: bool) -> Value {
    let observations = Shared::default();
    let (terminal_gate, terminal_reached, release_terminal) = SourceGate::new();
    let (eof_gate, eof_control) = if scenario == RESULT_BEFORE_EOF {
        let (gate, reached, release) = SourceGate::new();
        (Some(gate), Some((reached, release)))
    } else {
        (None, None)
    };
    let (dropped_tx, dropped_rx) = oneshot::channel();
    let mut call = Some(controlled_call(
        request(),
        &observations,
        terminal_gate,
        eof_gate,
        dropped_tx,
        true,
    ));
    let mut steps = vec![step("constructed", &observations)];
    bounded(terminal_reached).await.unwrap();
    steps.push(step("source-at-gate", &observations));

    if scenario == "models-stop-consumption" {
        let first = bounded(call.as_mut().unwrap().next())
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(first, AssistantStreamEvent::Start { .. }));
        steps.push(step("consumer-stopped-after-start", &observations));
    }
    if drop_consumer {
        drop(call.take());
        steps.push(step("consumer-dropped", &observations));
    }
    // This observation is made while the only terminal-producing path is blocked.
    // No sleeps, yields or timestamps are used as a substitute for that barrier.
    let before_release = snapshot(&observations);
    assert_eq!(before_release.terminal_count, 0);
    assert!(!before_release.source_eof);
    assert!(!before_release.source_dropped);
    let mut result = std::pin::pin!(async {
        match call {
            Some(call) => Some(call.result().await.expect("controlled completion")),
            None => None,
        }
    });
    let mut before_release = step("before-source-release", &observations);
    if eof_control.is_some() {
        assert!(result.as_mut().now_or_never().is_none());
        before_release["result_state"] = json!("pending");
    }
    steps.push(before_release);
    release_terminal.send(()).unwrap();
    let result_before_eof = if let Some((reached, release)) = eof_control {
        bounded(reached).await.expect("source reaches the EOF gate");
        // Poll the same public result future while EOF is held by the driver.
        let ready = result.as_mut().now_or_never();
        let message = ready.as_ref().map(|value| {
            let message = value.as_ref().expect("result future owns a call");
            json!({"stop": message.stop_reason.as_str(), "provider": message.provider, "model": message.model})
        });
        steps.push(json!({
            "name": "result-before-source-eof", "observed": snapshot(&observations),
            "result_state": if ready.is_some() { "ready" } else { "pending" },
            "result": message,
        }));
        release.send(()).expect("source waits at the EOF gate");
        ready
    } else {
        None
    };
    bounded(dropped_rx).await.unwrap();
    let after = snapshot(&observations);
    assert_eq!(after.dispatch_calls, 1);
    assert_eq!(after.terminal_count, 1);
    assert!(after.source_dropped);
    // Record EOF independently from Drop. A wrapper may release a source
    // without polling its tail; the differential must expose that mismatch.
    steps.push(step("source-released", &observations));

    let result = match result_before_eof {
        Some(result) => result,
        None => bounded(result).await,
    };
    if let Some(message) = &result {
        steps.push(json!({
            "name": "result-observed", "stop": message.stop_reason.as_str(),
            "provider": message.provider, "model": message.model,
        }));
    }
    json!({
        "schemaVersion": 2,
        "runtime": "rust",
        "scenario": scenario,
        "steps": steps,
        "runtimeDetails": {
            "consumer_handle_dropped": drop_consumer,
            "source_dropped": after.source_dropped,
            "result_observed": result.is_some(),
        },
    })
}

fn request_spans(recorder: &Recorder) -> Vec<Value> {
    request_span_snapshots(recorder, false)
}

fn request_usage_input(recorder: &Recorder) -> Option<AttributeValue> {
    recorder
        .spans()
        .into_iter()
        .find(|span| span.name == AI_REQUEST_SPAN)
        .and_then(|mut span| span.attributes.remove("usage.input"))
}

fn request_span_snapshots(recorder: &Recorder, include_details: bool) -> Vec<Value> {
    recorder
        .spans()
        .iter()
        .filter(|span| span.name == "pi.ai.request")
        .map(|span| {
            let attribute = |key| match span.attributes.get(key) {
                Some(AttributeValue::String(value)) => Some(value.as_str()),
                _ => None,
            };
            let mut snapshot = json!({
                "name": span.name,
                "provider": attribute("provider"), "model": attribute("model"),
                "settled": span.end_sequence.is_some(), "stop": attribute("stop"),
            });
            if include_details {
                snapshot["events"] = json!(
                    span.events
                        .iter()
                        .map(|event| &event.name)
                        .collect::<Vec<_>>()
                );
                snapshot["status"] = json!(span.status);
            }
            snapshot
        })
        .collect()
}

fn done_message(item: Option<Result<AssistantStreamEvent, CallFailure>>) -> AssistantMessage {
    let Some(Ok(AssistantStreamEvent::Done { reason, message })) = item else {
        panic!("expected public Done, got {item:?}");
    };
    assert_eq!(reason, StopReason::Stop);
    message
}

fn terminal_snapshot(message: &AssistantMessage) -> Value {
    json!({
        "kind": "done", "stop": message.stop_reason.as_str(),
        "provider": message.provider, "model": message.model,
    })
}

async fn observe_terminal_and_span() -> Value {
    let observations = Shared::default();
    let recorder = Recorder::default();
    let mut request = request();
    request.telemetry_context = Some(recorder.context());
    let (terminal_gate, terminal_reached, release_terminal) = SourceGate::new();
    let (eof_gate, eof_reached, release_eof) = SourceGate::new();
    let (dropped_tx, dropped_rx) = oneshot::channel();
    let mut call = controlled_call(
        request,
        &observations,
        terminal_gate,
        Some(eof_gate),
        dropped_tx,
        true,
    );
    let mut steps = vec![step("constructed", &observations)];
    bounded(terminal_reached).await.unwrap();
    steps.push(step("source-at-gate", &observations));
    assert!(matches!(
        bounded(call.next()).await,
        Some(Ok(AssistantStreamEvent::Start { .. }))
    ));

    let terminal = {
        // Observe the same pending next operation on both sides of the gate.
        let mut next = std::pin::pin!(call.next());
        assert!(next.as_mut().now_or_never().is_none());
        steps.push(json!({
            "name": "before-source-release", "observed": snapshot(&observations),
            "public_start": "start", "next_state": "pending", "request_spans": request_spans(&recorder),
        }));
        release_terminal.send(()).unwrap();
        bounded(eof_reached)
            .await
            .expect("source reaches the EOF gate");
        let ready = next.as_mut().now_or_never().map(done_message);
        steps.push(json!({
            "name": "terminal-and-span-before-source-eof", "observed": snapshot(&observations),
            "next_state": if ready.is_some() { "ready" } else { "pending" },
            "terminal": ready.as_ref().map(terminal_snapshot), "request_spans": request_spans(&recorder),
        }));
        release_eof.send(()).expect("source waits at the EOF gate");
        bounded(dropped_rx).await.expect("source is released");
        steps.push(step("source-released", &observations));
        match ready {
            Some(message) => message,
            None => done_message(bounded(next).await),
        }
    };

    assert!(
        bounded(call.next()).await.is_none(),
        "public stream reaches EOF after Done"
    );
    steps.push(json!({
        "name": "public-stream-completed", "terminal": terminal_snapshot(&terminal),
        "outer_eof": true, "request_spans": request_spans(&recorder),
    }));
    let result = bounded(call.result()).await.expect("controlled completion");
    steps.push(json!({
        "name": "result-observed", "stop": result.stop_reason.as_str(),
        "provider": result.provider, "model": result.model, "matches_terminal": terminal == result,
    }));
    json!({
        "schemaVersion": 2,
        "runtime": "rust",
        "scenario": TERMINAL_AND_SPAN_BEFORE_EOF,
        "steps": steps,
        "runtimeDetails": {
            "consumer_handle_dropped": false,
            "source_dropped": snapshot(&observations).source_dropped,
            "result_observed": true,
        },
    })
}

#[derive(Debug, Default)]
enum FaultSite {
    #[default]
    ResultAttributes,
    ObservePollFinish,
    FirstChunkEvent,
    DurationRecord {
        name: &'static str,
        marker: &'static str,
    },
    RequestSpanStart,
    InstrumentCreation {
        name: &'static str,
        kind: MetricKind,
        marker: &'static str,
    },
}

#[derive(Debug, Default)]
struct TelemetryFault {
    site: FaultSite,
    source: Shared,
    attempts: Mutex<Vec<Value>>,
    created_instruments: Mutex<Vec<Value>>,
    finish_calls: AtomicUsize,
    fail_span_finish: bool,
    span_backend_dropped: AtomicBool,
}

impl TelemetryFault {
    fn record_attempt(&self, attempt: Value, marker: &'static str) -> opi_telemetry::Result<()> {
        let mut attempts = self.attempts.lock().unwrap();
        attempts.push(attempt);
        if attempts.len() == 1 {
            return Err(TelemetryError::new(std::io::Error::other(marker)));
        }
        Ok(())
    }
}

#[derive(Debug)]
struct FaultingContext {
    inner: TelemetryContext,
    recorder: Recorder,
    fault: Arc<TelemetryFault>,
}

fn faulting_context(recorder: &Recorder, fault: Arc<TelemetryFault>) -> TelemetryContext {
    TelemetryContext::new(Arc::new(FaultingContext {
        inner: recorder.context(),
        recorder: recorder.clone(),
        fault,
    }))
}

impl ContextBackend for FaultingContext {
    fn start_span(&self, options: &SpanOptions) -> opi_telemetry::Result<Arc<dyn SpanBackend>> {
        if matches!(self.fault.site, FaultSite::RequestSpanStart) && options.name == AI_REQUEST_SPAN
        {
            self.fault.record_attempt(
                json!({
                    "name": options.name,
                    "provider": options.attributes.get("provider"), "model": options.attributes.get("model"),
                    "created_instruments": *self.fault.created_instruments.lock().unwrap(),
                    "observed": snapshot(&self.fault.source), "request_spans": request_spans(&self.recorder),
                }),
                REQUEST_SPAN_START_ERROR,
            )?;
        }
        let inner = self.inner.backend().start_span(options)?;
        if options.name == AI_REQUEST_SPAN
            || matches!(self.fault.site, FaultSite::ObservePollFinish)
                && options.name == AI_POLL_SPAN
        {
            return Ok(Arc::new(FaultingSpan {
                inner,
                fault: self.fault.clone(),
            }));
        }
        Ok(inner)
    }

    fn metrics(&self) -> Arc<dyn MetricsBackend> {
        let inner = self.inner.backend().metrics();
        if matches!(
            self.fault.site,
            FaultSite::InstrumentCreation { .. }
                | FaultSite::RequestSpanStart
                | FaultSite::DurationRecord { .. }
        ) {
            return Arc::new(FaultingMetrics {
                inner,
                recorder: self.recorder.clone(),
                fault: self.fault.clone(),
            });
        }
        inner
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self.inner.backend().as_any()
    }
}

#[derive(Debug)]
struct FaultingSpan {
    inner: Arc<dyn SpanBackend>,
    fault: Arc<TelemetryFault>,
}

impl ContextBackend for FaultingSpan {
    fn start_span(&self, options: &SpanOptions) -> opi_telemetry::Result<Arc<dyn SpanBackend>> {
        self.inner.start_span(options)
    }

    fn metrics(&self) -> Arc<dyn MetricsBackend> {
        self.inner.metrics()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self.inner.as_any()
    }
}

impl SpanBackend for FaultingSpan {
    fn set_attributes(&self, attributes: &Attributes) -> opi_telemetry::Result<()> {
        if matches!(self.fault.site, FaultSite::ResultAttributes)
            && let Some(AttributeValue::String(stop)) = attributes.get("stop")
            && attributes.contains_key("usage.input")
        {
            self.fault.record_attempt(
                json!({"observed": snapshot(&self.fault.source), "stop": stop}),
                RESULT_ATTRIBUTE_WRITE_ERROR,
            )?;
        }
        self.inner.set_attributes(attributes)
    }

    fn add_event(&self, name: &str, attributes: &Attributes) -> opi_telemetry::Result<()> {
        if matches!(self.fault.site, FaultSite::FirstChunkEvent) && name == AI_FIRST_CHUNK {
            self.fault.record_attempt(
                json!({"event": name, "observed": snapshot(&self.fault.source)}),
                FIRST_CHUNK_WRITE_ERROR,
            )?;
        }
        self.inner.add_event(name, attributes)
    }

    fn set_status(&self, status: SpanStatus) -> opi_telemetry::Result<()> {
        self.inner.set_status(status)
    }

    fn finish(&self, error: Option<ErrorStatus>) -> opi_telemetry::Result<()> {
        self.fault.finish_calls.fetch_add(1, Ordering::Relaxed);
        if matches!(self.fault.site, FaultSite::ObservePollFinish) {
            self.fault
                .attempts
                .lock()
                .unwrap()
                .push(json!(snapshot(&self.fault.source).source_dropped));
        }
        if self.fault.fail_span_finish {
            return Err(TelemetryError::new(std::io::Error::other(
                SPAN_FINISH_ERROR,
            )));
        }
        self.inner.finish(error)
    }
}

impl Drop for FaultingSpan {
    fn drop(&mut self) {
        self.fault
            .span_backend_dropped
            .store(true, Ordering::Relaxed);
    }
}

#[derive(Debug)]
struct FaultingMetrics {
    inner: Arc<dyn MetricsBackend>,
    recorder: Recorder,
    fault: Arc<TelemetryFault>,
}

impl MetricsBackend for FaultingMetrics {
    fn instrument(
        &self,
        definition: &InstrumentDefinition,
    ) -> opi_telemetry::Result<Arc<dyn InstrumentBackend>> {
        if let FaultSite::InstrumentCreation { name, kind, marker } = self.fault.site
            && definition.name == name
            && definition.kind == kind
        {
            self.fault.record_attempt(
                json!({
                    "name": definition.name, "kind": definition.kind, "unit": definition.unit,
                    "observed": snapshot(&self.fault.source),
                    "request_spans": request_spans(&self.recorder),
                }),
                marker,
            )?;
        }
        let instrument = self.inner.instrument(definition)?;
        if matches!(self.fault.site, FaultSite::RequestSpanStart) {
            self.fault.created_instruments.lock().unwrap().push(json!({
                "name": definition.name, "kind": definition.kind, "unit": definition.unit,
            }));
        }
        if let FaultSite::DurationRecord { name, marker } = self.fault.site
            && definition.name == name
            && definition.kind == MetricKind::Histogram
        {
            return Ok(Arc::new(FaultingDurationInstrument {
                inner: instrument,
                definition: definition.clone(),
                marker,
                recorder: self.recorder.clone(),
                fault: self.fault.clone(),
            }));
        }
        Ok(instrument)
    }
}

#[derive(Debug)]
struct FaultingDurationInstrument {
    inner: Arc<dyn InstrumentBackend>,
    definition: InstrumentDefinition,
    marker: &'static str,
    recorder: Recorder,
    fault: Arc<TelemetryFault>,
}

impl InstrumentBackend for FaultingDurationInstrument {
    fn record(
        &self,
        value: f64,
        attributes: &Attributes,
        context: Option<&TelemetryContext>,
    ) -> opi_telemetry::Result<()> {
        let mut attempt = json!({
            "name": self.definition.name, "kind": self.definition.kind, "unit": self.definition.unit,
            "observed": snapshot(&self.fault.source), "request_spans": request_span_snapshots(&self.recorder, true),
        });
        if self.definition.name == AI_REQUEST_DURATION {
            attempt["record_stop"] = json!(attributes.get("stop"));
            attempt["usage_input"] = json!(request_usage_input(&self.recorder));
        }
        self.fault.record_attempt(attempt, self.marker)?;
        self.inner.record(value, attributes, context)
    }
}

#[derive(Clone, Default)]
struct HostWarnings(Arc<Mutex<Vec<Value>>>);

impl Subscriber for HostWarnings {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.is_event()
            && metadata.level() <= &Level::WARN
            && metadata.target() == "opi_ai::telemetry"
    }

    fn new_span(&self, _: &TracingAttributes<'_>) -> Id {
        unreachable!("the host subscriber only records events");
    }

    fn record(&self, _: &Id, _: &Record<'_>) {}
    fn record_follows_from(&self, _: &Id, _: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut fields = HostFields::default();
        event.record(&mut fields);
        self.0.lock().unwrap().push(json!({
            "channel": "tracing", "level": event.metadata().level().as_str(),
            "target": event.metadata().target(), "fields": fields.0,
        }));
    }

    fn enter(&self, _: &Id) {}
    fn exit(&self, _: &Id) {}
}

#[derive(Default)]
struct HostFields(BTreeMap<String, String>);

impl Visit for HostFields {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.record_str(field, &format!("{value:?}"));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.insert(field.name().to_owned(), value.to_owned());
    }
}

async fn observe_result_after_release(
    request: Request,
    observations: &Shared,
    recorder: &Recorder,
) -> Vec<Value> {
    let (gate, reached, release) = SourceGate::new();
    let (dropped_tx, dropped_rx) = oneshot::channel();
    let mut call = controlled_call(request, observations, gate, None, dropped_tx, true);
    let mut steps = vec![step("constructed", observations)];
    bounded(reached).await.unwrap();
    steps.push(step("source-at-gate", observations));
    assert!(matches!(
        bounded(call.next()).await,
        Some(Ok(AssistantStreamEvent::Start { .. }))
    ));
    steps.push(json!({
        "name": "before-source-release", "observed": snapshot(observations),
        "public_start": "start", "request_spans": request_spans(recorder),
    }));
    release.send(()).unwrap();
    bounded(dropped_rx).await.expect("source is released");
    steps.push(step("source-released", observations));
    let terminal = done_message(bounded(call.next()).await);
    assert!(bounded(call.next()).await.is_none());
    steps.push(json!({
        "name": "public-stream-completed", "terminal": terminal_snapshot(&terminal), "outer_eof": true,
    }));
    let result = bounded(call.result()).await.expect("controlled completion");
    steps.push(json!({
        "name": "result-observed", "stop": result.stop_reason.as_str(),
        "provider": result.provider, "model": result.model, "matches_terminal": terminal == result,
    }));
    steps
}

async fn observe_result_attribute_failure(fail_span_finish: bool) -> Value {
    let observations = Shared::default();
    let recorder = Recorder::default();
    let fault = Arc::new(TelemetryFault {
        source: observations.clone(),
        fail_span_finish,
        ..Default::default()
    });
    let warnings = HostWarnings::default();
    // The driver and its spawned execution both run on this probe thread.
    let dispatch = Dispatch::new(warnings.clone());
    let _host = dispatcher::set_default(&dispatch);
    let mut request = request();
    request.telemetry_context = Some(faulting_context(&recorder, fault.clone()));
    let mut steps = observe_result_after_release(request, &observations, &recorder).await;
    steps.push(json!({
        "name": "result-attribute-write-failure-observed", "fault_delivery": "typed-err",
        "fault_marker": RESULT_ATTRIBUTE_WRITE_ERROR, "write_attempts": *fault.attempts.lock().unwrap(),
        "request_spans": request_spans(&recorder), "host_errors": *warnings.0.lock().unwrap(),
        "span_backend_dropped": fault.span_backend_dropped.load(Ordering::Relaxed),
        "span_finish_calls": fault.finish_calls.load(Ordering::Relaxed),
    }));
    json!({
        "schemaVersion": 2,
        "runtime": "rust",
        "scenario": RESULT_ATTRIBUTE_WRITE_FAILURE,
        "steps": steps,
        "runtimeDetails": {
            "consumer_handle_dropped": false,
            "source_dropped": snapshot(&observations).source_dropped,
            "result_observed": true,
        },
    })
}

async fn observe_terminal_gate(
    mut call: ChatCall,
    observations: &Shared,
    recorder: &Recorder,
    reached: oneshot::Receiver<()>,
    release: oneshot::Sender<()>,
    dropped: oneshot::Receiver<()>,
) -> [Value; 3] {
    bounded(reached)
        .await
        .expect("source reaches the terminal gate");
    assert!(matches!(
        bounded(call.next()).await,
        Some(Ok(AssistantStreamEvent::Start { .. }))
    ));
    let (held, terminal) = {
        let mut next = std::pin::pin!(call.next());
        assert!(next.as_mut().now_or_never().is_none());
        let held = json!({
            "name": "terminal-gate-held", "observed": snapshot(observations),
            "public_start": "start", "next_state": "pending", "terminal": null,
            "request_spans": request_spans(recorder),
        });
        release.send(()).expect("source waits at the terminal gate");
        bounded(dropped).await.expect("source is released");
        (held, done_message(bounded(next).await))
    };
    assert!(bounded(call.next()).await.is_none());
    let completed = json!({
        "name": "public-stream-completed", "terminal": terminal_snapshot(&terminal),
        "outer_eof": true, "observed": snapshot(observations),
    });
    let result = bounded(call.result()).await.expect("controlled completion");
    let result = json!({
        "name": "result-observed", "delivery": "ok", "stop": result.stop_reason.as_str(),
        "provider": result.provider, "model": result.model, "matches_terminal": terminal == result,
    });
    [held, completed, result]
}

async fn observe_first_chunk_failure() -> Value {
    let observations = Shared::default();
    let recorder = Recorder::default();
    let fault = Arc::new(TelemetryFault {
        site: FaultSite::FirstChunkEvent,
        source: observations.clone(),
        ..Default::default()
    });
    let warnings = HostWarnings::default();
    let dispatch = Dispatch::new(warnings.clone());
    let _host = dispatcher::set_default(&dispatch);
    let mut request = request();
    request.telemetry_context = Some(faulting_context(&recorder, fault.clone()));
    let (gate, reached, release) = SourceGate::new();
    let (dropped_tx, dropped_rx) = oneshot::channel();
    let call = controlled_call(request, &observations, gate, None, dropped_tx, true);
    let mut steps = vec![json!({
        "name": "constructed", "observed": snapshot(&observations),
        "request_spans": request_spans(&recorder),
    })];
    steps.extend(
        observe_terminal_gate(call, &observations, &recorder, reached, release, dropped_rx).await,
    );
    steps.push(json!({
        "name": "first-chunk-write-failure-observed", "fault_delivery": "typed-err",
        "fault_marker": FIRST_CHUNK_WRITE_ERROR, "write_attempts": *fault.attempts.lock().unwrap(),
        "request_spans": request_spans(&recorder), "host_errors": *warnings.0.lock().unwrap(),
        "span_backend_dropped": fault.span_backend_dropped.load(Ordering::Relaxed),
        "span_finish_calls": fault.finish_calls.load(Ordering::Relaxed),
    }));
    json!({
        "schemaVersion": 2,
        "runtime": "rust",
        "scenario": FIRST_CHUNK_WRITE_FAILURE,
        "steps": steps,
        "runtimeDetails": {
            "consumer_handle_dropped": false,
            "source_dropped": snapshot(&observations).source_dropped,
            "result_observed": true,
        },
    })
}

async fn observe_histogram_creation_failure() -> Value {
    let observations = Shared::default();
    let recorder = Recorder::default();
    let fault = Arc::new(TelemetryFault {
        site: FaultSite::InstrumentCreation {
            name: AI_REQUEST_DURATION,
            kind: MetricKind::Histogram,
            marker: HISTOGRAM_CREATION_ERROR,
        },
        source: observations.clone(),
        ..Default::default()
    });
    let warnings = HostWarnings::default();
    let dispatch = Dispatch::new(warnings.clone());
    let _host = dispatcher::set_default(&dispatch);
    let mut request = request();
    request.telemetry_context = Some(faulting_context(&recorder, fault.clone()));
    let (gate, reached, release) = SourceGate::new();
    let (dropped_tx, dropped_rx) = oneshot::channel();
    let mut call = controlled_call(request, &observations, gate, None, dropped_tx, true);
    let mut steps = vec![json!({
        "name": "call-boundary", "provider": "lifecycle-differential", "model": "mock-model",
        "delivery": "returned-handle", "error_message": null,
        "instrument_attempt_count": fault.attempts.lock().unwrap().len(),
        "observed": snapshot(&observations), "request_spans": request_spans(&recorder),
        "span_finish_calls": fault.finish_calls.load(Ordering::Relaxed),
        "span_backend_dropped": fault.span_backend_dropped.load(Ordering::Relaxed),
    })];
    bounded(reached)
        .await
        .expect("provider reaches the gate before consumption");
    steps.push(step("source-at-gate", &observations));
    assert!(matches!(
        bounded(call.next()).await,
        Some(Ok(AssistantStreamEvent::Start { .. }))
    ));
    steps.push(json!({
        "name": "before-source-release", "observed": snapshot(&observations), "public_start": "start",
    }));
    release.send(()).expect("source waits at the terminal gate");
    bounded(dropped_rx).await.expect("source is released");
    let terminal = done_message(bounded(call.next()).await);
    assert!(bounded(call.next()).await.is_none());
    steps.push(json!({
        "name": "public-stream-completed", "terminal": terminal_snapshot(&terminal), "outer_eof": true,
    }));
    let result = bounded(call.result()).await.expect("controlled completion");
    steps.push(json!({
        "name": "result-observed", "delivery": "ok", "stop": result.stop_reason.as_str(),
        "provider": result.provider, "model": result.model, "matches_terminal": terminal == result,
    }));
    steps.push(json!({
        "name": "histogram-creation-failure-observed", "fault_delivery": "typed-err",
        "fault_marker": HISTOGRAM_CREATION_ERROR, "instrument_attempts": *fault.attempts.lock().unwrap(),
        "observed": snapshot(&observations), "request_spans": request_spans(&recorder),
        "host_errors": *warnings.0.lock().unwrap(),
        "span_finish_calls": fault.finish_calls.load(Ordering::Relaxed),
        "span_backend_dropped": fault.span_backend_dropped.load(Ordering::Relaxed),
    }));
    json!({
        "schemaVersion": 2,
        "runtime": "rust",
        "scenario": HISTOGRAM_CREATION_FAILURE,
        "steps": steps,
        "runtimeDetails": {
            "consumer_handle_dropped": false,
            "source_dropped": snapshot(&observations).source_dropped,
            "result_observed": true,
        },
    })
}

async fn observe_request_span_start_failure() -> Value {
    let observations = Shared::default();
    let recorder = Recorder::default();
    let fault = Arc::new(TelemetryFault {
        site: FaultSite::RequestSpanStart,
        source: observations.clone(),
        ..Default::default()
    });
    let warnings = HostWarnings::default();
    let dispatch = Dispatch::new(warnings.clone());
    let _host = dispatcher::set_default(&dispatch);
    let mut request = request();
    request.telemetry_context = Some(faulting_context(&recorder, fault.clone()));
    let (gate, reached, release) = SourceGate::new();
    let (dropped_tx, dropped_rx) = oneshot::channel();
    let call = controlled_call(request, &observations, gate, None, dropped_tx, true);
    let mut steps = vec![json!({
        "name": "constructed", "call_delivery": "returned-handle",
        "span_start_attempt_count": fault.attempts.lock().unwrap().len(),
        "observed": snapshot(&observations), "request_spans": request_spans(&recorder),
    })];
    steps.extend(
        observe_terminal_gate(call, &observations, &recorder, reached, release, dropped_rx).await,
    );
    steps.push(json!({
        "name": "request-span-start-failure-observed", "fault_delivery": "typed-err",
        "fault_marker": REQUEST_SPAN_START_ERROR, "span_start_attempts": *fault.attempts.lock().unwrap(),
        "request_spans": request_spans(&recorder), "host_errors": *warnings.0.lock().unwrap(),
        "span_finish_calls": fault.finish_calls.load(Ordering::Relaxed),
        "span_backend_dropped": fault.span_backend_dropped.load(Ordering::Relaxed),
    }));
    json!({
        "schemaVersion": 2,
        "runtime": "rust",
        "scenario": REQUEST_SPAN_START_FAILURE,
        "steps": steps,
        "runtimeDetails": {
            "consumer_handle_dropped": false,
            "source_dropped": snapshot(&observations).source_dropped,
            "result_observed": true,
        },
    })
}

async fn observe_first_chunk_duration_failure() -> Value {
    let observations = Shared::default();
    let recorder = Recorder::default();
    let fault = Arc::new(TelemetryFault {
        site: FaultSite::DurationRecord {
            name: AI_FIRST_CHUNK_DURATION,
            marker: FIRST_CHUNK_DURATION_ERROR,
        },
        source: observations.clone(),
        ..Default::default()
    });
    let warnings = HostWarnings::default();
    let dispatch = Dispatch::new(warnings.clone());
    let _host = dispatcher::set_default(&dispatch);
    let mut request = request();
    request.telemetry_context = Some(faulting_context(&recorder, fault.clone()));
    let (terminal_gate, terminal_reached, release_terminal) = SourceGate::new();
    let (eof_gate, eof_reached, release_eof) = SourceGate::new();
    let (dropped_tx, dropped_rx) = oneshot::channel();
    let mut call = controlled_call(
        request,
        &observations,
        terminal_gate,
        Some(eof_gate),
        dropped_tx,
        false,
    );
    let spans = || request_span_snapshots(&recorder, true);
    let mut steps = vec![json!({
        "name": "constructed", "observed": snapshot(&observations), "request_spans": spans(),
    })];
    bounded(terminal_reached)
        .await
        .expect("source reaches the terminal gate");
    let terminal = {
        let mut next = std::pin::pin!(call.next());
        assert!(next.as_mut().now_or_never().is_none());
        steps.push(json!({
            "name": "terminal-gate-held", "observed": snapshot(&observations),
            "next_state": "pending", "request_spans": spans(),
        }));
        release_terminal
            .send(())
            .expect("source waits at the terminal gate");
        bounded(eof_reached)
            .await
            .expect("source reaches the EOF gate");
        let ready = next.as_mut().now_or_never().map(done_message);
        steps.push(json!({
            "name": "source-exit-checkpoint", "observed": snapshot(&observations), "eof_gate_reached": true,
            "next_state": if ready.is_some() { "ready" } else { "pending" },
            "terminal": ready.as_ref().map(terminal_snapshot), "request_spans": spans(),
        }));
        release_eof.send(()).expect("source waits at the EOF gate");
        bounded(dropped_rx).await.expect("source is released");
        match ready {
            Some(message) => message,
            None => done_message(bounded(next).await),
        }
    };
    assert!(bounded(call.next()).await.is_none());
    steps.push(json!({
        "name": "public-stream-completed", "terminal": terminal_snapshot(&terminal),
        "outer_eof": true, "observed": snapshot(&observations),
    }));
    let result = bounded(call.result()).await.expect("controlled completion");
    steps.push(json!({
        "name": "result-observed", "delivery": "ok", "stop": result.stop_reason.as_str(),
        "provider": result.provider, "model": result.model, "matches_terminal": terminal == result,
    }));
    let durations: Vec<_> = recorder
        .measurements()
        .iter()
        .filter(|entry| {
            matches!(
                entry.definition.name.as_str(),
                AI_FIRST_CHUNK_DURATION | AI_REQUEST_DURATION
            )
        })
        .map(|entry| json!({"name": entry.definition.name, "stop": entry.attributes.get("stop")}))
        .collect();
    steps.push(json!({
        "name": "first-chunk-duration-record-failure-observed", "fault_delivery": "typed-err",
        "fault_marker": FIRST_CHUNK_DURATION_ERROR, "record_attempts": *fault.attempts.lock().unwrap(),
        "request_spans": spans(), "duration_measurements": durations, "host_errors": *warnings.0.lock().unwrap(),
        "span_finish_calls": fault.finish_calls.load(Ordering::Relaxed),
        "span_backend_dropped": fault.span_backend_dropped.load(Ordering::Relaxed),
    }));
    json!({
        "schemaVersion": 2,
        "runtime": "rust",
        "scenario": FIRST_CHUNK_DURATION_FAILURE,
        "steps": steps,
        "runtimeDetails": {
            "consumer_handle_dropped": false,
            "source_dropped": snapshot(&observations).source_dropped,
            "result_observed": true,
        },
    })
}

async fn observe_request_duration_failure(fail_span_finish: bool) -> Value {
    let observations = Shared::default();
    let recorder = Recorder::default();
    let fault = Arc::new(TelemetryFault {
        site: FaultSite::DurationRecord {
            name: AI_REQUEST_DURATION,
            marker: REQUEST_DURATION_ERROR,
        },
        source: observations.clone(),
        fail_span_finish,
        ..Default::default()
    });
    let warnings = HostWarnings::default();
    let dispatch = Dispatch::new(warnings.clone());
    let _host = dispatcher::set_default(&dispatch);
    let mut request = request();
    request.telemetry_context = Some(faulting_context(&recorder, fault.clone()));
    let mut steps = observe_result_after_release(request, &observations, &recorder).await;
    let attempts = fault.attempts.lock().unwrap();
    let [attempt] = attempts.as_slice() else {
        panic!("the request duration record must fail once");
    };
    assert_eq!(
        attempt["usage_input"],
        json!(request_usage_input(&recorder))
    );
    let metric_names: Vec<_> = recorder
        .measurements()
        .into_iter()
        .map(|measurement| measurement.definition.name)
        .collect();
    steps.push(json!({
        "name": "request-duration-record-failure-observed", "fault_delivery": "typed-err",
        "fault_marker": REQUEST_DURATION_ERROR, "record_attempts": *attempts,
        "request_spans": request_span_snapshots(&recorder, true), "metric_names": metric_names,
        "host_errors": *warnings.0.lock().unwrap(), "span_finish_calls": fault.finish_calls.load(Ordering::Relaxed),
        "span_backend_dropped": fault.span_backend_dropped.load(Ordering::Relaxed),
    }));
    json!({
        "schemaVersion": 2,
        "runtime": "rust",
        "scenario": REQUEST_DURATION_FAILURE,
        "steps": steps,
        "runtimeDetails": {
            "consumer_handle_dropped": false,
            "source_dropped": snapshot(&observations).source_dropped,
            "result_observed": true,
        },
    })
}

struct GatedAuthFailure {
    observations: Shared,
    gate: Mutex<Option<(oneshot::Sender<()>, oneshot::Receiver<()>)>>,
}

impl AuthResolver for GatedAuthFailure {
    fn resolve(&self) -> BoxAuthFuture<'_, Result<ResolvedAuth, ProviderError>> {
        self.observations.lock().unwrap().auth_calls += 1;
        let (reached, release) = self.gate.lock().unwrap().take().expect("one resolution");
        Box::pin(async move {
            reached.send(()).expect("driver waits at auth gate");
            release.await.expect("driver releases auth explicitly");
            Err(ProviderError::AuthFailed(ProviderErrorSummary::sanitized(
                "controlled-auth-setup-failure",
            )))
        })
    }
}

async fn setup_failure(scenario: &str) -> Value {
    let auth_failure = scenario == "models-auth-setup-failure";
    let observations = Shared::default();
    let mut models = Models::new();
    let model = MockProvider::new("seed", vec![]).models()[0].clone();
    let (provider_id, phase, gate) = if auth_failure {
        let (reached_tx, reached_rx) = oneshot::channel();
        let (release_tx, release_rx) = oneshot::channel();
        let provider = ControlledProvider {
            models: vec![model.clone()],
            source: Mutex::new(None),
            observations: observations.clone(),
        };
        models
            .register_route(
                Box::new(provider),
                Arc::new(GatedAuthFailure {
                    observations: observations.clone(),
                    gate: Mutex::new(Some((reached_tx, release_rx))),
                }),
                AuthProvenanceSource::Static,
                CompatMetadata::default(),
            )
            .unwrap();
        (
            "lifecycle-differential",
            "authentication",
            Some((reached_rx, release_tx)),
        )
    } else {
        ("missing-provider", "route-lookup", None)
    };
    let model_id = model.id.clone();
    let record = ChatModelRecord::new(provider_id, model);
    let mut request = request();
    request.model = format!("{provider_id}:{model_id}");
    let mut call = models.stream_simple(record, request, Default::default());
    let at_construction = snapshot(&observations);
    let mut steps = vec![json!({
        "name": "constructed", "phase": phase, "provider": provider_id, "model": model_id,
        "auth_calls": at_construction.auth_calls, "dispatch_calls": at_construction.dispatch_calls,
    })];
    if let Some((reached, release)) = gate {
        bounded(reached).await.expect("auth reaches the gate");
        let at_gate = snapshot(&observations);
        assert_eq!(at_gate.auth_calls, 1);
        assert_eq!(at_gate.dispatch_calls, 0);
        steps.push(json!({
            "name": "auth-at-gate", "auth_calls": at_gate.auth_calls, "dispatch_calls": at_gate.dispatch_calls,
        }));
        release.send(()).expect("auth waits for release");
    }
    let failure = bounded(call.next())
        .await
        .expect("one terminal")
        .expect_err("setup fails");
    let cause = extract_diagnostic_error(failure.cause());
    assert_eq!(
        cause.name,
        if auth_failure { "AuthFailed" } else { "Config" }
    );
    let message = failure.message();
    steps.push(json!({
        "name": "failure-event", "delivery": "stream-item-err", "stop": message.stop_reason.as_str(),
        "message_provider": message.provider, "message_model": message.model,
        "error_message": message.error_message, "cause": cause.name,
        "cause_detail": format!("{:?}", failure.cause()),
    }));
    assert!(bounded(call.next()).await.is_none(), "no second terminal");
    let at_eof = snapshot(&observations);
    steps.push(json!({
        "name": "event-eof", "done": true, "auth_calls": at_eof.auth_calls, "dispatch_calls": at_eof.dispatch_calls,
    }));
    let result = bounded(call.result())
        .await
        .expect_err("same final failure");
    assert_eq!(failure, result);
    steps.push(json!({
        "name": "result-observed", "delivery": "typed-err", "stop": result.message().stop_reason.as_str(),
        "matches_event": failure == result,
    }));
    json!({
        "schemaVersion": 2,
        "runtime": "rust",
        "scenario": scenario,
        "steps": steps,
    })
}

const DEFERRED_RESULT_BEFORE_EOF: &str = "models-deferred-result-before-source-eof";
const DEFERRED_READY_BEFORE_EOF: &str = "models-deferred-ready-before-source-eof";
const DEFERRED_LOCAL_CANCEL_AFTER_DONE: &str = "models-deferred-local-cancel-after-done";
const DEFERRED_LOCAL_CANCEL_BEFORE_DONE: &str = "models-deferred-local-cancel-before-done";
const DEFERRED_STREAM_SOURCE_CLEANUP: &str = "models-deferred-stream-source-cleanup";

struct RecordingDeferredBackend {
    backend: ScriptedDeferredBackend,
    dispatches: Mutex<Vec<Value>>,
    fetch_gate: Mutex<Option<SourceGate>>,
    gated_source: Mutex<Option<ControlledSource>>,
    gated_cancel: Mutex<Option<tokio_util::sync::CancellationToken>>,
    remote_cancel_calls: AtomicUsize,
}

impl DeferredApi<()> for RecordingDeferredBackend {
    fn api_id(&self) -> &ApiId {
        self.backend.api_id()
    }

    fn provider_id(&self) -> &str {
        self.backend.provider_id()
    }

    fn model_id(&self) -> &str {
        self.backend.model_id()
    }

    fn default_options(&self) {}

    fn fetch_prepared(
        &self,
        handle: &DeferredHandle,
        options: &(),
        auth: ResolvedAuth,
        call: DeferredFetchCall,
    ) -> EventStream {
        self.dispatches
            .lock()
            .unwrap()
            .push(json!({"handle": handle, "wait_ms": call.wait_ms}));
        if let Some(source) = self.gated_source.lock().unwrap().take() {
            source.observations.lock().unwrap().dispatch_calls += 1;
            *self.gated_cancel.lock().unwrap() = Some(call.cancel.clone());
            return Box::pin(source);
        }
        let source = self.backend.fetch_prepared(handle, options, auth, call);
        if let Some(mut gate) = self.fetch_gate.lock().unwrap().take() {
            return Box::pin(
                futures_util::stream::once(async move {
                    std::future::poll_fn(|cx| gate.poll_release(cx)).await;
                    source
                })
                .flatten(),
            );
        }
        source
    }

    fn cancel_prepared<'a>(
        &'a self,
        handle: &'a DeferredHandle,
        options: &'a (),
        auth: ResolvedAuth,
        call: DeferredCancelCall,
    ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        self.remote_cancel_calls.fetch_add(1, Ordering::Relaxed);
        self.backend.cancel_prepared(handle, options, auth, call)
    }
}

fn register_recording_deferred_backend(
    resolver: Option<Arc<dyn AuthResolver>>,
) -> (Models, Arc<RecordingDeferredBackend>) {
    let backend = Arc::new(RecordingDeferredBackend {
        backend: ScriptedDeferredBackend::new("lifecycle-differential", "mock-model"),
        dispatches: Mutex::default(),
        fetch_gate: Mutex::default(),
        gated_source: Mutex::default(),
        gated_cancel: Mutex::default(),
        remote_cancel_calls: AtomicUsize::new(0),
    });
    let mut models = Models::new();
    models
        .register_deferred_api(
            backend.clone(),
            resolver.unwrap_or_else(|| {
                Arc::new(StaticAuthResolver::new(
                    AuthScheme::ApiKey,
                    secrecy::SecretString::from("in-memory-probe-key"),
                ))
            }),
            AuthProvenanceSource::Static,
        )
        .expect("register deferred backend");
    (models, backend)
}

fn deferred_fixture(
    pending_fetches: u64,
    resolver: Option<Arc<dyn AuthResolver>>,
) -> (
    Models,
    Arc<RecordingDeferredBackend>,
    DeferredHandle,
    AssistantMessage,
) {
    let (models, backend) = register_recording_deferred_backend(resolver);
    let partial = AssistantMessage {
        provider: backend.provider_id().into(),
        model: backend.model_id().into(),
        stop_reason: StopReason::Pending,
        ..base_assistant()
    };
    let mut ready = partial.clone();
    ready.stop_reason = StopReason::Stop;
    ready.content.push(AssistantContent::Text {
        text: "ready".into(),
        signature: None,
    });
    ready.usage =
        opi_ai::stream::Usage::reported(2, 3, 4, 5, None, None).with_total_tokens(Some(14));
    ready.usage.cost.input_cost = 0.25;
    ready.usage.cost.total = 0.25;
    let handle = backend.backend.submit(
        vec![
            AssistantStreamEvent::Start { partial },
            AssistantStreamEvent::Done {
                reason: StopReason::Stop,
                message: ready.clone(),
            },
        ],
        pending_fetches,
        Some(25),
    );
    (models, backend, handle, ready)
}

fn deferred_result_snapshot(result: &AssistantMessage) -> Value {
    json!({
        "stop": result.stop_reason.as_str(), "provider": result.provider, "model": result.model,
        "content": result.content, "deferred": result.deferred,
        "usage": {
            "input": result.usage.input_tokens, "output": result.usage.output_tokens,
            "cache_read": result.usage.cache_read_tokens, "cache_write": result.usage.cache_write_tokens,
            "cost": result.usage.cost.total_cost(),
        },
    })
}

fn deferred_poll_snapshot(
    recorder: &Recorder,
    parent_id: usize,
    span_offset: usize,
    metric_offset: usize,
) -> (Vec<Value>, Vec<Value>) {
    let spans = recorder.spans();
    let spans = &spans[span_offset..];
    let measurements = recorder
        .measurements()
        .into_iter()
        .skip(metric_offset)
        .map(|measurement| {
            let duration = measurement.definition.kind == MetricKind::Histogram;
            assert!(!duration || (measurement.value.is_finite() && measurement.value >= 0.0));
            let context = match measurement.span_id {
                None => "none",
                Some(id) if spans.first().is_some_and(|span| span.id == id) => "poll",
                Some(_) => "other",
            };
            json!({
                "name": measurement.definition.name, "kind": measurement.definition.kind,
                "unit": measurement.definition.unit, "attributes": measurement.attributes,
                "value": (!duration).then_some(measurement.value),
                "context": context,
            })
        })
        .collect();
    let spans = spans
        .iter()
        .map(|span| {
            json!({
                "name": span.name, "attributes": span.attributes, "status": span.status,
                "parent_is_caller": span.parent_id == Some(parent_id),
                "settled": span.end_sequence.is_some(),
                "events": span.events.iter().map(|event| &event.name).collect::<Vec<_>>(),
            })
        })
        .collect();
    (spans, measurements)
}

async fn observe_deferred_wait0() -> Value {
    let (models, backend, handle, _) = deferred_fixture(1, None);
    let recorder = Recorder::default();
    let parent = recorder
        .context()
        .start_span(SpanOptions::new("lifecycle-probe"))
        .expect("start caller span");
    let parent_id = recorder.spans()[0].id;
    let mut steps = Vec::new();
    for name in ["pending", "ready"] {
        let span_offset = recorder.spans().len();
        let metric_offset = recorder.measurements().len();
        let result = bounded(models.fetch_deferred(
            &handle,
            DeferredFetchCall {
                wait_ms: 0,
                telemetry_context: Some(parent.context()),
                ..Default::default()
            },
        ))
        .await
        .expect("controlled deferred fetch succeeds");
        let settled_at_result: Vec<_> = recorder.spans()[span_offset..]
            .iter()
            .map(|span| span.end_sequence.is_some())
            .collect();
        let metric_names_at_result: Vec<_> = recorder
            .measurements()
            .into_iter()
            .skip(metric_offset)
            .map(|measurement| measurement.definition.name)
            .collect();
        tokio::task::yield_now().await;

        let (spans, measurements) =
            deferred_poll_snapshot(&recorder, parent_id, span_offset, metric_offset);
        steps.push(json!({
            "name": name,
            "dispatches": std::mem::take(&mut *backend.dispatches.lock().unwrap()),
            "result": deferred_result_snapshot(&result),
            "settled_at_result": settled_at_result, "metric_names_at_result": metric_names_at_result,
            "spans": spans, "measurements": measurements,
        }));
    }
    assert_eq!(backend.backend.fetch_count(), 2);
    parent.finish(None).expect("finish caller span");
    json!({"schemaVersion": 2, "runtime": "rust", "scenario": DEFERRED_WAIT0, "steps": steps})
}

async fn observe_deferred_refetch(
    models: &Models,
    backend: &RecordingDeferredBackend,
    handle: &DeferredHandle,
) -> Value {
    let offset = backend.dispatches.lock().unwrap().len();
    let refetched = bounded(models.fetch_deferred(handle, DeferredFetchCall::default()))
        .await
        .expect("same handle stays fetchable");
    json!({
        "name": "handle-refetched", "result": deferred_result_snapshot(&refetched),
        "dispatches": &backend.dispatches.lock().unwrap()[offset..],
        "remote_cancel_calls": backend.remote_cancel_calls.load(Ordering::Relaxed),
    })
}

async fn observe_gated_deferred_fetch(scenario: &str, started: bool) -> Value {
    use futures_util::future::{Either, select};
    use tokio_util::sync::CancellationToken;

    let (models, backend, handle, mut ready) = deferred_fixture(0, None);
    let before_done = scenario == DEFERRED_LOCAL_CANCEL_BEFORE_DONE;
    let local_cancel = matches!(
        scenario,
        DEFERRED_LOCAL_CANCEL_BEFORE_DONE | DEFERRED_LOCAL_CANCEL_AFTER_DONE
    )
    .then(CancellationToken::new);
    if before_done {
        // This attempt exposes only the empty Start snapshot while Done is held.
        ready.content.clear();
    }
    let observations = Shared::default();
    let (terminal_gate, terminal_reached, release_terminal) = SourceGate::new();
    let (eof_gate, eof_reached, release_eof) = SourceGate::new();
    let (dropped_tx, dropped_rx) = oneshot::channel();
    *backend.gated_source.lock().unwrap() = Some(ControlledSource {
        observations: observations.clone(),
        terminal_gate,
        eof_gate: Some(eof_gate),
        dropped: Some(dropped_tx),
        message: ready,
        state: SourceState::Start,
    });
    let recorder = Recorder::default();
    let parent = recorder
        .context()
        .start_span(SpanOptions::new("lifecycle-probe"))
        .expect("start caller span");
    let parent_id = recorder.spans()[0].id;
    let observe = |name, result: Option<&AssistantMessage>| {
        let (spans, measurements) = deferred_poll_snapshot(&recorder, parent_id, 1, 0);
        let mut step = json!({
            "name": name, "observed": snapshot(&observations),
            "dispatches": &*backend.dispatches.lock().unwrap(),
            "result": result.map(deferred_result_snapshot),
            "spans": spans, "measurements": measurements,
        });
        if let Some(cancel) = &local_cancel {
            step["local_cancelled"] = json!(cancel.is_cancelled());
            step["backend_cancelled"] = json!(
                backend
                    .gated_cancel
                    .lock()
                    .unwrap()
                    .as_ref()
                    .map(CancellationToken::is_cancelled)
            );
            step["remote_cancel_calls"] =
                json!(backend.remote_cancel_calls.load(Ordering::Relaxed));
        }
        step
    };

    let call = DeferredFetchCall {
        wait_ms: 0,
        telemetry_context: Some(parent.context()),
        cancel: local_cancel.clone().unwrap_or_default(),
        ..Default::default()
    };
    let mut fetch = std::pin::pin!(async {
        if scenario == DEFERRED_READY_BEFORE_EOF {
            models.fetch_deferred_ready(&handle, call).await
        } else if started {
            models.stream_deferred(&handle, call).start().result().await
        } else {
            models.fetch_deferred(&handle, call).await
        }
    });
    let Either::Left((Ok(()), pending)) = bounded(select(terminal_reached, fetch.as_mut())).await
    else {
        panic!("the source must reach its terminal gate before the public result");
    };
    assert!(pending.now_or_never().is_none());
    let mut steps = vec![observe("terminal-gate-held", None)];
    if before_done {
        local_cancel.as_ref().expect("local cancel token").cancel();
        let failure = bounded(fetch)
            .await
            .expect_err("cancel the in-flight fetch");
        assert_eq!(failure.cause(), &ProviderError::Cancelled);
        let observe_failure = |name| {
            let mut step = observe(name, Some(failure.message()));
            step["result_delivery"] = json!("typed-err");
            step["typed_cause"] = json!("Cancelled");
            step["error_message"] = json!(failure.message().error_message);
            step
        };
        steps.push(observe_failure("local-cancel-observed"));
        bounded(dropped_rx).await.expect("source is released");
        steps.push(observe_failure("source-released"));
        steps.push(observe_deferred_refetch(&models, &backend, &handle).await);
        drop(release_terminal);
        parent.finish(None).expect("finish caller span");
        return json!({
            "schemaVersion": 2, "runtime": "rust", "scenario": scenario, "steps": steps,
        });
    }
    release_terminal
        .send(())
        .expect("release the terminal gate");

    // Keep polling the same fetch future while the source advances to the EOF gate.
    let early_result = match bounded(select(eof_reached, fetch.as_mut())).await {
        Either::Left((reached, pending)) => {
            reached.expect("source reaches the EOF gate");
            pending.now_or_never()
        }
        Either::Right((ready, reached)) => {
            bounded(reached).await.expect("source reaches the EOF gate");
            Some(ready)
        }
    }
    .transpose()
    .expect("controlled deferred fetch succeeds");
    steps.push(observe("eof-gate-held", early_result.as_ref()));
    let held_eof = if let Some(cancel) = &local_cancel {
        cancel.cancel();
        Some(release_eof)
    } else {
        release_eof.send(()).expect("release the EOF gate");
        None
    };

    let result = match early_result {
        Some(result) => {
            bounded(dropped_rx).await.expect("source is released");
            result
        }
        None => bounded(fetch)
            .await
            .expect("controlled deferred fetch succeeds"),
    };
    if local_cancel.is_some() {
        steps.push(observe("local-cancel-observed", Some(&result)));
    }
    steps.push(observe("source-released", Some(&result)));
    if local_cancel.is_some() {
        steps.push(observe_deferred_refetch(&models, &backend, &handle).await);
    }
    drop(held_eof);
    parent.finish(None).expect("finish caller span");
    json!({
        "schemaVersion": 2, "runtime": "rust", "scenario": scenario, "steps": steps,
    })
}

enum DeferredStreamMode {
    Direct,
    DirectStopAfterDone,
    DirectStopAfterPendingTail,
    DirectStopAfterEof,
    DirectCancelAfterDone,
    Started,
    DroppedConsumer,
    StartedAfterDone,
    ResumeAfterNext(DeferredTailResume),
}

#[derive(Clone, Copy)]
enum DeferredTailResume {
    Result,
    Started,
    Detached,
}

async fn observe_deferred_stream_mode(mode: DeferredStreamMode) -> Vec<Value> {
    let auth_calls = Arc::new(AtomicU32::new(0));
    let resolver = Arc::new(CountingAuthResolver::new(auth_calls.clone()));
    let (models, backend, handle, ready) = deferred_fixture(0, Some(resolver));
    let observations = Shared::default();
    let (terminal_gate, terminal_reached, release_terminal) = SourceGate::new();
    let (mut eof_gate, eof_reached, release_eof) = SourceGate::new();
    let eof_repolled =
        matches!(mode, DeferredStreamMode::ResumeAfterNext(_)).then(|| eof_gate.report_repoll());
    let (dropped_tx, mut dropped_rx) = oneshot::channel();
    *backend.gated_source.lock().unwrap() = Some(ControlledSource {
        observations: observations.clone(),
        terminal_gate,
        eof_gate: Some(eof_gate),
        dropped: Some(dropped_tx),
        message: ready,
        state: SourceState::Start,
    });
    let recorder = Recorder::default();
    let parent = recorder
        .context()
        .start_span(SpanOptions::new("lifecycle-probe"))
        .expect("start caller span");
    let parent_id = recorder.spans()[0].id;
    let finish_probe = matches!(
        mode,
        DeferredStreamMode::DirectStopAfterDone
            | DeferredStreamMode::DirectStopAfterPendingTail
            | DeferredStreamMode::DirectStopAfterEof
    )
    .then(|| {
        Arc::new(TelemetryFault {
            site: FaultSite::ObservePollFinish,
            source: observations.clone(),
            ..Default::default()
        })
    });
    let telemetry = match &finish_probe {
        Some(fault) => TelemetryContext::new(Arc::new(FaultingContext {
            inner: parent.context(),
            recorder: recorder.clone(),
            fault: fault.clone(),
        })),
        None => parent.context(),
    };
    let mut steps = Vec::new();
    let cancel = tokio_util::sync::CancellationToken::new();
    let prefix = match mode {
        DeferredStreamMode::DirectStopAfterDone => "stop-after-done-",
        DeferredStreamMode::DirectStopAfterPendingTail => "stop-after-pending-tail-",
        DeferredStreamMode::DirectStopAfterEof => "stop-after-public-eof-",
        DeferredStreamMode::DirectCancelAfterDone => "cancel-after-done-",
        DeferredStreamMode::ResumeAfterNext(DeferredTailResume::Result) => "resume-result-",
        DeferredStreamMode::ResumeAfterNext(DeferredTailResume::Started) => "resume-started-",
        DeferredStreamMode::ResumeAfterNext(DeferredTailResume::Detached) => "resume-detached-",
        _ => "",
    };
    let mut observe = |name, delivery: Value, eof_gate_reached| {
        let (spans, measurements) = deferred_poll_snapshot(&recorder, parent_id, 1, 0);
        let mut step = json!({
            "name": format!("{prefix}{name}"), "observed": snapshot(&observations), "eof_gate_reached": eof_gate_reached,
            "auth_calls": auth_calls.load(Ordering::SeqCst),
            "dispatches": &*backend.dispatches.lock().unwrap(), "delivery": delivery,
            "remote_cancel_calls": backend.remote_cancel_calls.load(Ordering::Relaxed),
            "spans": spans, "measurements": measurements,
        });
        if let Some(probe) = &finish_probe {
            step["poll_finish_source_dropped"] = json!(&*probe.attempts.lock().unwrap());
        }
        if matches!(
            mode,
            DeferredStreamMode::DirectCancelAfterDone | DeferredStreamMode::ResumeAfterNext(_)
        ) {
            step["local_cancelled"] = json!(cancel.is_cancelled());
            step["backend_cancelled"] = json!(
                backend
                    .gated_cancel
                    .lock()
                    .unwrap()
                    .as_ref()
                    .map(|token| token.is_cancelled())
            );
        }
        steps.push(step);
    };
    let done = |item| {
        let Some(Ok(AssistantStreamEvent::Done { reason, message })) = item else {
            panic!("expected one successful Done");
        };
        json!({"kind": "done", "reason": reason.as_str(), "message": deferred_result_snapshot(&message)})
    };
    let stream = models.stream_deferred(
        &handle,
        DeferredFetchCall {
            cancel: cancel.clone(),
            telemetry_context: Some(telemetry),
            ..Default::default()
        },
    );
    match mode {
        DeferredStreamMode::Started => {
            let mut stream = stream.start();
            bounded(terminal_reached)
                .await
                .expect("started source reaches terminal gate before consumption");
            assert!(matches!(
                bounded(stream.next()).await,
                Some(Ok(AssistantStreamEvent::Start { .. }))
            ));
            observe(
                "started-terminal-gate-held",
                json!({"kind": "start"}),
                !eof_reached.is_empty(),
            );
            release_terminal.send(()).expect("release terminal gate");
            bounded(eof_reached)
                .await
                .expect("started source reaches the EOF gate");
            let delivery = done(
                stream
                    .next()
                    .now_or_never()
                    .expect("started Done is available while source EOF is held"),
            );
            observe("started-done-observed", delivery, true);
            assert!(stream.next().now_or_never().is_none());
            observe(
                "started-next-eof-gate-held",
                json!({"kind": "pending"}),
                true,
            );
            release_eof.send(()).expect("release source EOF");
            assert!(bounded(stream.next()).await.is_none());
            observe("started-public-eof-observed", json!({"kind": "eof"}), true);
            let result = bounded(stream.result())
                .await
                .expect("started stream result");
            observe(
                "started-result-observed",
                json!({"kind": "result", "message": deferred_result_snapshot(&result)}),
                true,
            );
        }
        DeferredStreamMode::DroppedConsumer => {
            let stream = stream.start();
            bounded(terminal_reached)
                .await
                .expect("started source reaches the terminal gate");
            drop(stream);
            release_terminal
                .send(())
                .expect("release terminal gate after consumer Drop");
            bounded(eof_reached)
                .await
                .expect("detached source reaches the EOF gate");
            observe(
                "consumer-dropped-eof-gate-held",
                json!({"kind": "detached"}),
                true,
            );
            release_eof.send(()).expect("release source EOF");
            // The current-thread worker finishes the poll in the same turn as source Drop.
            bounded(dropped_rx)
                .await
                .expect("detached source is released");
            observe(
                "consumer-dropped-source-released",
                json!({"kind": "detached"}),
                true,
            );
        }
        DeferredStreamMode::StartedAfterDone => {
            let mut stream = stream;
            release_terminal.send(()).expect("release terminal gate");
            assert!(matches!(
                bounded(stream.next()).await,
                Some(Ok(AssistantStreamEvent::Start { .. }))
            ));
            let delivery = done(bounded(stream.next()).await);
            observe(
                "before-start-done-observed",
                delivery,
                !eof_reached.is_empty(),
            );

            let mut stream = stream.start();
            bounded(eof_reached)
                .await
                .expect("started collector drives its retained source");
            assert!(stream.next().now_or_never().is_none());
            observe(
                "cached-terminal-eof-gate-held",
                json!({"kind": "pending"}),
                true,
            );
            release_eof.send(()).expect("release source EOF");
            assert!(bounded(stream.next()).await.is_none());
            observe(
                "cached-terminal-public-eof-observed",
                json!({"kind": "eof"}),
                true,
            );
            let result = bounded(stream.result())
                .await
                .expect("cached terminal result");
            observe(
                "cached-terminal-result-observed",
                json!({"kind": "result", "message": deferred_result_snapshot(&result)}),
                true,
            );
        }
        DeferredStreamMode::Direct
        | DeferredStreamMode::DirectStopAfterDone
        | DeferredStreamMode::DirectStopAfterPendingTail
        | DeferredStreamMode::DirectStopAfterEof
        | DeferredStreamMode::DirectCancelAfterDone
        | DeferredStreamMode::ResumeAfterNext(_) => {
            let mut stream = stream;
            assert!(matches!(
                bounded(stream.next()).await,
                Some(Ok(AssistantStreamEvent::Start { .. }))
            ));
            assert!(stream.next().now_or_never().is_none());
            bounded(terminal_reached)
                .await
                .expect("source reaches terminal gate");
            observe(
                "terminal-gate-held",
                json!({"kind": "start"}),
                !eof_reached.is_empty(),
            );
            release_terminal.send(()).expect("release terminal gate");
            let delivery = done(bounded(stream.next()).await);
            observe("done-observed", delivery, !eof_reached.is_empty());
            if matches!(
                mode,
                DeferredStreamMode::DirectStopAfterDone
                    | DeferredStreamMode::DirectStopAfterPendingTail
            ) {
                let tail_polled = matches!(mode, DeferredStreamMode::DirectStopAfterPendingTail);
                if tail_polled {
                    assert!(futures_util::poll!(stream.next()).is_pending());
                    bounded(eof_reached)
                        .await
                        .expect("tail is held before Drop");
                    observe("next-eof-gate-held", json!({"kind": "pending"}), true);
                }
                drop(stream);
                assert_eq!(dropped_rx.try_recv(), Ok(()), "Drop releases the source");
                assert!(!cancel.is_cancelled());
                observe("consumer-stopped", json!({"kind": "stopped"}), tail_polled);
                assert!(
                    release_eof.send(()).is_err(),
                    "source EOF gate is gone after consumer Drop"
                );
                observe("after-eof-release", json!({"kind": "stopped"}), tail_polled);
            } else if let DeferredStreamMode::ResumeAfterNext(resume) = mode {
                observe_deferred_tail_resumption(
                    stream,
                    resume,
                    eof_reached,
                    eof_repolled.expect("resume control observes the next source poll"),
                    release_eof,
                    dropped_rx,
                    |name, delivery| observe(name, delivery, true),
                )
                .await;
            } else {
                let mut next_eof = stream.next();
                assert!(
                    futures_util::poll!(&mut next_eof).is_pending(),
                    "direct iteration waits for the retained source tail"
                );
                bounded(eof_reached)
                    .await
                    .expect("final next reaches source EOF gate");
                observe("next-eof-gate-held", json!({"kind": "pending"}), true);
                let held_eof = if matches!(mode, DeferredStreamMode::DirectCancelAfterDone) {
                    cancel.cancel();
                    Some(release_eof)
                } else {
                    release_eof.send(()).expect("release source EOF");
                    None
                };
                assert!(bounded(next_eof).await.is_none());
                observe("public-eof-observed", json!({"kind": "eof"}), true);
                if let Some(gate) = held_eof {
                    assert!(
                        gate.send(()).is_err(),
                        "local cancel releases the held source"
                    );
                }
                if matches!(mode, DeferredStreamMode::DirectStopAfterEof) {
                    drop(stream);
                    bounded(dropped_rx)
                        .await
                        .expect("source is already released");
                    observe("consumer-stopped", json!({"kind": "stopped"}), true);
                } else {
                    assert!(bounded(stream.next()).await.is_none());
                    let result = bounded(stream.result())
                        .await
                        .expect("cached direct result");
                    observe(
                        "source-released",
                        json!({"kind": "result", "message": deferred_result_snapshot(&result)}),
                        true,
                    );
                }
            }
        }
    }
    parent.finish(None).expect("finish caller span");
    steps
}

async fn observe_deferred_tail_resumption(
    mut stream: ChatStream,
    resume: DeferredTailResume,
    eof_reached: oneshot::Receiver<()>,
    eof_repolled: oneshot::Receiver<()>,
    release_eof: oneshot::Sender<()>,
    dropped: oneshot::Receiver<()>,
    mut observe: impl FnMut(&'static str, Value),
) {
    {
        let mut next = stream.next();
        // Completion needs to wake the next owner of the stream.
        let mut cx = Context::from_waker(futures_util::task::noop_waker_ref());
        assert!(Pin::new(&mut next).poll(&mut cx).is_pending());
    }
    bounded(eof_reached)
        .await
        .expect("abandoned next reaches the source EOF gate");
    assert!(eof_repolled.is_empty());
    observe("next-dropped-eof-gate-held", json!({"kind": "pending"}));
    let repolled = async {
        bounded(eof_repolled)
            .await
            .expect("new consumer polls the same EOF gate before release");
    };
    let result = match resume {
        DeferredTailResume::Result => {
            let result = tokio::spawn(stream.result());
            repolled.await;
            assert!(!result.is_finished());
            observe("resumed-eof-gate-held", json!({"kind": "pending"}));
            release_eof.send(()).expect("release source EOF");
            Some(
                bounded(result)
                    .await
                    .expect("result task joins")
                    .expect("cached direct result"),
            )
        }
        DeferredTailResume::Started => {
            let mut started = stream.start();
            repolled.await;
            assert!(started.next().now_or_never().is_none());
            observe("resumed-eof-gate-held", json!({"kind": "pending"}));
            release_eof.send(()).expect("release source EOF");
            assert!(bounded(started.next()).await.is_none());
            observe("public-eof-observed", json!({"kind": "eof"}));
            Some(
                bounded(started.result())
                    .await
                    .expect("cached started result"),
            )
        }
        DeferredTailResume::Detached => {
            drop(stream.start());
            repolled.await;
            observe("resumed-eof-gate-held", json!({"kind": "detached"}));
            release_eof.send(()).expect("release source EOF");
            None
        }
    };
    bounded(dropped).await.expect("resumed source is released");
    let delivery = match result {
        Some(message) => json!({"kind": "result", "message": deferred_result_snapshot(&message)}),
        None => json!({"kind": "detached"}),
    };
    observe("source-released", delivery);
}

async fn observe_deferred_stream_source_cleanup() -> Value {
    let mut steps = Vec::new();
    for mode in [
        DeferredStreamMode::Direct,
        DeferredStreamMode::Started,
        DeferredStreamMode::DroppedConsumer,
        DeferredStreamMode::StartedAfterDone,
        DeferredStreamMode::DirectStopAfterDone,
        DeferredStreamMode::DirectStopAfterPendingTail,
        DeferredStreamMode::DirectStopAfterEof,
        DeferredStreamMode::DirectCancelAfterDone,
        DeferredStreamMode::ResumeAfterNext(DeferredTailResume::Result),
        DeferredStreamMode::ResumeAfterNext(DeferredTailResume::Started),
        DeferredStreamMode::ResumeAfterNext(DeferredTailResume::Detached),
    ] {
        steps.extend(observe_deferred_stream_mode(mode).await);
    }
    json!({"schemaVersion": 3, "runtime": "rust", "scenario": DEFERRED_STREAM_SOURCE_CLEANUP, "steps": steps})
}

const DEFERRED_REMOTE_CANCEL: &str = "models-deferred-remote-cancel-then-fetch";

async fn observe_deferred_remote_cancel() -> Value {
    let (models, backend, handle, _) = deferred_fixture(1, None);
    let recorder = Recorder::default();
    let parent = recorder
        .context()
        .start_span(SpanOptions::new("lifecycle-probe"))
        .expect("start caller span");
    let parent_id = recorder.spans()[0].id;
    let (mut span_offset, mut metric_offset) = (1, 0);
    let mut observe = |name| {
        let (spans, measurements) =
            deferred_poll_snapshot(&recorder, parent_id, span_offset, metric_offset);
        span_offset += spans.len();
        metric_offset += measurements.len();
        let cancelled_handles = backend.backend.cancelled_handles();
        assert_eq!(
            backend.remote_cancel_calls.load(Ordering::Relaxed),
            cancelled_handles.len(),
        );
        json!({
            "name": name, "dispatches": std::mem::take(&mut *backend.dispatches.lock().unwrap()),
            "fetch_count": backend.backend.fetch_count(), "cancelled_handles": cancelled_handles,
            "spans": spans, "measurements": measurements,
        })
    };
    let fetch = || {
        models.fetch_deferred(
            &handle,
            DeferredFetchCall {
                wait_ms: 0,
                telemetry_context: Some(parent.context()),
                ..Default::default()
            },
        )
    };
    let pending = bounded(fetch()).await.expect("the first fetch is pending");
    let mut pending_step = observe("pending");
    pending_step["result"] = deferred_result_snapshot(&pending);

    bounded(models.cancel_deferred(
        &handle,
        DeferredCancelCall {
            telemetry_context: Some(parent.context()),
            ..Default::default()
        },
    ))
    .await
    .expect("explicit remote cancel succeeds");
    let mut cancelled_step = observe("cancelled");
    cancelled_step["result_delivery"] = json!("ok");
    cancelled_step["telemetry_forwarded"] =
        json!(backend.backend.cancel_telemetry_present() == [true]);

    let failure = bounded(fetch())
        .await
        .expect_err("a cancelled handle fails on explicit refetch");
    assert!(matches!(failure.cause(), ProviderError::Config(_)));
    let mut failed_step = observe("refetch-failed");
    failed_step["result"] = deferred_result_snapshot(failure.message());
    failed_step["result_delivery"] = json!("typed-err");
    failed_step["typed_cause"] = json!("Config");
    failed_step["error_message"] = json!(failure.message().error_message);

    parent.finish(None).expect("finish caller span");
    json!({
        "schemaVersion": 2, "runtime": "rust", "scenario": DEFERRED_REMOTE_CANCEL,
        "steps": [pending_step, cancelled_step, failed_step],
    })
}

const DEFERRED_AUTH_SETUP_FAILURE: &str = "models-deferred-auth-setup-failure";

async fn observe_deferred_auth_setup_failure() -> Value {
    use futures_util::future::{Either, select};

    let observations = Shared::default();
    let (reached_tx, reached) = oneshot::channel();
    let (release, release_rx) = oneshot::channel();
    let resolver = Arc::new(GatedAuthFailure {
        observations: observations.clone(),
        gate: Mutex::new(Some((reached_tx, release_rx))),
    });
    let (models, backend, handle, _) = deferred_fixture(1, Some(resolver));
    let recorder = Recorder::default();
    let parent = recorder
        .context()
        .start_span(SpanOptions::new("lifecycle-probe"))
        .expect("start caller span");
    let parent_id = recorder.spans()[0].id;
    let observe = |name| {
        let (spans, measurements) = deferred_poll_snapshot(&recorder, parent_id, 1, 0);
        json!({
            "name": name, "auth_calls": snapshot(&observations).auth_calls,
            "dispatches": &*backend.dispatches.lock().unwrap(),
            "remote_cancel_calls": backend.remote_cancel_calls.load(Ordering::Relaxed),
            "spans": spans, "measurements": measurements,
        })
    };
    let mut fetch = std::pin::pin!(models.fetch_deferred(
        &handle,
        DeferredFetchCall {
            wait_ms: 0,
            telemetry_context: Some(parent.context()),
            ..Default::default()
        },
    ));
    let Either::Left((Ok(()), pending)) = bounded(select(reached, fetch.as_mut())).await else {
        panic!("auth must reach its gate before the public result");
    };
    assert!(pending.now_or_never().is_none());
    let mut at_gate = observe("auth-at-gate");
    at_gate["handle"] = json!(handle);
    at_gate["wait_ms"] = json!(0);
    at_gate["result_state"] = json!("pending");
    release.send(()).expect("auth waits for release");

    let failure = bounded(fetch).await.expect_err("the auth resolver fails");
    assert!(matches!(failure.cause(), ProviderError::AuthFailed(_)));
    let mut failed = observe("fetch-failed");
    failed["result"] = deferred_result_snapshot(failure.message());
    failed["result_delivery"] = json!("typed-err");
    failed["typed_cause"] = json!("AuthFailed");
    failed["error_message"] = json!(failure.message().error_message);
    parent.finish(None).expect("finish caller span");
    json!({
        "schemaVersion": 2, "runtime": "rust", "scenario": DEFERRED_AUTH_SETUP_FAILURE,
        "steps": [at_gate, failed],
    })
}

const DEFERRED_POSITIVE_WAIT: &str = "models-deferred-positive-wait";

async fn observe_deferred_positive_wait() -> Value {
    use futures_util::future::{Either, select};

    let (models, backend, handle, _) = deferred_fixture(1, None);
    let recorder = Recorder::default();
    let parent = recorder
        .context()
        .start_span(SpanOptions::new("lifecycle-probe"))
        .expect("start caller span");
    let parent_id = recorder.spans()[0].id;
    let mut steps = Vec::new();
    for outcome in ["deferred", "ready"] {
        let (gate, reached, release) = SourceGate::new();
        assert!(backend.fetch_gate.lock().unwrap().replace(gate).is_none());
        let dispatch_offset = backend.dispatches.lock().unwrap().len();
        let span_offset = recorder.spans().len();
        let metric_offset = recorder.measurements().len();
        let observe = |name: &str, result: Option<&AssistantMessage>| {
            let (spans, measurements) =
                deferred_poll_snapshot(&recorder, parent_id, span_offset, metric_offset);
            json!({
                "name": name, "dispatches": &backend.dispatches.lock().unwrap()[dispatch_offset..],
                "remote_cancel_calls": backend.remote_cancel_calls.load(Ordering::Relaxed),
                "result": result.map(deferred_result_snapshot), "spans": spans, "measurements": measurements,
            })
        };
        let mut fetch = std::pin::pin!(models.fetch_deferred(
            &handle,
            DeferredFetchCall {
                wait_ms: 50,
                telemetry_context: Some(parent.context()),
                ..Default::default()
            },
        ));
        let Either::Left((Ok(()), pending)) = bounded(select(reached, fetch.as_mut())).await else {
            panic!("provider gate must be held before the public fetch returns");
        };
        assert!(pending.now_or_never().is_none());
        let held = format!("{outcome}-outcome-held");
        let mut at_gate = observe(&held, None);
        at_gate["result_state"] = json!("pending");
        steps.push(at_gate);
        release.send(()).expect("provider waits for release");

        let result = bounded(fetch).await.expect("controlled fetch succeeds");
        tokio::task::yield_now().await;
        steps.push(observe(outcome, Some(&result)));
    }
    assert_eq!(backend.backend.fetch_count(), 2);
    parent.finish(None).expect("finish caller span");
    json!({
        "schemaVersion": 2, "runtime": "rust", "scenario": DEFERRED_POSITIVE_WAIT,
        "steps": steps,
    })
}

const DEFERRED_HANDLE_EXPIRY: &str = "models-deferred-handle-expiry";
const DEFERRED_HANDLE_ROUTE_MISMATCH: &str = "models-deferred-handle-route-mismatch";
const DEFERRED_UNKNOWN_JOB_ID: &str = "models-deferred-unknown-job-id";

async fn observe_deferred_handle_checks(scenario: &str) -> Value {
    let expires_at_ms = (scenario == DEFERRED_HANDLE_EXPIRY).then_some(1_700_000_000_000);
    let auth_calls = Arc::new(AtomicU32::new(0));
    let resolver = Arc::new(CountingAuthResolver::new(auth_calls.clone()));
    let (models, backend) = register_recording_deferred_backend(Some(resolver));
    let partial = AssistantMessage {
        provider: backend.provider_id().into(),
        model: backend.model_id().into(),
        stop_reason: StopReason::Pending,
        // Matches the compiled Faux estimate for an empty prompt and "ready".
        usage: opi_ai::stream::Usage::reported(0, 2, 0, 0, None, None).with_total_tokens(Some(2)),
        ..base_assistant()
    };
    let text_started = AssistantMessage {
        content: vec![AssistantContent::Text {
            text: String::new(),
            signature: None,
        }],
        ..partial.clone()
    };
    let text_ended = AssistantMessage {
        content: vec![AssistantContent::Text {
            text: "ready".into(),
            signature: None,
        }],
        ..partial.clone()
    };
    let ready = AssistantMessage {
        stop_reason: StopReason::Stop,
        ..text_ended.clone()
    };
    let original = backend.backend.submit_with_expiry(
        vec![
            AssistantStreamEvent::Start { partial },
            AssistantStreamEvent::TextStart {
                content_index: 0,
                partial: text_started,
            },
            AssistantStreamEvent::TextDelta {
                content_index: 0,
                delta: "ready".into(),
                partial: text_ended.clone(),
            },
            AssistantStreamEvent::TextEnd {
                content_index: 0,
                content: "ready".into(),
                partial: text_ended,
            },
            AssistantStreamEvent::Done {
                reason: StopReason::Stop,
                message: ready,
            },
        ],
        0,
        Some(25),
        expires_at_ms,
    );
    let attempts = if let Some(expiry) = expires_at_ms {
        vec![
            ("before-expiry", original.clone(), Some(expiry - 1)),
            ("at-expiry", original.clone(), Some(expiry)),
            ("without-sdk-clock", original, None),
        ]
    } else if scenario == DEFERRED_UNKNOWN_JOB_ID {
        let unknown = DeferredHandle {
            id: "deferred-missing".into(),
            ..original.clone()
        };
        vec![
            ("unknown-job-id", unknown, None),
            ("original-handle-refetch", original, None),
        ]
    } else {
        let other_provider = DeferredHandle {
            provider: "other-provider".into(),
            ..original.clone()
        };
        let other_model = DeferredHandle {
            model_id: "other-model".into(),
            ..original.clone()
        };
        let other_api = DeferredHandle {
            api: "other-api".into(),
            ..original.clone()
        };
        vec![
            ("provider-mismatch", other_provider, None),
            ("model-mismatch", other_model, None),
            ("api-mismatch", other_api, None),
            ("original-handle-refetch", original, None),
        ]
    };
    let recorder = Recorder::default();
    let parent = recorder
        .context()
        .start_span(SpanOptions::new("lifecycle-probe"))
        .expect("start caller span");
    let parent_id = recorder.spans()[0].id;
    let mut steps = Vec::new();
    for (name, handle, now_ms) in attempts {
        let auth_offset = auth_calls.load(Ordering::SeqCst);
        let fetch_offset = backend.backend.fetch_count();
        let handle_validation = expires_at_ms.is_none().then(|| {
            let result = models.validate_deferred_handle(&handle, None);
            assert_eq!(auth_calls.load(Ordering::SeqCst), auth_offset);
            assert_eq!(backend.backend.fetch_count(), fetch_offset);
            match result {
                Ok(()) => "ok",
                Err(DeferredError::Unsupported { .. }) => "Unsupported",
                Err(DeferredError::RouteMismatch(_)) => "RouteMismatch",
                Err(error) => panic!("unexpected route validation result: {error}"),
            }
        });
        let span_offset = recorder.spans().len();
        let metric_offset = recorder.measurements().len();
        let result = bounded(models.fetch_deferred(
            &handle,
            DeferredFetchCall {
                now_ms,
                telemetry_context: Some(parent.context()),
                ..Default::default()
            },
        ))
        .await;
        let (message, delivery, cause) = match &result {
            Ok(message) => (message, "ok", None),
            Err(failure) => (
                failure.message(),
                "typed-err",
                Some(extract_diagnostic_error(failure.cause()).name),
            ),
        };
        let (spans, measurements) =
            deferred_poll_snapshot(&recorder, parent_id, span_offset, metric_offset);
        let mut step = json!({
            "name": name, "request_handle": handle, "sdk_now_ms": now_ms,
            "auth_calls": auth_calls.load(Ordering::SeqCst) - auth_offset,
            "fetch_count": backend.backend.fetch_count() - fetch_offset,
            "dispatches": std::mem::take(&mut *backend.dispatches.lock().unwrap()),
            "remote_cancel_calls": backend.remote_cancel_calls.load(Ordering::Relaxed),
            "result": deferred_result_snapshot(message), "result_delivery": delivery,
            "typed_cause": cause, "error_message": message.error_message,
            "spans": spans, "measurements": measurements,
        });
        if let Some(validation) = handle_validation {
            step["handle_validation"] = json!(validation);
        }
        steps.push(step);
    }
    parent.finish(None).expect("finish caller span");
    json!({
        "schemaVersion": 2, "runtime": "rust", "scenario": scenario,
        "steps": steps,
    })
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let scenario = std::env::args().nth(1).unwrap_or_default();
    let trace = match scenario.as_str() {
        "models-no-consumption" | "models-stop-consumption" | RESULT_BEFORE_EOF => {
            completion(&scenario, false).await
        }
        TERMINAL_AND_SPAN_BEFORE_EOF => observe_terminal_and_span().await,
        RESULT_ATTRIBUTE_WRITE_FAILURE => observe_result_attribute_failure(false).await,
        FIRST_CHUNK_WRITE_FAILURE => observe_first_chunk_failure().await,
        HISTOGRAM_CREATION_FAILURE => observe_histogram_creation_failure().await,
        REQUEST_SPAN_START_FAILURE => observe_request_span_start_failure().await,
        FIRST_CHUNK_DURATION_FAILURE => observe_first_chunk_duration_failure().await,
        REQUEST_DURATION_FAILURE => observe_request_duration_failure(false).await,
        "models-missing-provider" | "models-auth-setup-failure" => setup_failure(&scenario).await,
        DEFERRED_WAIT0 => observe_deferred_wait0().await,
        DEFERRED_REMOTE_CANCEL => observe_deferred_remote_cancel().await,
        DEFERRED_AUTH_SETUP_FAILURE => observe_deferred_auth_setup_failure().await,
        DEFERRED_POSITIVE_WAIT => observe_deferred_positive_wait().await,
        DEFERRED_STREAM_SOURCE_CLEANUP => observe_deferred_stream_source_cleanup().await,
        DEFERRED_HANDLE_EXPIRY | DEFERRED_HANDLE_ROUTE_MISMATCH | DEFERRED_UNKNOWN_JOB_ID => {
            observe_deferred_handle_checks(&scenario).await
        }
        DEFERRED_RESULT_BEFORE_EOF
        | DEFERRED_READY_BEFORE_EOF
        | DEFERRED_LOCAL_CANCEL_AFTER_DONE
        | DEFERRED_LOCAL_CANCEL_BEFORE_DONE => observe_gated_deferred_fetch(&scenario, false).await,
        other => {
            eprintln!("unknown lifecycle scenario: {other}");
            std::process::exit(2);
        }
    };
    println!(
        "{}",
        serde_json::to_string(&trace).expect("trace serializes")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn dropping_unpolled_consumer_does_not_stop_gated_source() {
        let trace = completion("models-no-consumption", true).await;
        let last = trace["steps"].as_array().unwrap().last().unwrap();
        assert_eq!(last["name"], "source-released");
        assert_eq!(last["observed"]["source_eof"], true);
        assert_eq!(last["observed"]["terminal_count"], 1);
        assert_eq!(trace["runtimeDetails"]["source_dropped"], true);
        assert_eq!(trace["runtimeDetails"]["result_observed"], false);
    }

    #[tokio::test]
    async fn dropping_after_first_event_does_not_stop_gated_source() {
        let trace = completion("models-stop-consumption", true).await;
        let last = trace["steps"].as_array().unwrap().last().unwrap();
        assert_eq!(last["name"], "source-released");
        assert_eq!(last["observed"]["source_eof"], true);
        assert_eq!(last["observed"]["terminal_count"], 1);
        assert_eq!(trace["runtimeDetails"]["source_dropped"], true);
        assert_eq!(trace["runtimeDetails"]["consumer_handle_dropped"], true);
        assert_eq!(trace["runtimeDetails"]["result_observed"], false);
    }

    #[tokio::test]
    async fn started_deferred_fetch_observes_local_cancellation_before_and_after_done() {
        for (scenario, stop) in [
            (DEFERRED_LOCAL_CANCEL_BEFORE_DONE, "aborted"),
            (DEFERRED_LOCAL_CANCEL_AFTER_DONE, "stop"),
        ] {
            let trace = observe_gated_deferred_fetch(scenario, true).await;
            let released = trace["steps"]
                .as_array()
                .unwrap()
                .iter()
                .find(|step| step["name"] == "source-released")
                .unwrap();
            assert_eq!(released["result"]["stop"], stop);
            assert_eq!(released["local_cancelled"], true);
            assert_eq!(released["backend_cancelled"], true);
            assert_eq!(released["remote_cancel_calls"], 0);
            assert_eq!(released["observed"]["source_eof"], false);
            assert_eq!(released["observed"]["source_dropped"], true);
            assert_eq!(released["spans"][0]["settled"], true);
        }
    }

    #[tokio::test]
    async fn result_telemetry_and_span_finish_failures_preserve_the_business_result() {
        for (trace, attempts, marker) in [
            (
                observe_result_attribute_failure(true).await,
                "write_attempts",
                RESULT_ATTRIBUTE_WRITE_ERROR,
            ),
            (
                observe_request_duration_failure(true).await,
                "record_attempts",
                REQUEST_DURATION_ERROR,
            ),
        ] {
            let steps = trace["steps"].as_array().unwrap();
            let fault = steps.last().unwrap();
            let result = &steps[steps.len() - 2];
            assert_eq!(result["name"], "result-observed");
            assert_eq!(result["matches_terminal"], true);
            assert_eq!(result["stop"], "stop");
            assert_eq!(fault[attempts].as_array().unwrap().len(), 1, "{marker}");
            assert_eq!(fault["span_finish_calls"], 1, "{marker}");
            assert_eq!(fault["span_backend_dropped"], true);
            assert_eq!(fault["request_spans"][0]["settled"], false);
            assert_eq!(
                fault["host_errors"],
                json!([
                    {"channel": "tracing", "level": "WARN", "target": "opi_ai::telemetry", "fields": {
                        "message": "AI telemetry observer failed to finish", "error": marker,
                    }},
                    {"channel": "tracing", "level": "WARN", "target": "opi_ai::telemetry", "fields": {
                        "message": "AI telemetry observer failed while closing after an error", "error": SPAN_FINISH_ERROR,
                    }},
                ])
            );
        }
    }

    #[test]
    fn manual_observer_can_retry_after_a_final_instrument_creation_failure() {
        use opi_ai::telemetry::{AI_COST, AI_POLL_SPAN, AiRequestAttributes, AiRequestTelemetry};
        use std::error::Error as _;

        const ERROR: &str = "controlled-cost-counter-creation-failure";
        let recorder = Recorder::default();
        let fault = Arc::new(TelemetryFault {
            site: FaultSite::InstrumentCreation {
                name: AI_COST,
                kind: MetricKind::Counter,
                marker: ERROR,
            },
            ..Default::default()
        });
        let context = faulting_context(&recorder, fault.clone());
        let start = || {
            AiRequestTelemetry::start(
                Some(&context),
                AiRequestAttributes::new("lifecycle-differential", "mock-model", "mock")
                    .as_poll(true),
            )
        };
        let error = start().unwrap_err();
        let cause = error
            .source()
            .unwrap()
            .downcast_ref::<std::io::Error>()
            .unwrap();
        assert_eq!(cause.to_string(), ERROR);
        assert_eq!(fault.attempts.lock().unwrap().len(), 1);
        assert_eq!(
            recorder.spans().len(),
            0,
            "failed instrument creation must leave no poll span"
        );

        let mut telemetry = start().unwrap().unwrap();
        let mut message = base_assistant();
        message.stop_reason = StopReason::Stop;
        telemetry.finish(Some(&message)).unwrap();
        assert_eq!(fault.attempts.lock().unwrap().len(), 2);
        let spans = recorder.spans();
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].name, AI_POLL_SPAN);
        assert!(spans[0].end_sequence.is_some());
    }

    #[test]
    fn manual_observer_can_retry_after_a_result_telemetry_failure() {
        use opi_ai::telemetry::{AiRequestAttributes, AiRequestTelemetry};
        use std::error::Error as _;

        let mut message = base_assistant();
        message.usage.input_tokens = 7;
        for (site, marker) in [
            (FaultSite::ResultAttributes, RESULT_ATTRIBUTE_WRITE_ERROR),
            (
                FaultSite::DurationRecord {
                    name: AI_REQUEST_DURATION,
                    marker: REQUEST_DURATION_ERROR,
                },
                REQUEST_DURATION_ERROR,
            ),
        ] {
            let recorder = Recorder::default();
            let fault = Arc::new(TelemetryFault {
                site,
                ..Default::default()
            });
            let context = faulting_context(&recorder, fault.clone());
            let mut telemetry = AiRequestTelemetry::start(
                Some(&context),
                AiRequestAttributes::new("lifecycle-differential", "mock-model", "mock"),
            )
            .unwrap()
            .unwrap();

            let error = telemetry.finish(Some(&message)).unwrap_err();
            let cause = error
                .source()
                .unwrap()
                .downcast_ref::<std::io::Error>()
                .unwrap();
            assert_eq!(cause.to_string(), marker);
            assert_eq!(fault.finish_calls.load(Ordering::Relaxed), 0);
            assert!(recorder.spans()[0].end_sequence.is_none());

            telemetry.finish(Some(&message)).unwrap();
            telemetry.finish(Some(&message)).unwrap();
            assert_eq!(fault.attempts.lock().unwrap().len(), 2);
            assert_eq!(fault.finish_calls.load(Ordering::Relaxed), 1);
            let spans = recorder.spans();
            assert!(spans[0].end_sequence.is_some());
            assert_eq!(
                spans[0].attributes["stop"],
                AttributeValue::String("stop".into())
            );
            assert_eq!(
                spans[0].attributes["usage.input"],
                AttributeValue::Number(7.0)
            );
        }
    }
}
