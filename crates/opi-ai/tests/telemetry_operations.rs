//! Assembled telemetry conformance closure over every real SDK operation
//! (task 20.55, DEL-12 / AIR-054..056 / V-46..48).
//!
//! The carrier substrate semantics are closed by `telemetry_carrier.rs`
//! (task 20.54) and one real wire producer by `codex_telemetry.rs`. This
//! suite closes the cross-path contract at the public seam: an external SDK
//! caller freezes a parent Span plus source Dispatch under recording
//! subscriber A while a different dispatch B is ambient at poll time, then
//! drives every public operation family — the four chat entries
//! (native/simple/direct plus the advanced `prepare_call` path with its
//! attempt retry and strict-variant rebuild), image generation, classifier
//! calls and deferred submit/fetch/cancel — and asserts:
//!
//! * adapter-side marker spans children of the frozen parent on dispatch A,
//!   never ambient B, with the caller context restored afterwards;
//! * default (`None`) and explicit noop carriers add no SDK spans, adopt no
//!   ambient root, and leave the business Result/send/poll/callback counts
//!   identical to a context-free twin run;
//! * the process-local context is absent from serialized handles/results and
//!   wire bodies/headers, and canary prompt/secret values never appear in
//!   any recorded telemetry field.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context as TaskContext, Poll};

