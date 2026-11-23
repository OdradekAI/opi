//! Shared setup and scripted responses for the provider collection target.

use std::collections::HashMap;
use std::sync::Mutex;

use opi_ai::auth::{AuthResolver, ResolvedAuth};
use opi_ai::catalog::{Catalog, CatalogOperation};
use opi_ai::credential::{
    BoxAuthFuture, Credential, CredentialSource, CredentialStore, CredentialStoreError,
};
use opi_ai::message::{AssistantContent, AssistantMessage};
use opi_ai::provider::{CacheRetention, Provider, ProviderError, Request, ThinkingConfig};
use opi_ai::test_support::{MockProvider, text_response};
use opi_ai::{ModelCapabilities, ModelInfo, WireApi};
use tokio_util::sync::CancellationToken;

pub(super) fn minimal_request(model: &str) -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: model.into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
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
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

/// Concatenate all text content carried by an assistant message.
pub(super) fn assistant_text(message: &AssistantMessage) -> String {
    message
        .content
        .iter()
        .filter_map(|content| match content {
            AssistantContent::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

/// Build a mock provider that streams a single text response.
pub(super) fn text_mock(id: &str, text: &str) -> Box<dyn Provider> {
    Box::new(MockProvider::new(id, vec![text_response(text)]))
}

/// Build a mock provider with `count` identical text response batches, for
/// tests that dispatch more than once.
pub(super) fn text_mock_repeated(id: &str, text: &str, count: usize) -> Box<dyn Provider> {
    let responses = (0..count).map(|_| text_response(text)).collect();
    Box::new(MockProvider::new(id, responses))
}

pub(super) const SECRET_VALUE: &str = "sk-super-secret-value-DO-NOT-LEAK";

pub(super) fn secret(value: &str) -> secrecy::SecretString {
    // secrecy 0.10 SecretString::new takes `Box<str>`.
    secrecy::SecretString::new(value.into())
}

/// Resolver that reports missing credentials before dispatch can reach a provider.
pub(super) struct MissingAuthResolver {
    pub(super) provider_id: String,
}

impl AuthResolver for MissingAuthResolver {
    fn resolve<'a>(&'a self) -> BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        let provider_id = self.provider_id.clone();
        Box::pin(async move { Err(ProviderError::CredentialNeeded { provider_id }) })
    }
}

/// In-memory fake credential store proving the trait is object-safe behind
/// `Arc<dyn CredentialStore>` and exercising probe/read/write/delete without
/// any keychain.
pub(super) struct FakeStore {
    pub(super) entries: Mutex<HashMap<String, Credential>>,
}

impl CredentialStore for FakeStore {
    fn read<'a>(
        &'a self,
        provider_id: &'a str,
    ) -> BoxAuthFuture<'a, Result<Option<Credential>, CredentialStoreError>> {
        Box::pin(async move { Ok(self.entries.lock().unwrap().get(provider_id).cloned()) })
    }
    fn write<'a>(
        &'a self,
        provider_id: &'a str,
        cred: &'a Credential,
    ) -> BoxAuthFuture<'a, Result<(), CredentialStoreError>> {
        Box::pin(async move {
            self.entries
                .lock()
                .unwrap()
                .insert(provider_id.to_owned(), cred.clone());
            Ok(())
        })
    }
    fn delete<'a>(
        &'a self,
        provider_id: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), CredentialStoreError>> {
        Box::pin(async move {
            self.entries.lock().unwrap().remove(provider_id);
            Ok(())
        })
    }
    fn probe<'a>(&'a self, provider_id: &'a str) -> BoxAuthFuture<'a, CredentialSource> {
        Box::pin(async move {
            if self.entries.lock().unwrap().contains_key(provider_id) {
                CredentialSource::Present {
                    label: format!("keychain opi:{provider_id}"),
                }
            } else {
                CredentialSource::Absent
            }
        })
    }
}

