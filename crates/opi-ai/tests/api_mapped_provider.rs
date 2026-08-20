use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use futures_util::StreamExt;
use opi_ai::auth::{AuthProvenanceSource, AuthResolver, AuthScheme, ResolvedAuth};
use opi_ai::credential::BoxAuthFuture;
use opi_ai::model_info::{ModelCapabilities, ModelInfoError, WireApi, WireCompat};
use opi_ai::provider::{
    CacheRetention, EventStream, ModelInfo, Provider, ProviderError, Request, ThinkingConfig,
};
use opi_ai::stream::AssistantStreamEvent;
use opi_ai::{ApiMappedProvider, CompatMetadata, ProviderCollection, ProviderHeaders};
use secrecy::{ExposeSecret, SecretString};
use tokio_util::sync::CancellationToken;

#[derive(PartialEq, Eq)]
struct RedactedTestSecret(String);

impl std::fmt::Debug for RedactedTestSecret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("<redacted>")
    }
}

#[derive(Debug, PartialEq, Eq)]
struct RouteCall {
    model: String,
    auth_scheme: AuthScheme,
    auth_secret: RedactedTestSecret,
}

type RouteCalls = Arc<Mutex<Vec<RouteCall>>>;
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
                provenance: opi_ai::AuthProvenance::default(),
            })
        })
    }
}

struct RecordingRoute {
    id: String,
    models: Vec<ModelInfo>,
    calls: RouteCalls,
}