use futures_core::Stream;
use opi_ai::anthropic::AnthropicProvider;
use opi_ai::api::openrouter_images::OpenRouterImagesApi;
use opi_ai::api::system_one::{TYPESAFE_SYSTEM_ONE_API_ID, TypesafeSystemOneApi};
use opi_ai::auth::{
    AuthProvenanceSource, AuthResolver, AuthScheme, ResolvedAuth, StaticAuthResolver,
};
use opi_ai::callbacks::RequestCallbacks;
use opi_ai::classifier::{
    ClassifierAnswer, ClassifierCall, ClassifierContext, ClassifierModelRecord, ClassifierQuestion,
    ClassifierStopReason,
};
use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
use opi_ai::deferred::{DeferredApi, DeferredCancelCall, DeferredFetchCall};
use opi_ai::http::{HttpClient, HttpClientBuilder};
use opi_ai::images::{
    ImageModelRecord, ImageOutputModality, ImagesCall, ImagesContext, ImagesInput, ImagesOutput,
    ImagesStopReason,
};
use opi_ai::message::ToolDef;
use opi_ai::model_info::{AnthropicMessagesCompat, WireCompat};
use opi_ai::models::{ApiId, ChatApi, ChatModelRecord, Models, SimpleOptions};
use opi_ai::provider::{EventStream, ProviderError, Request, ThinkingConfig};
use opi_ai::retry::AcquisitionPolicy;
use opi_ai::stream::{AssistantStreamEvent, ChatStream};
use opi_ai::telemetry::TelemetryContext;
use opi_ai::transcript::{Context, normalize_context};
use opi_ai::{ModelCapabilities, ModelInfo, WireApi};
use tracing::Level;
use tracing_core::field::{Field, Visit};
use tracing_core::span::{Attributes, Current, Id, Record};
use tracing_core::{Dispatch, Event, Metadata, Subscriber, dispatcher};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ---------------------------------------------------------------------------
// Recording subscriber: spans with resolved parents, events with field values.
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Recording {
    next: u64,
    spans: Vec<(&'static str, Option<u64>)>,
    events: Vec<(&'static str, Vec<(String, String)>)>,
    metadata: std::collections::HashMap<u64, &'static Metadata<'static>>,
    entered: std::collections::HashMap<std::thread::ThreadId, Vec<Id>>,
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

fn recorder() -> (Arc<Mutex<Recording>>, Dispatch) {
    let records = Arc::new(Mutex::new(Recording::default()));
    let dispatch = Dispatch::new(Recorder(records.clone()));
    (records, dispatch)
}

/// The prompt and credential canaries used by the serialization/field
/// negatives: neither may appear in any recorded telemetry value.
const PROMPT_CANARY: &str = "CANARY-PROMPT-telemetry-operations";
const SECRET_CANARY: &str = "CANARY-SECRET-telemetry-operations";

/// Drive a future to completion with `ambient` installed as the poll-time
/// default dispatch, mirroring a host that observes under its own subscriber.
async fn await_under_ambient<T>(ambient: &Dispatch, future: impl Future<Output = T>) -> T {
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
struct AdapterObservations {
    dispatches: AtomicUsize,
    telemetry_present: Mutex<Vec<bool>>,
    pending_first_poll: bool,
    polls: AtomicUsize,
    events: AtomicUsize,
    drops: AtomicUsize,
}

struct MarkerChatApi {
    observed: Arc<AdapterObservations>,
    /// Which terminal the scripted stream delivers: plain text or a
    /// deferred-response terminal carrying a handle for the marker deferred
    /// backend.
    terminal: MarkerTerminal,
}

#[derive(Clone, Copy, PartialEq)]
enum MarkerTerminal {
    Text,
    Deferred,
}

fn deferred_handle() -> opi_ai::deferred::DeferredHandle {
    opi_ai::deferred::DeferredHandle {
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
struct MarkerStream {
    context: Option<TelemetryContext>,
    events: std::vec::IntoIter<Result<AssistantStreamEvent, ProviderError>>,
    pending_first_poll: bool,
    observed: Option<Arc<AdapterObservations>>,
    dropped: bool,
}

impl MarkerStream {
    fn new(context: Option<TelemetryContext>, events: Vec<AssistantStreamEvent>) -> Self {
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

fn marker_record() -> ChatModelRecord {
    ChatModelRecord::new("telemetry", marker_model())
}

struct StaticBearer;

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

fn marker_models(
    observed: Arc<AdapterObservations>,
) -> (Models, opi_ai::models::ChatApiHandle<()>) {
    let mut models = Models::new();
    let handle = models
        .register_chat_api(
            Arc::new(MarkerChatApi {
                observed,
                terminal: MarkerTerminal::Text,
            }),
            Arc::new(StaticBearer),
            AuthProvenanceSource::Static,
        )
        .expect("register marker chat api");
    (models, handle)
}

fn marker_request(context: Option<TelemetryContext>) -> Request {
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

fn message_text(message: &opi_ai::message::AssistantMessage) -> String {
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
fn assert_no_canaries(recording: &Mutex<Recording>) {
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
fn probe_spans(recording: &Mutex<Recording>) -> Vec<(&'static str, Option<u64>)> {
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
fn assert_all_parented(spans: &[(&'static str, Option<u64>)], parent_id: u64) {
    assert!(!spans.is_empty(), "expected adapter marker spans");
    for (name, parent) in spans {
        assert_eq!(
            *parent,
            Some(parent_id),
            "span '{name}' not parented by the frozen caller parent"
        );
    }
}

// ---------------------------------------------------------------------------
// V-46 / AIR-054: default and explicit noop carriers stay passive: no SDK
// spans, no ambient-root adoption, business work exactly once.
// ---------------------------------------------------------------------------
#[tokio::test]
async fn default_and_noop_contexts_run_business_once_without_sdk_observation() {
    let (a_recording, a_dispatch) = recorder();
    let observed = Arc::new(AdapterObservations::default());
    let (models, _handle) = marker_models(Arc::clone(&observed));

    // Omitted context: business runs, and even a recording ambient default
    // observes no SDK-emitted span or event.
    let message = await_under_ambient(
        &a_dispatch,
        models.complete_simple(
            marker_record(),
            marker_request(None),
            SimpleOptions::default(),
        ),
    )
    .await
    .expect("omitted context still completes business work");
    assert_eq!(message_text(&message), "marker response");
    assert_eq!(observed.dispatches.load(Ordering::SeqCst), 1);
    assert_eq!(*observed.telemetry_present.lock().unwrap(), vec![false]);
    assert!(
        a_recording.lock().unwrap().spans.is_empty(),
        "the SDK must not emit automatic call/attempt spans"
    );
    assert!(a_recording.lock().unwrap().events.is_empty());

    // Explicit noop: business runs exactly once with identical output, the
    // adapter's own markers are silenced, and no ambient root is adopted.
    let message = await_under_ambient(
        &a_dispatch,
        models.complete_simple(
            marker_record(),
            marker_request(Some(TelemetryContext::noop())),
            SimpleOptions::default(),
        ),
    )
    .await
    .expect("noop context still completes business work");
    assert_eq!(message_text(&message), "marker response");
    assert_eq!(observed.dispatches.load(Ordering::SeqCst), 2);
    assert_eq!(
        *observed.telemetry_present.lock().unwrap(),
        vec![false, true]
    );
    assert!(
        a_recording.lock().unwrap().spans.is_empty(),
        "noop markers must be shadowed, not routed to any ambient dispatch"
    );
    assert_no_canaries(&a_recording);
}

// ---------------------------------------------------------------------------
// V-47 / AIR-055: scoped Stream poll and drop sections through a real chat
// operation: one short scope per poll, the drop scope on abandonment.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn abandoned_chat_call_scopes_poll_and_drop_to_the_frozen_parent() {
    let (a_recording, a_dispatch) = recorder();
    let (b_recording, b_dispatch) = recorder();
    let observed = Arc::new(AdapterObservations::default());
    let (models, handle) = marker_models(Arc::clone(&observed));

    let parent =
        dispatcher::with_default(&a_dispatch, || tracing::span!(Level::INFO, "caller_parent"));
    let parent_id = parent.id().unwrap().into_u64();
    let context = dispatcher::with_default(&b_dispatch, || TelemetryContext::from_span(parent));

    let mut call = models.stream(
        marker_record(),
        marker_request(Some(context)),
        handle.options(()),
    );
    let waker = futures_util::task::noop_waker();
    let mut cx = TaskContext::from_waker(&waker);
    let first = dispatcher::with_default(&b_dispatch, || {
        futures_util::StreamExt::poll_next_unpin(&mut call, &mut cx)
    });
    assert!(
        first.is_ready(),
        "the first poll must deliver the start event"
    );
    drop(call);

    let spans = probe_spans(&a_recording);
    let polls = spans
        .iter()
        .filter(|(name, _)| *name == "chat_poll_marker")
        .count();
    let drops = spans
        .iter()
        .filter(|(name, _)| *name == "chat_drop_marker")
        .count();
    assert_eq!(polls, 1, "each poll runs exactly one scoped section");
    assert_eq!(drops, 1, "abandonment runs one scoped drop section");
    assert_all_parented(&spans, parent_id);
    assert!(b_recording.lock().unwrap().spans.is_empty());
}

fn business_counts(observed: &AdapterObservations) -> [usize; 4] {
    [
        observed.dispatches.load(Ordering::SeqCst),
        observed.polls.load(Ordering::SeqCst),
        observed.events.load(Ordering::SeqCst),
        observed.drops.load(Ordering::SeqCst),
    ]
}

/// Observe the caller immediately after one adapter section, while the caller's
/// own parent and dispatch are still installed.
fn ambient_section<T>(ambient: &Dispatch, parent: &tracing::Span, body: impl FnOnce() -> T) -> T {
    dispatcher::with_default(ambient, || {
        parent.in_scope(|| {
            let output = body();
            assert_eq!(tracing::Span::current().id(), parent.id());
            tracing::span!(Level::INFO, "ambient_restore_marker").in_scope(|| ());
            output
        })
    })
}

fn assert_ambient_restored(recording: &Mutex<Recording>, parents: &[u64]) {
    let state = recording.lock().unwrap();
    let restored: Vec<_> = state
        .spans
        .iter()
        .filter(|(name, _)| *name == "ambient_restore_marker")
        .map(|(_, parent)| *parent)
        .collect();
    assert_eq!(
        restored,
        parents.iter().copied().map(Some).collect::<Vec<_>>()
    );
    assert!(
        state
            .spans
            .iter()
            .all(|(name, _)| *name == "ambient_parent" || *name == "ambient_restore_marker")
    );
    assert!(state.entered.values().all(Vec::is_empty));
}

#[test]
fn pending_chat_stream_preserves_frozen_context_across_threads() {
    let (a_recording, a_dispatch) = recorder();
    let (b_recording, b_dispatch) = recorder();
    let observed = Arc::new(AdapterObservations {
        pending_first_poll: true,
        ..Default::default()
    });
    let (models, handle) = marker_models(Arc::clone(&observed));
    let parent =
        dispatcher::with_default(&a_dispatch, || tracing::span!(Level::INFO, "caller_parent"));
    let parent_id = parent.id().unwrap().into_u64();
    let context = dispatcher::with_default(&b_dispatch, || TelemetryContext::from_span(parent));
    let ambient_parent = dispatcher::with_default(&b_dispatch, || {
        tracing::span!(Level::INFO, "ambient_parent")
    });
    let first_ambient_id = ambient_parent.id().unwrap().into_u64();
    let mut call = models.stream(
        marker_record(),
        marker_request(Some(context)),
        handle.options(()),
    );
    assert_eq!(business_counts(&observed), [0, 0, 0, 0]);

    let waker = futures_util::task::noop_waker();
    let mut cx = TaskContext::from_waker(&waker);
    let pending = ambient_section(&b_dispatch, &ambient_parent, || {
        futures_util::StreamExt::poll_next_unpin(&mut call, &mut cx)
    });
    assert!(pending.is_pending());
    assert_eq!(business_counts(&observed), [1, 1, 0, 0]);

    let resumed = std::thread::spawn(move || {
        let ambient_parent = dispatcher::with_default(&b_dispatch, || {
            tracing::span!(Level::INFO, "ambient_parent")
        });
        let ambient_id = ambient_parent.id().unwrap().into_u64();
        let waker = futures_util::task::noop_waker();
        let mut cx = TaskContext::from_waker(&waker);
        let mut events = Vec::new();
        for _ in 0..3 {
            match ambient_section(&b_dispatch, &ambient_parent, || {
                futures_util::StreamExt::poll_next_unpin(&mut call, &mut cx)
            }) {
                Poll::Ready(Some(Ok(event))) => events.push(event),
                other => panic!("expected one scripted event per poll, got {other:?}"),
            }
        }
        assert!(matches!(
            ambient_section(&b_dispatch, &ambient_parent, || {
                futures_util::StreamExt::poll_next_unpin(&mut call, &mut cx)
            }),
            Poll::Ready(None)
        ));
        (events, ambient_id)
    })
    .join()
    .expect("stream resumes on another thread");

    let (events, resumed_ambient_id) = resumed;
    assert!(matches!(events[0], AssistantStreamEvent::Start { .. }));
    assert!(matches!(
        &events[1],
        AssistantStreamEvent::TextDelta { delta, .. } if delta == "marker response"
    ));
    match &events[2] {
        AssistantStreamEvent::Done { reason, message } => {
            assert_eq!(*reason, opi_ai::stream::StopReason::Stop);
            assert_eq!(message_text(message), "marker response");
        }
        other => panic!("expected the original terminal, got {other:?}"),
    }
    assert_eq!(business_counts(&observed), [1, 4, 3, 1]);
    let spans = probe_spans(&a_recording);
    assert_eq!(
        spans
            .iter()
            .filter(|(name, _)| *name == "chat_poll_marker")
            .count(),
        4
    );
    assert_eq!(
        spans
            .iter()
            .filter(|(name, _)| *name == "chat_drop_marker")
            .count(),
        1
    );
    assert_all_parented(&spans, parent_id);
    assert!(
        a_recording
            .lock()
            .unwrap()
            .entered
            .values()
            .all(Vec::is_empty)
    );
    assert_ambient_restored(
        &b_recording,
        &[
            first_ambient_id,
            resumed_ambient_id,
            resumed_ambient_id,
            resumed_ambient_id,
            resumed_ambient_id,
        ],
    );
}

#[test]
fn constructed_unpolled_event_stream_scopes_drop() {
    let (a_recording, a_dispatch) = recorder();
    let (b_recording, b_dispatch) = recorder();
    let observed = Arc::new(AdapterObservations::default());
    let api = MarkerChatApi {
        observed: Arc::clone(&observed),
        terminal: MarkerTerminal::Text,
    };
    let parent =
        dispatcher::with_default(&a_dispatch, || tracing::span!(Level::INFO, "caller_parent"));
    let parent_id = parent.id().unwrap().into_u64();
    let context = dispatcher::with_default(&b_dispatch, || TelemetryContext::from_span(parent));
    let stream = api.stream_prepared(
        marker_request(Some(context)),
        (),
        opi_ai::test_support::resolved_bearer_auth(),
    );
    assert_eq!(business_counts(&observed), [1, 0, 0, 0]);
    assert!(probe_spans(&a_recording).is_empty());
    let ambient_parent = dispatcher::with_default(&b_dispatch, || {
        tracing::span!(Level::INFO, "ambient_parent")
    });
    let ambient_id = ambient_parent.id().unwrap().into_u64();
    ambient_section(&b_dispatch, &ambient_parent, || drop(stream));
    assert_eq!(business_counts(&observed), [1, 0, 0, 1]);
    assert_eq!(
        probe_spans(&a_recording),
        vec![("chat_drop_marker", Some(parent_id))]
    );
    assert!(
        a_recording
            .lock()
            .unwrap()
            .entered
            .values()
            .all(Vec::is_empty)
    );
    assert_ambient_restored(&b_recording, &[ambient_id]);
}

#[test]
fn unpolled_models_call_does_not_construct_adapter_stream() {
    let (a_recording, a_dispatch) = recorder();
    let (b_recording, b_dispatch) = recorder();
    let observed = Arc::new(AdapterObservations::default());
    let (models, handle) = marker_models(Arc::clone(&observed));
    let parent =
        dispatcher::with_default(&a_dispatch, || tracing::span!(Level::INFO, "caller_parent"));
    let context = dispatcher::with_default(&b_dispatch, || TelemetryContext::from_span(parent));
    let call = models.stream(
        marker_record(),
        marker_request(Some(context)),
        handle.options(()),
    );
    assert_eq!(business_counts(&observed), [0, 0, 0, 0]);
    let ambient_parent = dispatcher::with_default(&b_dispatch, || {
        tracing::span!(Level::INFO, "ambient_parent")
    });
    let ambient_id = ambient_parent.id().unwrap().into_u64();
    ambient_section(&b_dispatch, &ambient_parent, || drop(call));
    assert_eq!(business_counts(&observed), [0, 0, 0, 0]);
    assert!(observed.telemetry_present.lock().unwrap().is_empty());
    assert!(probe_spans(&a_recording).is_empty());
    assert_ambient_restored(&b_recording, &[ambient_id]);
}

#[test]
fn noop_pending_stream_restores_ambient_and_preserves_business() {
    let (b_recording, b_dispatch) = recorder();
    let mut restored_parents = Vec::new();
    let mut outcomes = Vec::new();
    for context in [None, Some(TelemetryContext::noop())] {
        let observed = Arc::new(AdapterObservations {
            pending_first_poll: true,
            ..Default::default()
        });
        let (models, handle) = marker_models(Arc::clone(&observed));
        let mut call = models.stream(marker_record(), marker_request(context), handle.options(()));
        assert_eq!(business_counts(&observed), [0, 0, 0, 0]);
        let ambient_parent = dispatcher::with_default(&b_dispatch, || {
            tracing::span!(Level::INFO, "ambient_parent")
        });
        let ambient_id = ambient_parent.id().unwrap().into_u64();
        let waker = futures_util::task::noop_waker();
        let mut cx = TaskContext::from_waker(&waker);
        assert!(
            ambient_section(&b_dispatch, &ambient_parent, || {
                futures_util::StreamExt::poll_next_unpin(&mut call, &mut cx)
            })
            .is_pending()
        );
        assert_eq!(business_counts(&observed), [1, 1, 0, 0]);
        for _ in 0..3 {
            match ambient_section(&b_dispatch, &ambient_parent, || {
                futures_util::StreamExt::poll_next_unpin(&mut call, &mut cx)
            }) {
                Poll::Ready(Some(Ok(AssistantStreamEvent::Done { message, .. }))) => {
                    outcomes.push(message_text(&message));
                }
                Poll::Ready(Some(Ok(_))) => {}
                other => panic!("expected a scripted business event, got {other:?}"),
            }
        }
        assert!(matches!(
            ambient_section(&b_dispatch, &ambient_parent, || {
                futures_util::StreamExt::poll_next_unpin(&mut call, &mut cx)
            }),
            Poll::Ready(None)
        ));
        assert_eq!(business_counts(&observed), [1, 4, 3, 1]);
        restored_parents.extend([ambient_id; 5]);
    }
    assert_eq!(outcomes, vec!["marker response", "marker response"]);
    assert_ambient_restored(&b_recording, &restored_parents);
}

// ---------------------------------------------------------------------------
// V-46 / AIR-054: the advanced `prepare_call` entry — attempt retry and the
// rebuilt strict-downgrade variant — keeps the frozen context on the real
// anthropic wire producer across every attempt.
// ---------------------------------------------------------------------------

const ANTHROPIC_SSE_OK: &str = r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_abc","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-5-20250514","stop_reason":null,"usage":{"input_tokens":25,"output_tokens":0}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":15}}

event: message_stop
data: {"type":"message_stop"}

"#;

const STRICT_LIMITS_400: &str = r#"{"type":"error","error":{"type":"invalid_request_error","message":"Too many strict tools (65). Reduce the number of strict tools or convert some to non-strict."}}"#;

async fn anthropic_models(base_url: String) -> Models {
    let model = ModelInfo::new(
        "strict-model",
        "Strict Model",
        WireApi::AnthropicMessages,
        ModelCapabilities::new(1000, 100),
    )
    .with_compat(WireCompat::AnthropicMessages(AnthropicMessagesCompat {
        supports_eager_tool_input_streaming: false,
        force_adaptive_thinking: false,
        supports_temperature: true,
        supports_strict_tools: true,
        supports_mid_convo_system_messages: false,
        supports_mid_convo_tool_changes: false,
    }))
    .expect("valid compat");
    let provider = AnthropicProvider::for_route(
        "test".into(),
        vec![model],
        Some(base_url),
        Default::default(),
        std::sync::Arc::new(HttpClientBuilder::new().build().expect("client build")),
        false,
    );
    let mut models = Models::new();
    models
        .register_route(
            Box::new(provider),
            std::sync::Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                secrecy::SecretString::from(SECRET_CANARY),
            )),
            AuthProvenanceSource::Static,
            opi_ai::models::CompatMetadata {
                openai_compatible: false,
                profile: None,
            },
        )
        .expect("register anthropic route");
    models
}

fn strict_tool(name: &str, preference: StrictPreference) -> ToolDef {
    ToolDef {
        name: name.into(),
        description: "telemetry conformance tool".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {"path": {"type": "string"}},
            "required": ["path"]
        }),
        constrained_sampling: Some(ConstrainedSampling::JsonSchema { strict: preference }),
    }
}

fn strict_request(context: Option<TelemetryContext>, callbacks: RequestCallbacks) -> Request {
    Request {
        projection: Default::default(),
        model: "test:strict-model".into(),
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
            tools: vec![strict_tool("prefer-tool", StrictPreference::Prefer)],
        })
        .expect("valid context declarations"),
        max_tokens: Some(64),
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: Default::default(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: Default::default(),
        session_id: None,
        telemetry_context: context,
        callbacks,
        transport: Default::default(),
    }
}

async fn wait_retry(admission: opi_ai::retry::RetryAdmission) {
    tokio::time::sleep(std::time::Duration::from_millis(admission.delay_ms.max(1))).await;
}

#[tokio::test]
async fn advanced_prepare_retries_and_strict_variants_keep_the_frozen_context() {
    let (a_recording, a_dispatch) = recorder();
    let (b_recording, b_dispatch) = recorder();

    let server = MockServer::start().await;
    // Variant 1 (prefer-strict): one source-qualified strict-limit 400.
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(400).set_body_string(STRICT_LIMITS_400))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    // Variant 2 (rebuilt, prefer removed): one retryable 429, then success.
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(429))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(ANTHROPIC_SSE_OK),
        )
        .mount(&server)
        .await;
    let models = anthropic_models(server.uri()).await;

    let parent =
        dispatcher::with_default(&a_dispatch, || tracing::span!(Level::INFO, "caller_parent"));
    let parent_id = parent.id().unwrap().into_u64();
    let context = dispatcher::with_default(&b_dispatch, || TelemetryContext::from_span(parent));

    // Payload and response callbacks run inside the instrumented producer;
    // they are the host's observation probes for each attempt.
    let callbacks = RequestCallbacks::new()
        .with_on_payload(std::sync::Arc::new(|_body| {
            tracing::span!(Level::INFO, "attempt_payload_marker").in_scope(|| ());
            Ok(None)
        }))
        .with_on_response(std::sync::Arc::new(|_status, _headers| {
            tracing::span!(Level::INFO, "attempt_response_marker").in_scope(|| ());
            Ok(())
        }));

    let mut policy = AcquisitionPolicy::with_max_retries(1);
    policy.jitter_seed = Some(0);
    let call = models
        .prepare_call_with_policy(
            "test:strict-model",
            strict_request(Some(context), callbacks),
            policy,
        )
        .await
        .expect("prepare call");

    // Variant 1: the strict-limit 400 fails the first variant.
    let first = ChatStream::new(call.start_attempt().expect("v1 send 1"))
        .result()
        .await
        .expect_err("strict-limit 400 fails the first variant");
    assert!(matches!(
        first.cause(),
        ProviderError::StrictToolLimits { .. }
    ));
    assert!(call.retry_admission().is_none());
    assert!(call.strict_variant_admission().is_some());

    // Variant 2: one ordinary retry, then success.
    let second = ChatStream::new(call.start_attempt().expect("v2 send 1"))
        .result()
        .await
        .expect_err("429 on the rebuilt variant");
    assert!(matches!(second.cause(), ProviderError::RateLimited { .. }));
    wait_retry(call.retry_admission().expect("ordinary retry admission")).await;
    let message = ChatStream::new(call.start_attempt().expect("v2 send 2"))
        .result()
        .await
        .expect("variant 2 completes");
    assert_eq!(message_text(&message), "Hello");

    // Every attempt's producer sections observed the frozen parent on A.
    // Payload callbacks freeze once per payload variant (prefer-strict and
    // the rebuilt one); the response callback fires only on the acquired
    // final response — together they cover variant 1, the rebuilt variant's
    // first send, and its post-retry success.
    let spans = probe_spans(&a_recording);
    let payloads = spans
        .iter()
        .filter(|(name, _)| *name == "attempt_payload_marker")
        .count();
    let responses = spans
        .iter()
        .filter(|(name, _)| *name == "attempt_response_marker")
        .count();
    assert_eq!(
        payloads, 2,
        "payload callbacks ran once per payload variant"
    );
    assert_eq!(
        responses, 1,
        "response callback fired on the acquired response"
    );
    assert_all_parented(&spans, parent_id);
    assert!(
        b_recording.lock().unwrap().spans.is_empty(),
        "ambient B must never observe the retried operation"
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 3);
    assert_no_canaries(&a_recording);

    // Invariance twin: identical wire sequence without any context produces
    // the same sends and the same final message, and adds nothing to A.
    let spans_before_twin = probe_spans(&a_recording).len();
    let twin = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(400).set_body_string(STRICT_LIMITS_400))
        .up_to_n_times(1)
        .mount(&twin)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(429))
        .up_to_n_times(1)
        .mount(&twin)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(ANTHROPIC_SSE_OK),
        )
        .mount(&twin)
        .await;
    let models = anthropic_models(twin.uri()).await;
    let mut policy = AcquisitionPolicy::with_max_retries(1);
    policy.jitter_seed = Some(0);
    let call = models
        .prepare_call_with_policy(
            "test:strict-model",
            strict_request(None, RequestCallbacks::default()),
            policy,
        )
        .await
        .expect("prepare twin");
    assert!(
        ChatStream::new(call.start_attempt().expect("twin v1"))
            .result()
            .await
            .is_err()
    );
    assert!(
        ChatStream::new(call.start_attempt().expect("twin v2 send 1"))
            .result()
            .await
            .is_err()
    );
    wait_retry(call.retry_admission().expect("twin retry admission")).await;
    let twin_message = ChatStream::new(call.start_attempt().expect("twin v2 send 2"))
        .result()
        .await
        .expect("twin completes");
    assert_eq!(message_text(&twin_message), "Hello");
    assert_eq!(twin.received_requests().await.unwrap().len(), 3);
    assert_eq!(
        probe_spans(&a_recording).len(),
        spans_before_twin,
        "the context-free twin must not add SDK observation"
    );
}

