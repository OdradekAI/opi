//! LLM provider abstraction.

use std::pin::Pin;
use std::str::FromStr;

use futures_core::Stream;
use reqwest::header::{HeaderName, HeaderValue};
use tokio_util::sync::CancellationToken;

use crate::credential::BoxAuthFuture;
use crate::message::{InputContent, Message, OutputContent};
use crate::model_info::{ThinkingLevel, WireApi};
use crate::stream::AssistantStreamEvent;

pub use crate::model_info::ModelInfo;

/// Per-attempt dispatch context for providers that stamp retry-observability
/// wire headers (task 20.20, `amz-sdk-invocation-id`/`amz-sdk-request`).
///
/// One invocation id is generated per logical call; the 1-based attempt
/// index and the frozen total budget advance per send of the frozen payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttemptContext {
    /// The logical call's stable invocation identifier.
    pub invocation_id: String,
    /// 1-based attempt index within the episode.
    pub attempt: u32,
    /// The frozen total attempt budget.
    pub max_attempts: u32,
    /// The payload variant this attempt dispatches (task 20.11): the
    /// ordinary frozen payload, or the single rebuilt strict-downgrade
    /// variant.
    pub payload_variant: PayloadVariant,
}

/// The payload variant of one attempt (task 20.11, V-34/V-36).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadVariant {
    /// The ordinary post-projection payload; ordinary acquisition retries
    /// reuse its frozen body.
    Ordinary,
    /// The rebuilt Anthropic strict downgrade: prefer-strict removed,
    /// require kept, payload callback re-run and re-frozen under its own
    /// variant label.
    AnthropicStrictDowngrade,
}

impl PayloadVariant {
    /// The payload-freeze slot label (see
    /// [`crate::callbacks::RequestCallbacks`]).
    pub fn label(self) -> &'static str {
        match self {
            Self::Ordinary => crate::callbacks::ORDINARY_PAYLOAD_VARIANT,
            Self::AnthropicStrictDowngrade => crate::callbacks::STRICT_DOWNGRADE_PAYLOAD_VARIANT,
        }
    }
}

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

    /// Start a streaming request using already-resolved authentication.
    ///
    /// This is the prepared-auth dispatch seam and the sole provider entry
    /// point: the collection resolves the route and authentication once per
    /// logical call, then every retry attempt reuses that frozen authentication
    /// through this entry. It must NOT consult a resolver or perform auth
    /// preparation — it consumes the supplied
    /// [`ResolvedAuth`](crate::auth::ResolvedAuth) directly at the wire
    /// boundary, attaching the secret via [`secrecy::ExposeSecret`] immediately
    /// before the HTTP request.
    ///
    /// API-key and Bearer routes consume `resolved.secret`; AWS routes consume
    /// the typed [`AuthScheme::AwsSigV4`](crate::auth::AuthScheme::AwsSigV4)
    /// credential variant. Every variant is prepared once by the collection
    /// and reused unchanged across retry attempts.
    fn stream_prepared(&self, request: Request, auth: crate::auth::ResolvedAuth) -> EventStream;

    /// Dispatch one attempt with its wire-visible attempt context (task
    /// 20.20). The default ignores the context and reuses
    /// [`stream_prepared`](Self::stream_prepared); AWS wires override it to
    /// stamp the source's per-attempt `amz-sdk-invocation-id` and
    /// `amz-sdk-request` headers onto the signed request.
    fn stream_attempt(
        &self,
        request: Request,
        auth: crate::auth::ResolvedAuth,
        _attempt: AttemptContext,
    ) -> EventStream {
        self.stream_prepared(request, auth)
    }

    /// The provider's protocol-default acquisition policy for ordinary
    /// [`prepare_call`](crate::Models::prepare_call) entries (AIR-007).
    /// Helper/Codex wires default to zero extra sends; a provider that owns a
    /// retry protocol (Bedrock's frozen Smithy configuration) returns it
    /// here. An explicit per-call policy still overrides this default.
    fn acquisition_policy_default(&self) -> crate::retry::AcquisitionPolicy {
        crate::retry::AcquisitionPolicy::default()
    }

    /// The provider's session-scoped connection resource owner, if any
    /// (task 20.17). Providers that hold per-session connection pools or
    /// in-flight generation work return their
    /// [`SessionResourceOwner`](crate::codex_resources::SessionResourceOwner)
    /// surface here so [`Models::cleanup_session_resources`](crate::Models::cleanup_session_resources)
    /// can drive the cleanup barrier across every registered provider.
    /// The default `None` excludes resource-less providers from cleanup.
    fn session_resources(&self) -> Option<&dyn crate::codex_resources::SessionResourceOwner> {
        None
    }

    /// Replace this provider's effective model catalog before it is shared.
    ///
    /// Mapped providers use this construction-time hook to materialize
    /// extension model additions and overrides into their concrete routes.
    /// Providers that cannot safely replace their catalog reject the update.
    /// Replace the effective model catalog.
    ///
    /// Implementations that support replacement must leave their current
    /// catalog unchanged when returning `Err`, so collection and mapped-route
    /// callers can preserve atomic replacement semantics.
    fn replace_model_catalog(&mut self, _models: Vec<ModelInfo>) -> Result<(), ProviderError> {
        Err(ProviderError::Config(ProviderErrorSummary::sanitized(
            format!(
                "provider '{}' does not support effective model catalogs",
                self.id()
            ),
        )))
    }

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

