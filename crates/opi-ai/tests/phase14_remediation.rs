use std::sync::{Arc, Mutex};

use futures_util::StreamExt;
use opi_ai::anthropic::AnthropicProvider;
use opi_ai::auth::{AuthResolver, AuthScheme, ResolvedAuth};
use opi_ai::credential::BoxAuthFuture;
use opi_ai::http::HttpClient;
use opi_ai::model_info::{
    AnthropicMessagesCompat, ThinkingLevel, ThinkingLevelMap, ThinkingLevelMapping, WireCompat,
};
use opi_ai::openai_chat::OpenAiChatEvent;
use opi_ai::provider::{
    CacheRetention, EventStream, Provider, ProviderError, Request, ThinkingConfig,
};
use opi_ai::provider_collection::{AuthDescriptor, CompatMetadata, ProviderCollection, SecretKey};
use opi_ai::stream::{CumulativeUsage, Pricing, calculate_cumulative_cost};
use opi_ai::{ApiMappedProvider, ModelCapabilities, ModelInfo, WireApi};
use secrecy::SecretString;
use tokio_util::sync::CancellationToken;

fn request(model: &str) -> Request {
    Request {
        model: model.into(),
        system: None,
        messages: Vec::new(),
        tools: Vec::new(),
        max_tokens: None,
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: Vec::new(),
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: Vec::new(),
        cache_retention: CacheRetention::None,
        session_id: None,
    }
}

#[derive(Default)]
struct CountingResolver {
    calls: std::sync::atomic::AtomicUsize,
}

impl AuthResolver for CountingResolver {
    fn resolve<'a>(&'a self) -> BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Box::pin(async {
            Ok(ResolvedAuth {
                scheme: AuthScheme::ApiKey,
                secret: SecretString::from("unused"),
                base_url: Some("http://127.0.0.1:9".into()),
                account_id: None,
            })
        })
    }
}

#[tokio::test]
async fn direct_anthropic_rejects_unsupported_thinking_before_auth() {
    let resolver = Arc::new(CountingResolver::default());
    let model = ModelInfo::new(
        "thinking-model",
        "Thinking Model",
        WireApi::AnthropicMessages,
        ModelCapabilities::new(1000, 100).with_thinking(true),
    );
    let provider = AnthropicProvider::for_route(
        resolver.clone(),
        "test".into(),
        vec![model],
        None,
        Default::default(),
        Arc::new(HttpClient::new()),
        false,
    );
    let mut request = request("test:thinking-model");
    request.thinking = ThinkingConfig {
        enabled: true,
        budget_tokens: None,
        level: ThinkingLevel::XHigh,
    };

    let error = provider
        .stream(request)
        .next()
        .await
        .expect("preflight result")
        .expect_err("unsupported thinking must fail");

    assert!(matches!(error, ProviderError::UnsupportedCapability { .. }));
    assert_eq!(
        resolver.calls.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "preflight must run before auth resolution"
    );
}

#[test]
fn anthropic_request_honors_adaptive_thinking_and_temperature_compat() {
    let model = ModelInfo::new(
        "copilot-model",
        "Copilot Model",
        WireApi::AnthropicMessages,
        ModelCapabilities::new(1000, 100).with_thinking(true),
    )
    .with_compat(WireCompat::AnthropicMessages(AnthropicMessagesCompat {
        supports_eager_tool_input_streaming: false,
        force_adaptive_thinking: true,
        supports_temperature: false,
    }))
    .unwrap();
    let provider = AnthropicProvider::for_route(
        Arc::new(CountingResolver::default()),
        "copilot".into(),
        vec![model],
        None,
        Default::default(),
        Arc::new(HttpClient::new()),
        false,
    );
    let mut request = request("copilot:copilot-model");
    request.temperature = Some(0.4);
    request.thinking = ThinkingConfig {
        enabled: true,
        budget_tokens: Some(4096),
        level: ThinkingLevel::High,
    };

    let body = provider.build_request_body(&request);

    assert!(body.get("temperature").is_none());
    assert_eq!(body["thinking"], serde_json::json!({"type": "adaptive"}));
}

