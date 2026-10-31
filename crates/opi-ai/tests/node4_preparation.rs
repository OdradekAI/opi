use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[path = "support/catalog_store.rs"]
mod catalog_fixture;

use futures_util::future::BoxFuture;
use opi_ai::auth::{AuthScheme, ResolvedAuth, StaticAuthResolver};
use opi_ai::catalog::{DynamicCatalogSource, ModelsStore};
use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
use opi_ai::message::{
    AssistantContent, AssistantMessage, InputContent, Message, ToolDef, UserMessage,
};
use opi_ai::model_info::{AnthropicMessagesCompat, WireCompat};
use opi_ai::models::{ApiId, ChatApi, ChatApiHandle};
use opi_ai::provider::{
    CacheRetention, EventStream, Provider, ProviderError, Request, ThinkingConfig,
};
use opi_ai::stream::{ChatStream, StopReason, Usage};
use opi_ai::transcript::{Context, SystemMessage, normalize_context};
use opi_ai::{
    AuthProvenanceSource, ChatModelRecord, ModelCapabilities, ModelInfo, Models, WireApi,
};
use secrecy::SecretString;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

const CHAT_OK: &str = "data: {\"id\":\"chat-test\",\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"ok\"},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1}}\n\ndata: [DONE]\n\n";
const RESPONSES_OK: &str = "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"response-test\",\"model\":\"gpt-4o\",\"output\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}}\n\n";

fn request(model: &str) -> Request {
    Request {
        model: model.into(),
        context: normalize_context(Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "hello".into(),
                }],
                timestamp_ms: 1,
            })],
            tools: vec![],
        })
        .unwrap(),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::Disabled,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
        transport: Default::default(),
    }
}

struct RealWireApi {
    id: ApiId,
    provider: Arc<dyn Provider>,
    record: ModelInfo,
}

impl ChatApi<()> for RealWireApi {
    fn api_id(&self) -> &ApiId {
        &self.id
    }
    fn provider_id(&self) -> &str {
        self.provider.id()
    }
    fn complete_record(&self) -> ModelInfo {
        self.record.clone()
    }
    fn simple_options(&self) {}
    fn stream_prepared(&self, request: Request, (): (), auth: ResolvedAuth) -> EventStream {
        self.provider.stream_prepared(request, auth)
    }
}

async fn real_wire(
    wire: WireApi,
    context_window: u64,
    max_output: u64,
) -> (MockServer, Models, ChatApiHandle<()>, ChatModelRecord) {
    let server = MockServer::start().await;
    let (provider, sse): (Arc<dyn Provider>, _) = match wire {
        WireApi::OpenAiCompletions => (
            Arc::new(opi_ai::openai_chat::OpenAiChatProvider::new(Some(
                server.uri(),
            ))),
            CHAT_OK,
        ),
        WireApi::OpenAiResponses => (
            Arc::new(opi_ai::openai_responses::OpenAiResponsesProvider::new(
                Some(server.uri()),
            )),
            RESPONSES_OK,
        ),
        _ => unreachable!("the fixture only exposes the two migrated OpenAI wires"),
    };
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse),
        )
        .mount(&server)
        .await;
    let provider_id = provider.id().to_owned();
    let record = ModelInfo::new(
        "gpt-4o",
        "Test model",
        wire,
        ModelCapabilities::new(context_window, max_output).with_streaming(true),
    );
    let mut models = Models::new();
    let handle = models
        .register_chat_api(
            Arc::new(RealWireApi {
                id: ApiId::new(format!("test/{}", wire.as_str())),
                provider,
                record: record.clone(),
            }),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("fake-key"),
            )),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    (
        server,
        models,
        handle,
        ChatModelRecord::new(provider_id, record),
    )
}

async fn bodies(server: &MockServer) -> Vec<serde_json::Value> {
    server
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .map(|request| serde_json::from_slice(&request.body).unwrap())
        .collect()
}