/// Preferred transport for providers that support multiple transports
/// (task 20.16, DEL-07). `Auto` is the normalized default: providers with a
/// WebSocket transport attempt it first and fall back to SSE only under the
/// bounded transport rules; `Sse` never attempts an alternate transport;
/// `Websocket` uses a fresh connection per call; `WebsocketCached` additionally
/// reuses a session-scoped connection and its continuation state when one is
/// available. The preference is a transport-layer hint only: it never appears
/// in request bodies or wire payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransportPreference {
    /// Provider default resolution (WebSocket first where supported).
    #[default]
    Auto,
    /// HTTP SSE only; never attempts an alternate transport.
    Sse,
    /// WebSocket with a fresh connection per call.
    Websocket,
    /// WebSocket with connection and continuation reuse when a session is
    /// active.
    WebsocketCached,
}

/// A single request to a provider.
#[derive(Clone)]
pub struct Request {
    pub model: String,
    /// Canonical normalized transcript (AIR-020): the system prompt and tool
    /// declarations live in the transcript's system messages, never in
    /// independent runtime fields. Construct the value through
    /// [`crate::transcript::normalize_context`], which folds the public
    /// `Context` shorthand into one leading system message; adapters consume
    /// only this canonical input.
    pub context: crate::transcript::TranscriptContext,
    /// Per-request output limit. `Models::prepare_call` resolves
    /// `None` to the selected registry model's output capacity. Both implicit
    /// and explicit ceilings are reduced to fit the estimated remaining context
    /// with 4,096 tokens of safety room. Direct provider adapter calls do not
    /// perform this collection-owned resolution.
    pub max_tokens: Option<u64>,
    pub temperature: Option<f64>,
    /// Per-request sampling parameters merged over the model base and the
    /// effective thinking level (see [`crate::model_info::resolve_sampling_params`]).
    /// Only OpenAI-compatible adapters apply the merged map.
    pub sampling_params: crate::model_info::SamplingParams,
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
    /// Functional callbacks (task 20.51): header transform, payload,
    /// response, and raw-event observers applied by the real direct adapters
    /// at their source-defined timing. The default is empty; see
    /// [`crate::callbacks`].
    pub callbacks: crate::callbacks::RequestCallbacks,
    /// Preferred transport for providers that support multiple transports
    /// (task 20.16). Defaults to [`TransportPreference::Auto`].
    pub transport: TransportPreference,
}

