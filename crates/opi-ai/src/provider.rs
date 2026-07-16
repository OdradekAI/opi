//! LLM provider abstraction (S8.1).

use std::pin::Pin;

use futures_core::Stream;
use tokio_util::sync::CancellationToken;

use crate::credential::BoxAuthFuture;
use crate::message::{InputContent, Message, ToolDef};
use crate::stream::AssistantStreamEvent;

/// Provider trait — each concrete provider (Anthropic, OpenAI, etc.) implements this.
///
/// Runtime catalog refresh is an object-safe substrate. The coding agent has
/// no production refresh trigger; callers that invoke [`Provider::refresh_models`]
/// own collection-level atomic replacement.
pub trait Provider: Send + Sync {
    /// Unique identifier for this provider instance (e.g. "anthropic").
    fn id(&self) -> &str;

    /// Models supported by this provider.
    fn models(&self) -> &[ModelInfo];

    /// Start a streaming request. Returns an `EventStream` that yields events
    /// until a terminal event (`Done` or `Error`) is reached or the caller
    /// cancels via `Request::cancel`.
    fn stream(&self, request: Request) -> EventStream;

    /// Refresh this provider's model catalog at runtime.
    ///
    /// Static providers return `Ok(None)`. Dynamic providers return
    /// `Ok(Some(models))` with the latest model list, or an explicit error.
    /// The caller owns collection-level atomicity: it collects all provider
    /// results and only replaces the registry-owned dynamic catalogs after
    /// every provider succeeds.
    fn refresh_models(&self) -> BoxAuthFuture<'_, Result<Option<Vec<ModelInfo>>, ProviderError>> {
        Box::pin(async { Ok(None) })
    }
}

/// Stream of assistant events from a provider.
pub type EventStream =
    Pin<Box<dyn Stream<Item = Result<AssistantStreamEvent, ProviderError>> + Send>>;

/// Hint for cache-related behaviour on a provider request.
///
/// `None` preserves provider defaults. `Disabled` suppresses all cache/affinity
/// fields. `Short` requests ordinary ephemeral markers; `Long` requests one-hour
/// markers where the model and provider support them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CacheRetention {
    /// Preserve provider defaults — no explicit cache/affinity signal.
    #[default]
    None,
    /// Suppress all cache-control and session-affinity fields on the wire.
    Disabled,
    /// Ordinary ephemeral cache markers.
    Short,
    /// One-hour cache markers (only emitted when both the model capability and
    /// the provider wire path support them).
    Long,
}

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
    /// Per-request HTTP timeout. `None` uses the provider/client default.
    /// Timeout is distinct from cancellation: a timeout produces
    /// `ProviderError::Timeout`; a cancelled request produces
    /// `ProviderError::Cancelled`.
    pub timeout: Option<std::time::Duration>,
    /// Additional HTTP headers appended to the outbound request. Headers with
    /// names that appear in the provider-managed auth set (e.g. `authorization`,
    /// `x-api-key`) are rejected with [`ProviderError::RequestFailed`] before
    /// any network call. Default empty.
    pub extra_headers: Vec<(String, String)>,
    /// Cache-control and session-affinity hint for this request.
    pub cache_retention: CacheRetention,
    /// Opaque session identifier carried through the agent loop so providers
    /// can map it to session-affinity or prompt-cache-key headers. `None` means
    /// no session is active.
    pub session_id: Option<String>,
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

/// Validate extra headers supplied on a [`Request`].
///
/// Rejects header names that are empty, contain control characters, or match
/// the provider-managed auth header set. Auth header rejection is
/// case-insensitive and non-exhaustive: it covers the headers the built-in
/// providers manage (`authorization`, `x-api-key`, `api-key`, `anthropic-version`,
/// `content-type`), not every possible header a custom profile might set.
///
/// Returns `Err(ProviderError::RequestFailed(...))` on the first invalid header.
pub fn validate_extra_headers(headers: &[(String, String)]) -> Result<(), ProviderError> {
    /// Headers managed by built-in providers that extra_headers must not override.
    const RESERVED: &[&str] = &[
        "authorization",
        "x-api-key",
        "api-key",
        "anthropic-version",
        "content-type",
    ];

    for (name, _value) in headers {
        if name.is_empty() {
            return Err(ProviderError::RequestFailed(
                "extra_headers contains an empty header name".into(),
            ));
        }
        if name.contains(|c: char| c.is_control() || c == ':') {
            return Err(ProviderError::RequestFailed(format!(
                "extra_headers name contains invalid characters: {name:?}"
            )));
        }
        let lower = name.to_ascii_lowercase();
        if RESERVED.contains(&lower.as_str()) {
            return Err(ProviderError::RequestFailed(format!(
                "extra_headers name '{name}' is reserved for provider-managed auth"
            )));
        }
    }
    Ok(())
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
    /// Declared model capabilities (context window, output tokens, image,
    /// streaming, thinking, cache-control).
    pub capabilities: crate::registry::ModelCapabilities,
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
        && !model.capabilities.supports_images
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
    /// No credential is available for the provider. Non-retryable: the caller
    /// must obtain a credential (interactive login or a typed non-interactive
    /// diagnostic) before retrying. Distinct from [`AuthFailed`](Self::AuthFailed)
    /// because it routes to the login/credential-needed path, not retry. It
    /// never starts login automatically.
    #[error("credential needed for provider '{provider_id}'; run `/login {provider_id}`")]
    CredentialNeeded { provider_id: String },
    /// A previously valid credential was rejected by the provider (e.g. 401 on
    /// an OAuth token). Non-retryable and never auto-relogs-in: the current
    /// turn ends and a later explicit `/login` is required.
    #[error("credential revoked for provider '{provider_id}'; re-login required")]
    CredentialRevoked { provider_id: String },
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
            ProviderError::AuthFailed(_)
            | ProviderError::CredentialNeeded { .. }
            | ProviderError::CredentialRevoked { .. } => ProviderErrorCategory::Auth,
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
