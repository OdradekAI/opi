//! Shared recording subscriber, ambient scope, canaries, and marker stream fixtures.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context as TaskContext, Poll};

use futures_core::Stream;
use opi_ai::api::contract::ChatApi;
use opi_ai::auth::{AuthResolver, AuthScheme, ResolvedAuth};
use opi_ai::models::ChatModelRecord;
use opi_ai::provider::{EventStream, ProviderError, Request, ThinkingConfig};
use opi_ai::stream::AssistantStreamEvent;
use opi_ai::telemetry::TelemetryContext;
use opi_ai::transcript::{Context, normalize_context};
use opi_ai::types::ApiId;
use opi_ai::{ModelCapabilities, ModelInfo, WireApi};
use tracing::Level;
use tracing_core::field::{Field, Visit};
use tracing_core::span::{Attributes, Current, Id, Record};
use tracing_core::{Dispatch, Event, Metadata, Subscriber, dispatcher};

#[derive(Default)]
pub(super) struct Recording {
    next: u64,
    pub(super) spans: Vec<(&'static str, Option<u64>)>,
    pub(super) events: Vec<(&'static str, Vec<(String, String)>)>,
    metadata: std::collections::HashMap<u64, &'static Metadata<'static>>,
    pub(super) entered: std::collections::HashMap<std::thread::ThreadId, Vec<Id>>,
}

struct Recorder(Arc<Mutex<Recording>>);

impl Subscriber for Recorder {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, attributes: &Attributes<'_>) -> Id {
        let mut state = self.0.lock().unwrap();
        state.next += 1;
        let id = Id::from_u64(state.next);
        let parent = attributes.parent().cloned().or_else(|| {
            attributes
                .is_contextual()
                .then(|| {
                    state
                        .entered
                        .get(&std::thread::current().id())
                        .and_then(|stack| stack.last())
                        .cloned()
                })
                .flatten()
        });
        state
            .spans
            .push((attributes.metadata().name(), parent.map(|id| id.into_u64())));
        state.metadata.insert(id.into_u64(), attributes.metadata());
        id
    }
    fn record(&self, _: &Id, _: &Record<'_>) {}
    fn record_follows_from(&self, _: &Id, _: &Id) {}
    fn event(&self, event: &Event<'_>) {
        let mut fields = Vec::new();
        let mut visitor = FieldCollector(&mut fields);
        event.record(&mut visitor);
        self.0
            .lock()
            .unwrap()
            .events
            .push((event.metadata().name(), fields));
    }
    fn enter(&self, id: &Id) {
        self.0
            .lock()
            .unwrap()
            .entered
            .entry(std::thread::current().id())
            .or_default()
            .push(id.clone());
    }
    fn exit(&self, _: &Id) {
        self.0
            .lock()
            .unwrap()
            .entered
            .get_mut(&std::thread::current().id())
            .unwrap()
            .pop();
    }
    fn current_span(&self) -> Current {
        let state = self.0.lock().unwrap();
        match state
            .entered
            .get(&std::thread::current().id())
            .and_then(|stack| stack.last())
        {
            Some(id) => Current::new(id.clone(), state.metadata[&id.into_u64()]),
            None => Current::none(),
        }
    }
}

struct FieldCollector<'a>(&'a mut Vec<(String, String)>);

impl Visit for FieldCollector<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0.push((field.name().to_owned(), format!("{value:?}")));
    }
    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.push((field.name().to_owned(), value.to_owned()));
    }
}

pub(super) fn recorder() -> (Arc<Mutex<Recording>>, Dispatch) {
    let records = Arc::new(Mutex::new(Recording::default()));
    let dispatch = Dispatch::new(Recorder(records.clone()));
    (records, dispatch)
}

/// The prompt and credential canaries used by the serialization/field
/// negatives: neither may appear in any recorded telemetry value.
pub(super) const PROMPT_CANARY: &str = "CANARY-PROMPT-telemetry-operations";
pub(super) const SECRET_CANARY: &str = "CANARY-SECRET-telemetry-operations";

/// Drive a future to completion with `ambient` installed as the poll-time
/// default dispatch, mirroring a host that observes under its own subscriber.
pub(super) async fn await_under_ambient<T>(
    ambient: &Dispatch,
    future: impl Future<Output = T>,
) -> T {
    let mut future = Box::pin(future);
    futures_util::future::poll_fn(|cx| {
        dispatcher::with_default(ambient, || Future::poll(future.as_mut(), cx))
    })
    .await
}