impl Request {
    /// Returns true when any user or tool-result message contains image content.
    pub fn contains_image_content(&self) -> bool {
        self.context.messages().iter().any(|message| match message {
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
        return Err(ProviderError::RequestFailed(
            ProviderErrorSummary::sanitized(format!(
                "request header '{name}' is reserved for GitHub Copilot"
            )),
        ));
    }

    let mut headers = vec![("Openai-Intent".into(), "conversation-edits".into())];
    if request.contains_image_content() {
        headers.push(("Copilot-Vision-Request".into(), "true".into()));
    }
    Ok(headers)
}

/// Derive GitHub Copilot's initiator from the final conversation message.
pub(crate) fn github_copilot_initiator(request: &Request) -> &'static str {
    match request.context.messages().last() {
        Some(Message::User(_)) | None => "user",
        Some(_) => "agent",
    }
}

/// Validate extra headers supplied on a [`Request`].
///
/// Rejects header names that are empty, contain control characters, or match
/// the provider-managed auth header set. Reserved-name rejection reuses the
/// single canonical `RESERVED_PROVIDER_HEADERS` list in `provider_headers`
/// shared with `ProviderHeaders`, so every wire enforces the same gate.
///
/// Returns `Err(ProviderError::RequestFailed(...))` on the first invalid header.
pub fn validate_extra_headers(headers: &[(String, String)]) -> Result<(), ProviderError> {
    use crate::provider_headers::RESERVED_PROVIDER_HEADERS;

    for (name, value) in headers {
        if name.is_empty() {
            return Err(ProviderError::RequestFailed(
                ProviderErrorSummary::attested_static(
                    "extra_headers contains an empty header name",
                ),
            ));
        }
        if name.contains(|c: char| c.is_control() || c == ':') {
            return Err(ProviderError::RequestFailed(
                ProviderErrorSummary::sanitized(format!(
                    "extra_headers name contains invalid characters: {name:?}"
                )),
            ));
        }
        HeaderName::from_str(name).map_err(|_| {
            ProviderError::RequestFailed(ProviderErrorSummary::sanitized(format!(
                "extra_headers name contains invalid characters: {name:?}"
            )))
        })?;
        HeaderValue::from_str(value).map_err(|_| {
            ProviderError::RequestFailed(ProviderErrorSummary::sanitized(format!(
                "extra_headers contains an invalid value for header '{name}'"
            )))
        })?;
        let lower = name.to_ascii_lowercase();
        if RESERVED_PROVIDER_HEADERS.contains(&lower.as_str()) {
            return Err(ProviderError::RequestFailed(
                ProviderErrorSummary::sanitized(format!(
                    "extra_headers name '{name}' is reserved for provider-managed auth"
                )),
            ));
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
/// still work (no preflight). Unsupported thinking levels are preflight errors
/// on every wire: if thinking is enabled and the model's
/// `thinking_level_map` cannot resolve the requested level, the request is
/// rejected with `UnsupportedCapability` before any network call.
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
    validate_request_for_model(provider.id(), model, request)
}

/// Validate a request against already-resolved model metadata.
///
/// Dispatchers that resolve through a registry must use this entry point so
/// extension overrides and dynamic catalogs are checked instead of the
/// provider's construction-time catalog.
pub fn validate_request_for_model(
    provider_id: &str,
    model: Option<&ModelInfo>,
    request: &Request,
) -> Result<(), ProviderError> {
    // Image preflight: a known text-only model rejects image input before the call.
    if request.contains_image_content()
        && let Some(model) = model
        && !model.capabilities.supports_images
    {
        return Err(ProviderError::UnsupportedCapability(
            ProviderErrorSummary::sanitized(format!(
                "model '{}' for provider '{}' does not support image input",
                model.id, provider_id
            )),
        ));
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
            other => ProviderError::Config(ProviderErrorSummary::sanitized(other.to_string())),
        })?;
        // Resolve the level only when thinking is enabled: a disabled request
        // with a stale level must not be rejected (the level is unused). An
        // enabled request still requires thinking capability even when its
        // selected level is `None`.
        if request.thinking.enabled {
            if !model.capabilities.supports_thinking {
                return Err(ProviderError::UnsupportedCapability(
                    ProviderErrorSummary::sanitized(format!(
                        "model '{}' for provider '{}' does not support thinking",
                        model.id, provider_id
                    )),
                ));
            }
            model
                .thinking_level_map
                .resolve(request.thinking.level)
                .map_err(|error| {
                    ProviderError::UnsupportedCapability(ProviderErrorSummary::sanitized(
                        error.to_string(),
                    ))
                })?;
        }
    }

    Ok(())
}

/// Redaction-safe public summary carried by free-form [`ProviderError`] classes.
///
/// The inner string is private so a custom [`Provider`] cannot put a dynamic
/// upstream payload into `ProviderError` by constructing a variant directly.
/// Dynamic untrusted text can only enter through [`Self::from_untrusted`],
/// which intentionally discards it. Closed no-input constructors expose the
/// limited summaries custom providers can retain.
///
/// ```compile_fail
/// use opi_ai::provider::ProviderError;
///
/// let raw_upstream = String::from("arbitrary provider response");
/// let _ = ProviderError::ProviderSide(raw_upstream);
/// ```
///
/// Static and deliberately leaked provider text are not safe conversions:
///
/// ```compile_fail
/// use opi_ai::provider::ProviderError;
///
/// const PROVIDER_CANARY: &str = "static-provider-canary";
/// let _ = ProviderError::ProviderSide(PROVIDER_CANARY.into());
/// ```
///
/// ```compile_fail
/// use opi_ai::provider::ProviderError;
///
/// let leaked: &'static str = Box::leak(String::from("leaked-provider-canary").into_boxed_str());
/// let _ = ProviderError::ProviderSide(leaked.into());
/// ```
#[derive(Clone, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(transparent)]
pub struct ProviderErrorSummary(String);

impl ProviderErrorSummary {
    const REDACTED: &'static str = "[REDACTED]";

