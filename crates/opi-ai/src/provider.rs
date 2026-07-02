//! LLM provider abstraction (S8.1).

use std::pin::Pin;

use futures_core::Stream;
use tokio_util::sync::CancellationToken;

use crate::message::{InputContent, Message, ToolDef};
use crate::stream::AssistantStreamEvent;

/// Provider trait — each concrete provider (Anthropic, OpenAI, etc.) implements this.
pub trait Provider: Send + Sync {
    /// Unique identifier for this provider instance (e.g. "anthropic").
    fn id(&self) -> &str;

    /// Models supported by this provider.
    fn models(&self) -> &[ModelInfo];

    /// Start a streaming request. Returns an `EventStream` that yields events
    /// until a terminal event (`Done` or `Error`) is reached or the caller
    /// cancels via `Request::cancel`.
    fn stream(&self, request: Request) -> EventStream;
}

/// Stream of assistant events from a provider.
pub type EventStream =
    Pin<Box<dyn Stream<Item = Result<AssistantStreamEvent, ProviderError>> + Send>>;

/// A single request to a provider.
pub struct Request {
    pub model: String,
    pub system: Option<String>,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolDef>,
    pub max_tokens: Option<u64>,
    pub temperature: Option<f64>,
    pub thinking: ThinkingConfig,
    pub stop_sequences: Vec<String>,
    pub metadata: Option<serde_json::Value>,
    pub cancel: CancellationToken,
}

impl Request {
    /// Returns true when any user message contains image input.
    pub fn contains_image_input(&self) -> bool {
        self.messages.iter().any(|message| match message {
            Message::User(user) => user
                .content
                .iter()
                .any(|content| matches!(content, InputContent::Image { .. })),
            _ => false,
        })
    }
}

/// Thinking/reasoning configuration for extended thinking models.
#[derive(Debug, Clone, Default)]
pub struct ThinkingConfig {
    pub enabled: bool,
    pub budget_tokens: Option<u64>,
}

/// Metadata about a model offered by a provider.
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
    pub context_window: u64,
    pub max_output_tokens: u64,
    pub supports_images: bool,
    pub supports_streaming: bool,
    pub supports_thinking: bool,
}

/// Validate request content against model capabilities known by the provider.
///
/// Returns `Err(UnsupportedCapability)` for a known text-only model receiving
/// image input, before any network call is attempted. Unknown model IDs are
/// left to the provider implementation so configured custom deployments can
/// still work (no preflight). Thinking and tool capability are handled at the
/// harness/config layer (`CodingHarness` clamps thinking off for non-thinking
/// models per the spec's "reject or clamp where possible"), so they are not
/// re-preflighted here.
pub fn validate_request_capabilities(
    provider: &dyn Provider,
    request: &Request,
) -> Result<(), ProviderError> {
    let model_id = request
        .model
        .split_once(':')
        .map(|(provider_id, model_id)| {
            if provider_id == provider.id() {
                model_id
            } else {
                request.model.as_str()
            }
        })
        .unwrap_or(request.model.as_str());

    let model = provider.models().iter().find(|m| m.id == model_id);

    // Image preflight: a known text-only model rejects image input before the call.
    if request.contains_image_input()
        && let Some(model) = model
        && !model.supports_images
    {
        return Err(ProviderError::UnsupportedCapability(format!(
            "model '{}' for provider '{}' does not support image input",
            model.id,
            provider.id()
        )));
    }

    Ok(())
}

/// Errors that can occur during provider streaming.
///
/// The [`ProviderError::category`] taxonomy exposes the nine Phase 12 classes
/// (auth, config, request, network, rate_limit, provider, stream, capability,
/// cancelled). `Timeout` is retained as a distinct variant but classifies as
/// `Network`, since the spec defines the network class as "DNS, TLS, proxy,
/// timeout".
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("rate limited")]
    RateLimited { retry_after_ms: Option<u64> },
    #[error("request timed out")]
    Timeout,
    #[error("request failed: {0}")]
    RequestFailed(String),
    #[error("stream error: {0}")]
    StreamError(String),
    #[error("authentication failed: {0}")]
    AuthFailed(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("invalid provider configuration: {0}")]
    Config(String),
    #[error("provider error: {0}")]
    ProviderSide(String),
    #[error("unsupported capability: {0}")]
    UnsupportedCapability(String),
    #[error("cancelled")]
    Cancelled,
}

impl ProviderError {
    /// Whether this error is retryable (rate-limited, timeout, or transient
    /// network failure). Retry timing/backoff behavior is owned by the agent
    /// runtime; this is the taxonomy-layer retryability signal.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ProviderError::RateLimited { .. } | ProviderError::Timeout | ProviderError::Network(_)
        )
    }

    /// Stable diagnostic category for this provider error.
    ///
    /// `opi-ai` cannot depend on `opi-agent`'s shared `Diagnostic` model, so
    /// the provider-side classification surface is this small taxonomy. The
    /// `opi-agent` diagnostic layer maps each [`ProviderErrorCategory`] into a
    /// diagnostic `code`/`severity`/`source` triple.
    pub fn category(&self) -> ProviderErrorCategory {
        match self {
            ProviderError::AuthFailed(_) => ProviderErrorCategory::Auth,
            ProviderError::Config(_) => ProviderErrorCategory::Config,
            ProviderError::RequestFailed(_) => ProviderErrorCategory::Request,
            ProviderError::Timeout | ProviderError::Network(_) => ProviderErrorCategory::Network,
            ProviderError::RateLimited { .. } => ProviderErrorCategory::RateLimit,
            ProviderError::ProviderSide(_) => ProviderErrorCategory::Provider,
            ProviderError::StreamError(_) => ProviderErrorCategory::Stream,
            ProviderError::UnsupportedCapability(_) => ProviderErrorCategory::Capability,
            ProviderError::Cancelled => ProviderErrorCategory::Cancelled,
        }
    }

    /// Server-advised delay before retrying, in milliseconds.
    ///
    /// Only [`ProviderError::RateLimited`] carries a `retry_after_ms` today;
    /// every other variant returns `None`.
    pub fn retry_after_ms(&self) -> Option<u64> {
        match self {
            ProviderError::RateLimited { retry_after_ms } => *retry_after_ms,
            _ => None,
        }
    }
}

/// Diagnostic category for a [`ProviderError`].
///
/// This is the opi-ai-owned classification substrate consumed by the
/// `opi-agent` diagnostic layer. Keeping it here means provider error
/// classification can be tested without any network access and without a
/// dependency on the shared `Diagnostic` model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderErrorCategory {
    /// Authentication failed (missing/invalid/expired credentials).
    Auth,
    /// Invalid provider configuration (bad endpoint, unsupported model, bad profile).
    Config,
    /// Local pre-request validation/schema failure (before the wire call).
    Request,
    /// Network/transport failure (DNS, TLS, proxy, timeout, connection).
    Network,
    /// Rate limited; a retry may succeed after a delay.
    RateLimit,
    /// Provider-side 4xx/5xx response with a safe body excerpt.
    Provider,
    /// Streaming response failed mid-flight.
    Stream,
    /// Unsupported image/tool/thinking capability rejected before the call.
    Capability,
    /// User/runtime cancellation. Timing/backoff/stream-abort behavior is owned
    /// by task 12.7; this class is the taxonomy/diagnostic-layer slot only.
    Cancelled,
}

/// Discriminant for the kind of provider backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    OpenAI,
    Anthropic,
    Google,
    Mistral,
    Bedrock,
    Azure,
}