/// A mock provider that implements `refresh_models` to return a dynamic catalog.
pub(super) struct RefreshProvider {
    id: &'static str,
    builtin_models: Vec<ModelInfo>,
    refreshed_models: std::sync::Mutex<Option<Vec<ModelInfo>>>,
    pub(super) events: std::sync::Mutex<
        Option<Vec<Result<opi_ai::AssistantStreamEvent, opi_ai::provider::ProviderError>>>,
    >,
}

impl RefreshProvider {
    pub(super) fn new(id: &'static str, builtin_models: Vec<ModelInfo>) -> Self {
        Self {
            id,
            builtin_models,
            refreshed_models: std::sync::Mutex::new(None),
            events: std::sync::Mutex::new(Some(Vec::new())),
        }
    }

    pub(super) fn with_refresh(mut self, models: Vec<ModelInfo>) -> Self {
        self.refreshed_models = std::sync::Mutex::new(Some(models));
        self
    }
}

impl Provider for RefreshProvider {
    fn id(&self) -> &str {
        self.id
    }

    fn models(&self) -> &[ModelInfo] {
        // Leak a static to satisfy the &[ModelInfo] return type.
        // Only used in tests; the leak is intentional and bounded.
        Box::leak(self.builtin_models.clone().into_boxed_slice())
    }

    fn stream_prepared(
        &self,
        _request: Request,
        _auth: ResolvedAuth,
    ) -> opi_ai::provider::EventStream {
        let events = self.events.lock().unwrap().take().unwrap_or_default();
        Box::pin(futures_util::stream::iter(events))
    }

    fn refresh_models(
        &self,
        _context: opi_ai::CatalogRefreshContext,
    ) -> opi_ai::credential::BoxAuthFuture<
        '_,
        Result<Option<Vec<ModelInfo>>, opi_ai::provider::ProviderError>,
    > {
        let result = self.refreshed_models.lock().unwrap().clone();
        Box::pin(async move { Ok(result) })
    }
}

pub(super) fn model_info(id: &str, display: &str) -> ModelInfo {
    ModelInfo::new(
        id,
        display,
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(128_000, 4096),
    )
}

/// A mock provider whose `refresh_models` returns an error.
pub(super) struct ErrorRefreshProvider {
    pub(super) id: &'static str,
    pub(super) error_msg: &'static str,
}

impl Provider for ErrorRefreshProvider {
    fn id(&self) -> &str {
        self.id
    }

    fn models(&self) -> &[ModelInfo] {
        &[]
    }

    fn stream_prepared(
        &self,
        _request: Request,
        _auth: ResolvedAuth,
    ) -> opi_ai::provider::EventStream {
        Box::pin(futures_util::stream::empty())
    }

    fn refresh_models(
        &self,
        _context: opi_ai::CatalogRefreshContext,
    ) -> opi_ai::credential::BoxAuthFuture<
        '_,
        Result<Option<Vec<ModelInfo>>, opi_ai::provider::ProviderError>,
    > {
        let msg = self.error_msg;
        Box::pin(async move {
            Err(opi_ai::provider::ProviderError::ProviderSide(
                opi_ai::provider::ProviderErrorSummary::from_untrusted(msg),
            ))
        })
    }
}

/// Injected auth context answering from a fixed map: no process environment.
struct MapAuthContext(std::collections::BTreeMap<&'static str, &'static str>);

impl opi_ai::auth::AuthContext for MapAuthContext {
    fn env(&self, name: &str) -> Result<Option<String>, opi_ai::auth::AuthContextError> {
        Ok(self.0.get(name).map(|value| (*value).to_owned()))
    }
    fn file_exists(&self, _path: &str) -> Result<bool, opi_ai::auth::AuthContextError> {
        Ok(false)
    }
}

pub(super) fn map_context(
    pairs: &[(&'static str, &'static str)],
) -> std::sync::Arc<dyn opi_ai::auth::AuthContext> {
    std::sync::Arc::new(MapAuthContext(pairs.iter().copied().collect()))
}