/// Recursively assert no serialized object key names a telemetry or trace
/// artifact: the process-local context never crosses a wire or persistent
/// format.
fn assert_no_telemetry_keys(value: &serde_json::Value) {
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

// ---------------------------------------------------------------------------
// V-46 / AIR-054: image generation — ordinary and native entries on the real
// OpenRouter image adapter keep the frozen context inside the producer
// attempt and off the wire.
// ---------------------------------------------------------------------------

fn image_record(base_url: String) -> ImageModelRecord {
    ImageModelRecord {
        cost_tiers: Vec::new(),
        headers: Vec::new(),
        input_limits: None,
        input: vec![
            opi_ai::catalog::CatalogInputModality::Text,
            opi_ai::catalog::CatalogInputModality::Image,
        ],
        provider_id: "openrouter".into(),
        id: "google/gemini-2.5-flash-image".into(),
        name: "Gemini 2.5 Flash Image (via OpenRouter)".into(),
        api: ApiId::new("openrouter-images"),
        base_url,
        output: vec![ImageOutputModality::Image],
        cost: None,
    }
}

fn image_models(
    record: &ImageModelRecord,
) -> (
    Models,
    opi_ai::models::ImageApiHandle<opi_ai::api::openrouter_images::OpenRouterImagesOptions>,
) {
    let mut models = Models::new();
    let handle = models
        .register_image_api(
            std::sync::Arc::new(OpenRouterImagesApi::new(HttpClient::new(), record.clone())),
            std::sync::Arc::new(StaticAuthResolver::new(
                AuthScheme::Bearer,
                secrecy::SecretString::from(SECRET_CANARY),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register image api");
    (models, handle)
}

fn image_context() -> ImagesContext {
    ImagesContext {
        input: vec![ImagesInput::Text {
            text: PROMPT_CANARY.into(),
        }],
    }
}

fn image_ok_response() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "id": "gen-123",
        "choices": [
            {
                "message": {
                    "role": "assistant",
                    "content": "Done",
                    "images": [
                        { "image_url": "data:image/png;base64,QUJD" }
                    ]
                },
                "finish_reason": "stop"
            }
        ]
    }))
}

