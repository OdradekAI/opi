//! LLM provider abstraction.

use std::pin::Pin;
use std::str::FromStr;

use futures_core::Stream;
use http::header::{HeaderName, HeaderValue};
use tokio_util::sync::CancellationToken;

use crate::credential::BoxAuthFuture;
use crate::message::{InputContent, Message, OutputContent};
use crate::model_info::ThinkingLevel;
use crate::stream::AssistantStreamEvent;

pub use crate::model_info::ModelInfo;

pub mod operation;
pub mod policies;
pub mod routes;

use operation::{DeferredApi, SimpleOptions};

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
    /// The rebuilt Bedrock Converse payload with strict sampling disabled
    /// for `Prefer` tools.
    BedrockStrictDowngrade,
}

impl PayloadVariant {
    /// The payload-freeze slot label (see
    /// [`crate::callbacks::RequestCallbacks`]).
    pub fn label(self) -> &'static str {
        match self {
            Self::Ordinary => crate::callbacks::ORDINARY_PAYLOAD_VARIANT,
            Self::AnthropicStrictDowngrade => crate::callbacks::STRICT_DOWNGRADE_PAYLOAD_VARIANT,
            Self::BedrockStrictDowngrade => "bedrock_strict_downgrade",
        }
    }
}

/// Provider trait — each concrete provider (Anthropic, OpenAI, etc.) implements this.
///
/// Runtime catalog refresh is an object-safe substrate. The coding agent has
/// no production refresh trigger; callers that invoke [`Provider::refresh_models`]
/// own collection-level atomic replacement.
pub trait Provider: Send + Sync {
    /// Validate explicit neutral options before authentication. The default
    /// rejects options this adapter has not admitted rather than losing them.
    fn validate_simple_options(
        &self,
        _model: &ModelInfo,
        options: &SimpleOptions,
    ) -> Result<(), ProviderError> {
        options.validate_default()
    }

    /// Normalize a neutral simple request before shared preparation.
    ///
    /// Providers may override this when their neutral entry preserves an
    /// unsupported thinking level for wire-native validation instead of
    /// clamping it to the nearest catalog level.
    fn prepare_simple_request(&self, model: &ModelInfo, request: &mut Request) {
        if request.thinking.enabled
            && let Some(level) = model
                .thinking_level_map
                .nearest_supported(request.thinking.level)
        {
            request.thinking.level = level;
        }
    }

    /// Decide whether an authenticated route may expose one catalog model.
    ///
    /// Providers with source-specific protocol restrictions override this hook
    /// so Models does not inspect wire identities during availability reads.
    fn model_allowed_after_auth(&self, _model: &ModelInfo, _responses_only: bool) -> bool {
        true
    }

    /// Whether this route owns neutral request preparation completely.
    fn skips_shared_request_preparation(&self, _model: &ModelInfo) -> bool {
        false
    }

    /// Apply provider-owned thinking budget policy after shared ceilings are
    /// resolved. The default preserves Anthropic and non-adaptive Bedrock
    /// numeric budget compatibility used by the built-in routes.
    fn prepare_thinking_request(
        &self,
        model: &ModelInfo,
        request: &mut Request,
        context_available: Option<u64>,
    ) {
        if !request.thinking.enabled {
            return;
        }
        let numeric_anthropic = matches!(
            &model.compat,
            crate::WireCompat::AnthropicMessages(compat) if !compat.force_adaptive_thinking
        );
        let numeric_bedrock = model.wire_api == crate::WireApi::BedrockConverseStream
            && !crate::model_info::supports_bedrock_adaptive_thinking(
                &model.id,
                &model.display_name,
            );
        if (numeric_anthropic || numeric_bedrock) && request.thinking.budget_tokens.is_none() {
            request.thinking.budget_tokens = Some(match request.thinking.level {
                crate::ThinkingLevel::Minimal => 1_024,
                crate::ThinkingLevel::Low => 2_048,
                crate::ThinkingLevel::Medium => 8_192,
                crate::ThinkingLevel::High
                | crate::ThinkingLevel::XHigh
                | crate::ThinkingLevel::Max => 16_384,
                crate::ThinkingLevel::None => 1_024,
            });
        }
        let numeric_claude = numeric_anthropic
            || (numeric_bedrock
                && (model.id.to_lowercase().contains("anthropic.claude")
                    || model.id.to_lowercase().contains("anthropic/claude")
                    || model.display_name.to_lowercase().contains("claude")));
        if numeric_claude
            && let (Some(budget), Some(limit)) =
                (request.thinking.budget_tokens, request.max_tokens)
        {
            let combined = limit
                .saturating_add(budget)
                .min(model.capabilities.max_output_tokens);
            request.max_tokens =
                Some(context_available.map_or(combined, |available| combined.min(available)));
        }
        if let (Some(budget), Some(limit)) = (request.thinking.budget_tokens, request.max_tokens) {
            request.thinking.budget_tokens = Some(budget.min(limit.saturating_sub(1_024)));
        }
    }

    /// Fallback wire identity for route metadata when a caller supplies no
    /// complete catalog record.
    fn default_wire_api(&self) -> crate::WireApi {
        crate::WireApi::OpenAiCompletions
    }
    /// Unique identifier for this provider instance (e.g. "anthropic").
    fn id(&self) -> &str;

    /// Models supported by this provider.
    fn models(&self) -> &[ModelInfo];

    /// Optional explicit Deferred backend owned by this provider route.
    /// Registration remains separate from the ordinary model catalog, while
    /// the production collection can reuse the same route auth resolver.
    fn deferred_api(&self) -> Option<std::sync::Arc<dyn DeferredApi<()>>> {
        None
    }