    /// Construct the closed redacted-detail summary without accepting text.
    pub fn redacted() -> Self {
        Self(Self::REDACTED.to_owned())
    }

    /// Construct the closed summary for credentials rejected by a provider.
    pub fn authentication_rejected() -> Self {
        Self("provider rejected credentials".to_owned())
    }

    /// Replace an arbitrary upstream error payload with a stable safe summary.
    ///
    /// The input is accepted so custom providers have an explicit producer
    /// boundary, but it is never retained in public `Display`, `Debug`, or
    /// serialized diagnostics.
    pub fn from_untrusted(_payload: impl AsRef<str>) -> Self {
        Self::redacted()
    }

    /// Return the safe summary text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Construct an in-crate summary after applying the shared defense-in-depth
    /// redactor. Raw provider response bodies use [`Self::from_untrusted`]
    /// instead; this path is for locally-produced context and transport errors.
    /// Build a summary with the canonical credential redaction applied.
    ///
    /// Public since task 20.51: functional-callback failures and advanced
    /// error constructors need the same redaction pipeline as adapters.
    pub fn sanitized(summary: impl AsRef<str>) -> Self {
        Self(crate::http::safe_excerpt(summary.as_ref()))
    }

    /// Construct from crate-owned, caller-attested static diagnostic text.
    pub(crate) fn attested_static(summary: &'static str) -> Self {
        Self(summary.to_owned())
    }

