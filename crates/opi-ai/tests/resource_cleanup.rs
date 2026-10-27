use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use futures_util::{StreamExt, poll};
use opi_ai::auth::{AuthResolver, AuthScheme, ResolvedAuth};
use opi_ai::http::{HttpClientBuilder, RedirectPolicy};
use opi_ai::openai_codex_responses::OpenAiCodexResponsesProvider;
use opi_ai::provider::{CacheRetention, ProviderError, Request, ThinkingConfig};
use opi_ai::transcript::{Context, normalize_context};
use opi_ai::{ChatModelRecord, ModelCapabilities, ModelInfo, Models, WireApi};
use tokio_util::sync::CancellationToken;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

fn model() -> ModelInfo {
    ModelInfo::new(
        "gpt-5.4",
        "test",
        WireApi::OpenAiCodexResponses,
        ModelCapabilities::new(272_000, 128_000).with_streaming(true),
    )
}

fn request() -> Request {
    Request {
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
        cancel: CancellationToken::new(),
        timeout: Some(std::time::Duration::from_secs(2)),
        extra_headers: vec![],
        cache_retention: CacheRetention::Disabled,
        session_id: Some("complete-session".into()),
        telemetry_context: None,
        callbacks: Default::default(),
        transport: Default::default(),
    }
}

struct Resolver {
    gate: Option<Arc<tokio::sync::Semaphore>>,
    entered: Arc<AtomicUsize>,
}

impl AuthResolver for Resolver {
    fn resolve(
        &self,
    ) -> opi_ai::credential::BoxAuthFuture<'_, Result<ResolvedAuth, ProviderError>> {
        Box::pin(async move {
            self.entered.fetch_add(1, Ordering::SeqCst);
            if let Some(gate) = &self.gate {
                gate.acquire().await.unwrap().forget();
            }
            Ok(ResolvedAuth {
                scheme: AuthScheme::Bearer,
                secret: secrecy::SecretString::from("fake"),
                base_url: None,
                account_id: Some("fake-account".into()),
                provenance: Default::default(),
            })
        })
    }
}

fn models(
    server: &MockServer,
    gate: Option<Arc<tokio::sync::Semaphore>>,
    entered: Arc<AtomicUsize>,
) -> Models {
    let client = HttpClientBuilder::new()
        .redirects(RedirectPolicy::None)
        .build()
        .unwrap();
    let provider =
        OpenAiCodexResponsesProvider::new(Some(server.uri()), vec![model()], Arc::new(client));
    let mut models = Models::new();
    models
        .register_route(
            Box::new(provider),
            Arc::new(Resolver { gate, entered }),
            opi_ai::AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    models
}

async fn peer(delay: std::time::Duration) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    Mock::given(method("POST")).respond_with(ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_string("event: response.completed\ndata: {\"type\":\"response.completed\",\"response\":{\"id\":\"r1\",\"model\":\"gpt-5.4\",\"output\":[]}}\n\n")
        .set_delay(delay)).mount(&server).await;
    server
}

fn call(models: &Models, request: Request) -> opi_ai::models::ChatCall {
    models.stream_simple(
        ChatModelRecord::new("openai-codex", model()),
        request,
        Default::default(),
    )
}

