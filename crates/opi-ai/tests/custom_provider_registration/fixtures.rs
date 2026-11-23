//! Registry mocks and typed Chat adapters shared by this target.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use opi_ai::WireApi;
use opi_ai::api::contract::ChatApi;
use opi_ai::auth::{AuthResolver, AuthScheme, ResolvedAuth};
use opi_ai::credential::BoxAuthFuture;
use opi_ai::message::AssistantContent;
use opi_ai::provider::{ModelInfo, Provider, ProviderError, Request, ThinkingConfig};
use opi_ai::registry::ModelCapabilities;
use opi_ai::test_support::{MockProvider, text_response};
use opi_ai::types::ApiId;
use secrecy::{ExposeSecret, SecretString};
use tokio_util::sync::CancellationToken;

pub(super) fn custom_model(id: &str, display: &str) -> ModelInfo {
    ModelInfo::new(
        id,
        display,
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(50_000, 2_048).with_streaming(true),
    )
}

pub(super) fn custom_provider(id: &str, models: Vec<ModelInfo>) -> Box<dyn Provider> {
    let responses = vec![text_response("custom response")];
    let provider = MockProvider::new_with_models(id, models, responses);
    Box::new(provider) as Box<dyn Provider>
}

pub(super) fn assistant_text(message: &opi_ai::message::AssistantMessage) -> String {
    message
        .content
        .iter()
        .filter_map(|content| match content {
            AssistantContent::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// Typed native options for the echo adapter: the label must reach the
/// adapter unchanged.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct EchoOptions {
    pub(super) label: String,
}

/// A second, incompatible options type for the same ApiId.
#[derive(Clone, Debug)]
pub(super) struct OtherOptions {
    #[expect(dead_code)]
    pub(super) value: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ObservedCall {
    pub(super) selected_model: ModelInfo,
    pub(super) request_model: String,
    pub(super) max_tokens: Option<u64>,
    pub(super) temperature: Option<f64>,
    pub(super) options_label: String,
    pub(super) auth_secret: String,
}

pub(super) type Observed = Arc<Mutex<Vec<ObservedCall>>>;

/// One custom chat adapter: records the effective request, options, and
/// resolved-auth secret, then completes with a fixed text.
pub(super) struct EchoChatApi {
    pub(super) api: ApiId,
    pub(super) provider_id: String,
    pub(super) record: ModelInfo,
    pub(super) text: String,
    pub(super) observed: Observed,
}

impl ChatApi<EchoOptions> for EchoChatApi {
    fn api_id(&self) -> &ApiId {
        &self.api
    }

    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    fn complete_record(&self) -> ModelInfo {
        self.record.clone()
    }

    fn simple_options(&self) -> EchoOptions {
        EchoOptions {
            label: "simple".into(),
        }
    }

    fn stream_prepared(
        &self,
        request: Request,
        options: EchoOptions,
        auth: ResolvedAuth,
    ) -> opi_ai::provider::EventStream {
        self.record_call(self.record.clone(), request, options, auth)
    }

    fn stream_model_resource_prepared(
        &self,
        model: ModelInfo,
        request: Request,
        options: EchoOptions,
        auth: ResolvedAuth,
        _resources: Option<opi_ai::codex_resources::CallResourceLease>,
    ) -> opi_ai::provider::EventStream {
        self.record_call(model, request, options, auth)
    }
}

impl EchoChatApi {
    fn record_call(
        &self,
        model: ModelInfo,
        request: Request,
        options: EchoOptions,
        auth: ResolvedAuth,
    ) -> opi_ai::provider::EventStream {
        self.observed.lock().unwrap().push(ObservedCall {
            selected_model: model,
            request_model: request.model,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            options_label: options.label,
            auth_secret: auth.secret.expose_secret().to_owned(),
        });
        let events = text_response(&self.text);
        Box::pin(futures_util::stream::iter(events.into_iter().map(Ok)))
    }
}

/// An adapter with an incompatible options type, registered under the same
/// ApiId to prove changed-type failures.
pub(super) struct TypedChatApi {
    pub(super) api: ApiId,
    pub(super) provider_id: String,
    pub(super) record: ModelInfo,
}

impl ChatApi<OtherOptions> for TypedChatApi {
    fn api_id(&self) -> &ApiId {
        &self.api
    }

    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    fn complete_record(&self) -> ModelInfo {
        self.record.clone()
    }

    fn simple_options(&self) -> OtherOptions {
        OtherOptions { value: 0 }
    }

    fn stream_prepared(
        &self,
        _request: Request,
        _options: OtherOptions,
        _auth: ResolvedAuth,
    ) -> opi_ai::provider::EventStream {
        let events = text_response("typed");
        Box::pin(futures_util::stream::iter(events.into_iter().map(Ok)))
    }
}

pub(super) struct CountingResolver {
    pub(super) calls: AtomicUsize,
}

impl AuthResolver for CountingResolver {
    fn resolve(&self) -> BoxAuthFuture<'_, Result<ResolvedAuth, ProviderError>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {
            Ok(ResolvedAuth {
                env: Default::default(),
                scheme: AuthScheme::ApiKey,
                secret: SecretString::from("resolver-secret"),
                base_url: None,
                account_id: None,
                provenance: opi_ai::AuthProvenance::default(),
            })
        })
    }
}

pub(super) fn echo_record_info(max_output: u64) -> ModelInfo {
    ModelInfo::new(
        "echo-model",
        "Echo model",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(100_000, max_output).with_streaming(true),
    )
}

pub(super) fn models_request() -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: String::new(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: Some("be terse".into()),
            messages: vec![],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: opi_ai::provider::CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

pub(super) fn observed_call(observed: &Observed) -> ObservedCall {
    observed.lock().unwrap().last().unwrap().clone()
}

pub(super) fn counting_resolver() -> Arc<CountingResolver> {
    Arc::new(CountingResolver {
        calls: AtomicUsize::new(0),
    })
}