    /// Construct a safe HTTP failure summary from status alone.
    ///
    /// Callers must not read a provider-controlled response body solely to
    /// construct this summary.
    pub(crate) fn from_http_response(status: u16) -> Self {
        Self(format!("HTTP {status}: {}", Self::REDACTED))
    }
}

impl std::ops::Deref for ProviderErrorSummary {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl std::fmt::Display for ProviderErrorSummary {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl std::fmt::Debug for ProviderErrorSummary {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_tuple("ProviderErrorSummary")
            .field(&self.as_str())
            .finish()
    }
}

impl PartialEq<str> for ProviderErrorSummary {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for ProviderErrorSummary {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

/// Errors that can occur during provider streaming.
///
/// The [`ProviderError::category`] taxonomy exposes nine closed classes
/// (auth, config, request, network, rate_limit, provider, stream, capability,
/// cancelled). `Timeout` is retained as a distinct variant but classifies as
/// `Network`, since the spec defines the network class as "DNS, TLS, proxy,
/// timeout".
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ProviderError {
    /// Adapter-owned canonical state at failure, consumed by `ChatStream`.
    #[doc(hidden)]
    #[error("{0}")]
    StreamFailure(Box<ProviderStreamFailure>),
    #[error("rate limited")]
    RateLimited { retry_after_ms: Option<u64> },
    #[error("request timed out")]
    Timeout,
    #[error("request failed: {0}")]
    RequestFailed(ProviderErrorSummary),
    /// A functional callback failed (task 20.51): header transform, payload,
    /// response, or raw observer. A functional failure of the call, never
    /// retryable, never swallowed, and never triggering a new call.
    #[error("callback phase failed: {0}")]
    CallbackPhase(ProviderErrorSummary),
    /// A source-qualified strict-limit rejection (task 20.11, V-34): the
    /// Anthropic wire answered 400 `invalid_request_error` with one of the
    /// documented strict-tool limit messages on a request that actually
    /// carried prefer-strict tools. Never an ordinary transport retry: the
    /// acquisition owner may admit exactly one rebuilt downgrade variant,
    /// and a second occurrence (or one without prefer-strict tools) is a
    /// terminal failure.
    #[error("strict tool limits exceeded: {detail}")]
    StrictToolLimits { detail: ProviderErrorSummary },
    #[error("stream error: {0}")]
    StreamError(ProviderErrorSummary),
    #[error("authentication failed: {0}")]
    AuthFailed(ProviderErrorSummary),
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
    /// The provider requires a non-secret account identifier alongside its
    /// bearer token, but the credential did not contain one.
    #[error("credential for provider '{provider_id}' is missing its account id; re-login required")]
    AccountIdMissing { provider_id: String },
    /// The user cancelled login-method selection before any credential flow
    /// or persistence operation began.
    #[error("login cancelled for provider '{provider_id}'")]
    LoginCancelled { provider_id: String },
    #[error("network error: {0}")]
    Network(ProviderErrorSummary),
    #[error("invalid provider configuration: {0}")]
    Config(ProviderErrorSummary),
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
    ProviderSide(ProviderErrorSummary),
    /// A structured service-model failure carrying the wire status and error
    /// name (task 20.20). AWS wires surface this so the acquisition owner can
    /// classify through the source's exact trait/name rules; other consumers
    /// see the same taxonomy category as [`ProviderSide`](Self::ProviderSide).
    #[error("service fault {status}: {detail}")]
    ServiceFault {
        /// The HTTP status code of the failed response.
        status: u16,
        /// The wire service error type name, when exposed.
        #[doc(hidden)]
        error_name: Option<String>,
        /// Server-advised retry delay in milliseconds, when present.
        retry_after_ms: Option<u64>,
        /// Redacted human-readable detail.
        detail: ProviderErrorSummary,
    },
    #[error("unsupported capability: {0}")]
    UnsupportedCapability(ProviderErrorSummary),
    #[error("cancelled")]
    Cancelled,
}

/// Internal transport for a mapper's latest state and its redacted cause.
///
/// The fields and constructor are private: this is carried through the
/// prepared-provider seam and unpacked before a public `CallFailure` is
/// delivered. Its diagnostic formatting exposes only the typed cause.
#[doc(hidden)]
#[derive(Clone, PartialEq)]
pub struct ProviderStreamFailure {
    message: crate::message::AssistantMessage,
    cause: ProviderError,
    response_received: bool,
}

impl ProviderStreamFailure {
    /// The original typed classification, without the adapter-state envelope.
    pub fn cause(&self) -> &ProviderError {
        &self.cause
    }
}

impl std::fmt::Display for ProviderStreamFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.cause.fmt(formatter)
    }
}

impl std::fmt::Debug for ProviderStreamFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.cause, formatter)
    }
}