#[test]
fn anthropic_direct_catalog_preserves_fixed_thinking_and_temperature() {
    let provider = AnthropicProvider::new("unused".into(), None);
    let mut request = request("anthropic:claude-sonnet-4-5-20250514");
    request.temperature = Some(0.4);
    request.thinking = ThinkingConfig {
        enabled: true,
        budget_tokens: Some(4096),
        level: ThinkingLevel::High,
    };

    let body = provider.build_request_body(&request);

    assert_eq!(body["temperature"], 0.4);
    assert_eq!(
        body["thinking"],
        serde_json::json!({"type": "enabled", "budget_tokens": 4096})
    );
}

#[test]
fn initial_chat_tool_call_arguments_are_emitted_after_start() {
    let chunk = serde_json::json!({
        "id": "response-1",
        "model": "gpt-4o",
        "choices": [{
            "index": 0,
            "delta": {
                "tool_calls": [{
                    "index": 0,
                    "id": "call-1",
                    "type": "function",
                    "function": {"name": "read", "arguments": "{\"path\":"}
                }]
            }
        }]
    });
    let sse = format!("data: {chunk}\n\n");
    let events = match opi_ai::openai_chat::parse_sse_events(&sse)
        .next()
        .expect("one frame")
    {
        opi_ai::openai_chat::ParsedEvent::Valid(events) => events,
        opi_ai::openai_chat::ParsedEvent::UsageError(error) => panic!("{error}"),
        opi_ai::openai_chat::ParsedEvent::Malformed { error, .. } => panic!("{error}"),
    };

    assert!(matches!(
        events.first(),
        Some(OpenAiChatEvent::ToolCallStart { .. })
    ));
    assert!(matches!(
        events.get(1),
        Some(OpenAiChatEvent::ToolCallDelta { arguments, .. }) if arguments == "{\"path\":"
    ));
}

#[test]
fn model_metadata_rejects_thinking_map_without_thinking_capability() {
    let model = ModelInfo::new(
        "bad-thinking",
        "Bad Thinking",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(1000, 100),
    )
    .with_thinking_level_map(
        ThinkingLevelMap::disabled()
            .with_mapping(ThinkingLevel::High, ThinkingLevelMapping::Identity),
    );

    assert!(model.validate().is_err());
}

#[test]
fn model_metadata_rejects_long_cache_without_cache_control() {
    let model = ModelInfo::new(
        "bad-cache",
        "Bad Cache",
        WireApi::AnthropicMessages,
        ModelCapabilities::new(1000, 100).with_long_cache_retention(true),
    );

    assert!(model.validate().is_err());
}

struct CountingRoute {
    model: Vec<ModelInfo>,
    calls: Arc<std::sync::atomic::AtomicUsize>,
}

impl Provider for CountingRoute {
    fn id(&self) -> &str {
        "mapped"
    }

    fn models(&self) -> &[ModelInfo] {
        &self.model
    }

    fn stream(&self, _request: Request) -> EventStream {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Box::pin(futures_util::stream::empty())
    }

    fn replace_model_catalog(&mut self, models: Vec<ModelInfo>) -> Result<(), ProviderError> {
        self.model = models;
        Ok(())
    }
}

fn unsupported_xhigh_model() -> ModelInfo {
    ModelInfo::new(
        "reasoning",
        "Reasoning",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(1000, 100).with_thinking(true),
    )
}

#[tokio::test]
async fn mapped_provider_preflights_before_route_dispatch() {
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let model = unsupported_xhigh_model();
    let mapped = ApiMappedProvider::try_new(
        "mapped",
        vec![model.clone()],
        [(
            WireApi::OpenAiCompletions,
            Box::new(CountingRoute {
                model: vec![model],
                calls: calls.clone(),
            }) as Box<dyn Provider>,
        )],
    )
    .unwrap();
    let mut request = request("mapped:reasoning");
    request.thinking = ThinkingConfig {
        enabled: true,
        budget_tokens: None,
        level: ThinkingLevel::XHigh,
    };

    let error = mapped
        .stream(request)
        .next()
        .await
        .expect("preflight result")
        .expect_err("unsupported thinking must fail");

    assert!(matches!(error, ProviderError::UnsupportedCapability { .. }));
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
}

#[test]
fn mapped_provider_materializes_an_effective_model_catalog_into_routes() {
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let original = unsupported_xhigh_model();
    let mut mapped = ApiMappedProvider::try_new(
        "mapped",
        vec![original.clone()],
        [(
            WireApi::OpenAiCompletions,
            Box::new(CountingRoute {
                model: vec![original.clone()],
                calls: calls.clone(),
            }) as Box<dyn Provider>,
        )],
    )
    .unwrap();
    let added = ModelInfo::new(
        "added",
        "Added",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(2000, 200),
    );

    mapped
        .replace_model_catalog(vec![original, added])
        .expect("effective catalog");
    let stream = mapped.stream(request("mapped:added"));

    drop(stream);
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert!(mapped.models().iter().any(|model| model.id == "added"));
}

