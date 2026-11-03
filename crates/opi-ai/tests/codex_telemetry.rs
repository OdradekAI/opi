use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::Context as TaskContext;
use std::thread::ThreadId;

use futures_core::Stream;
use opi_ai::auth::{AuthResolver, AuthScheme, ResolvedAuth};
use opi_ai::callbacks::RequestCallbacks;
use opi_ai::http::{HttpClientBuilder, RedirectPolicy};
use opi_ai::openai_codex_responses::OpenAiCodexResponsesProvider;
use opi_ai::provider::{CacheRetention, Request, ThinkingConfig};
use opi_ai::telemetry::TelemetryContext;
use opi_ai::transcript::{Context, normalize_context};
use opi_ai::{ChatModelRecord, ModelCapabilities, ModelInfo, Models, WireApi};
use tracing_core::span::{Attributes, Current, Id, Record};
use tracing_core::{Dispatch, Event, Metadata, Subscriber, dispatcher};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[derive(Default)]
struct Recording {
    next: u64,
    spans: Vec<(&'static str, Option<u64>)>,
    metadata: HashMap<u64, &'static Metadata<'static>>,
    entered: HashMap<ThreadId, Vec<Id>>,
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
    fn event(&self, _: &Event<'_>) {}
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

fn recorder() -> (Arc<Mutex<Recording>>, Dispatch) {
    let records = Arc::new(Mutex::new(Recording::default()));
    let dispatch = Dispatch::new(Recorder(records.clone()));
    (records, dispatch)
}

fn model() -> ModelInfo {
    ModelInfo::new(
        "gpt-5.4",
        "test",
        WireApi::OpenAiCodexResponses,
        ModelCapabilities::new(272_000, 128_000).with_streaming(true),
    )
}

fn request(context: TelemetryContext, callbacks: RequestCallbacks) -> Request {
    Request {
        projection: Default::default(),
        model: "openai-codex:gpt-5.4".into(),
        context: normalize_context(Context {
            system_prompt: None,
            messages: vec![],
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
        timeout: Some(std::time::Duration::from_secs(2)),
        extra_headers: vec![],
        cache_retention: CacheRetention::Disabled,
        session_id: None,
        telemetry_context: Some(context),
        callbacks,
        transport: Default::default(),
    }
}

async fn peer() -> MockServer {
    let peer = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&peer)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).insert_header("content-type", "text/event-stream")
            .set_body_string("data: {\"type\":\"response.completed\",\"response\":{\"id\":\"telemetry-result\",\"model\":\"gpt-5.4\",\"output\":[]}}\n\n"))
        .mount(&peer).await;
    peer
}

struct StaticBearer;

impl AuthResolver for StaticBearer {
    fn resolve(
        &self,
    ) -> opi_ai::credential::BoxAuthFuture<'_, Result<ResolvedAuth, opi_ai::provider::ProviderError>>
    {
        Box::pin(async {
            Ok(ResolvedAuth {
                env: Default::default(),
                scheme: AuthScheme::Bearer,
                secret: secrecy::SecretString::from("fake-key"),
                base_url: None,
                account_id: Some("fake-account".into()),
                provenance: Default::default(),
            })
        })
    }
}

fn models(peer: &MockServer) -> Models {
    let mut models = Models::new();
    let client = Arc::new(
        HttpClientBuilder::new()
            .redirects(RedirectPolicy::None)
            .build()
            .unwrap(),
    );
    models
        .register_route(
            Box::new(OpenAiCodexResponsesProvider::new(
                Some(peer.uri()),
                vec![model()],
                client,
            )),
            Arc::new(StaticBearer),
            opi_ai::AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    models
}

#[tokio::test]
async fn real_codex_callbacks_keep_source_parent_across_b_poll_and_pending() {
    let peer = peer().await;
    let models = models(&peer);
    let (source, source_dispatch) = recorder();
    let (ambient, ambient_dispatch) = recorder();
    let parent =
        dispatcher::with_default(&source_dispatch, || tracing::info_span!("source_parent"));
    let parent_id = parent.id().unwrap().into_u64();
    // Capture while B is default too: the Span, not either ambient scope, owns A.
    let context =
        dispatcher::with_default(&ambient_dispatch, || TelemetryContext::from_span(parent));
    let headers = Arc::new(AtomicUsize::new(0));
    let payloads = Arc::new(AtomicUsize::new(0));
    let responses = Arc::new(AtomicUsize::new(0));
    let raws = Arc::new(AtomicUsize::new(0));
    let entered = Arc::new(tokio::sync::Notify::new());
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let h = headers.clone();
    let p = payloads.clone();
    let r = responses.clone();
    let raw_count = raws.clone();
    let callback_entered = entered.clone();
    let callback_gate = gate.clone();
    let callbacks = RequestCallbacks::new()
        .with_async_header_transform(move |values| {
            let count = h.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                tracing::info_span!("header_probe").in_scope(|| ());
                tokio::task::yield_now().await;
                Ok(values)
            }
        })
        .with_async_on_payload(move |_| {
            let count = p.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                tracing::info_span!("payload_probe").in_scope(|| ());
                tokio::task::yield_now().await;
                Ok(None)
            }
        })
        .with_async_on_response(move |status, _| {
            let count = r.clone();
            async move {
                assert_eq!(status, 200);
                count.fetch_add(1, Ordering::SeqCst);
                tracing::info_span!("response_probe").in_scope(|| ());
                tokio::task::yield_now().await;
                Ok(())
            }
        })
        .with_async_on_raw(move |value| {
            let count = raw_count.clone();
            let entered = callback_entered.clone();
            let gate = callback_gate.clone();
            async move {
                assert_eq!(value["type"], "response.completed");
                count.fetch_add(1, Ordering::SeqCst);
                tracing::info_span!("raw_before_pending").in_scope(|| ());
                entered.notify_one();
                gate.acquire().await.unwrap().forget();
                tracing::info_span!("raw_after_pending").in_scope(|| ());
                Ok(())
            }
        });
    let mut call = models.stream_simple(
        ChatModelRecord::new("openai-codex", model()),
        request(context, callbacks),
        Default::default(),
    );
    let waker = futures_util::task::noop_waker();
    let mut cx = TaskContext::from_waker(&waker);
    assert!(
        dispatcher::with_default(&ambient_dispatch, || Pin::new(&mut call).poll_next(&mut cx))
            .is_pending()
    );
    tokio::time::timeout(std::time::Duration::from_secs(1), entered.notified())
        .await
        .unwrap();
    assert!(
        dispatcher::with_default(&ambient_dispatch, || Pin::new(&mut call).poll_next(&mut cx))
            .is_pending()
    );
    assert_eq!(raws.load(Ordering::SeqCst), 1);
    gate.add_permits(1);
    let mut result = Box::pin(call.result());
    let message = futures_util::future::poll_fn(|cx| {
        dispatcher::with_default(&ambient_dispatch, || {
            Future::poll(Pin::as_mut(&mut result), cx)
        })
    })
    .await
    .unwrap();
    assert_eq!(message.response_id.as_deref(), Some("telemetry-result"));
    for count in [headers, payloads, responses, raws] {
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }
    let observed: Vec<_> = source
        .lock()
        .unwrap()
        .spans
        .iter()
        .copied()
        .filter(|(name, _)| {
            matches!(
                *name,
                "header_probe"
                    | "payload_probe"
                    | "response_probe"
                    | "raw_before_pending"
                    | "raw_after_pending"
            )
        })
        .collect();
    assert_eq!(
        observed,
        vec![
            ("header_probe", Some(parent_id)),
            ("payload_probe", Some(parent_id)),
            ("response_probe", Some(parent_id)),
            ("raw_before_pending", Some(parent_id)),
            ("raw_after_pending", Some(parent_id)),
        ]
    );
    assert!(
        ambient.lock().unwrap().spans.is_empty(),
        "B never becomes the SDK source"
    );
    dispatcher::with_default(&ambient_dispatch, || {
        tracing::info_span!("independent_host_log").in_scope(|| ())
    });
    assert_eq!(
        ambient.lock().unwrap().spans,
        vec![("independent_host_log", None)]
    );
    let requests = peer.received_requests().await.unwrap();
    assert_eq!(
        requests.len(),
        2,
        "one Upgrade rejection and one frozen-body SSE fallback"
    );
    assert_eq!(requests[0].method.as_str(), "GET");
    assert_eq!(requests[1].method.as_str(), "POST");
    let body: serde_json::Value = serde_json::from_slice(&requests[1].body).unwrap();
    assert!(body.get("telemetry_context").is_none());
    assert!(body.get("telemetryContext").is_none());
}