#[tokio::test]
async fn cleanup_cancels_first_polled_call_while_auth_is_pending() {
    let server = peer(std::time::Duration::ZERO).await;
    let entered = Arc::new(AtomicUsize::new(0));
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let models = models(&server, Some(gate.clone()), entered.clone());
    let request = request();
    let parent = request.cancel.clone();
    let mut call = call(&models, request);
    assert!(poll!(call.next()).is_pending());
    assert_eq!(entered.load(Ordering::SeqCst), 1);
    let cleanup = models.cleanup_session_resources(None);
    let (outcome, report) = tokio::time::timeout(std::time::Duration::from_secs(1), async {
        tokio::join!(call.result(), cleanup)
    })
    .await
    .expect("cleanup cancels the auth waiter without releasing external auth work");
    assert!(matches!(
        outcome.unwrap_err().cause(),
        ProviderError::Cancelled
    ));
    assert!(report.unwrap().cancelled_generations >= 1);
    assert!(!parent.is_cancelled());
    gate.add_permits(1);
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn retained_native_api_keeps_its_namespace_after_provider_route_replacement() {
    use opi_ai::openai_codex_responses::{CodexChatApi, CodexOptions};
    use opi_ai::test_support::MockProvider;

    let server = peer(std::time::Duration::ZERO).await;
    let provider = Arc::new(OpenAiCodexResponsesProvider::new(
        Some(server.uri()),
        vec![model()],
        Arc::new(HttpClientBuilder::new().build().unwrap()),
    ));
    let mut models = Models::new();
    let handle = models
        .register_chat_api(
            Arc::new(CodexChatApi::new(provider, model()).unwrap()),
            Arc::new(Resolver {
                gate: None,
                entered: Arc::new(AtomicUsize::new(0)),
            }),
            opi_ai::AuthProvenanceSource::Static,
        )
        .unwrap();
    let entered = Arc::new(AtomicUsize::new(0));
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    models
        .register_route(
            Box::new(MockProvider::new("openai-codex", vec![])),
            Arc::new(Resolver {
                gate: Some(gate.clone()),
                entered: entered.clone(),
            }),
            opi_ai::AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    let input = request();
    let parent = input.cancel.clone();
    let mut retained = models.stream(
        ChatModelRecord::new("openai-codex", model()),
        input,
        handle.options(CodexOptions::default()),
    );
    assert!(poll!(retained.next()).is_pending());
    assert_eq!(
        entered.load(Ordering::SeqCst),
        1,
        "the replacement resolver is pending"
    );
    let report = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        models.cleanup_session_resources(None),
    )
    .await
    .expect("cleanup must not need the caller to repoll pending auth")
    .unwrap();
    assert_eq!(
        report.cancelled_generations, 1,
        "native API retains its registered namespace"
    );
    let outcome = tokio::time::timeout(std::time::Duration::from_secs(1), retained.result())
        .await
        .expect("the native auth waiter observes its original namespace cutoff");
    assert!(matches!(
        outcome.unwrap_err().cause(),
        ProviderError::Cancelled
    ));
    assert!(!parent.is_cancelled());
    gate.add_permits(1);
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn cleanup_cancels_registered_prepared_call_without_waiting_for_its_handle_drop() {
    let server = peer(std::time::Duration::ZERO).await;
    let models = models(&server, None, Arc::new(AtomicUsize::new(0)));
    let prepared = models
        .prepare_call("openai-codex:gpt-5.4", request())
        .await
        .unwrap();
    let mut unpolled = prepared.start_attempt().unwrap();
    let report = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        models.cleanup_session_resources(None),
    )
    .await
    .expect("no producer is running, so cleanup cannot wait on a caller-owned PreparedCall")
    .unwrap();
    assert!(report.cancelled_generations >= 1);
    assert!(matches!(
        unpolled.next().await.unwrap().unwrap_err(),
        ProviderError::Cancelled
    ));
    assert!(prepared.start_attempt().is_err());
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn disabled_cache_keeps_complete_session_cleanup_identity() {
    let server = peer(std::time::Duration::from_secs(1)).await;
    let models = models(&server, None, Arc::new(AtomicUsize::new(0)));
    let mut call = call(&models, request());
    assert!(poll!(call.next()).is_pending());
    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        while server.received_requests().await.unwrap().len() < 2 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the same logical call is now waiting on its SSE fallback");
    let (outcome, report) = tokio::join!(
        call.result(),
        models.cleanup_session_resources(Some("complete-session"))
    );
    assert!(matches!(
        outcome.unwrap_err().cause(),
        ProviderError::Cancelled
    ));
    assert_eq!(report.unwrap().cancelled_generations, 1);
}

#[tokio::test]
async fn cleanup_cutoff_begins_on_first_poll_and_unpolled_calls_start_after_it() {
    let server = peer(std::time::Duration::ZERO).await;
    let entered = Arc::new(AtomicUsize::new(0));
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let models = models(&server, Some(gate.clone()), entered.clone());
    let cleanup = models.cleanup_session_resources(None);
    let mut old = call(&models, request());
    let later = call(&models, request());
    assert!(poll!(old.next()).is_pending());
    let (old_result, report) = tokio::time::timeout(std::time::Duration::from_secs(1), async {
        tokio::join!(old.result(), cleanup)
    })
    .await
    .expect("first-poll cutoff covers work started after Future construction");
    assert!(matches!(
        old_result.unwrap_err().cause(),
        ProviderError::Cancelled
    ));
    assert_eq!(report.unwrap().cancelled_generations, 1);
    gate.add_permits(1);
    assert!(later.result().await.is_ok());
    assert_eq!(entered.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn cleanup_does_not_wait_for_a_dormant_auth_call_to_be_repolled() {
    let server = peer(std::time::Duration::ZERO).await;
    let entered = Arc::new(AtomicUsize::new(0));
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let models = models(&server, Some(gate), entered.clone());
    let mut retained = call(&models, request());
    assert!(poll!(retained.next()).is_pending());
    assert_eq!(entered.load(Ordering::SeqCst), 1);
    let report = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        models.cleanup_session_resources(None),
    )
    .await
    .expect("a dormant caller-owned auth future cannot hold cleanup hostage")
    .unwrap();
    assert_eq!(report.cancelled_generations, 1);
    assert!(matches!(
        retained.result().await.unwrap_err().cause(),
        ProviderError::Cancelled
    ));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn cleanup_does_not_wait_for_a_dormant_advanced_prepare_to_be_repolled() {
    let server = peer(std::time::Duration::ZERO).await;
    let entered = Arc::new(AtomicUsize::new(0));
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let models = models(&server, Some(gate), entered.clone());
    let mut retained = Box::pin(models.prepare_call("openai-codex:gpt-5.4", request()));
    assert!(poll!(retained.as_mut()).is_pending());
    assert_eq!(entered.load(Ordering::SeqCst), 1);
    let report = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        models.cleanup_session_resources(None),
    )
    .await
    .expect("a dormant caller-owned prepare future is not a live producer")
    .unwrap();
    assert_eq!(report.cancelled_generations, 1);
    assert!(matches!(
        retained.await,
        Err(opi_ai::models::ModelsError::CallCancelled)
    ));
    assert!(server.received_requests().await.unwrap().is_empty());
}

struct SharedCodex(Arc<OpenAiCodexResponsesProvider>);

impl opi_ai::provider::Provider for SharedCodex {
    fn id(&self) -> &str {
        opi_ai::provider::Provider::id(self.0.as_ref())
    }
    fn models(&self) -> &[ModelInfo] {
        opi_ai::provider::Provider::models(self.0.as_ref())
    }
    fn stream_prepared(
        &self,
        request: Request,
        auth: ResolvedAuth,
    ) -> opi_ai::provider::EventStream {
        self.0.stream_prepared(request, auth)
    }
    fn stream_resource_attempt(
        &self,
        request: Request,
        auth: ResolvedAuth,
        attempt: opi_ai::provider::AttemptContext,
        resources: Option<opi_ai::codex_resources::CallResourceLease>,
    ) -> opi_ai::provider::EventStream {
        self.0
            .stream_resource_attempt(request, auth, attempt, resources)
    }
    fn session_resources(&self) -> Option<&dyn opi_ai::codex_resources::SessionResourceOwner> {
        Some(self.0.as_ref())
    }
}

fn shared_provider(server: &MockServer) -> Arc<OpenAiCodexResponsesProvider> {
    let client = Arc::new(
        HttpClientBuilder::new()
            .redirects(RedirectPolicy::None)
            .build()
            .unwrap(),
    );
    Arc::new(OpenAiCodexResponsesProvider::new(
        Some(server.uri()),
        vec![model()],
        client,
    ))
}

fn register_shared(models: &mut Models, provider: Arc<OpenAiCodexResponsesProvider>) {
    models
        .register_route(
            Box::new(SharedCodex(provider)),
            Arc::new(Resolver {
                gate: None,
                entered: Arc::new(AtomicUsize::new(0)),
            }),
            opi_ai::AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
}

async fn wait_requests(peer: &MockServer, count: usize) {
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while peer.received_requests().await.unwrap().len() < count {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("controlled calls reached their expected transport phase");
}

#[tokio::test]
async fn shared_provider_and_client_keep_models_namespaces_and_parent_tokens_isolated() {
    let server = peer(std::time::Duration::from_millis(250)).await;
    let provider = shared_provider(&server);
    let mut models_a = Models::new();
    let mut models_b = Models::new();
    register_shared(&mut models_a, provider.clone());
    register_shared(&mut models_b, provider);
    let input_a = request();
    let parent_a = input_a.cancel.clone();
    let input_b = request();
    let parent_b = input_b.cancel.clone();
    let mut call_a = call(&models_a, input_a);
    let mut call_b = call(&models_b, input_b);
    assert!(poll!(call_a.next()).is_pending());
    assert!(poll!(call_b.next()).is_pending());
    wait_requests(&server, 4).await;
    let (a, cleanup) = tokio::join!(
        call_a.result(),
        models_a.cleanup_session_resources(Some("complete-session"))
    );
    assert!(matches!(a.unwrap_err().cause(), ProviderError::Cancelled));
    assert_eq!(cleanup.unwrap().cancelled_generations, 1);
    assert!(!parent_a.is_cancelled());
    assert!(!parent_b.is_cancelled());
    assert_eq!(
        call_b.result().await.unwrap().response_id.as_deref(),
        Some("r1")
    );
    assert!(!parent_b.is_cancelled());
    assert_eq!(server.received_requests().await.unwrap().len(), 4);
}

fn require_owned<T: Send + 'static>(value: T) -> T {
    value
}

#[tokio::test]
async fn owned_cleanup_future_survives_models_drop() {
    let server = peer(std::time::Duration::ZERO).await;
    let models = models(
        &server,
        Some(Arc::new(tokio::sync::Semaphore::new(0))),
        Arc::new(AtomicUsize::new(0)),
    );
    let mut retained = call(&models, request());
    assert!(poll!(retained.next()).is_pending());
    let cleanup = require_owned(models.cleanup_session_resources(Some("complete-session")));
    drop(models);
    let (result, report) = tokio::join!(retained.result(), cleanup);
    assert!(matches!(
        result.unwrap_err().cause(),
        ProviderError::Cancelled
    ));
    assert_eq!(report.unwrap().cancelled_generations, 1);
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn models_drop_keeps_unpolled_and_started_owned_calls_alive() {
    for started in [false, true] {
        let server = peer(std::time::Duration::from_millis(100)).await;
        let models = models(&server, None, Arc::new(AtomicUsize::new(0)));
        let input = request();
        let parent = input.cancel.clone();
        let mut retained = call(&models, input);
        if started {
            assert!(poll!(retained.next()).is_pending());
            wait_requests(&server, 2).await;
        }
        drop(models);
        assert_eq!(
            retained.result().await.unwrap().response_id.as_deref(),
            Some("r1")
        );
        assert!(!parent.is_cancelled());
        assert_eq!(server.received_requests().await.unwrap().len(), 2);
    }
}

#[tokio::test]
async fn provider_replacement_keeps_the_old_owned_call_alive() {
    let server = peer(std::time::Duration::from_millis(100)).await;
    let provider = shared_provider(&server);
    let mut models = Models::new();
    register_shared(&mut models, provider.clone());
    let mut retained = call(&models, request());
    assert!(poll!(retained.next()).is_pending());
    wait_requests(&server, 2).await;
    register_shared(&mut models, provider);
    assert_eq!(
        retained.result().await.unwrap().response_id.as_deref(),
        Some("r1")
    );
    assert_eq!(
        call(&models, request())
            .result()
            .await
            .unwrap()
            .response_id
            .as_deref(),
        Some("r1")
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 4);
}

#[tokio::test]
async fn dropping_a_polled_cleanup_does_not_revoke_its_close_marks() {
    let server = peer(std::time::Duration::ZERO).await;
    let models = models(&server, None, Arc::new(AtomicUsize::new(0)));
    let entered = Arc::new(tokio::sync::Notify::new());
    let callback_entered = entered.clone();
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let mut input = request();
    let parent = input.cancel.clone();
    input.callbacks = opi_ai::callbacks::RequestCallbacks::new().with_async_on_payload(move |_| {
        let entered = callback_entered.clone();
        let gate = gate.clone();
        async move {
            entered.notify_one();
            gate.acquire().await.unwrap().forget();
            Ok(None)
        }
    });
    let mut retained = call(&models, input);
    assert!(poll!(retained.next()).is_pending());
    tokio::time::timeout(std::time::Duration::from_secs(1), entered.notified())
        .await
        .unwrap();
    let mut cleanup = Box::pin(models.cleanup_session_resources(None));
    assert!(poll!(cleanup.as_mut()).is_pending());
    drop(cleanup);
    assert!(matches!(
        retained.result().await.unwrap_err().cause(),
        ProviderError::Cancelled
    ));
    tokio::time::timeout(
        std::time::Duration::from_secs(1),
        models.cleanup_session_resources(None),
    )
    .await
    .expect("another cleanup confirms the remaining local producer exit")
    .unwrap();
    assert!(!parent.is_cancelled());
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn cleanup_interrupts_a_pending_raw_callback_without_caller_repoll() {
    let server = peer(std::time::Duration::ZERO).await;
    let models = models(&server, None, Arc::new(AtomicUsize::new(0)));
    let entered = Arc::new(tokio::sync::Notify::new());
    let callback_entered = entered.clone();
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let mut input = request();
    input.callbacks = opi_ai::callbacks::RequestCallbacks::new().with_async_on_raw(move |_| {
        let entered = callback_entered.clone();
        let gate = gate.clone();
        async move {
            entered.notify_one();
            gate.acquire().await.unwrap().forget();
            Ok(())
        }
    });
    let mut retained = call(&models, input);
    assert!(poll!(retained.next()).is_pending());
    tokio::time::timeout(std::time::Duration::from_secs(1), entered.notified())
        .await
        .unwrap();
    tokio::time::timeout(
        std::time::Duration::from_secs(1),
        models.cleanup_session_resources(None),
    )
    .await
    .expect("cleanup must cancel the producer while its raw observer is pending")
    .unwrap();
    assert!(matches!(
        retained.result().await.unwrap_err().cause(),
        ProviderError::Cancelled
    ));
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[derive(Clone)]
struct CleanupProbe {
    id: &'static str,
    fail: bool,
    cutoffs: Arc<AtomicUsize>,
    releases: Arc<AtomicUsize>,
}

impl opi_ai::codex_resources::SessionResourceOwner for CleanupProbe {
    fn new_namespace(&self) -> Arc<dyn opi_ai::codex_resources::SessionResourceOwner> {
        Arc::new(self.clone())
    }
    fn cleanup_session_resources(&self, _: Option<&str>) -> opi_ai::codex_resources::CleanupHandle {
        self.cutoffs.fetch_add(1, Ordering::SeqCst);
        let probe = self.clone();
        opi_ai::codex_resources::CleanupHandle::from_future(async move {
            assert_eq!(
                probe.cutoffs.load(Ordering::SeqCst),
                2,
                "all cutoffs precede any release await"
            );
            probe.releases.fetch_add(1, Ordering::SeqCst);
            let mut report = opi_ai::codex_resources::CleanupReport {
                cancelled_generations: 1,
                ..Default::default()
            };
            if probe.fail {
                report
                    .unconfirmed_effects
                    .push("controlled release unconfirmed".into());
                Err(opi_ai::codex_resources::CleanupError::Incomplete { report })
            } else {
                report.closed_connections = 1;
                Ok(report)
            }
        })
    }
}

impl opi_ai::provider::Provider for CleanupProbe {
    fn id(&self) -> &str {
        self.id
    }
    fn models(&self) -> &[ModelInfo] {
        &[]
    }
    fn stream_prepared(&self, _: Request, _: ResolvedAuth) -> opi_ai::provider::EventStream {
        Box::pin(futures_util::stream::empty())
    }
    fn session_resources(&self) -> Option<&dyn opi_ai::codex_resources::SessionResourceOwner> {
        Some(self)
    }
}

#[tokio::test]
async fn cleanup_failure_still_resolves_other_owners_and_reports_confirmed_effects() {
    let mut models = Models::new();
    let cutoffs = Arc::new(AtomicUsize::new(0));
    let releases = Arc::new(AtomicUsize::new(0));
    for (id, fail) in [("fails", true), ("succeeds", false)] {
        models
            .register_route(
                Box::new(CleanupProbe {
                    id,
                    fail,
                    cutoffs: cutoffs.clone(),
                    releases: releases.clone(),
                }),
                Arc::new(Resolver {
                    gate: None,
                    entered: Arc::new(AtomicUsize::new(0)),
                }),
                opi_ai::AuthProvenanceSource::Static,
                Default::default(),
            )
            .unwrap();
    }
    let cleanup = models.cleanup_session_resources(None);
    assert_eq!(
        cutoffs.load(Ordering::SeqCst),
        0,
        "construction creates no cutoff"
    );
    let error = cleanup.await.unwrap_err();
    let opi_ai::codex_resources::CleanupError::Incomplete { report } = error else {
        panic!("incomplete release is typed");
    };
    assert_eq!(cutoffs.load(Ordering::SeqCst), 2);
    assert_eq!(releases.load(Ordering::SeqCst), 2);
    assert_eq!(report.cancelled_generations, 2);
    assert_eq!(report.closed_connections, 1);
    assert_eq!(
        report.unconfirmed_effects,
        vec!["controlled release unconfirmed"]
    );
}

#[tokio::test]
async fn cleanup_first_poll_includes_namespaces_registered_after_future_creation() {
    let mut models = Models::new();
    let cutoffs = Arc::new(AtomicUsize::new(0));
    let releases = Arc::new(AtomicUsize::new(0));
    let first = CleanupProbe {
        id: "first",
        fail: false,
        cutoffs: cutoffs.clone(),
        releases: releases.clone(),
    };
    models
        .register_route(
            Box::new(first),
            Arc::new(Resolver {
                gate: None,
                entered: Arc::new(AtomicUsize::new(0)),
            }),
            opi_ai::AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    let cleanup = models.cleanup_session_resources(None);
    let second = CleanupProbe {
        id: "second",
        fail: false,
        cutoffs: cutoffs.clone(),
        releases: releases.clone(),
    };
    models
        .register_route(
            Box::new(second),
            Arc::new(Resolver {
                gate: None,
                entered: Arc::new(AtomicUsize::new(0)),
            }),
            opi_ai::AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    let report = cleanup.await.unwrap();
    assert_eq!(cutoffs.load(Ordering::SeqCst), 2);
    assert_eq!(releases.load(Ordering::SeqCst), 2);
    assert_eq!(report.closed_connections, 2);
}

#[test]
fn unpolled_owned_call_does_not_retain_unrelated_namespaces_after_models_drop() {
    let mut models = Models::new();
    let unrelated = Arc::new(AtomicUsize::new(0));
    let selected = Arc::new(AtomicUsize::new(0));
    for (id, marker) in [
        ("unrelated", unrelated.clone()),
        ("selected", selected.clone()),
    ] {
        models
            .register_route(
                Box::new(CleanupProbe {
                    id,
                    fail: false,
                    cutoffs: marker,
                    releases: Arc::new(AtomicUsize::new(0)),
                }),
                Arc::new(Resolver {
                    gate: None,
                    entered: Arc::new(AtomicUsize::new(0)),
                }),
                opi_ai::AuthProvenanceSource::Static,
                Default::default(),
            )
            .unwrap();
    }
    let owned = models.stream_simple(
        ChatModelRecord::new("selected", model()),
        request(),
        Default::default(),
    );
    drop(models);
    assert_eq!(
        Arc::strong_count(&unrelated),
        1,
        "unrelated owner is released when Models drops"
    );
    assert!(
        Arc::strong_count(&selected) > 1,
        "owned call retains its selected namespace and provider"
    );
    drop(owned);
    assert_eq!(Arc::strong_count(&selected), 1);
}
