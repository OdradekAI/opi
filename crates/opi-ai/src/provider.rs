//! LLM provider abstraction (S8.1).

use std::pin::Pin;

use futures_core::Stream;
use tokio_util::sync::CancellationToken;

use crate::credential::BoxAuthFuture;
use crate::message::{InputContent, Message, OutputContent, ToolDef};
use crate::model_info::{ThinkingLevel, WireApi};
use crate::stream::AssistantStreamEvent;

pub use crate::model_info::ModelInfo;

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
    /// Returns true when any user or tool-result message contains image content.
    pub fn contains_image_content(&self) -> bool {
        self.messages.iter().any(|message| match message {
            Message::User(user) => user
                .content
                .iter()
                .any(|content| matches!(content, InputContent::Image { .. })),
            Message::ToolResult(tool_result) => tool_result
                .content
                .iter()
                .any(|content| matches!(content, OutputContent::Image { .. })),
            _ => false,
        })
    }
}

const GITHUB_COPILOT_MANAGED_HEADERS: &[&str] = &[
    "user-agent",
    "editor-version",
    "editor-plugin-version",
    "copilot-integration-id",
    "x-initiator",
    "openai-intent",
    "copilot-vision-request",
];

/// Build the request-dependent GitHub Copilot headers other than `X-Initiator`.
///
/// All Copilot-managed names are rejected in per-request headers before any
/// network call. Concrete wire providers append the separately derived
/// initiator so the same policy applies to every Copilot route.
pub(crate) fn github_copilot_route_headers(
    request: &Request,
) -> Result<Vec<(String, String)>, ProviderError> {
    if let Some((name, _)) = request.extra_headers.iter().find(|(name, _)| {
        GITHUB_COPILOT_MANAGED_HEADERS.contains(&name.to_ascii_lowercase().as_str())
    }) {
        return Err(ProviderError::RequestFailed(format!(
            "request header '{name}' is reserved for GitHub Copilot"
        )));
    }

    let mut headers = vec![("Openai-Intent".into(), "conversation-edits".into())];
    if request.contains_image_content() {
        headers.push(("Copilot-Vision-Request".into(), "true".into()));
    }
    Ok(headers)
}

/// Derive GitHub Copilot's initiator from the final conversation message.
pub(crate) fn github_copilot_initiator(request: &Request) -> &'static str {
    match request.messages.last() {
        Some(Message::User(_)) | None => "user",
        Some(_) => "agent",
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
#[derive(Debug, Clone)]
pub struct ThinkingConfig {
    pub enabled: bool,
    pub budget_tokens: Option<u64>,
    pub level: ThinkingLevel,
}

impl Default for ThinkingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            budget_tokens: None,
            level: ThinkingLevel::None,
        }
    }
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
    if request.contains_image_content()
        && let Some(model) = model
        && !model.capabilities.supports_images
    {
        return Err(ProviderError::UnsupportedCapability(format!(
            "model '{}' for provider '{}' does not support image input",
            model.id,
            provider.id()
        )));
    }

    if let Some(model) = model {
        model.validate().map_err(|error| match error {
            crate::model_info::ModelInfoError::WireCompatMismatch {
                model_id,
                wire_api,
                compat_wire,
            } => ProviderError::WireCompatMismatch {
                model_id,
                wire_api,
                compat_wire,
            },
            other => ProviderError::Config(other.to_string()),
        })?;
        model
            .thinking_level_map
            .resolve(request.thinking.level)
            .map_err(|error| ProviderError::UnsupportedCapability(error.to_string()))?;
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
    #[error("unknown model '{model_id}' for provider '{provider_id}'")]
    UnknownModel {
        provider_id: String,
        model_id: String,
    },
    #[error("provider '{provider_id}' has no route for wire API '{wire_api}'")]
    MissingWireRoute {
        provider_id: String,
        wire_api: WireApi,
    },
    #[error(
        "model '{model_id}' wire API '{wire_api}' does not match compatibility wire '{compat_wire}'"
    )]
    WireCompatMismatch {
        model_id: String,
        wire_api: WireApi,
        compat_wire: WireApi,
    },
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
            ProviderError::Config(_)
            | ProviderError::MissingWireRoute { .. }
            | ProviderError::WireCompatMismatch { .. } => ProviderErrorCategory::Config,
            ProviderError::RequestFailed(_) | ProviderError::UnknownModel { .. } => {
                ProviderErrorCategory::Request
            }
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