#[tokio::test]
async fn native_chat_omits_absent_max_while_simple_resolves_the_default() {
    let (server, models, api, record) = real_wire(WireApi::OpenAiCompletions, 10_000, 1_000).await;
    models
        .complete(record.clone(), request("openai:gpt-4o"), api.options(()))
        .await
        .unwrap();
    models
        .complete_simple(record, request("openai:gpt-4o"), Default::default())
        .await
        .unwrap();
    let sent = bodies(&server).await;
    assert!(sent[0].get("max_tokens").is_none());
    assert!(sent[0].get("max_completion_tokens").is_none());
    assert_eq!(sent[1]["max_completion_tokens"], 1_000);
}

#[tokio::test]
async fn native_chat_explicit_limit_is_not_context_clamped() {
    let (server, models, api, record) = real_wire(WireApi::OpenAiCompletions, 10_000, 1_000).await;
    let mut input = request("openai:gpt-4o");
    input.max_tokens = Some(20_000);
    models
        .complete(record, input, api.options(()))
        .await
        .unwrap();
    assert_eq!(bodies(&server).await[0]["max_completion_tokens"], 20_000);
}

#[tokio::test]
async fn native_responses_zero_omits_max_output_tokens() {
    let (server, models, api, record) = real_wire(WireApi::OpenAiResponses, 10_000, 1_000).await;
    let mut input = request("openai-responses:gpt-4o");
    input.max_tokens = Some(0);
    models
        .complete(record, input, api.options(()))
        .await
        .unwrap();
    assert!(bodies(&server).await[0].get("max_output_tokens").is_none());
}

#[tokio::test]
async fn native_responses_truthy_small_limit_uses_the_wire_floor() {
    let (server, models, api, record) = real_wire(WireApi::OpenAiResponses, 10_000, 1_000).await;
    for limit in [1, 15, 16, 17] {
        let mut input = request("openai-responses:gpt-4o");
        input.max_tokens = Some(limit);
        models
            .complete(record.clone(), input, api.options(()))
            .await
            .unwrap();
    }
    let sent = bodies(&server).await;
    assert_eq!(
        sent.iter()
            .map(|body| body["max_output_tokens"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        vec![16, 16, 16, 17]
    );
}

#[tokio::test]
async fn simple_preparation_counts_canonical_system_changes_after_the_usage_anchor() {
    let (server, models, _api, record) = real_wire(WireApi::OpenAiCompletions, 10_000, 9_000).await;
    let mut input = request("openai:gpt-4o");
    input.max_tokens = Some(9_000);
    input.context = normalize_context(Context {
        system_prompt: None,
        messages: vec![
            Message::Assistant(AssistantMessage {
                content: vec![AssistantContent::Text {
                    text: "done".into(),
                    signature: None,
                }],
                api: opi_ai::ApiKind::OpenAi,
                deferred: None,
                provider: "openai".into(),
                model: "gpt-4o".into(),
                response_model: None,
                wire: Some(WireApi::OpenAiCompletions),
                response_id: None,
                usage: Usage::reported(500, 20, 100, 50, None, None),
                stop_reason: StopReason::Stop,
                error_message: None,
                timestamp_ms: 2,
                diagnostics: vec![],
                duration_ms: None,
                raw_stop_reason: None,
                provider_thinking_level: None,
            }),
            Message::System(SystemMessage {
                content: "s".repeat(3_500),
                timestamp_ms: 3,
                ..Default::default()
            }),
        ],
        tools: vec![],
    })
    .unwrap();
    models
        .complete_simple(record, input, Default::default())
        .await
        .unwrap();
    // Positive usage 670 plus the later 3500 UTF-16 units / 3.5 = 1000.
    assert_eq!(bodies(&server).await[0]["max_completion_tokens"], 4_234);
}

struct SuccessThenFailureSource {
    calls: AtomicUsize,
}

impl DynamicCatalogSource for SuccessThenFailureSource {
    fn id(&self) -> &str {
        "explicit-source"
    }
    fn fetch(
        &self,
        _token: CancellationToken,
    ) -> BoxFuture<'static, Result<HashMap<String, opi_ai::catalog::ModelsStoreEntry>, ProviderError>>
    {
        let first = self.calls.fetch_add(1, Ordering::SeqCst) == 0;
        Box::pin(async move {
            if first {
                Ok(catalog_fixture::catalogs(HashMap::from([(
                    "openai".into(),
                    vec![ModelInfo::new(
                        "source-only",
                        "Published source model",
                        WireApi::OpenAiCompletions,
                        ModelCapabilities::new(10_000, 1_000),
                    )],
                )])))
            } else {
                Err(ProviderError::ProviderSide(
                    opi_ai::provider::ProviderErrorSummary::from_untrusted("source unavailable"),
                ))
            }
        })
    }
}

#[tokio::test]
async fn failed_explicit_source_retains_catalog_when_static_provider_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let store_path = dir.path().join("catalog.json");
    let mut models = Models::new();
    models
        .register_route(
            Box::new(opi_ai::openai_chat::OpenAiChatProvider::new(None)),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("fake-key"),
            )),
            AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    models.set_models_store(ModelsStore::open(&store_path).unwrap());
    models.register_catalog_source(Arc::new(SuccessThenFailureSource {
        calls: AtomicUsize::new(0),
    }));
    models.refresh(true).await.unwrap();
    assert!(models.resolve("openai:source-only").is_ok());
    let report = models.refresh(true).await.unwrap();
    assert!(report.find("explicit-source").unwrap().outcome.is_failure());
    assert!(
        models.resolve("openai:source-only").is_ok(),
        "failed source retains memory revision"
    );
    let restored = ModelsStore::open(&store_path).unwrap().snapshot();
    assert_eq!(restored.catalogs["openai"].models[0].id(), "source-only");
}

