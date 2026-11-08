//! Controlled lifecycle observations, not a live-provider or full parity probe.
//! The source cannot emit its terminal until the driver releases a named gate.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use futures_util::{Stream, StreamExt};
use opi_ai::provider::{EventStream, ProviderError, Request};
use opi_ai::stream::{AssistantStreamEvent, StopReason};
use opi_ai::test_support::{MockProvider, base_assistant, single_route_collection};
use opi_ai::{ChatModelRecord, ModelInfo, Models, Provider};
use serde::Serialize;
use serde_json::{Value, json};
use tokio::sync::oneshot;

#[derive(Clone, Default, Serialize)]
struct Observations {
    setup_calls: usize,
    source_started: bool,
    terminal_count: usize,
    source_eof: bool,
    source_dropped: bool,
}

type Shared = Arc<Mutex<Observations>>;

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
        self.observations.lock().unwrap().setup_calls += 1;
        Box::pin(self.source.lock().unwrap().take().expect("one request"))
    }
}

struct ControlledSource {
    observations: Shared,
    reached: Option<oneshot::Sender<()>>,
    release: oneshot::Receiver<()>,
    dropped: Option<oneshot::Sender<()>>,
    state: SourceState,
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
                let mut partial = base_assistant();
                partial.stop_reason = StopReason::Pending;
                Poll::Ready(Some(Ok(AssistantStreamEvent::Start { partial })))
            }
            SourceState::AtGate => {
                if let Some(reached) = self.reached.take() {
                    reached.send(()).expect("driver waits at source gate");
                }
                let Poll::Ready(released) = Pin::new(&mut self.release).poll(cx) else {
                    return Poll::Pending;
                };
                released.expect("driver releases source explicitly");
                self.observations.lock().unwrap().terminal_count += 1;
                self.state = SourceState::Tail;
                Poll::Ready(Some(Ok(AssistantStreamEvent::Done {
                    reason: StopReason::Stop,
                    message: base_assistant(),
                })))
            }
            SourceState::Tail | SourceState::End => {
                self.observations.lock().unwrap().source_eof = true;
                self.state = SourceState::End;
                Poll::Ready(None)
            }
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

async fn completion(consume_one: bool, drop_consumer: bool) -> Value {
    let observations = Shared::default();
    let (reached_tx, reached_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    let (dropped_tx, dropped_rx) = oneshot::channel();
    let provider = ControlledProvider {
        models: MockProvider::new("seed", vec![]).models().to_vec(),
        source: Mutex::new(Some(ControlledSource {
            observations: observations.clone(),
            reached: Some(reached_tx),
            release: release_rx,
            dropped: Some(dropped_tx),
            state: SourceState::Start,
        })),
        observations: observations.clone(),
    };
    let models = single_route_collection(Box::new(provider));
    let model = models
        .get_model("lifecycle-differential", "mock-model")
        .unwrap();
    let record = ChatModelRecord::new("lifecycle-differential", model.clone());
    let mut call = Some(models.stream_simple(record, request(), Default::default()));
    let mut steps = vec![step("constructed", &observations)];
    bounded(reached_rx).await.unwrap();
    steps.push(step("source-at-gate", &observations));

    if consume_one {
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
    steps.push(step("before-source-release", &observations));
    release_tx.send(()).unwrap();
    bounded(dropped_rx).await.unwrap();
    let after = snapshot(&observations);
    assert_eq!(after.setup_calls, 1);
    assert_eq!(after.terminal_count, 1);
    assert!(after.source_dropped);
    // Record EOF independently from Drop. A wrapper may release a source
    // without polling its tail; the differential must expose that mismatch.
    steps.push(step("source-released", &observations));

    // The retained-handle cases observe the real public result, after source
    // completion without consumer polling. Drop-only tests cannot observe a
    // discarded result and deliberately do not manufacture one.
    let result = match call {
        Some(call) => Some(bounded(call.result()).await.expect("controlled completion")),
        None => None,
    };
    let terminal = result.as_ref().map(|message| message.stop_reason.as_str());
    if result.is_some() {
        steps.push(json!({"name": "result-observed", "stop": terminal}));
    }
    json!({
        "schemaVersion": 2,
        "runtime": "rust",
        "scenario": if consume_one { "lazy-stop-consumption" } else { "lazy-no-consumption" },
        "steps": steps,
        "shared": {
            "setup_calls": after.setup_calls,
            "source_started": after.source_started,
            "terminal_before_release": before_release.terminal_count,
            "source_terminal_count": after.terminal_count,
            "source_completed": after.source_eof,
            "consumer_polled": consume_one,
            "terminal_kind": terminal,
        },
        "runtimeDetails": {
            "consumer_handle_dropped": drop_consumer,
            "source_dropped": after.source_dropped,
            "result_observed": result.is_some(),
        },
    })
}

async fn setup_failure() -> Value {
    let models = Models::new();
    let model = MockProvider::new("seed", vec![]).models()[0].clone();
    let record = ChatModelRecord::new("missing-provider", model);
    let mut call = models.stream_simple(record, request(), Default::default());
    let failure = bounded(call.next())
        .await
        .expect("one terminal")
        .expect_err("setup fails");
    assert!(bounded(call.next()).await.is_none(), "no second terminal");
    let result = bounded(call.result())
        .await
        .expect_err("same final failure");
    assert_eq!(failure, result);
    json!({
        "schemaVersion": 2,
        "runtime": "rust",
        "scenario": "lazy-setup-failure",
        "steps": [
            {"name": "constructed"},
            {"name": "failure-event", "cause": format!("{:?}", failure.cause())},
            {"name": "event-eof"},
            {"name": "result-observed", "stop": result.message().stop_reason.as_str()},
        ],
        "shared": {
            "terminal_kind": result.message().stop_reason.as_str(),
            "terminal_count": 1,
            "result_matches_event": failure == result,
        },
        "runtimeDetails": {
            "cause": format!("{:?}", result.cause()),
            "error_message": result.message().error_message,
            "failure_input": "unregistered provider record",
        },
    })
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let scenario = std::env::args().nth(1).unwrap_or_default();
    let trace = match scenario.as_str() {
        "lazy-no-consumption" => completion(false, false).await,
        "lazy-stop-consumption" => completion(true, false).await,
        "lazy-setup-failure" => setup_failure().await,
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
        let trace = completion(false, true).await;
        assert_eq!(trace["shared"]["source_completed"], true);
        assert_eq!(trace["shared"]["source_terminal_count"], 1);
        assert_eq!(trace["runtimeDetails"]["source_dropped"], true);
        assert!(trace["shared"]["terminal_kind"].is_null());
    }

    #[tokio::test]
    async fn dropping_after_first_event_does_not_stop_gated_source() {
        let trace = completion(true, true).await;
        assert_eq!(trace["shared"]["source_completed"], true);
        assert_eq!(trace["shared"]["source_terminal_count"], 1);
        assert_eq!(trace["runtimeDetails"]["source_dropped"], true);
        assert_eq!(trace["runtimeDetails"]["consumer_handle_dropped"], true);
        assert!(trace["shared"]["terminal_kind"].is_null());
    }
}