#[tokio::test]
async fn image_generation_freezes_context_off_wire_and_keeps_business_identical() {
    let (a_recording, a_dispatch) = recorder();
    let (b_recording, b_dispatch) = recorder();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(image_ok_response())
        .mount(&server)
        .await;
    let record = image_record(server.uri());
    let (models, handle) = image_models(&record);

    let parent =
        dispatcher::with_default(&a_dispatch, || tracing::span!(Level::INFO, "caller_parent"));
    let parent_id = parent.id().unwrap().into_u64();
    let context = dispatcher::with_default(&b_dispatch, || TelemetryContext::from_span(parent));

    let markers = RequestCallbacks::new()
        .with_on_payload(std::sync::Arc::new(|_body| {
            tracing::span!(Level::INFO, "image_payload_marker").in_scope(|| ());
            Ok(None)
        }))
        .with_on_response(std::sync::Arc::new(|_status, _headers| {
            tracing::span!(Level::INFO, "image_response_marker").in_scope(|| ());
            Ok(())
        }));
    let call = ImagesCall {
        telemetry_context: Some(context.clone()),
        callbacks: markers,
        ..Default::default()
    };
    let result = await_under_ambient(
        &b_dispatch,
        models.generate_images(record.clone(), image_context(), call),
    )
    .await;
    assert_eq!(result.stop_reason, ImagesStopReason::Stop);
    assert_eq!(
        result.output,
        vec![
            ImagesOutput::Text {
                text: "Done".into()
            },
            ImagesOutput::Image {
                mime_type: "image/png".into(),
                data: "QUJD".into()
            }
        ]
    );

    // The native entry freezes the same carrier through its typed options.
    let call = ImagesCall {
        telemetry_context: Some(context),
        callbacks: RequestCallbacks::new()
            .with_on_payload(std::sync::Arc::new(|_body| {
                tracing::span!(Level::INFO, "image_payload_marker").in_scope(|| ());
                Ok(None)
            }))
            .with_on_response(std::sync::Arc::new(|_status, _headers| {
                tracing::span!(Level::INFO, "image_response_marker").in_scope(|| ());
                Ok(())
            })),
        ..Default::default()
    };
    let native = models
        .generate_images_native(
            record.clone(),
            image_context(),
            call,
            handle.options(Default::default()),
        )
        .await;
    assert_eq!(native.stop_reason, ImagesStopReason::Stop);
    assert_eq!(native.output, result.output);

    let spans = probe_spans(&a_recording);
    assert!(
        spans
            .iter()
            .any(|(name, _)| *name == "image_payload_marker")
            && spans
                .iter()
                .any(|(name, _)| *name == "image_response_marker")
    );
    assert_all_parented(&spans, parent_id);
    assert!(b_recording.lock().unwrap().spans.is_empty());

    // Invariance twin without any context: same output, no new observation.
    let spans_before_twin = probe_spans(&a_recording).len();
    let twin = models
        .generate_images(record, image_context(), ImagesCall::default())
        .await;
    assert_eq!(twin.stop_reason, ImagesStopReason::Stop);
    assert_eq!(twin.output, result.output);
    assert_eq!(probe_spans(&a_recording).len(), spans_before_twin);

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 3);
    for sent in &requests {
        let body: serde_json::Value = serde_json::from_slice(&sent.body).expect("json body");
        assert_no_telemetry_keys(&body);
        assert!(
            !sent
                .headers
                .keys()
                .any(|name| name.as_str().to_ascii_lowercase().contains("telemetry")),
            "no telemetry header may cross the image wire"
        );
    }
    assert_no_canaries(&a_recording);
}