struct AuthoritativeEmptyProvider {
    inner: opi_ai::openai_chat::OpenAiChatProvider,
    rounds: AtomicUsize,
}

impl Provider for AuthoritativeEmptyProvider {
    fn id(&self) -> &str {
        self.inner.id()
    }
    fn models(&self) -> &[ModelInfo] {
        self.inner.models()
    }
    fn stream_prepared(&self, request: Request, auth: ResolvedAuth) -> EventStream {
        self.inner.stream_prepared(request, auth)
    }
    fn refresh_models(
        &self,
    ) -> opi_ai::credential::BoxAuthFuture<'_, Result<Option<Vec<ModelInfo>>, ProviderError>> {
        let first = self.rounds.fetch_add(1, Ordering::SeqCst) == 0;
        Box::pin(async move { Ok(if first { None } else { Some(vec![]) }) })
    }
}

#[tokio::test]
async fn explicit_source_failure_does_not_revoke_a_providers_authoritative_empty() {
    let dir = tempfile::tempdir().unwrap();
    let store_path = dir.path().join("catalog.json");
    let mut models = Models::new();
    models
        .register_route(
            Box::new(AuthoritativeEmptyProvider {
                inner: opi_ai::openai_chat::OpenAiChatProvider::new(None),
                rounds: AtomicUsize::new(0),
            }),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("fake-key"),
            )),
            AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    models.set_models_store(ModelsStore::open(&store_path).unwrap());
    models.register_catalog_source(Arc::new(SuccessThenFailureSource {
        calls: AtomicUsize::new(0),
    }));
    models.refresh(true).await.unwrap();
    assert!(models.resolve("openai:source-only").is_ok());
    models.refresh(true).await.unwrap();
    assert!(models.resolve("openai:source-only").is_err());
    assert!(models.resolve("openai:gpt-4o").is_err());
    let snapshot = ModelsStore::open(&store_path).unwrap().snapshot();
    assert!(snapshot.catalogs["openai"].models.is_empty());
}

struct NativeThenAbsentProvider {
    id: &'static str,
    calls: AtomicUsize,
}