    /// Freeze source-derived routing after authentication and before attempts.
    ///
    /// The model and authentication are owned snapshots of this call. A route
    /// that resolves endpoint or deployment configuration may update them and
    /// return an immutable adapter used by every acquisition attempt. The
    /// default retains this adapter and does no source work.
    fn prepare_model_call(
        &self,
        _model: &mut ModelInfo,
        _auth: &mut crate::auth::ResolvedAuth,
    ) -> Result<Option<std::sync::Arc<dyn Provider>>, ProviderError> {
        Ok(None)
    }

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

    /// Dispatch using the logical call's already registered resource generation.
    /// The default excludes resource-less adapters; resource-owning adapters
    /// retain the lease in their producer until its local connection exits.
    fn stream_resource_attempt(
        &self,
        request: Request,
        auth: crate::auth::ResolvedAuth,
        attempt: AttemptContext,
        _resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        self.stream_attempt(request, auth, attempt)
    }

    /// Dispatch using the frozen complete record and the neutral options of
    /// this logical call. Metadata-aware adapters consume this record at the
    /// wire boundary; the default forwards resource and attempt ownership.
    fn stream_model_resource_attempt(
        &self,
        _model: ModelInfo,
        _options: SimpleOptions,
        request: Request,
        auth: crate::auth::ResolvedAuth,
        attempt: AttemptContext,
        resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        self.stream_resource_attempt(request, auth, attempt, resources)
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
    /// The context names the selected owner and forwards its optional force
    /// choice. The caller owns collection-level publication: successful owners
    /// publish through the store ACK while failed owners retain prior records.
    fn refresh_models(
        &self,
        _context: crate::catalog::CatalogRefreshContext,
    ) -> BoxAuthFuture<'_, Result<Option<Vec<ModelInfo>>, ProviderError>> {
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
/// (task 20.16, DEL-07). `Auto` is the normalized default. Codex applies the
/// same bounded reconnect and SSE fallback policy to all WebSocket preferences.
/// `Websocket` sends full input, while `Auto` and `WebsocketCached` can use
/// a session connection's continuation state. `Sse` selects HTTP SSE
/// exclusively. The preference guides transport selection only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransportPreference {
    /// Provider default resolution (WebSocket first where supported).
    #[default]
    Auto,
    /// HTTP SSE only; never attempts an alternate transport.
    Sse,
    /// WebSocket with full input.
    Websocket,
    /// WebSocket with continuation reuse when the session connection allows it.
    WebsocketCached,
}

/// A single request to a provider.
#[derive(Clone)]
pub struct Request {
    /// Frozen temporal and replay fidelity for this canonical input.
    pub projection: crate::transcript::ProjectionOptions,
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
    /// `None` is absent; `Some` preserves an explicit empty map without clearing
    /// lower-layer keys. Only OpenAI-compatible adapters apply the merged map.
    pub sampling_params: Option<crate::model_info::SamplingParams>,
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
    /// Explicit process-local observation context. Omission preserves ambient
    /// host logging and never captures a Span implicitly. Never serialized.
    pub telemetry_context: Option<crate::telemetry::TelemetryContext>,
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

/// Validate extra headers supplied on a [`Request`].
///
/// Rejects header names that are empty, contain control characters, or match
/// the provider-managed auth header set. Reserved-name rejection reuses the
/// single canonical `RESERVED_PROVIDER_HEADERS` list in
/// `types::headers` shared with `ProviderHeaders`, so every wire enforces
/// the same gate.
///
/// Returns `Err(ProviderError::RequestFailed(...))` on the first invalid header.
pub fn validate_extra_headers(headers: &[(String, String)]) -> Result<(), ProviderError> {
    use crate::types::headers::RESERVED_PROVIDER_HEADERS;

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

#[cfg(any(feature = "builtins", feature = "provider-protocol-common"))]
pub(crate) fn request_projection(
    provider: &dyn Provider,
    request: &Request,
    fallback_wire: crate::WireApi,
) -> Result<crate::transcript::ProjectionFacts, ProviderError> {
    let id = request
        .model
        .strip_prefix(&format!("{}:", provider.id()))
        .unwrap_or(&request.model);
    let model = provider.models().iter().find(|model| model.id == id);
    let wire = model.map_or(fallback_wire, |model| model.wire_api);
    crate::transcript::compile_request_projection(request, provider.id(), model, wire)
        .map(|projection| projection.facts)
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
    validate_request_structure_for_model(provider_id, model, request)?;
    if let Some(model) = model {
        validate_thinking_for_model(provider_id, model, request)?;
    }
    Ok(())
}

/// Validate model metadata and input content independently of neutral thinking.
/// Native adapters use this when their typed options own reasoning semantics.
pub fn validate_request_structure_for_model(
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
        let projection = crate::transcript::compile_request_projection(
            request,
            provider_id,
            Some(model),
            model.wire_api,
        )?;
        let declarations = projection
            .transcript
            .messages()
            .iter()
            .filter_map(|message| match message {
                Message::System(system) => Some(system.tools_added.iter()),
                _ => None,
            })
            .flatten();
        crate::constrained_sampling::validate_required_shapes(declarations).map_err(|error| {
            ProviderError::Config(ProviderErrorSummary::sanitized(error.detail()))
        })?;
    }
    Ok(())
}

fn validate_thinking_for_model(
    provider_id: &str,
    model: &ModelInfo,
    request: &Request,
) -> Result<(), ProviderError> {
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
    Ok(())
}

pub use crate::errors::{
    DiagnosticErrorInfo, ProviderError, ProviderErrorCategory, ProviderErrorSummary,
    ProviderStreamFailure, extract_diagnostic_error,
};