// ---------------------------------------------------------------------------
// V-46 / AIR-054: classifier calls — ordinary and native entries on the real
// Typesafe System One adapter.
// ---------------------------------------------------------------------------

fn classifier_record(base_url: String) -> ClassifierModelRecord {
    ClassifierModelRecord {
        headers: Vec::new(),
        input_limits: None,
        provider_id: "typesafe".into(),
        id: "jev-latest".into(),
        name: "Jev".into(),
        api: ApiId::new(TYPESAFE_SYSTEM_ONE_API_ID),
        base_url,
        input: vec![opi_ai::classifier::ClassifierInputModality::Text],
        context_window: 64000,
        cost: None,
    }
}

fn classifier_models(
    record: &ClassifierModelRecord,
) -> (
    Models,
    opi_ai::models::ClassifierApiHandle<opi_ai::api::system_one::TypesafeSystemOneOptions>,
) {
    let mut models = Models::new();
    let handle = models
        .register_classifier_api(
            std::sync::Arc::new(TypesafeSystemOneApi::new(HttpClient::new(), record.clone())),
            std::sync::Arc::new(StaticAuthResolver::new(
                AuthScheme::Bearer,
                secrecy::SecretString::from(SECRET_CANARY),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register classifier api");
    (models, handle)
}

fn classify_state() -> ClassifierContext {
    ClassifierContext {
        state: serde_json::json!({ "text": PROMPT_CANARY }),
        questions: [(
            "category".to_owned(),
            ClassifierQuestion::Choice {
                instructions: "Classify the message".into(),
                criteria: [
                    ("success".to_owned(), "Successful".to_owned()),
                    ("failure".to_owned(), "Failed".to_owned()),
                ]
                .into_iter()
                .collect(),
            },
        )]
        .into_iter()
        .collect(),
        images: None,
    }
}

fn classifier_ok_response() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "answers": {
            "category": {
                "type": "choice",
                "choice": "success",
                "probabilities": { "success": 0.9, "failure": 0.1 },
                "confidence": 0.8
            }
        }
    }))
}