impl Provider for NativeThenAbsentProvider {
    fn id(&self) -> &str {
        self.id
    }
    fn models(&self) -> &[ModelInfo] {
        &[]
    }
    fn stream_prepared(&self, _: Request, _: ResolvedAuth) -> EventStream {
        unreachable!("catalog-only provider")
    }
    fn refresh_models(
        &self,
    ) -> opi_ai::BoxAuthFuture<'_, Result<Option<Vec<ModelInfo>>, ProviderError>> {
        let publish = self.calls.fetch_add(1, Ordering::SeqCst) == 0;
        Box::pin(async move {
            Ok(publish.then(|| {
                vec![ModelInfo::new(
                    "native-old",
                    "Native old",
                    WireApi::OpenAiCompletions,
                    ModelCapabilities::new(10_000, 1_000),
                )]
            }))
        })
    }
}

struct IndependentSource {
    fail: bool,
}

impl DynamicCatalogSource for IndependentSource {
    fn id(&self) -> &str {
        "source-b"
    }
    fn fetch(
        &self,
        _: CancellationToken,
    ) -> BoxFuture<'static, Result<HashMap<String, opi_ai::catalog::ModelsStoreEntry>, ProviderError>>
    {
        let fail = self.fail;
        Box::pin(async move {
            if fail {
                Err(ProviderError::Network(
                    opi_ai::provider::ProviderErrorSummary::from_untrusted("unavailable"),
                ))
            } else {
                Ok(catalog_fixture::catalogs(HashMap::from([(
                    "provider-b".into(),
                    vec![ModelInfo::new(
                        "source-b-model",
                        "Source B",
                        WireApi::OpenAiCompletions,
                        ModelCapabilities::new(10_000, 1_000),
                    )],
                )])))
            }
        })
    }
}

fn register_native_catalog_provider(
    models: &mut Models,
    id: &'static str,
    already_published: bool,
) {
    models
        .register_route(
            Box::new(NativeThenAbsentProvider {
                id,
                calls: AtomicUsize::new(usize::from(already_published)),
            }),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("fake-key"),
            )),
            AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
}

#[tokio::test]
async fn unrelated_explicit_source_does_not_block_native_none_clear() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("catalog.json");
    let mut models = Models::new();
    register_native_catalog_provider(&mut models, "provider-a", false);
    register_native_catalog_provider(&mut models, "provider-b", true);
    models.set_models_store(ModelsStore::open(&path).unwrap());
    models.register_catalog_source(Arc::new(IndependentSource { fail: false }));
    models.refresh(true).await.unwrap();
    assert!(models.resolve("provider-a:native-old").is_ok());
    assert!(models.resolve("provider-b:source-b-model").is_ok());
    let persisted: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(
        persisted["providers"]["provider-a"]["x-opi"]["publisher"],
        serde_json::json!({"kind":"native"})
    );
    assert_eq!(
        persisted["providers"]["provider-b"]["x-opi"]["publisher"],
        serde_json::json!({"kind":"source","id":"source-b"})
    );
    models.refresh(true).await.unwrap();
    assert!(
        models.resolve("provider-a:native-old").is_err(),
        "source B cannot retain native A's revoked snapshot"
    );
    assert!(models.resolve("provider-b:source-b-model").is_ok());
    assert!(
        !ModelsStore::open(&path)
            .unwrap()
            .snapshot()
            .catalogs
            .contains_key("provider-a")
    );
}

#[tokio::test]
async fn restored_native_catalog_clears_despite_unrelated_source_failure() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("catalog.json");
    let mut first = Models::new();
    register_native_catalog_provider(&mut first, "provider-a", false);
    register_native_catalog_provider(&mut first, "provider-b", true);
    first.set_models_store(ModelsStore::open(&path).unwrap());
    first.register_catalog_source(Arc::new(IndependentSource { fail: false }));
    first.refresh(true).await.unwrap();
    drop(first);

    let mut restored = Models::new();
    register_native_catalog_provider(&mut restored, "provider-a", true);
    register_native_catalog_provider(&mut restored, "provider-b", true);
    restored.set_models_store(ModelsStore::open(&path).unwrap());
    restored.register_catalog_source(Arc::new(IndependentSource { fail: true }));
    restored.refresh(false).await.unwrap();
    assert!(restored.resolve("provider-a:native-old").is_ok());
    assert!(restored.resolve("provider-b:source-b-model").is_ok());
    restored.refresh(true).await.unwrap();
    assert!(restored.resolve("provider-a:native-old").is_err());
    assert!(
        restored.resolve("provider-b:source-b-model").is_ok(),
        "source B's persisted publication keeps its owner"
    );
    let snapshot = ModelsStore::open(&path).unwrap().snapshot();
    assert!(!snapshot.catalogs.contains_key("provider-a"));
    assert_eq!(
        snapshot.catalogs["provider-b"].models[0].id(),
        "source-b-model"
    );
}

