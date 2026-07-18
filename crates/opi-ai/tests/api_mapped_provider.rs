use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use futures_util::StreamExt;
use opi_ai::auth::{AuthResolver, AuthScheme, ResolvedAuth};
use opi_ai::credential::BoxAuthFuture;
use opi_ai::model_info::{ModelCapabilities, WireApi, WireCompat};
use opi_ai::provider::{
    CacheRetention, EventStream, ModelInfo, Provider, ProviderError, Request, ThinkingConfig,
};
use opi_ai::stream::AssistantStreamEvent;
use opi_ai::{ApiMappedProvider, ProviderHeaders};
use secrecy::SecretString;
use tokio_util::sync::CancellationToken;

type RouteCalls = Arc<Mutex<Vec<String>>>;
type RouteLogs = BTreeMap<WireApi, RouteCalls>;

#[derive(Default)]
struct CountingResolver {
    calls: AtomicUsize,
}

impl AuthResolver for CountingResolver {
    fn resolve<'a>(&'a self) -> BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {
            Ok(ResolvedAuth {
                scheme: AuthScheme::Bearer,
                secret: SecretString::from("test-token"),
                base_url: None,
                account_id: None,
            })
        })
    }
}

struct RecordingRoute {
    id: String,
    models: Vec<ModelInfo>,
    auth: Arc<dyn AuthResolver>,
    calls: Arc<Mutex<Vec<String>>>,
}

impl RecordingRoute {
    fn new(
        id: &str,
        models: Vec<ModelInfo>,
        auth: Arc<dyn AuthResolver>,
        calls: Arc<Mutex<Vec<String>>>,
    ) -> Self {
        Self {
            id: id.into(),
            models,
            auth,
            calls,
        }
    }
}