// ---------------------------------------------------------------------------
// Marker chat adapter: a custom ChatApi whose EventStream runs its poll and
// drop sections under the caller's frozen context, exactly like the real
// wire adapters instrument their producer tasks.
// ---------------------------------------------------------------------------

/// Counts adapter dispatches and records whether each request carried an
/// explicit telemetry carrier.
#[derive(Default)]
pub(super) struct AdapterObservations {
    pub(super) dispatches: AtomicUsize,
    pub(super) telemetry_present: Mutex<Vec<bool>>,
    pub(super) pending_first_poll: bool,
    pub(super) polls: AtomicUsize,
    pub(super) events: AtomicUsize,
    pub(super) drops: AtomicUsize,
}

pub(super) struct MarkerChatApi {
    pub(super) observed: Arc<AdapterObservations>,
    /// Which terminal the scripted stream delivers: plain text or a
    /// deferred-response terminal carrying a handle for the marker deferred
    /// backend.
    pub(super) terminal: MarkerTerminal,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum MarkerTerminal {
    Text,
    Deferred,
}

pub(super) fn deferred_handle() -> opi_ai::types::deferred::DeferredHandle {
    opi_ai::types::deferred::DeferredHandle {
        provider: "telemetry".into(),
        model_id: "marker-model".into(),
        api: "tele-deferred".into(),
        id: "deferred-op-1".into(),
        expires_at_ms: None,
        poll_after_ms: None,
        data: None,
    }
}

fn marker_events(terminal: MarkerTerminal) -> Vec<AssistantStreamEvent> {
    match terminal {
        MarkerTerminal::Text => opi_ai::test_support::text_response("marker response"),
        MarkerTerminal::Deferred => {
            let mut pending = opi_ai::test_support::base_assistant();
            pending.provider = "telemetry".into();
            pending.model = "marker-model".into();
            pending.stop_reason = opi_ai::stream::StopReason::Deferred;
            pending.deferred = Some(Box::new(deferred_handle()));
            vec![
                AssistantStreamEvent::Start {
                    partial: pending.clone(),
                },
                AssistantStreamEvent::Done {
                    reason: opi_ai::stream::StopReason::Deferred,
                    message: pending,
                },
            ]
        }
    }
}

/// Event-stream probe: every poll runs one scoped section emitting
/// `chat_poll_marker`; dropping the stream runs one scoped section emitting
/// `chat_drop_marker`.
pub(super) struct MarkerStream {
    context: Option<TelemetryContext>,
    events: std::vec::IntoIter<Result<AssistantStreamEvent, ProviderError>>,
    pending_first_poll: bool,
    observed: Option<Arc<AdapterObservations>>,
    dropped: bool,
}

impl MarkerStream {
    pub(super) fn new(
        context: Option<TelemetryContext>,
        events: Vec<AssistantStreamEvent>,
    ) -> Self {
        Self {
            context,
            events: events
                .into_iter()
                .map(Ok::<_, ProviderError>)
                .collect::<Vec<_>>()
                .into_iter(),
            pending_first_poll: false,
            observed: None,
            dropped: false,
        }
    }