#[test]
fn collection_preflights_registry_model_before_provider_dispatch() {
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut collection = ProviderCollection::new();
    collection
        .register(
            Box::new(CountingRoute {
                model: vec![unsupported_xhigh_model()],
                calls: calls.clone(),
            }),
            AuthDescriptor::StaticApiKey {
                value: SecretKey::new("unused"),
            },
            CompatMetadata::default(),
        )
        .unwrap();
    let mut request = request("mapped:reasoning");
    request.thinking = ThinkingConfig {
        enabled: true,
        budget_tokens: None,
        level: ThinkingLevel::XHigh,
    };

    let error = match collection.dispatch_stream("mapped:reasoning", request) {
        Ok(_) => panic!("unsupported thinking must fail"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("unsupported"));
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
}

#[test]
fn cumulative_cost_uses_exact_u64_totals_above_public_usage_range() {
    let total = u64::from(u32::MAX) + 1_000_000;
    let usage = CumulativeUsage::from_totals(
        total,
        total + 1,
        total + 2,
        total + 3,
        Some(total / 2),
        None,
        1,
        0,
    );
    let pricing = Pricing {
        input_cost_per_mtok: 1.0,
        output_cost_per_mtok: 2.0,
        cache_read_cost_per_mtok: 0.5,
        cache_write_cost_per_mtok: 1.25,
    };

    let cost = calculate_cumulative_cost(&usage, &pricing);
    let one_hour = total / 2;
    let short = (total + 3) - one_hour;
    let expected = total as f64 / 1_000_000.0
        + (total + 1) as f64 * 2.0 / 1_000_000.0
        + (total + 2) as f64 * 0.5 / 1_000_000.0
        + short as f64 * 1.25 / 1_000_000.0
        + one_hour as f64 * 2.0 / 1_000_000.0;

    assert_eq!(usage.as_usage().input_tokens, u32::MAX);
    assert!((cost.total_cost() - expected).abs() < 1e-9);
}

struct LoggedRefreshProvider {
    id: &'static str,
    log: Arc<Mutex<Vec<String>>>,
    result: Result<Option<Vec<ModelInfo>>, &'static str>,
}

impl Provider for LoggedRefreshProvider {
    fn id(&self) -> &str {
        self.id
    }

    fn models(&self) -> &[ModelInfo] {
        &[]
    }

    fn stream(&self, _request: Request) -> EventStream {
        Box::pin(futures_util::stream::empty())
    }

    fn refresh_models(&self) -> BoxAuthFuture<'_, Result<Option<Vec<ModelInfo>>, ProviderError>> {
        self.log.lock().unwrap().push(self.id.into());
        let result = self.result.clone();
        Box::pin(
            async move { result.map_err(|message| ProviderError::ProviderSide(message.into())) },
        )
    }
}

#[tokio::test]
async fn refresh_collects_every_provider_after_the_first_error() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut collection = ProviderCollection::new();
    for provider in [
        LoggedRefreshProvider {
            id: "a-error",
            log: log.clone(),
            result: Err("first"),
        },
        LoggedRefreshProvider {
            id: "b-success",
            log: log.clone(),
            result: Ok(Some(vec![ModelInfo::new(
                "fresh",
                "Fresh",
                WireApi::OpenAiCompletions,
                ModelCapabilities::new(1000, 100),
            )])),
        },
        LoggedRefreshProvider {
            id: "c-error",
            log: log.clone(),
            result: Err("later"),
        },
    ] {
        collection
            .register(
                Box::new(provider),
                AuthDescriptor::StaticApiKey {
                    value: SecretKey::new("unused"),
                },
                CompatMetadata::default(),
            )
            .unwrap();
    }

    let error = collection.refresh().await.expect_err("batch must fail");

    assert!(error.to_string().contains("first"));
    assert_eq!(
        *log.lock().unwrap(),
        ["a-error", "b-success", "c-error"],
        "refresh must call every provider in sorted order"
    );
    assert!(collection.resolve("b-success:fresh").is_err());
}