#[tokio::test]
async fn restored_source_catalog_retains_on_same_provider_failure() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("catalog.json");
    let mut first = Models::new();
    register_native_catalog_provider(&mut first, "openai", true);
    first.set_models_store(ModelsStore::open(&path).unwrap());
    first.register_catalog_source(Arc::new(SuccessThenFailureSource {
        calls: AtomicUsize::new(0),
    }));
    first.refresh(true).await.unwrap();
    drop(first);

    let mut restored = Models::new();
    register_native_catalog_provider(&mut restored, "openai", true);
    restored.set_models_store(ModelsStore::open(&path).unwrap());
    restored.register_catalog_source(Arc::new(SuccessThenFailureSource {
        calls: AtomicUsize::new(1),
    }));
    restored.refresh(true).await.unwrap();
    assert!(restored.resolve("openai:source-only").is_ok());
    assert_eq!(
        ModelsStore::open(&path).unwrap().snapshot().catalogs["openai"].models[0].id(),
        "source-only"
    );
}

#[tokio::test]
async fn legacy_catalog_has_unknown_publisher_and_malformed_publisher_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("catalog.json");
    let store = ModelsStore::open(&path).unwrap();
    store
        .begin()
        .commit(catalog_fixture::catalogs(HashMap::from([(
            "provider-a".into(),
            vec![ModelInfo::new(
                "legacy",
                "Legacy",
                WireApi::OpenAiCompletions,
                ModelCapabilities::new(10_000, 1_000),
            )],
        )])))
        .await
        .unwrap();
    let mut persisted: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert!(persisted["providers"]["provider-a"].get("x-opi").is_none());
    let mut models = Models::new();
    register_native_catalog_provider(&mut models, "provider-a", true);
    models.set_models_store(ModelsStore::open(&path).unwrap());
    models.register_catalog_source(Arc::new(IndependentSource { fail: true }));
    models.refresh(false).await.unwrap();
    assert!(models.resolve("provider-a:legacy").is_ok());
    models.refresh(true).await.unwrap();
    assert!(
        models.resolve("provider-a:legacy").is_err(),
        "unknown ownership is not attributed to an unrelated source"
    );
    for malformed in [
        serde_json::json!(false),
        serde_json::json!({"kind":"source","id":0}),
        serde_json::json!({"kind":"unexpected"}),
    ] {
        persisted["providers"]["provider-a"]["x-opi"] = serde_json::json!({"publisher":malformed});
        std::fs::write(&path, serde_json::to_vec(&persisted).unwrap()).unwrap();
        assert!(matches!(
            ModelsStore::open(&path),
            Err(opi_ai::catalog::ModelsStoreError::Corrupt { .. })
        ));
    }
}

struct ForwardingAnthropic(opi_ai::anthropic::AnthropicProvider);

impl Provider for ForwardingAnthropic {
    fn id(&self) -> &str {
        self.0.id()
    }
    fn models(&self) -> &[ModelInfo] {
        self.0.models()
    }
    fn stream_prepared(&self, request: Request, auth: ResolvedAuth) -> EventStream {
        self.0.stream_prepared(request, auth)
    }
}