impl RecordingRoute {
    fn new(id: &str, models: Vec<ModelInfo>, calls: RouteCalls) -> Self {
        Self {
            id: id.into(),
            models,
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

    fn stream_prepared(&self, request: Request, auth: ResolvedAuth) -> EventStream {
        let calls = Arc::clone(&self.calls);
        Box::pin(futures_util::stream::once(async move {
            calls.lock().unwrap().push(RouteCall {
                model: request.model,
                auth_scheme: auth.scheme,
                auth_secret: RedactedTestSecret(auth.secret.expose_secret().to_owned()),
            });
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

struct CatalogRoute {
    id: String,
    models: Vec<ModelInfo>,
    observed_ids: Arc<Mutex<Vec<String>>>,
    reject_id: Option<String>,
}

impl CatalogRoute {
    fn new(
        models: Vec<ModelInfo>,
        observed_ids: Arc<Mutex<Vec<String>>>,
        reject_id: Option<&str>,
    ) -> Self {
        *observed_ids.lock().unwrap() = models.iter().map(|model| model.id.clone()).collect();
        Self {
            id: "acme".into(),
            models,
            observed_ids,
            reject_id: reject_id.map(str::to_owned),
        }
    }
}

impl Provider for CatalogRoute {
    fn id(&self) -> &str {
        &self.id
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    fn stream_prepared(&self, _request: Request, _auth: opi_ai::auth::ResolvedAuth) -> EventStream {
        Box::pin(futures_util::stream::empty())
    }

    fn replace_model_catalog(&mut self, models: Vec<ModelInfo>) -> Result<(), ProviderError> {
        if self
            .reject_id
            .as_ref()
            .is_some_and(|reject_id| models.iter().any(|model| model.id == *reject_id))
        {
            return Err(ProviderError::Config(
                opi_ai::provider::ProviderErrorSummary::redacted(),
            ));
        }
        *self.observed_ids.lock().unwrap() = models.iter().map(|model| model.id.clone()).collect();
        self.models = models;
        Ok(())
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
            .stream_prepared(
                request(&format!("acme:{id}")),
                opi_ai::test_support::resolved_auth(),
            )
            .next()
            .await
            .unwrap()
            .unwrap();
        assert!(event.is_terminal());
        assert_eq!(
            logs[&wire].lock().unwrap().as_slice(),
            &[RouteCall {
                model: format!("acme:{id}"),
                auth_scheme: AuthScheme::ApiKey,
                auth_secret: RedactedTestSecret("test-key".to_owned()),
            }]
        );
    }
    assert_eq!(provider.id(), "acme");
    assert_eq!(provider.models().len(), 3);
}

#[tokio::test]
async fn mapped_provider_uses_collection_prepared_auth_once_across_retries() {
    let (provider, auth, logs) = mapped_fixture();
    let mut collection = ProviderCollection::new();
    collection
        .register_route(
            Box::new(provider),
            auth.clone(),
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .unwrap();
    let prepared = collection
        .prepare_call("acme:responses", request("acme:responses"))
        .await
        .unwrap();

    for _ in 0..3 {
        prepared
            .start_attempt()
            .unwrap()
            .next()
            .await
            .unwrap()
            .unwrap();
    }
    assert_eq!(auth.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        logs[&WireApi::OpenAiResponses].lock().unwrap().as_slice(),
        [
            RouteCall {
                model: "acme:responses".to_owned(),
                auth_scheme: AuthScheme::Bearer,
                auth_secret: RedactedTestSecret("test-token".to_owned()),
            },
            RouteCall {
                model: "acme:responses".to_owned(),
                auth_scheme: AuthScheme::Bearer,
                auth_secret: RedactedTestSecret("test-token".to_owned()),
            },
            RouteCall {
                model: "acme:responses".to_owned(),
                auth_scheme: AuthScheme::Bearer,
                auth_secret: RedactedTestSecret("test-token".to_owned()),
            },
        ]
    );
    assert!(
        !format!("{:?}", logs[&WireApi::OpenAiResponses].lock().unwrap()).contains("test-token")
    );
}

#[tokio::test]
async fn unknown_model_fails_before_route_or_network() {
    let (provider, auth, logs) = mapped_fixture();
    let error = provider
        .stream_prepared(
            request("acme:unknown"),
            opi_ai::test_support::resolved_auth(),
        )
        .next()
        .await
        .unwrap()
        .unwrap_err();
    assert!(matches!(error, ProviderError::UnknownModel { .. }));
    assert_eq!(auth.calls.load(Ordering::SeqCst), 0);
    assert!(logs.values().all(|log| log.lock().unwrap().is_empty()));

    let error = provider
        .stream_prepared(request("other:chat"), opi_ai::test_support::resolved_auth())
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
    assert!(matches!(
        error,
        opi_ai::ApiMapError::InvalidModel {
            provider_id,
            model_id,
            source: ModelInfoError::WireCompatMismatch { .. },
        } if provider_id == "acme" && model_id == "chat"
    ));
}

#[test]
fn zero_model_token_limits_fail_at_construction() {
    let cases = [
        (
            "context_window",
            ModelCapabilities::new(0, 16_384).with_streaming(true),
        ),
        (
            "max_output_tokens",
            ModelCapabilities::new(128_000, 0).with_streaming(true),
        ),
    ];

    for (field, capabilities) in cases {
        let invalid = ModelInfo::new(
            format!("zero-{field}"),
            "Invalid",
            WireApi::OpenAiCompletions,
            capabilities,
        );
        let error = ApiMappedProvider::try_new("acme", vec![invalid], BTreeMap::new()).unwrap_err();
        assert!(matches!(
            error,
            opi_ai::ApiMapError::InvalidModel {
                provider_id,
                model_id,
                source: ModelInfoError::InvalidCapabilities {
                    field: invalid_field,
                    ..
                },
            } if provider_id == "acme"
                && model_id == format!("zero-{field}")
                && invalid_field == field
        ));
    }
}

#[test]
fn detached_default_model_capabilities_remain_constructible() {
    let capabilities = ModelCapabilities::default();
    assert_eq!(capabilities.context_window, 0);
    assert_eq!(capabilities.max_output_tokens, 0);
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

    let mut routes: BTreeMap<WireApi, Box<dyn Provider>> = BTreeMap::new();
    routes.insert(
        WireApi::OpenAiCompletions,
        Box::new(RecordingRoute::new(
            "hidden-route",
            vec![model("chat", WireApi::OpenAiCompletions)],
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
fn mapped_provider_rejects_route_model_with_different_capabilities() {
    let catalog_model = model("chat", WireApi::OpenAiCompletions);
    let route_model = ModelInfo::new(
        "chat",
        "chat",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(64_000, 8_192).with_streaming(true),
    );
    let routes = [(
        WireApi::OpenAiCompletions,
        Box::new(RecordingRoute::new(
            "acme",
            vec![route_model],
            Arc::new(Mutex::new(Vec::new())),
        )) as Box<dyn Provider>,
    )];

    let error = ApiMappedProvider::try_new("acme", vec![catalog_model], routes).unwrap_err();
    assert!(matches!(
        error,
        opi_ai::ApiMapError::RouteCatalogMismatch {
            provider_id,
            wire_api: WireApi::OpenAiCompletions,
        } if provider_id == "acme"
    ));
}

#[test]
fn mapped_provider_rejects_route_catalog_subsets_and_supersets() {
    let catalog = vec![
        model("chat-a", WireApi::OpenAiCompletions),
        model("chat-b", WireApi::OpenAiCompletions),
    ];
    let route_catalogs = [
        vec![catalog[0].clone()],
        vec![
            catalog[0].clone(),
            catalog[1].clone(),
            model("chat-extra", WireApi::OpenAiCompletions),
        ],
    ];

    for route_catalog in route_catalogs {
        let routes = [(
            WireApi::OpenAiCompletions,
            Box::new(RecordingRoute::new(
                "acme",
                route_catalog,
                Arc::new(Mutex::new(Vec::new())),
            )) as Box<dyn Provider>,
        )];
        let error = ApiMappedProvider::try_new("acme", catalog.clone(), routes).unwrap_err();
        assert!(matches!(
            error,
            opi_ai::ApiMapError::RouteCatalogMismatch {
                provider_id,
                wire_api: WireApi::OpenAiCompletions,
            } if provider_id == "acme"
        ));
    }
}

#[test]
fn mapped_catalog_replacement_preflights_empty_route_before_mutating_any_route() {
    let first_ids = Arc::new(Mutex::new(Vec::new()));
    let second_ids = Arc::new(Mutex::new(Vec::new()));
    let old_catalog = vec![
        model("anthropic-old", WireApi::AnthropicMessages),
        model("chat-old", WireApi::OpenAiCompletions),
    ];
    let routes = [
        (
            WireApi::AnthropicMessages,
            Box::new(CatalogRoute::new(
                vec![old_catalog[0].clone()],
                Arc::clone(&first_ids),
                None,
            )) as Box<dyn Provider>,
        ),
        (
            WireApi::OpenAiCompletions,
            Box::new(CatalogRoute::new(
                vec![old_catalog[1].clone()],
                Arc::clone(&second_ids),
                None,
            )) as Box<dyn Provider>,
        ),
    ];
    let mut provider = ApiMappedProvider::try_new("acme", old_catalog, routes).unwrap();

    let error = provider
        .replace_model_catalog(vec![model("anthropic-new", WireApi::AnthropicMessages)])
        .unwrap_err();

    assert!(error.to_string().contains("would leave route"));
    assert_eq!(first_ids.lock().unwrap().as_slice(), &["anthropic-old"]);
    assert_eq!(second_ids.lock().unwrap().as_slice(), &["chat-old"]);
    assert_eq!(provider.models()[0].id, "anthropic-old");
}

#[test]
fn mapped_catalog_replacement_rolls_back_routes_after_late_rejection() {
    let first_ids = Arc::new(Mutex::new(Vec::new()));
    let second_ids = Arc::new(Mutex::new(Vec::new()));
    let old_catalog = vec![
        model("anthropic-old", WireApi::AnthropicMessages),
        model("chat-old", WireApi::OpenAiCompletions),
    ];
    let routes = [
        (
            WireApi::AnthropicMessages,
            Box::new(CatalogRoute::new(
                vec![old_catalog[0].clone()],
                Arc::clone(&first_ids),
                None,
            )) as Box<dyn Provider>,
        ),
        (
            WireApi::OpenAiCompletions,
            Box::new(CatalogRoute::new(
                vec![old_catalog[1].clone()],
                Arc::clone(&second_ids),
                Some("chat-rejected"),
            )) as Box<dyn Provider>,
        ),
    ];
    let mut provider = ApiMappedProvider::try_new("acme", old_catalog, routes).unwrap();
    let replacement = vec![
        model("anthropic-new", WireApi::AnthropicMessages),
        model("chat-rejected", WireApi::OpenAiCompletions),
    ];

    let error = provider.replace_model_catalog(replacement).unwrap_err();

    assert_eq!(
        error.to_string(),
        "invalid provider configuration: [REDACTED]"
    );
    assert_eq!(first_ids.lock().unwrap().as_slice(), &["anthropic-old"]);
    assert_eq!(second_ids.lock().unwrap().as_slice(), &["chat-old"]);
    assert_eq!(
        provider
            .models()
            .iter()
            .map(|model| model.id.as_str())
            .collect::<Vec<_>>(),
        ["anthropic-old", "chat-old"]
    );
}

#[test]
fn provider_headers_reject_all_reserved_configured_and_request_names() {
    const RESERVED_NAMES: [&str; 13] = [
        "authorization",
        "x-api-key",
        "api-key",
        "anthropic-version",
        "anthropic-beta",
        "content-type",
        "chatgpt-account-id",
        "openai-beta",
        "session-id",
        "session_id",
        "x-client-request-id",
        "x-session-affinity",
        "x-initiator",
    ];

    let headers = ProviderHeaders::try_new(vec![("X-Acme".into(), "opi".into())]).unwrap();
    let merged = headers
        .merge_request(
            &[("X-Route".into(), "static".into())],
            &[("X-Request".into(), "dynamic".into())],
        )
        .unwrap();
    assert_eq!(merged.len(), 3);

    for reserved in RESERVED_NAMES {
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
async fn concrete_custom_route_ignores_credential_base_url_and_uses_model_base_url() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let auth_server = MockServer::start().await;
    let model_server = MockServer::start().await;
    let default_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(403).set_body_string("echoed-key-canary-must-not-surface"),
        )
        .mount(&model_server)
        .await;
    let routed_model =
        model("claude", WireApi::AnthropicMessages).with_base_url(model_server.uri());
    let route = opi_ai::anthropic::AnthropicProvider::for_route(
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

    // The resolved auth carries the credential's base_url; a non-Copilot route
    // must ignore it and dispatch to the model's own base_url instead.
    let resolved = ResolvedAuth {
        scheme: AuthScheme::Bearer,
        secret: SecretString::from("oauth-token"),
        base_url: Some(auth_server.uri()),
        account_id: None,
        provenance: opi_ai::AuthProvenance::default(),
    };
    let error = provider
        .stream_prepared(request("acme:claude"), resolved)
        .next()
        .await
        .unwrap()
        .unwrap_err();
    assert!(matches!(error, ProviderError::AuthFailed(_)));
    let rendered = format!("{error:?} {error}");
    assert!(!rendered.contains("echoed-key-canary-must-not-surface"));
    assert!(auth_server.received_requests().await.unwrap().is_empty());
    let requests = model_server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0]
            .headers
            .get("x-acme")
            .and_then(|value| value.to_str().ok()),
        Some("opi")
    );
    assert!(requests[0].headers.get("anthropic-beta").is_none());
    assert!(default_server.received_requests().await.unwrap().is_empty());
}