fn marker_callbacks() -> RequestCallbacks {
    RequestCallbacks::new()
        .with_on_payload(std::sync::Arc::new(|_body| {
            tracing::span!(Level::INFO, "classifier_payload_marker").in_scope(|| ());
            Ok(None)
        }))
        .with_on_response(std::sync::Arc::new(|_status, _headers| {
            tracing::span!(Level::INFO, "classifier_response_marker").in_scope(|| ());
            Ok(())
        }))
}

#[tokio::test]
async fn classifier_calls_freeze_context_off_wire_and_keep_business_identical() {
    let (a_recording, a_dispatch) = recorder();
    let (b_recording, b_dispatch) = recorder();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(classifier_ok_response())
        .mount(&server)
        .await;
    let record = classifier_record(server.uri());
    let (models, handle) = classifier_models(&record);

    let parent =
        dispatcher::with_default(&a_dispatch, || tracing::span!(Level::INFO, "caller_parent"));
    let parent_id = parent.id().unwrap().into_u64();
    let context = dispatcher::with_default(&b_dispatch, || TelemetryContext::from_span(parent));

    let call = ClassifierCall {
        telemetry_context: Some(context.clone()),
        callbacks: marker_callbacks(),
        ..Default::default()
    };
    let result = await_under_ambient(
        &b_dispatch,
        models.classify(record.clone(), classify_state(), call),
    )
    .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    assert!(result.error.is_none());
    assert_eq!(
        result.answers.get("category"),
        Some(&ClassifierAnswer::Choice {
            choice: "success".into(),
            probabilities: [("success".to_owned(), 0.9), ("failure".to_owned(), 0.1)]
                .into_iter()
                .collect(),
            confidence: 0.8,
        })
    );

    // Native entry with typed options freezes the same carrier.
    let call = ClassifierCall {
        telemetry_context: Some(context),
        callbacks: marker_callbacks(),
        ..Default::default()
    };
    let native = models
        .classify_native(
            record.clone(),
            classify_state(),
            call,
            handle.options(Default::default()),
        )
        .await;
    assert_eq!(native.stop_reason, ClassifierStopReason::Stop);
    assert_eq!(native.answers, result.answers);

    let spans = probe_spans(&a_recording);
    assert_all_parented(&spans, parent_id);
    assert!(b_recording.lock().unwrap().spans.is_empty());

    // Invariance twin without any context: same answers, no new observation.
    let spans_before_twin = probe_spans(&a_recording).len();
    let twin = models
        .classify(record, classify_state(), ClassifierCall::default())
        .await;
    assert_eq!(twin.stop_reason, ClassifierStopReason::Stop);
    assert_eq!(twin.answers, result.answers);
    assert_eq!(probe_spans(&a_recording).len(), spans_before_twin);

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 3);
    for sent in &requests {
        let body: serde_json::Value = serde_json::from_slice(&sent.body).expect("json body");
        assert_no_telemetry_keys(&body);
    }
    assert_no_canaries(&a_recording);
}