impl Provider for RecordingRoute {
    fn id(&self) -> &str {
        &self.id
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    fn stream(&self, request: Request) -> EventStream {
        let auth = Arc::clone(&self.auth);
        let calls = Arc::clone(&self.calls);
        Box::pin(futures_util::stream::once(async move {
            auth.resolve().await?;
            calls.lock().unwrap().push(request.model);
            Ok(AssistantStreamEvent::Error {
                reason: opi_ai::stream::StopReason::Error,
                message: opi_ai::message::AssistantMessage {
                    content: vec![],
                    api: opi_ai::ApiKind::OpenAi,
                    provider: "acme".into(),
                    model: String::new(),
                    response_model: None,
                    response_id: None,
                    usage: Default::default(),
                    stop_reason: opi_ai::stream::StopReason::Error,
                    error_message: Some("recorded".into()),
                    timestamp_ms: 0,
                },
            })
        }))
    }
}

fn model(id: &str, wire: WireApi) -> ModelInfo {
    ModelInfo::new(
        id,
        id,
        wire,
        ModelCapabilities::new(128_000, 16_384).with_streaming(true),
    )
}

fn request(model: &str) -> Request {
    Request {
        model: model.into(),
        system: None,
        messages: vec![],
        tools: vec![],
        max_tokens: None,
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    }
}

fn mapped_fixture() -> (ApiMappedProvider, Arc<CountingResolver>, RouteLogs) {
    let models = vec![
        model("claude", WireApi::AnthropicMessages),
        model("chat", WireApi::OpenAiCompletions),
        model("responses", WireApi::OpenAiResponses),
    ];
    let auth = Arc::new(CountingResolver::default());
    let shared_auth: Arc<dyn AuthResolver> = auth.clone();
    let mut logs = BTreeMap::new();
    let mut routes: BTreeMap<WireApi, Box<dyn Provider>> = BTreeMap::new();
    for wire in [
        WireApi::AnthropicMessages,
        WireApi::OpenAiCompletions,
        WireApi::OpenAiResponses,
    ] {
        let calls = Arc::new(Mutex::new(Vec::new()));
        logs.insert(wire, Arc::clone(&calls));
        routes.insert(
            wire,
            Box::new(RecordingRoute::new(
                "acme",
                models
                    .iter()
                    .filter(|model| model.wire_api == wire)
                    .cloned()
                    .collect(),
                Arc::clone(&shared_auth),
                calls,
            )),
        );
    }
    (
        ApiMappedProvider::try_new("acme", models, routes).unwrap(),
        auth,
        logs,
    )
}

#[tokio::test]
async fn mapped_provider_dispatches_one_catalog_across_three_wires() {
    let (provider, _, logs) = mapped_fixture();

    for (id, wire) in [
        ("claude", WireApi::AnthropicMessages),
        ("chat", WireApi::OpenAiCompletions),
        ("responses", WireApi::OpenAiResponses),
    ] {
        let event = provider
            .stream(request(&format!("acme:{id}")))
            .next()
            .await
            .unwrap()
            .unwrap();
        assert!(event.is_terminal());
        assert_eq!(
            logs[&wire].lock().unwrap().as_slice(),
            &[format!("acme:{id}")]
        );
    }
    assert_eq!(provider.id(), "acme");
    assert_eq!(provider.models().len(), 3);
}

#[tokio::test]
async fn mapped_routes_share_one_lazy_auth_resolver() {
    let (provider, auth, _) = mapped_fixture();
    assert_eq!(auth.calls.load(Ordering::SeqCst), 0);

    provider
        .stream(request("acme:claude"))
        .next()
        .await
        .unwrap()
        .unwrap();
    provider
        .stream(request("acme:chat"))
        .next()
        .await
        .unwrap()
        .unwrap();

    assert_eq!(auth.calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn mapped_provider_re_resolves_auth_for_every_stream() {
    let (provider, auth, _) = mapped_fixture();
    for _ in 0..3 {
        provider
            .stream(request("responses"))
            .next()
            .await
            .unwrap()
            .unwrap();
    }
    assert_eq!(auth.calls.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn unknown_model_fails_before_route_or_network() {
    let (provider, auth, logs) = mapped_fixture();
    let error = provider
        .stream(request("acme:unknown"))
        .next()
        .await
        .unwrap()
        .unwrap_err();
    assert!(matches!(error, ProviderError::UnknownModel { .. }));
    assert_eq!(auth.calls.load(Ordering::SeqCst), 0);
    assert!(logs.values().all(|log| log.lock().unwrap().is_empty()));

    let error = provider
        .stream(request("other:chat"))
        .next()
        .await
        .unwrap()
        .unwrap_err();
    assert!(matches!(error, ProviderError::UnknownModel { .. }));
}

#[test]
fn missing_route_fails_at_construction() {
    let error = ApiMappedProvider::try_new(
        "acme",
        vec![model("chat", WireApi::OpenAiCompletions)],
        BTreeMap::new(),
    )
    .unwrap_err();
    assert!(error.to_string().contains("openai-completions"));
}

#[test]
fn wire_compat_mismatch_fails_at_construction() {
    let mut mismatched = model("chat", WireApi::OpenAiCompletions);
    mismatched.compat = WireCompat::AnthropicMessages(Default::default());
    let error = ApiMappedProvider::try_new("acme", vec![mismatched], BTreeMap::new()).unwrap_err();
    assert!(error.to_string().contains("compatibility"));
}

#[tokio::test]
async fn mapped_provider_refresh_is_static_none() {
    let (provider, _, _) = mapped_fixture();
    assert!(provider.refresh_models().await.unwrap().is_none());
}

#[test]
fn mapped_provider_rejects_duplicate_models_routes_and_route_id_mismatch() {
    let duplicate = model("chat", WireApi::OpenAiCompletions);
    let error =
        ApiMappedProvider::try_new("acme", vec![duplicate.clone(), duplicate], BTreeMap::new())
            .unwrap_err();
    assert!(error.to_string().contains("duplicate model"));

    let duplicate_routes: Vec<(WireApi, Box<dyn Provider>)> = (0..2)
        .map(|_| {
            (
                WireApi::OpenAiCompletions,
                Box::new(RecordingRoute::new(
                    "acme",
                    vec![model("chat", WireApi::OpenAiCompletions)],
                    Arc::new(CountingResolver::default()),
                    Arc::new(Mutex::new(Vec::new())),
                )) as Box<dyn Provider>,
            )
        })
        .collect();
    let error = ApiMappedProvider::try_new(
        "acme",
        vec![model("chat", WireApi::OpenAiCompletions)],
        duplicate_routes,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        opi_ai::ApiMapError::DuplicateRoute {
            provider_id,
            wire_api: WireApi::OpenAiCompletions,
        } if provider_id == "acme"
    ));

    let auth: Arc<dyn AuthResolver> = Arc::new(CountingResolver::default());
    let mut routes: BTreeMap<WireApi, Box<dyn Provider>> = BTreeMap::new();
    routes.insert(
        WireApi::OpenAiCompletions,
        Box::new(RecordingRoute::new(
            "hidden-route",
            vec![model("chat", WireApi::OpenAiCompletions)],
            auth,
            Arc::new(Mutex::new(Vec::new())),
        )),
    );
    let error = ApiMappedProvider::try_new(
        "acme",
        vec![model("chat", WireApi::OpenAiCompletions)],
        routes,
    )
    .unwrap_err();
    assert!(error.to_string().contains("hidden-route"));
}

#[test]
fn provider_headers_separate_configured_and_request_values() {
    let headers = ProviderHeaders::try_new(vec![("X-Acme".into(), "opi".into())]).unwrap();
    let merged = headers
        .merge_request(
            &[("X-Route".into(), "static".into())],
            &[("X-Request".into(), "dynamic".into())],
        )
        .unwrap();
    assert_eq!(merged.len(), 3);

    for reserved in [
        "authorization",
        "x-api-key",
        "api-key",
        "anthropic-version",
        "content-type",
        "chatgpt-account-id",
    ] {
        assert!(ProviderHeaders::try_new(vec![(reserved.into(), "x".into())]).is_err());
        assert!(
            headers
                .merge_request(&[], &[(reserved.into(), "x".into())])
                .is_err()
        );
    }
    assert!(ProviderHeaders::try_new(vec![("bad\nname".into(), "x".into())]).is_err());
    assert!(ProviderHeaders::try_new(vec![("X-Acme".into(), "bad\nvalue".into())]).is_err());
}

#[tokio::test]
async fn concrete_route_uses_auth_model_provider_base_precedence_and_revocation_typing() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    struct BaseUrlResolver {
        calls: Arc<AtomicUsize>,
        base_url: String,
    }
    impl AuthResolver for BaseUrlResolver {
        fn resolve<'a>(&'a self) -> BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let base_url = self.base_url.clone();
            Box::pin(async move {
                Ok(ResolvedAuth {
                    scheme: AuthScheme::Bearer,
                    secret: SecretString::from("oauth-token"),
                    base_url: Some(base_url),
                    account_id: None,
                })
            })
        }
    }

    let auth_server = MockServer::start().await;
    let model_server = MockServer::start().await;
    let default_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(403).set_body_string("must-not-surface"))
        .mount(&auth_server)
        .await;
    let calls = Arc::new(AtomicUsize::new(0));
    let auth: Arc<dyn AuthResolver> = Arc::new(BaseUrlResolver {
        calls: Arc::clone(&calls),
        base_url: auth_server.uri(),
    });
    let routed_model =
        model("claude", WireApi::AnthropicMessages).with_base_url(model_server.uri());
    let route = opi_ai::anthropic::AnthropicProvider::for_route(
        auth,
        "acme".into(),
        vec![routed_model.clone()],
        Some(default_server.uri()),
        ProviderHeaders::try_new(vec![("X-Acme".into(), "opi".into())]).unwrap(),
        Arc::new(opi_ai::http::HttpClient::new()),
        false,
    );
    let mut routes: BTreeMap<WireApi, Box<dyn Provider>> = BTreeMap::new();
    routes.insert(WireApi::AnthropicMessages, Box::new(route));
    let provider = ApiMappedProvider::try_new("acme", vec![routed_model], routes).unwrap();

    let error = provider
        .stream(request("acme:claude"))
        .next()
        .await
        .unwrap()
        .unwrap_err();
    assert!(matches!(
        error,
        ProviderError::CredentialRevoked { ref provider_id } if provider_id == "acme"
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let requests = auth_server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0]
            .headers
            .get("x-acme")
            .and_then(|value| value.to_str().ok()),
        Some("opi")
    );
    assert!(requests[0].headers.get("anthropic-beta").is_none());
    assert!(model_server.received_requests().await.unwrap().is_empty());
    assert!(default_server.received_requests().await.unwrap().is_empty());
}