    fn with_observations(mut self, observed: Arc<AdapterObservations>) -> Self {
        self.pending_first_poll = observed.pending_first_poll;
        self.observed = Some(observed);
        self
    }
}

impl Stream for MarkerStream {
    type Item = Result<AssistantStreamEvent, ProviderError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<Self::Item>> {
        let MarkerStream {
            context,
            events,
            pending_first_poll,
            observed,
            dropped: _,
        } = self.get_mut();
        let mut poll = || {
            if let Some(observed) = observed {
                observed.polls.fetch_add(1, Ordering::SeqCst);
            }
            if std::mem::take(pending_first_poll) {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            let next = events.next();
            if next.is_some()
                && let Some(observed) = observed
            {
                observed.events.fetch_add(1, Ordering::SeqCst);
            }
            Poll::Ready(next)
        };
        match context {
            Some(context) => context.in_scope(|| {
                tracing::span!(Level::INFO, "chat_poll_marker").in_scope(|| ());
                poll()
            }),
            None => poll(),
        }
    }
}

impl Drop for MarkerStream {
    fn drop(&mut self) {
        if self.dropped {
            return;
        }
        self.dropped = true;
        let dropped = || {
            if let Some(observed) = &self.observed {
                observed.drops.fetch_add(1, Ordering::SeqCst);
            }
        };
        if let Some(context) = &self.context {
            context.in_scope(|| {
                tracing::span!(Level::INFO, "chat_drop_marker").in_scope(|| ());
                dropped();
            });
        } else {
            dropped();
        }
    }
}

impl ChatApi<()> for MarkerChatApi {
    fn api_id(&self) -> &ApiId {
        static API: std::sync::OnceLock<ApiId> = std::sync::OnceLock::new();
        API.get_or_init(|| ApiId::new("telemetry-chat"))
    }
    fn provider_id(&self) -> &str {
        "telemetry"
    }
    fn complete_record(&self) -> ModelInfo {
        marker_model()
    }
    fn simple_options(&self) {}
    fn stream_prepared(&self, request: Request, _options: (), _auth: ResolvedAuth) -> EventStream {
        self.observed.dispatches.fetch_add(1, Ordering::SeqCst);
        self.observed
            .telemetry_present
            .lock()
            .unwrap()
            .push(request.telemetry_context.is_some());
        Box::pin(
            MarkerStream::new(request.telemetry_context, marker_events(self.terminal))
                .with_observations(Arc::clone(&self.observed)),
        )
    }
}

fn marker_model() -> ModelInfo {
    ModelInfo::new(
        "marker-model",
        "Marker Model",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(100_000, 4_096).with_streaming(true),
    )
}

pub(super) fn marker_record() -> ChatModelRecord {
    ChatModelRecord::new("telemetry", marker_model())
}

pub(super) struct StaticBearer;

impl AuthResolver for StaticBearer {
    fn resolve(
        &self,
    ) -> opi_ai::credential::BoxAuthFuture<'_, Result<ResolvedAuth, ProviderError>> {
        Box::pin(async {
            Ok(ResolvedAuth {
                env: Default::default(),
                scheme: AuthScheme::Bearer,
                secret: secrecy::SecretString::from(SECRET_CANARY),
                base_url: None,
                account_id: Some("telemetry-account".into()),
                provenance: Default::default(),
            })
        })
    }
}

pub(super) fn marker_request(context: Option<TelemetryContext>) -> Request {
    Request {
        projection: Default::default(),
        model: "telemetry:marker-model".into(),
        context: normalize_context(Context {
            system_prompt: None,
            messages: vec![opi_ai::message::Message::User(
                opi_ai::message::UserMessage {
                    content: vec![opi_ai::message::InputContent::Text {
                        text: PROMPT_CANARY.into(),
                    }],
                    timestamp_ms: 0,
                },
            )],
            tools: vec![],
        })
        .unwrap(),
        max_tokens: Some(64),
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: Default::default(),
        timeout: Some(std::time::Duration::from_secs(5)),
        extra_headers: vec![],
        cache_retention: Default::default(),
        session_id: None,
        telemetry_context: context,
        callbacks: Default::default(),
        transport: Default::default(),
    }
}

pub(super) fn message_text(message: &opi_ai::message::AssistantMessage) -> String {
    message
        .content
        .iter()
        .filter_map(|content| match content {
            opi_ai::message::AssistantContent::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// Assert no recorded telemetry value carries either canary.
pub(super) fn assert_no_canaries(recording: &Mutex<Recording>) {
    let state = recording.lock().unwrap();
    for (_, fields) in &state.events {
        for (name, value) in fields {
            assert!(
                !value.contains(PROMPT_CANARY) && !value.contains(SECRET_CANARY),
                "telemetry field '{name}' leaked a canary: {value:?}"
            );
        }
    }
}

/// All probe spans except the capture-time parent itself.
pub(super) fn probe_spans(recording: &Mutex<Recording>) -> Vec<(&'static str, Option<u64>)> {
    recording
        .lock()
        .unwrap()
        .spans
        .iter()
        .copied()
        .filter(|(name, _)| *name != "caller_parent")
        .collect()
}

/// Every adapter marker must be a child of the frozen caller parent.
pub(super) fn assert_all_parented(spans: &[(&'static str, Option<u64>)], parent_id: u64) {
    assert!(!spans.is_empty(), "expected adapter marker spans");
    for (name, parent) in spans {
        assert_eq!(
            *parent,
            Some(parent_id),
            "span '{name}' not parented by the frozen caller parent"
        );
    }
}

/// Recursively assert no serialized object key names a telemetry or trace
/// artifact: the process-local context never crosses a wire or persistent
/// format.
pub(super) fn assert_no_telemetry_keys(value: &serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, inner) in map {
                let lowered = key.to_ascii_lowercase();
                assert!(
                    !lowered.contains("telemetry") && !lowered.contains("trace"),
                    "serialized key '{key}' must not carry a telemetry artifact"
                );
                assert_no_telemetry_keys(inner);
            }
        }
        serde_json::Value::Array(items) => items.iter().for_each(assert_no_telemetry_keys),
        _ => {}
    }
}