// ---------------------------------------------------------------------------
// V-46 / AIR-054: deferred submit/fetch/cancel — the custom backend observes
// the frozen context on every operation, the handle serializes without any
// telemetry artifact, and abandoning a fetch scopes its drop.
// ---------------------------------------------------------------------------

struct MarkerDeferredApi;

impl DeferredApi<()> for MarkerDeferredApi {
    fn api_id(&self) -> &ApiId {
        static API: std::sync::OnceLock<ApiId> = std::sync::OnceLock::new();
        API.get_or_init(|| ApiId::new("tele-deferred"))
    }
    fn provider_id(&self) -> &str {
        "telemetry"
    }
    fn model_id(&self) -> &str {
        "marker-model"
    }
    fn default_options(&self) {}
    fn fetch_prepared(
        &self,
        _handle: &opi_ai::deferred::DeferredHandle,
        _options: &(),
        _auth: ResolvedAuth,
        call: DeferredFetchCall,
    ) -> EventStream {
        Box::pin(MarkerStream::new(
            call.telemetry_context,
            opi_ai::test_support::text_response("deferred final"),
        ))
    }
    fn cancel_prepared<'a>(
        &'a self,
        _handle: &'a opi_ai::deferred::DeferredHandle,
        _options: &'a (),
        _auth: ResolvedAuth,
        call: DeferredCancelCall,
    ) -> opi_ai::credential::BoxAuthFuture<'a, Result<(), ProviderError>> {
        Box::pin(async move {
            if let Some(context) = &call.telemetry_context {
                context.in_scope(|| {
                    tracing::span!(Level::INFO, "deferred_cancel_marker").in_scope(|| ());
                });
            }
            Ok(())
        })
    }
}