pub(super) fn first_catalog_chat_model(provider: &str) -> String {
    let catalog = Catalog::builtin().expect("builtin catalog loads");
    catalog
        .models(provider, CatalogOperation::Chat)
        .expect("provider has chat entries")
        .next()
        .expect("non-empty chat catalog")
        .id
        .clone()
}

pub(super) fn node5_reply(wire: WireApi) -> &'static str {
    match wire {
        WireApi::OpenAiCompletions | WireApi::MistralConversations => concat!(
            "data: {\"id\":\"x\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"hi\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"x\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1}}\n\n",
            "data: [DONE]\n\n"
        ),
        WireApi::OpenAiResponses
        | WireApi::AzureOpenAiResponses
        | WireApi::OpenAiCodexResponses => concat!(
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"hi\"}\n\n",
            "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"r\",\"usage\":{\"input_tokens\":1,\"output_tokens\":1,\"total_tokens\":2}}}\n\n"
        ),
        WireApi::AnthropicMessages => concat!(
            "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"model\":\"fixture\",\"usage\":{\"input_tokens\":1,\"output_tokens\":0}}}\n\n",
            "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
            "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"hi\"}}\n\n",
            "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
            "event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":1}}\n\n",
            "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n"
        ),
        WireApi::PiMessages => concat!(
            "data: {\"type\":\"start\"}\n\n",
            "data: {\"type\":\"text_start\",\"contentIndex\":0}\n\n",
            "data: {\"type\":\"text_delta\",\"contentIndex\":0,\"delta\":\"hi\"}\n\n",
            "data: {\"type\":\"text_end\",\"contentIndex\":0,\"content\":\"hi\"}\n\n",
            "data: {\"type\":\"done\",\"reason\":\"stop\",\"usage\":{\"input\":1,\"output\":1,\"cacheRead\":0,\"cacheWrite\":0,\"totalTokens\":2}}\n\n"
        ),
        WireApi::GoogleGenerativeAi => {
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"hi\"}],\"role\":\"model\"},\"finishReason\":\"STOP\",\"index\":0}],\"usageMetadata\":{\"promptTokenCount\":1,\"candidatesTokenCount\":1,\"totalTokenCount\":2}}\n\n"
        }
        _ => panic!("fixture missing for {wire}"),
    }
}

pub(super) fn accepted_failure_cause<'a>(
    error: &'a opi_ai::provider::ProviderError,
    request: &opi_ai::provider::Request,
) -> &'a opi_ai::provider::ProviderError {
    use opi_ai::provider::ProviderError;
    use sha2::Digest as _;
    let ProviderError::StreamFailure(failure) = error else {
        panic!("accepted projection must retain its failure carrier: {error:?}");
    };
    let facts = failure
        .message()
        .projection
        .as_ref()
        .expect("accepted projection is known");
    let bytes = serde_json::to_vec(request.context.messages()).unwrap();
    assert_eq!(
        facts.source_prefix.message_count,
        request.context.messages().len()
    );
    assert_eq!(
        facts.source_prefix.sha256,
        format!("{:x}", sha2::Sha256::digest(bytes))
    );
    assert_eq!(
        facts.temporal.mode,
        opi_ai::transcript::ProjectionMode::Lossless
    );
    assert!(facts.temporal.affected_ranges.is_empty());
    assert!(facts.replay.losses.is_empty());
    failure.cause()
}

pub(super) fn accepted_models_failure_cause<'a>(
    error: &'a opi_ai::ModelsError,
    request: &opi_ai::provider::Request,
) -> &'a opi_ai::provider::ProviderError {
    let opi_ai::ModelsError::Provider(error) = error else {
        panic!("accepted projection must retain its provider failure: {error:?}");
    };
    accepted_failure_cause(error, request)
}