impl ProviderError {
    pub(crate) fn with_partial(self, message: crate::message::AssistantMessage) -> Self {
        match self {
            Self::StreamFailure(_) => self,
            cause => Self::StreamFailure(Box::new(ProviderStreamFailure {
                message,
                cause,
                response_received: false,
            })),
        }
    }

    /// Carry a transport handoff fact without changing the public cause or
    /// canonical partial. Only the private producer snapshot sets this flag.
    pub(crate) fn after_response(mut self) -> Self {
        if let Self::StreamFailure(failure) = &mut self {
            failure.response_received = true;
        }
        self
    }

    pub(crate) fn response_received(&self) -> bool {
        matches!(self, Self::StreamFailure(failure) if failure.response_received)
    }

    pub(crate) fn into_stream_parts(
        self,
    ) -> (Option<crate::message::AssistantMessage>, ProviderError) {
        match self {
            Self::StreamFailure(failure) => (Some(failure.message), failure.cause),
            cause => (None, cause),
        }
    }

    /// Whether this error is retryable (rate-limited, timeout, or transient
    /// network failure). Retry timing/backoff behavior is owned by the agent
    /// runtime; this is the taxonomy-layer retryability signal.
    pub fn is_retryable(&self) -> bool {
        if let Self::StreamFailure(failure) = self {
            return failure.cause.is_retryable();
        }
        if let Self::ServiceFault {
            status, error_name, ..
        } = self
        {
            // The taxonomy-layer projection of the source's classification;
            // the acquisition owner re-classifies exactly through the frozen
            // mode rules.
            return crate::bedrock::retry::classify_service_failure(*status, error_name.as_deref())
                .is_retryable();
        }
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
            ProviderError::StreamFailure(failure) => failure.cause.category(),
            ProviderError::AuthFailed(_)
            | ProviderError::CredentialNeeded { .. }
            | ProviderError::CredentialRevoked { .. }
            | ProviderError::AccountIdMissing { .. } => ProviderErrorCategory::Auth,
            ProviderError::Config(_)
            | ProviderError::MissingWireRoute { .. }
            | ProviderError::WireCompatMismatch { .. } => ProviderErrorCategory::Config,
            ProviderError::RequestFailed(_)
            | ProviderError::CallbackPhase(_)
            | ProviderError::StrictToolLimits { .. }
            | ProviderError::UnknownModel { .. } => ProviderErrorCategory::Request,
            ProviderError::Timeout | ProviderError::Network(_) => ProviderErrorCategory::Network,
            ProviderError::RateLimited { .. } => ProviderErrorCategory::RateLimit,
            ProviderError::ServiceFault { .. } => ProviderErrorCategory::Provider,
            ProviderError::ProviderSide(_) => ProviderErrorCategory::Provider,
            ProviderError::StreamError(_) => ProviderErrorCategory::Stream,
            ProviderError::UnsupportedCapability(_) => ProviderErrorCategory::Capability,
            ProviderError::Cancelled | ProviderError::LoginCancelled { .. } => {
                ProviderErrorCategory::Cancelled
            }
        }
    }

    /// Server-advised delay before retrying, in milliseconds.
    ///
    /// A [`ProviderError::RateLimited`] cause carries `retry_after_ms`;
    /// an adapter-state envelope preserves that delay unchanged.
    pub fn retry_after_ms(&self) -> Option<u64> {
        match self {
            ProviderError::StreamFailure(failure) => failure.cause.retry_after_ms(),
            ProviderError::RateLimited { retry_after_ms } => *retry_after_ms,
            ProviderError::ServiceFault { retry_after_ms, .. } => *retry_after_ms,
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
    /// User/runtime cancellation. The retry loop owns timing, backoff, and
    /// stream-abort behavior; this is the taxonomy/diagnostic-layer slot.
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

// ---------------------------------------------------------------------------
// Public typed diagnostic extraction (task 20.53.3, P20-S53-3).
// Pinned source: `.repo/pi-1.3.0/packages/ai/src/utils/diagnostics.ts`
// (`DiagnosticErrorInfo`/`extractDiagnosticError`).

/// Typed, secret-safe diagnostic details extracted from a [`ProviderError`]
/// (pinned `DiagnosticErrorInfo`; task 20.53.3).
///
/// `name` is the type-level error name (the [`ProviderError`] variant
/// discriminant), `message` is the [`ProviderError`] display text whose
/// detail carrier is a [`ProviderErrorSummary`] - secret-safe by
/// construction - and the typed code members preserve the structured
/// carriers a [`ProviderError::ServiceFault`] holds: its HTTP status and the
/// wire service error type name. The pinned JavaScript shape also carries a
/// `stack`; Rust has no equivalent execution-stack source, so no stack field
/// exists here.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DiagnosticErrorInfo {
    /// Type-level error name: the `ProviderError` variant discriminant.
    pub name: &'static str,
    /// Secret-safe human message (the error's display text).
    pub message: String,
    /// Typed HTTP status carried by a service-model fault.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_status: Option<u16>,
    /// Wire service error type name exposed by a service-model fault.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_name: Option<String>,
}

/// Extract typed diagnostic details from a [`ProviderError`] (pinned
/// `extractDiagnosticError`; task 20.53.3, P20-S53-3).
///
/// Preserves the typed code carriers (`ServiceFault` HTTP status and wire
/// error name) and the secret-safe source detail text; never retains raw
/// provider payloads (those never enter [`ProviderErrorSummary`]).
pub fn extract_diagnostic_error(error: &ProviderError) -> DiagnosticErrorInfo {
    let (name, http_status, error_name) = match error {
        ProviderError::StreamFailure(_) => ("StreamFailure", None, None),
        ProviderError::RateLimited { .. } => ("RateLimited", None, None),
        ProviderError::Timeout => ("Timeout", None, None),
        ProviderError::RequestFailed(_) => ("RequestFailed", None, None),
        ProviderError::CallbackPhase(_) => ("CallbackPhase", None, None),
        ProviderError::StrictToolLimits { .. } => ("StrictToolLimits", None, None),
        ProviderError::StreamError(_) => ("StreamError", None, None),
        ProviderError::AuthFailed(_) => ("AuthFailed", None, None),
        ProviderError::CredentialNeeded { .. } => ("CredentialNeeded", None, None),
        ProviderError::CredentialRevoked { .. } => ("CredentialRevoked", None, None),
        ProviderError::AccountIdMissing { .. } => ("AccountIdMissing", None, None),
        ProviderError::LoginCancelled { .. } => ("LoginCancelled", None, None),
        ProviderError::Network(_) => ("Network", None, None),
        ProviderError::Config(_) => ("Config", None, None),
        ProviderError::UnknownModel { .. } => ("UnknownModel", None, None),
        ProviderError::MissingWireRoute { .. } => ("MissingWireRoute", None, None),
        ProviderError::WireCompatMismatch { .. } => ("WireCompatMismatch", None, None),
        ProviderError::ProviderSide(_) => ("ProviderSide", None, None),
        ProviderError::ServiceFault {
            status, error_name, ..
        } => ("ServiceFault", Some(*status), error_name.clone()),
        ProviderError::UnsupportedCapability(_) => ("UnsupportedCapability", None, None),
        ProviderError::Cancelled => ("Cancelled", None, None),
    };
    DiagnosticErrorInfo {
        name,
        message: error.to_string(),
        http_status,
        error_name,
    }
}