#[tokio::test]
async fn forwarding_adapter_preserves_one_advanced_strict_and_retry_budget_owner() {
    let server = MockServer::start().await;
    for response in [
        ResponseTemplate::new(429).insert_header("retry-after", "0"),
        ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": {"type": "invalid_request_error", "message": "Too many strict tools (65)."}
        })),
        ResponseTemplate::new(429).insert_header("retry-after", "0"),
    ] {
        Mock::given(method("POST"))
            .respond_with(response)
            .up_to_n_times(1)
            .mount(&server)
            .await;
    }
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string("event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"model\":\"strict\",\"usage\":{\"input_tokens\":1,\"output_tokens\":0}}}\n\nevent: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":1}}\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}\n\n"))
        .mount(&server).await;
    let model = ModelInfo::new(
        "strict",
        "Strict",
        WireApi::AnthropicMessages,
        ModelCapabilities::new(100_000, 1_000),
    )
    .with_compat(WireCompat::AnthropicMessages(AnthropicMessagesCompat {
        supports_strict_tools: true,
        supports_eager_tool_input_streaming: false,
        force_adaptive_thinking: false,
        supports_temperature: true,
        supports_mid_convo_system_messages: false,
        supports_mid_convo_tool_changes: false,
    }))
    .unwrap();
    let provider = opi_ai::anthropic::AnthropicProvider::for_route(
        "test".into(),
        vec![model],
        Some(server.uri()),
        Default::default(),
        Arc::new(opi_ai::http::HttpClient::new()),
        false,
    );
    let mut models = Models::new();
    models
        .register_route(
            Box::new(ForwardingAnthropic(provider)),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                SecretString::from("fake-key"),
            )),
            AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    let mut input = request("test:strict");
    input.max_tokens = Some(64);
    input.context = normalize_context(Context {
        system_prompt: None, messages: vec![],
        tools: vec![ToolDef {
            name: "tool".into(), description: "d".into(),
            input_schema: serde_json::json!({"type":"object","properties":{"x":{"type":"string"}},"required":["x"]}),
            constrained_sampling: Some(ConstrainedSampling::JsonSchema { strict: StrictPreference::Prefer }),
        }],
    }).unwrap();
    let callbacks = Arc::new(AtomicUsize::new(0));
    let count = callbacks.clone();
    input.callbacks =
        opi_ai::callbacks::RequestCallbacks::new().with_on_payload(Arc::new(move |_| {
            count.fetch_add(1, Ordering::SeqCst);
            Ok(None)
        }));
    let call = models
        .prepare_call_with_policy(
            "test:strict",
            input,
            opi_ai::retry::AcquisitionPolicy::with_max_retries(1),
        )
        .await
        .unwrap();
    let first = ChatStream::new(call.start_attempt().unwrap())
        .result()
        .await
        .unwrap_err();
    assert!(matches!(first.cause(), ProviderError::RateLimited { .. }));
    let admission = call.retry_admission().unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(admission.delay_ms)).await;
    let strict = ChatStream::new(call.start_attempt().unwrap())
        .result()
        .await
        .unwrap_err();
    assert!(matches!(
        strict.cause(),
        ProviderError::StrictToolLimits { .. }
    ));
    assert!(call.strict_variant_admission().is_some());
    assert_eq!(
        bodies(&server).await.len(),
        2,
        "an inner driver cannot consume the strict branch"
    );
    let third = ChatStream::new(call.start_attempt().unwrap())
        .result()
        .await
        .unwrap_err();
    assert!(matches!(third.cause(), ProviderError::RateLimited { .. }));
    let admission = call.retry_admission().unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(admission.delay_ms)).await;
    ChatStream::new(call.start_attempt().unwrap())
        .result()
        .await
        .unwrap();
    let sent = bodies(&server).await;
    assert_eq!(sent.len(), 4);
    assert_eq!(sent[0]["tools"][0]["strict"], true);
    assert_eq!(sent[1]["tools"][0]["strict"], true);
    assert!(sent[2]["tools"][0].get("strict").is_none());
    assert!(sent[3]["tools"][0].get("strict").is_none());
    assert_eq!(callbacks.load(Ordering::SeqCst), 2);
}