#[tokio::test]
async fn deferred_submit_fetch_and_cancel_freeze_the_context_and_the_handle_stays_serializable() {
    let (a_recording, a_dispatch) = recorder();
    let (b_recording, b_dispatch) = recorder();
    let observed = Arc::new(AdapterObservations::default());
    let mut models = Models::new();
    models
        .register_chat_api(
            Arc::new(MarkerChatApi {
                observed,
                terminal: MarkerTerminal::Deferred,
            }),
            Arc::new(StaticBearer),
            AuthProvenanceSource::Static,
        )
        .expect("register deferred-submit chat api");
    models
        .register_deferred_api(
            Arc::new(MarkerDeferredApi),
            Arc::new(StaticBearer),
            AuthProvenanceSource::Static,
        )
        .expect("register marker deferred backend");

    let parent =
        dispatcher::with_default(&a_dispatch, || tracing::span!(Level::INFO, "caller_parent"));
    let parent_id = parent.id().unwrap().into_u64();
    let context = dispatcher::with_default(&b_dispatch, || TelemetryContext::from_span(parent));

    // Submit: a normal chat call whose terminal is the deferred handle.
    let submitted = await_under_ambient(
        &b_dispatch,
        models.complete_simple(
            marker_record(),
            marker_request(Some(context.clone())),
            SimpleOptions::default(),
        ),
    )
    .await
    .expect("deferred submission succeeds");
    assert_eq!(submitted.stop_reason, opi_ai::stream::StopReason::Deferred);
    let handle = submitted
        .deferred
        .clone()
        .expect("the terminal carries the deferred handle");
    assert_eq!(&*handle, &deferred_handle());

    // The serialized handle carries exactly its source-owned identity
    // fields; no telemetry or trace artifact rides along.
    let serialized = serde_json::to_value(&*handle).expect("handle serializes");
    assert_no_telemetry_keys(&serialized);
    let mut keys: Vec<&str> = serialized
        .as_object()
        .expect("handle serializes to an object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, vec!["api", "id", "modelId", "provider"]);

    // The whole submitted terminal message (including the embedded handle)
    // serializes without telemetry artifacts.
    let message_json = serde_json::to_value(&submitted).expect("message serializes");
    assert_no_telemetry_keys(&message_json);

    // Fetch: the backend's stream sections observe the frozen context.
    let fetched = await_under_ambient(
        &b_dispatch,
        models.fetch_deferred(
            &handle,
            DeferredFetchCall {
                telemetry_context: Some(context.clone()),
                ..Default::default()
            },
        ),
    )
    .await
    .expect("deferred fetch succeeds");
    assert_eq!(message_text(&fetched), "deferred final");
    assert_eq!(fetched.stop_reason, opi_ai::stream::StopReason::Stop);

    // Cancel: the backend's synchronous section observes the same context.
    models
        .cancel_deferred(
            &handle,
            DeferredCancelCall {
                telemetry_context: Some(context.clone()),
                ..Default::default()
            },
        )
        .await
        .expect("deferred cancel succeeds");

    // Abandonment: a fetched stream polled once then dropped runs one
    // scoped drop section in the backend.
    let mut stream = models.stream_deferred(
        &handle,
        DeferredFetchCall {
            telemetry_context: Some(context),
            ..Default::default()
        },
    );
    let waker = futures_util::task::noop_waker();
    let mut cx = TaskContext::from_waker(&waker);
    let first = dispatcher::with_default(&b_dispatch, || {
        futures_util::StreamExt::poll_next_unpin(&mut stream, &mut cx)
    });
    assert!(first.is_ready());
    drop(stream);

    let spans = probe_spans(&a_recording);
    assert_all_parented(&spans, parent_id);
    for expected in [
        "chat_poll_marker",
        "deferred_cancel_marker",
        "chat_drop_marker",
    ] {
        assert!(
            spans.iter().any(|(name, _)| *name == expected),
            "expected a parented '{expected}' marker"
        );
    }
    assert!(b_recording.lock().unwrap().spans.is_empty());
    assert_no_canaries(&a_recording);
}

// ---------------------------------------------------------------------------
// V-46 / AIR-054: the four chat entries freeze the caller parent/Dispatch.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn native_simple_and_direct_entries_attribute_adapters_to_the_frozen_parent() {
    let (a_recording, a_dispatch) = recorder();
    let (b_recording, b_dispatch) = recorder();
    let observed = Arc::new(AdapterObservations::default());
    let (models, handle) = marker_models(Arc::clone(&observed));

    let parent =
        dispatcher::with_default(&a_dispatch, || tracing::span!(Level::INFO, "caller_parent"));
    let parent_id = parent.id().unwrap().into_u64();
    let context = dispatcher::with_default(&b_dispatch, || TelemetryContext::from_span(parent));

    // Native entry driven to the single final outcome.
    let message = await_under_ambient(
        &b_dispatch,
        models.complete(
            marker_record(),
            marker_request(Some(context.clone())),
            handle.options(()),
        ),
    )
    .await
    .expect("native entry succeeds with an explicit context");
    assert_eq!(message_text(&message), "marker response");

    assert_all_parented(&probe_spans(&a_recording), parent_id);
    assert!(
        b_recording.lock().unwrap().spans.is_empty(),
        "ambient B must never observe the SDK operation"
    );

    // The simple entry's neutral conversion must not drop or replace the
    // frozen carrier.
    let message = await_under_ambient(
        &b_dispatch,
        models.complete_simple(
            marker_record(),
            marker_request(Some(context.clone())),
            SimpleOptions::default(),
        ),
    )
    .await
    .expect("simple entry succeeds with an explicit context");
    assert_eq!(message_text(&message), "marker response");
    assert_all_parented(&probe_spans(&a_recording), parent_id);
    assert!(b_recording.lock().unwrap().spans.is_empty());

    // The direct entry with injected authentication freezes the same carrier.
    let message = await_under_ambient(
        &b_dispatch,
        models.complete_direct(
            marker_record(),
            marker_request(Some(context)),
            handle.options(()),
            opi_ai::test_support::resolved_bearer_auth(),
        ),
    )
    .await
    .expect("direct entry succeeds with an explicit context");
    assert_eq!(message_text(&message), "marker response");
    assert_all_parented(&probe_spans(&a_recording), parent_id);
    assert!(b_recording.lock().unwrap().spans.is_empty());

    // Business invariance: a context-free twin produces the same text with
    // the same dispatch count, and neither recorder observed SDK spans.
    let dispatches_with = observed.dispatches.load(Ordering::SeqCst);
    let message = models
        .complete_simple(
            marker_record(),
            marker_request(None),
            SimpleOptions::default(),
        )
        .await
        .expect("context-free twin succeeds");
    assert_eq!(message_text(&message), "marker response");
    assert_eq!(
        observed.dispatches.load(Ordering::SeqCst),
        dispatches_with + 1,
        "telemetry must not change how often business work runs"
    );
    // A second Models instance registering the same ApiId stays isolated:
    // its markers attribute to its own caller parent, never to the first
    // instance's frozen parent.
    let (c_recording, c_dispatch) = recorder();
    let second_parent = dispatcher::with_default(&c_dispatch, || {
        tracing::span!(Level::INFO, "second_caller_parent")
    });
    let second_parent_id = second_parent.id().unwrap().into_u64();
    let second_context =
        dispatcher::with_default(&b_dispatch, || TelemetryContext::from_span(second_parent));
    let (second_models, _second_handle) = marker_models(Arc::new(AdapterObservations::default()));
    let spans_before_second_instance = probe_spans(&a_recording).len();
    let message = await_under_ambient(
        &b_dispatch,
        second_models.complete_simple(
            marker_record(),
            marker_request(Some(second_context)),
            SimpleOptions::default(),
        ),
    )
    .await
    .expect("second instance succeeds");
    assert_eq!(message_text(&message), "marker response");
    let second_spans: Vec<_> = c_recording
        .lock()
        .unwrap()
        .spans
        .iter()
        .copied()
        .filter(|(name, _)| *name != "second_caller_parent")
        .collect();
    assert_all_parented(&second_spans, second_parent_id);
    assert_eq!(
        probe_spans(&a_recording).len(),
        spans_before_second_instance,
        "the second instance's markers must never reach the first instance's parent"
    );
    assert_no_canaries(&a_recording);
    assert_no_canaries(&b_recording);
    assert_no_canaries(&c_recording);
}
