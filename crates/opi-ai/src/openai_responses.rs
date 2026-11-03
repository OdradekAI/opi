//! Standard OpenAI Responses provider.
//!
//! `previous_response_id` is intentionally absent until opi's request model
//! carries prior-response state for server-side response chaining.

use std::sync::Arc;

use futures_util::{StreamExt, stream};
use secrecy::ExposeSecret;
use tokio_util::sync::CancellationToken;

use crate::auth::{AuthInvalidPolicy, AuthScheme, ResolvedAuth};
use crate::http::HttpClient;
use crate::model_info::WireApi;
use crate::openai_responses_shared::{
    ParsedEvent, ResponsesEvent, ResponsesMapper, drain_sse_frames, parse_sse_frames,
};
use crate::provider::{
    CacheRetention, EventStream, ModelInfo, Provider, ProviderError, ProviderErrorSummary, Request,
    github_copilot_initiator, github_copilot_route_headers,
};
use crate::provider_headers::ProviderHeaders;
use crate::registry::ModelCapabilities;
use crate::stream::{AssistantStreamEvent, CancelAwareReceiverStream, send_or_cancel};

const DEFAULT_BASE_URL: &str = "https://api.openai.com";
const DEFAULT_RESPONSES_PATH: &str = "/v1/responses";

/// Native OpenAI Responses request flags.
#[derive(Debug, Clone)]
pub struct ResponsesConfig {
    pub source: crate::model_info::OpenAiResponsesSourceCompat,
    pub store: Option<bool>,
    pub reasoning_effort: Option<String>,
    pub send_session_id_header: bool,
}
impl Default for ResponsesConfig {
    fn default() -> Self {
        Self {
            source: crate::model_info::OpenAiResponsesCompat::default().source,
            store: None,
            reasoning_effort: None,
            send_session_id_header: true,
        }
    }
}

/// Construction-time session-affinity policy for the reusable Responses wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponsesAffinityPolicy {
    /// Built-in direct Responses: cache key and request id are automatic.
    Direct,
    /// Custom/proxy Responses without the reviewed affinity opt-in.
    CustomDisabled,
    /// Custom/proxy Responses with the reviewed full affinity mapping enabled.
    CustomOptIn,
}

/// Standard OpenAI Responses provider, also used as a route by mapped
/// providers whose model metadata declares the standard Responses wire.
#[derive(Clone)]
pub struct OpenAiResponsesProvider {
    base_url: String,
    models: Vec<ModelInfo>,
    config: ResponsesConfig,
    provider_id: String,
    headers: ProviderHeaders,
    route_headers: Vec<(String, String)>,
    catalog_compat: bool,
    auth_invalid_policy: AuthInvalidPolicy,
    affinity_policy: ResponsesAffinityPolicy,
    copilot_headers: bool,
    client: Arc<HttpClient>,
    /// Typed native options set per dispatch by the public adapter (task
    /// 20.37); `None` on the plain provider path.
    native_options: Option<OpenAiResponsesOptions>,
    /// Frozen secret-free classification for this prepared call.
    chatgpt_signin: Option<bool>,
}

/// Built-in OpenAI Responses model metadata.
pub fn model_catalog() -> Vec<ModelInfo> {
    vec![
        ModelInfo::new(
            "gpt-4o",
            "GPT-4o",
            WireApi::OpenAiResponses,
            ModelCapabilities::new(128_000, 16_384)
                .with_images(true)
                .with_streaming(true),
        ),
        ModelInfo::new(
            "gpt-4o-mini",
            "GPT-4o Mini",
            WireApi::OpenAiResponses,
            ModelCapabilities::new(128_000, 16_384)
                .with_images(true)
                .with_streaming(true),
        ),
        ModelInfo::new(
            "o3",
            "o3",
            WireApi::OpenAiResponses,
            ModelCapabilities::new(200_000, 100_000)
                .with_images(true)
                .with_streaming(true)
                .with_thinking(true),
        ),
        ModelInfo::new(
            "o4-mini",
            "o4-mini",
            WireApi::OpenAiResponses,
            ModelCapabilities::new(200_000, 100_000)
                .with_images(true)
                .with_streaming(true)
                .with_thinking(true),
        ),
    ]
}

impl OpenAiResponsesProvider {
    pub fn new(base_url: Option<String>) -> Self {
        Self::with_client(base_url, Arc::new(HttpClient::new()))
    }

    /// Create with a shared HTTP client.
    pub fn with_client(base_url: Option<String>, client: Arc<HttpClient>) -> Self {
        Self::with_auth(base_url, ResponsesConfig::default(), client)
            .with_auth_invalid_policy(AuthInvalidPolicy::Static)
    }

    /// Create with native Responses request flags.
    pub fn new_with_config(base_url: Option<String>, config: ResponsesConfig) -> Self {
        Self::with_auth(base_url, config, Arc::new(HttpClient::new()))
            .with_auth_invalid_policy(AuthInvalidPolicy::Static)
    }

    /// Build with provider route config. The secret arrives per-call via
    /// [`crate::auth::ResolvedAuth`] on `Provider::stream_prepared`.
    pub fn with_auth(
        base_url: Option<String>,
        config: ResponsesConfig,
        client: Arc<HttpClient>,
    ) -> Self {
        Self::with_auth_extra(
            base_url,
            config,
            "openai-responses".into(),
            Vec::new(),
            client,
        )
        .with_auth_invalid_policy(AuthInvalidPolicy::CredentialManaged)
        .with_affinity_policy(ResponsesAffinityPolicy::Direct)
    }

    /// Build with provider id and route headers.
    pub fn with_auth_extra(
        base_url: Option<String>,
        config: ResponsesConfig,
        provider_id: String,
        extra_headers: Vec<(String, String)>,
        client: Arc<HttpClient>,
    ) -> Self {
        Self {
            base_url: base_url.unwrap_or_else(|| DEFAULT_BASE_URL.into()),
            models: model_catalog()
                .into_iter()
                .map(|mut model| {
                    if let crate::WireCompat::OpenAiResponses(compat) = &mut model.compat {
                        compat.source = config.source.clone();
                    }
                    model
                })
                .collect(),
            config,
            provider_id,
            headers: ProviderHeaders::default(),
            route_headers: extra_headers,
            catalog_compat: false,
            auth_invalid_policy: AuthInvalidPolicy::Static,
            affinity_policy: ResponsesAffinityPolicy::CustomDisabled,
            copilot_headers: false,
            client,
            native_options: None,
            chatgpt_signin: None,
        }
    }

    /// Clone with the typed dispatch state for one call (task 20.38): the
    /// azure dual-wire adapter compiles through this exact configuration
    /// (record + native options) without mutating a registered provider.
    pub(crate) fn dispatch_configured(
        &self,
        model: ModelInfo,
        options: OpenAiResponsesOptions,
    ) -> Self {
        let mut provider = self.clone();
        provider.models = vec![model];
        provider.native_options = Some(options);
        provider.catalog_compat = true;
        provider
    }

    /// Build a standard Responses route for a mapped provider.
    pub fn for_route(
        default_base_url: Option<String>,
        provider_id: String,
        headers: ProviderHeaders,
        models: Vec<ModelInfo>,
        client: Arc<HttpClient>,
    ) -> Self {
        Self {
            base_url: default_base_url.unwrap_or_else(|| DEFAULT_BASE_URL.into()),
            models,
            config: ResponsesConfig::default(),
            provider_id,
            headers,
            route_headers: Vec::new(),
            catalog_compat: true,
            auth_invalid_policy: AuthInvalidPolicy::Static,
            affinity_policy: ResponsesAffinityPolicy::CustomOptIn,
            copilot_headers: false,
            client,
            native_options: None,
            chatgpt_signin: None,
        }
    }

    /// Override how this route classifies provider 401/403 responses.
    pub fn with_auth_invalid_policy(mut self, policy: AuthInvalidPolicy) -> Self {
        self.auth_invalid_policy = policy;
        self
    }

    /// Override direct-versus-custom session-affinity behavior.
    pub fn with_affinity_policy(mut self, policy: ResponsesAffinityPolicy) -> Self {
        self.affinity_policy = policy;
        self
    }

    /// Enable GitHub Copilot's request-dependent route headers.
    pub fn with_copilot_headers(mut self) -> Self {
        self.copilot_headers = true;
        self
    }

    /// The resolved record's exact wire (task 20.38): replay identity follows
    /// the record (azure responses records replay under their own wire).
    fn resolve_wire(&self, model_id: &str) -> crate::model_info::WireApi {
        self.models
            .iter()
            .find(|model| model.id == model_id)
            .map(|model| model.wire_api)
            .unwrap_or(crate::model_info::WireApi::OpenAiResponses)
    }

    fn resolve_config(&self, model_id: &str) -> ResponsesConfig {
        if self.catalog_compat
            && let Some(ModelInfo {
                compat:
                    crate::model_info::WireCompat::OpenAiResponses(compat)
                    | crate::model_info::WireCompat::AzureOpenAiResponses(compat),
                ..
            }) = self.models.iter().find(|model| model.id == model_id)
        {
            return ResponsesConfig {
                source: compat.source.clone(),
                store: compat.store,
                reasoning_effort: compat.reasoning_effort.clone(),
                send_session_id_header: compat.send_session_id_header,
            };
        }
        self.config.clone()
    }

    fn prepared_signin(&self, final_base_url: &str, auth: &ResolvedAuth) -> bool {
        self.provider_id == "openai"
            && final_base_url == "https://api.openai.com/v1"
            && !auth.secret.expose_secret().starts_with("sk-")
    }

    fn affinity_format(
        &self,
        model_id: &str,
        config: &ResponsesConfig,
    ) -> crate::model_info::SessionAffinityFormat {
        config.source.session_affinity_format.unwrap_or_else(|| {
            let base = self
                .models
                .iter()
                .find(|model| model.id == model_id)
                .and_then(|model| model.base_url.as_deref())
                .unwrap_or(&self.base_url);
            let openrouter = self.provider_id == "openrouter" || base.contains("openrouter.ai");
            if openrouter {
                crate::model_info::SessionAffinityFormat::OpenRouter
            } else {
                crate::model_info::SessionAffinityFormat::OpenAi
            }
        })
    }

    fn resolve_route_path(&self, model_id: &str) -> String {
        let model = self.models.iter().find(|model| model.id == model_id);
        let selected_path = match model.map(|model| &model.compat) {
            Some(crate::model_info::WireCompat::OpenAiResponses(compat)) if self.catalog_compat => {
                compat.responses_path.as_str()
            }
            _ => DEFAULT_RESPONSES_PATH,
        };
        if selected_path == DEFAULT_RESPONSES_PATH
            && model.is_some_and(|model| model.base_url.is_some())
        {
            "/responses".into()
        } else {
            selected_path.into()
        }
    }

    /// Access the shared HTTP client.
    pub fn http_client(&self) -> &Arc<HttpClient> {
        &self.client
    }

    /// Build the standard Responses request body.
    pub fn build_request_body(
        &self,
        request: &Request,
    ) -> Result<serde_json::Value, crate::provider::ProviderError> {
        self.compile_request_body(request).map(|(body, _, _)| body)
    }

    /// Compile the body plus the per-call constrained-sampling facts (task
    /// 20.11) and the wire-native effort stamp (task 20.37) the stream
    /// mapper records on the reply.
    #[allow(clippy::type_complexity)]
    /// Compile the standard Responses body. `pub(crate)` (task 20.38): the
    /// azure-openai-responses adapter reuses this exact pinned parameter
    /// assembly (store/off/summary/include/replay semantics) instead of
    /// duplicating it.
    pub(crate) fn compile_request_body(
        &self,
        request: &Request,
    ) -> Result<
        (
            serde_json::Value,
            Option<crate::openai_responses_shared::ResponsesWireTools>,
            Option<String>,
        ),
        crate::provider::ProviderError,
    > {
        let model_id = request
            .model
            .split_once(':')
            .map(|(_, id)| id)
            .unwrap_or(&request.model);
        let config = self.resolve_config(model_id);
        let wire = self.resolve_wire(model_id);
        let ordinary = wire == crate::model_info::WireApi::OpenAiResponses;
        let signin = ordinary && self.chatgpt_signin.unwrap_or(false);
        let strict = config
            .source
            .supports_strict_mode
            .unwrap_or(wire == crate::model_info::WireApi::AzureOpenAiResponses);
        let selected_model = self.models.iter().find(|model| model.id == model_id);
        let projection = crate::transcript::compile_request_projection(
            request,
            &self.provider_id,
            selected_model,
            self.resolve_wire(model_id),
        )?;
        let transcript = projection.transcript.as_ref();
        let system_prompt = crate::transcript::get_initial_system_message(transcript.messages())
            .map(crate::transcript::system_message_text)
            .unwrap_or_default();
        let wire_facts = crate::openai_responses_shared::compile_projected_response_tools(
            transcript.messages(),
            &projection.tools.request_tools,
            crate::openai_responses_shared::ResponsesToolOptions {
                default_strict: Some(false),
                supports_strict_mode: strict,
                supports_grammar_tools: config.source.supports_grammar_tools.unwrap_or(false),
            },
        )?;
        let grammar_map = wire_facts
            .as_ref()
            .map(|wire| wire.grammar_input_properties.clone())
            .unwrap_or_default();

        // Canonical replay projection (task 20.7) before body building.
        let replay_route = crate::replay::ReplayRoute {
            wire: self.resolve_wire(model_id),
            provider: self.provider_id.clone(),
            model: model_id.to_owned(),
            supports_images: crate::replay::supports_images_in_catalog(&self.models, model_id),
        };
        let replay = &projection.replay;
        // Task 20.37: native options own effort/summary/service tier/tool
        // choice on the public adapter path; the plain path keeps the
        // neutral thinking intent. Mid-conversation effort stamps ride the
        // replayed input list when the compat flag is on.
        let native = self.native_options.as_ref();
        let requested_level = match native {
            Some(options) if !options.simple => options.reasoning_effort,
            _ => (request.thinking.enabled
                && request.thinking.level != crate::model_info::ThinkingLevel::None)
                .then_some(request.thinking.level),
        };
        let summary_spec = native
            .filter(|options| !options.simple)
            .and_then(|options| options.reasoning_summary.as_ref());
        // Summary without effort defaults the effort to medium (source
        // `options?.reasoningEffort ?? (summary ? "medium" : undefined)`).
        let requested_level = requested_level.or_else(|| {
            matches!(
                summary_spec,
                Some(
                    ResponsesReasoningSummary::Auto
                        | ResponsesReasoningSummary::Detailed
                        | ResponsesReasoningSummary::Concise
                )
            )
            .then_some(crate::model_info::ThinkingLevel::Medium)
        });
        let model_record = self.models.iter().find(|model| model.id == model_id);
        let model_reasons = model_record
            .map(|model| model.capabilities.supports_thinking)
            .unwrap_or(false);
        let resolve = |level: crate::model_info::ThinkingLevel| {
            model_record.map(|model| model.thinking_level_map.resolve(level))
        };
        let mut wire_effort: Option<String> = None;
        let mut off_branch = false;
        if model_reasons {
            match requested_level {
                Some(level) => {
                    if let Some(Ok(Some(effort))) = resolve(level) {
                        wire_effort = Some(effort);
                    }
                }
                None if self.provider_id != "github-copilot" => {
                    // Off branch (source `off ?? "none"`): reasoning models
                    // explicitly select the off mapping; copilot-style
                    // routes skip it.
                    if let Some(Ok(off)) = resolve(crate::model_info::ThinkingLevel::None) {
                        off_branch = true;
                        wire_effort = Some(off.unwrap_or_else(|| "none".into()));
                    }
                }
                None => {}
            }
        }
        // Mid-conversation stamps (task 20.37): the request effort stays
        // fixed while configuration items carry the level.
        let stamping = config.source.supports_mid_convo_effort.unwrap_or(false)
            && ordinary
            && !signin
            && requested_level.is_some_and(|level| level != crate::model_info::ThinkingLevel::None)
            && wire_effort.is_some();
        let stamp_effort = stamping.then(|| wire_effort.clone().unwrap_or_default());
        let instruction_role = if config.source.supports_developer_role.unwrap_or(true)
            && selected_model.is_some_and(|model| model.capabilities.supports_thinking)
        {
            "developer"
        } else {
            "system"
        };
        let mut input = crate::openai_responses_shared::convert_projected_messages(
            &replay.messages,
            &replay_route,
            &grammar_map,
            stamp_effort.as_deref(),
            &config.source,
            instruction_role,
        )?;
        if !system_prompt.is_empty() {
            input.insert(
                0,
                serde_json::json!({"role":instruction_role,"content":system_prompt}),
            );
        }
        let mut body = serde_json::json!({
            "model": model_id,
            "stream": true,
            "input": input,
        });
        if (!ordinary || (config.source.supports_max_output_tokens.unwrap_or(true) && !signin))
            && let Some(max_tokens) = request.max_tokens.filter(|limit| *limit > 0)
        {
            body["max_output_tokens"] = serde_json::Value::Number(max_tokens.max(16).into());
        }
        if !signin
            && let Some(temperature) = request.temperature
            && let Some(number) = serde_json::Number::from_f64(temperature)
        {
            body["temperature"] = serde_json::Value::Number(number);
        }
        if let Some(wire) = &wire_facts
            && !wire.tools.is_empty()
        {
            body["tools"] = serde_json::Value::Array(wire.tools.clone());
        }
        // Source semantics: store:false is sent unconditionally unless a
        // catalog/profile override pins another value.
        body["store"] = serde_json::Value::Bool(config.store.unwrap_or(false));
        if let Some(service_tier) = native
            .filter(|options| !options.simple)
            .and_then(|options| options.service_tier.as_ref())
        {
            body["service_tier"] = serde_json::Value::String(service_tier.wire_name().into());
        }
        if let Some(tool_choice) = native.and_then(|options| options.tool_choice.as_ref()) {
            body["tool_choice"] = tool_choice.wire_value();
        }
        // The cache key and retention policy are independent of transport
        // affinity headers and of one another.
        let cache_key_enabled = request.cache_retention != CacheRetention::Disabled
            && match self.affinity_policy {
                ResponsesAffinityPolicy::Direct => true,
                ResponsesAffinityPolicy::CustomDisabled => false,
                ResponsesAffinityPolicy::CustomOptIn => config.send_session_id_header,
            };
        if cache_key_enabled
            && let Some(session_id) = request.session_id.as_deref().filter(|id| !id.is_empty())
        {
            body["prompt_cache_key"] =
                serde_json::Value::String(session_id.chars().take(64).collect());
        }
        if ordinary && !signin {
            let explicit = config
                .source
                .supports_explicit_prompt_cache_mode
                .unwrap_or(false);
            let long = config.source.supports_long_cache_retention.unwrap_or(true);
            if explicit {
                match request.cache_retention {
                    CacheRetention::Disabled => {
                        body["prompt_cache_options"] = serde_json::json!({"mode":"explicit"})
                    }
                    CacheRetention::Long if long => {
                        body["prompt_cache_options"] = serde_json::json!({"ttl":"30m"})
                    }
                    _ => {}
                }
            } else if long && request.cache_retention == CacheRetention::Long {
                body["prompt_cache_retention"] = serde_json::json!("24h");
            }
        }
        if let Some(effort) = &wire_effort {
            // Fixed request effort while configuration items carry the
            // level (source CONFIGURATION_UPDATE_REQUEST_EFFORT).
            let request_effort = if stamping { "medium" } else { effort.as_str() };
            let reasoning = if off_branch {
                // The off branch sends effort only (source).
                serde_json::json!({"effort": request_effort})
            } else {
                let summary = match summary_spec {
                    Some(ResponsesReasoningSummary::Detailed) => "detailed",
                    Some(ResponsesReasoningSummary::Concise) => "concise",
                    // Absent and explicit null both land on the wire default
                    // `auto` (source `options?.reasoningSummary || "auto"`).
                    _ => "auto",
                };
                serde_json::json!({"effort": request_effort, "summary": summary})
            };
            body["reasoning"] = reasoning;
            if !off_branch {
                body["include"] = serde_json::json!(["reasoning.encrypted_content"]);
            } else if self.provider_id == "xai" {
                // xai reasoning models always request encrypted reasoning
                // (source), including on the off branch.
                body["include"] = serde_json::json!(["reasoning.encrypted_content"]);
            }
        }
        // Last so model and request sampling parameters override named
        // request fields (model base < effective level < request, per key).
        let effective_level = requested_level.unwrap_or(crate::model_info::ThinkingLevel::None);
        if let Some(model) = self.models.iter().find(|model| model.id == model_id)
            && let Some(sampling) = crate::model_info::resolve_sampling_params(
                model,
                effective_level,
                &request.sampling_params,
            )
        {
            for (key, value) in sampling {
                body[key.as_str()] = value;
            }
        }
        Ok((body, wire_facts, stamp_effort))
    }

    /// Map a complete in-memory SSE body without HTTP.
    pub fn stream_from_sse(&self, sse_body: &str, cancel: CancellationToken) -> EventStream {
        let mut mapper = ResponsesMapper::new(
            &self.provider_id,
            crate::model_info::WireApi::OpenAiResponses,
            "",
        );
        let mut events = Vec::new();
        for frame in parse_sse_frames(sse_body) {
            match ResponsesEvent::try_from_frame(&frame) {
                ParsedEvent::Valid(event) => {
                    events.extend(mapper.process(event));
                }
                ParsedEvent::UsageError(error) => {
                    events.push(Err(mapper.failure(ProviderError::StreamError(
                        ProviderErrorSummary::sanitized(error),
                    ))));
                    break;
                }
                ParsedEvent::Malformed { .. } => {
                    events.push(Err(mapper.failure(ProviderError::StreamError(
                        ProviderErrorSummary::attested_static(
                            "malformed OpenAI Responses SSE frame",
                        ),
                    ))));
                    break;
                }
            }
        }
        let _cancel = cancel;
        Box::pin(stream::iter(events))
    }

    /// Stamp the measured response duration onto a terminal Done message
    /// (AIR-061); already-known values and non-Done items pass unchanged.
    pub(crate) fn stamp_done_duration(
        item: Result<AssistantStreamEvent, crate::provider::ProviderError>,
        activated_at: Option<std::time::Instant>,
    ) -> Result<AssistantStreamEvent, crate::provider::ProviderError> {
        match (item, activated_at) {
            (
                Ok(AssistantStreamEvent::Done {
                    reason,
                    mut message,
                }),
                Some(activated_at),
            ) => {
                if message.duration_ms.is_none() {
                    message.duration_ms = Some(activated_at.elapsed().as_millis() as u64);
                }
                Ok(AssistantStreamEvent::Done { reason, message })
            }
            (item, _) => item,
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn stream_http(
        client: reqwest::Client,
        resolved: ResolvedAuth,
        base_url: String,
        route_path: String,
        auth_invalid_policy: AuthInvalidPolicy,
        extra_headers: Vec<(String, String)>,
        provider_id: String,
        requested_model: String,
        wire_facts: Option<crate::openai_responses_shared::ResponsesWireTools>,
        effort_stamp: Option<String>,
        requested_service_tier: Option<crate::stream::ServiceTier>,
        body: serde_json::Value,
        cancel: CancellationToken,
        timeout: Option<std::time::Duration>,
        callbacks: crate::callbacks::RequestCallbacks,
        snapshot: crate::stream::ProducerSnapshot,
        tx: tokio::sync::mpsc::Sender<Result<AssistantStreamEvent, ProviderError>>,
    ) -> Result<(), ProviderError> {
        let secret = resolved.secret.expose_secret();
        let bearer = match resolved.scheme {
            AuthScheme::ApiKey if provider_id == "cloudflare-ai-gateway" => {
                format!("Bearer {secret}")
            }
            AuthScheme::Bearer => format!("Bearer {secret}"),
            AuthScheme::ApiKey | AuthScheme::AwsSigV4(_) | AuthScheme::AnthropicFederation => {
                return Err(ProviderError::Config(
                    ProviderErrorSummary::attested_static(
                        "OpenAI Responses accepts only Bearer authentication",
                    ),
                ));
            }
        };
        let url = crate::endpoint::join_endpoint(&base_url, &route_path);
        let mut request = client
            .post(url)
            .header(
                if provider_id == "cloudflare-ai-gateway" {
                    "cf-aig-authorization"
                } else {
                    "authorization"
                },
                bearer,
            )
            .header("content-type", "application/json");
        if let Some(timeout) = timeout {
            request = request.timeout(timeout);
        }
        for (name, value) in extra_headers {
            request = request.header(name, value);
        }
        let response = request
            .body(serde_json::to_string(&body).unwrap_or_default())
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    ProviderError::Timeout
                } else {
                    ProviderError::Network(ProviderErrorSummary::sanitized(error.to_string()))
                }
            })?;
        // Response observer (task 20.51): fires once the status and headers
        // exist, before any body processing or error mapping.
        let observed_headers: Vec<(String, String)> = response
            .headers()
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_ascii_lowercase(),
                    value.to_str().unwrap_or_default().to_string(),
                )
            })
            .collect();
        let status = response.status();
        if !status.is_success() {
            let headers = response.headers().clone();
            let cause = map_http_status(status, &headers, auth_invalid_policy, &provider_id);
            return Err(if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                crate::http::openai_rate_limit_detail(response, cause, &cancel).await
            } else {
                cause
            });
        }
        // Response callback (V-36): fired exactly once on the successfully
        // acquired response; rejected dispatches never masquerade.
        snapshot.received_response();

        let mut bytes = response.bytes_stream();
        let mut buffer = String::new();
        let mut decoder = crate::text_decode::StreamTextDecoder::new();
        let mut response_activated_at: Option<std::time::Instant> = None;
        let mut mapper = ResponsesMapper::new(
            &provider_id,
            crate::model_info::WireApi::OpenAiResponses,
            &requested_model,
        )
        .with_requested_service_tier(requested_service_tier);
        // Task 20.11: the call's constrained-sampling facts ride the mapper.
        if let Some(wire) = wire_facts {
            mapper.set_grammar_tools(wire.grammar_input_properties);
            mapper.set_strict_tools(wire.strict_tools);
            mapper.seed_diagnostics(wire.diagnostics);
        }
        // Wire-native effort stamp (task 20.37): recorded on the reply for
        // configuration_update replay.
        mapper.set_provider_thinking_level(effort_stamp);
        mapper.set_snapshot(snapshot);
        callbacks
            .notify_response_async(status.as_u16(), &observed_headers)
            .await
            .map_err(|error| mapper.failure(error))?;
        let result: Result<(), ProviderError> = async {
            loop {
                let chunk = tokio::select! {
                    _ = cancel.cancelled() => return Err(mapper.failure(ProviderError::Cancelled)),
                    chunk = async {
                        response_activated_at.get_or_insert_with(std::time::Instant::now);
                        bytes.next().await
                    } => match chunk {
                        Some(chunk) => chunk,
                        None => break,
                    },
                };
                let chunk = chunk.map_err(|error| {
                    if error.is_timeout() {
                        ProviderError::Timeout
                    } else if error.is_connect() {
                        ProviderError::Network(ProviderErrorSummary::sanitized(error.to_string()))
                    } else {
                        ProviderError::StreamError(ProviderErrorSummary::sanitized(
                            error.to_string(),
                        ))
                    }
                })?;
                buffer.push_str(&decoder.push(&chunk));
                for frame in drain_sse_frames(&mut buffer) {
                    // Raw observer (task 20.51): the frame's parsed wire JSON,
                    // strictly before normalization; failure preserves partial.
                    if !mapper.saw_done
                        && let Ok(raw) = serde_json::from_str::<serde_json::Value>(&frame.data)
                        && let Err(error) = callbacks.notify_raw_async(&raw).await
                    {
                        return Err(mapper.failure(error));
                    }
                    match ResponsesEvent::try_from_frame(&frame) {
                        ParsedEvent::Valid(event) => {
                            for item in mapper.process(event) {
                                let item = Self::stamp_done_duration(item, response_activated_at);
                                if !send_or_cancel(&cancel, &tx, item).await? {
                                    return Ok(());
                                }
                            }
                        }
                        ParsedEvent::UsageError(error) => {
                            return Err(mapper.failure(ProviderError::StreamError(
                                ProviderErrorSummary::sanitized(error),
                            )));
                        }
                        ParsedEvent::Malformed { .. } => {
                            return Err(mapper.failure(ProviderError::StreamError(
                                ProviderErrorSummary::attested_static(
                                    "malformed OpenAI Responses SSE frame",
                                ),
                            )));
                        }
                    }
                }
            }
            let eof_tail = decoder.finish();
            buffer.push_str(&eof_tail);
            if !mapper.saw_done {
                let _ = send_or_cancel(
                    &cancel,
                    &tx,
                    Err(mapper.failure(ProviderError::StreamError(
                        ProviderErrorSummary::attested_static(
                            "stream ended without a terminal event",
                        ),
                    ))),
                )
                .await;
            }
            Ok(())
        }
        .await;
        result.map_err(|cause| mapper.failure(cause))
    }
}

fn map_http_status(
    status: reqwest::StatusCode,
    headers: &reqwest::header::HeaderMap,
    policy: AuthInvalidPolicy,
    provider_id: &str,
) -> ProviderError {
    match status.as_u16() {
        401 | 403 => policy.error(provider_id),
        429 => ProviderError::RateLimited {
            retry_after_ms: crate::retry::parse_retry_after(headers),
        },
        408 | 504 => ProviderError::Timeout,
        code => ProviderError::ProviderSide(ProviderErrorSummary::from_http_response(code)),
    }
}

impl Provider for OpenAiResponsesProvider {
    fn prepare_model_call(
        &self,
        model: &mut ModelInfo,
        auth: &mut ResolvedAuth,
    ) -> Result<Option<Arc<dyn Provider>>, ProviderError> {
        let selected_base = model.base_url.as_deref().unwrap_or(&self.base_url);
        let final_base =
            crate::auth::builtin::effective_base_url(&self.provider_id, selected_base, auth);
        let mut prepared = self.clone();
        prepared.chatgpt_signin = Some(self.prepared_signin(&final_base, auth));
        if final_base != selected_base {
            model.base_url = Some(final_base);
        }
        Ok(Some(Arc::new(prepared)))
    }

    fn validate_simple_options(
        &self,
        model: &ModelInfo,
        _options: &crate::models::SimpleOptions,
    ) -> Result<(), ProviderError> {
        validate_complete_responses_record(model)
    }

    fn stream_model_resource_attempt(
        &self,
        model: ModelInfo,
        options: crate::models::SimpleOptions,
        request: Request,
        auth: crate::auth::ResolvedAuth,
        attempt: crate::provider::AttemptContext,
        resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        let mut configured = self.clone();
        configured.models = vec![model];
        configured.native_options = Some(OpenAiResponsesOptions::from_simple(&options));
        configured.catalog_compat = true;
        configured.stream_resource_attempt(request, auth, attempt, resources)
    }

    fn id(&self) -> &str {
        &self.provider_id
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    fn stream_prepared(
        &self,
        mut request: Request,
        resolved: crate::auth::ResolvedAuth,
    ) -> EventStream {
        let projection = /* frozen call */ match crate::provider::request_projection(self, &request, crate::WireApi::OpenAiResponses) {
            Ok(facts) => facts,
            Err(error) => return Box::pin(futures_util::stream::once(async move { Err(error) })),
        };
        let event_projection = projection.clone();
        crate::stream::project_call(event_projection, || -> EventStream {
            let model_id = request
                .model
                .split_once(':')
                .map(|(_, id)| id)
                .unwrap_or(&request.model);
            let selected_base = self
                .models
                .iter()
                .find(|model| model.id == model_id)
                .and_then(|model| model.base_url.as_deref())
                .unwrap_or(&self.base_url);
            let effective_base = crate::auth::builtin::effective_base_url(
                &self.provider_id,
                selected_base,
                &resolved,
            );
            if effective_base != selected_base {
                let mut configured = self.clone();
                if let Some(model) = configured
                    .models
                    .iter_mut()
                    .find(|model| model.id == model_id)
                {
                    model.base_url = Some(effective_base);
                } else {
                    configured.base_url = effective_base;
                }
                return configured.stream_prepared(request, resolved);
            }
            if self.chatgpt_signin.is_none() {
                let mut configured = self.clone();
                configured.chatgpt_signin = Some(self.prepared_signin(&effective_base, &resolved));
                return configured.stream_prepared(request, resolved);
            }
            request.callbacks = request.callbacks.ensure_call_scope();
            // Same as `stream` but skips per-call auth resolution: the collection
            // resolved and froze authentication once for the logical call.
            if let Err(error) = crate::provider::validate_request_capabilities(self, &request) {
                return Box::pin(stream::once(async move { Err(error) }));
            }
            let default_base_url = self.base_url.clone();
            let provider_id = self.provider_id.clone();
            let auth_invalid_policy = self.auth_invalid_policy;
            let affinity_policy = self.affinity_policy;
            let model_id = request
                .model
                .split_once(':')
                .map(|(_, id)| id)
                .unwrap_or(&request.model);
            let model_base_url = self
                .models
                .iter()
                .find(|model| model.id == model_id)
                .and_then(|model| model.base_url.clone());
            let config = self.resolve_config(model_id);
            let affinity_format = self.affinity_format(model_id, &config);
            let route_path = self.resolve_route_path(model_id);
            let copilot_headers = if self.copilot_headers {
                match github_copilot_route_headers(&request) {
                    Ok(mut headers) => {
                        headers.push((
                            "X-Initiator".into(),
                            github_copilot_initiator(&request).into(),
                        ));
                        Ok(headers)
                    }
                    Err(error) => Err(error),
                }
            } else {
                Ok(Vec::new())
            };
            let session_id = request.session_id.as_deref().filter(|id| !id.is_empty());
            let affinity_enabled = session_id.is_some()
                && match affinity_policy {
                    ResponsesAffinityPolicy::Direct => true,
                    ResponsesAffinityPolicy::CustomDisabled => false,
                    ResponsesAffinityPolicy::CustomOptIn => config.send_session_id_header,
                };
            let extra_headers = copilot_headers.and_then(|copilot_headers| {
                // Generated affinity is a default: ordinary configured, route,
                // model and request values retain their existing precedence.
                let mut route_headers = Vec::new();
                if affinity_enabled && let Some(session_id) = session_id {
                    use crate::model_info::SessionAffinityFormat;
                    match affinity_format {
                        SessionAffinityFormat::OpenAi => {
                            route_headers.push(("session_id".into(), session_id.to_owned()));
                            route_headers
                                .push(("x-client-request-id".into(), session_id.to_owned()));
                        }
                        SessionAffinityFormat::OpenAiNoSession => route_headers
                            .push(("x-client-request-id".into(), session_id.to_owned())),
                        SessionAffinityFormat::OpenRouter => {
                            route_headers.push(("x-session-id".into(), session_id.to_owned()))
                        }
                    }
                }
                route_headers.extend_from_slice(self.headers.configured());
                route_headers.extend_from_slice(&self.route_headers);
                route_headers.extend(copilot_headers);
                ProviderHeaders::default().merge_model_request(
                    self.models
                        .iter()
                        .find(|model| model.id == model_id)
                        .map(|model| model.headers.as_slice())
                        .unwrap_or_default(),
                    &route_headers,
                    &request.extra_headers,
                )
            });
            let requested_model = request
                .model
                .split_once(':')
                .map(|(_, id)| id)
                .unwrap_or(&request.model)
                .to_owned();
            let (body, wire_facts, effort_stamp) = match self.compile_request_body(&request) {
                Ok(compiled) => compiled,
                Err(error) => return Box::pin(stream::once(async move { Err(error) })),
            };

            let callbacks = request.callbacks.clone();
            let cancel = request.cancel.clone();
            // Explicit frozen telemetry context (task 20.54/AIR-055): the producer
            // task runs under the captured parent/Dispatch when provided; omission
            // observes nothing. Full cross-path proof closes in 20.55.
            let requested_service_tier = self
                .native_options
                .as_ref()
                .filter(|options| !options.simple)
                .and_then(|options| options.service_tier)
                .map(|tier| match tier {
                    ResponsesServiceTier::Auto => crate::stream::ServiceTier::Auto,
                    ResponsesServiceTier::Default => crate::stream::ServiceTier::Default,
                    ResponsesServiceTier::Flex => crate::stream::ServiceTier::Flex,
                    ResponsesServiceTier::Scale => crate::stream::ServiceTier::Scale,
                    ResponsesServiceTier::Priority => crate::stream::ServiceTier::Priority,
                    ResponsesServiceTier::Fast => crate::stream::ServiceTier::Fast,
                });
            let telemetry = request.telemetry_context.clone();
            let timeout = request.timeout;
            let client = self.client.client().clone();
            let snapshot = crate::stream::ProducerSnapshot::with_projection(projection);
            let producer_snapshot = snapshot.clone();
            let (tx, rx) = tokio::sync::mpsc::channel(64);
            let tx_closed = tx.clone();
            let producer_cancel = cancel.clone();
            let stream_cancel = cancel.clone();

            let producer = async move {
                let work = async {
                    let extra_headers = match extra_headers {
                        Ok(headers) => headers,
                        Err(error) => {
                            let _ = send_or_cancel(&producer_cancel, &tx, Err(error)).await;
                            return;
                        }
                    };
                    let extra_headers =
                        match callbacks.apply_header_transform_async(&extra_headers).await {
                            Ok(headers) => headers,
                            Err(error) => {
                                let _ = send_or_cancel(&producer_cancel, &tx, Err(error)).await;
                                return;
                            }
                        };
                    let body = match callbacks
                        .apply_payload_async(crate::callbacks::ORDINARY_PAYLOAD_VARIANT, body)
                        .await
                        .and_then(|body| {
                            crate::callbacks::validate_openai_payload_shape(
                                &body,
                                crate::callbacks::OpenAiPayloadShape::Responses,
                            )?;
                            Ok(body)
                        }) {
                        Ok(body) => body,
                        Err(error) => {
                            let _ = send_or_cancel(&producer_cancel, &tx, Err(error)).await;
                            return;
                        }
                    };
                    let base_url = crate::auth::builtin::effective_base_url(
                        &provider_id,
                        &model_base_url.unwrap_or(default_base_url),
                        &resolved,
                    );
                    if let Err(error) = Self::stream_http(
                        client,
                        resolved,
                        base_url,
                        route_path,
                        auth_invalid_policy,
                        extra_headers,
                        provider_id,
                        requested_model,
                        wire_facts,
                        effort_stamp,
                        requested_service_tier,
                        body,
                        cancel,
                        timeout,
                        callbacks,
                        producer_snapshot,
                        tx.clone(),
                    )
                    .await
                    {
                        let _ = send_or_cancel(&producer_cancel, &tx, Err(error)).await;
                    }
                };

                tokio::select! {
                    biased;
                    _ = producer_cancel.cancelled() => (),
                    _ = tx_closed.closed() => (),
                    _ = work => {},
                }
            };
            if let Some(context) = telemetry {
                tokio::spawn(context.instrument(producer));
            } else {
                tokio::spawn(producer);
            }
            Box::pin(CancelAwareReceiverStream::with_snapshot(
                rx,
                stream_cancel,
                snapshot,
            ))
        })
    }

    fn replace_model_catalog(&mut self, models: Vec<ModelInfo>) -> Result<(), ProviderError> {
        self.models = models;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Task 20.37 (P20-S37/ONB-03): typed native options and the public
// openai-responses direct adapter.
// ---------------------------------------------------------------------------

/// Full Responses `tool_choice` shape. The neutral simple entry maps only
/// `Auto`/`None`; `Required` and named forcing stay native.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponsesToolChoice {
    None,
    Auto,
    Required,
    Named(String),
}

impl ResponsesToolChoice {
    fn wire_value(&self) -> serde_json::Value {
        match self {
            Self::None => serde_json::json!("none"),
            Self::Auto => serde_json::json!("auto"),
            Self::Required => serde_json::json!("required"),
            Self::Named(name) => serde_json::json!({
                "type": "function",
                "name": name,
            }),
        }
    }
}

/// Typed Responses reasoning-summary value. `Null` keeps the typed surface's
/// explicit-JSON-null distinct from omission; both land on the wire default
/// `auto` (source `options?.reasoningSummary || "auto"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponsesReasoningSummary {
    Auto,
    Detailed,
    Concise,
    Null,
}

/// Request-side service tier (Responses native; the wire string passes
/// through exactly).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponsesServiceTier {
    Auto,
    Default,
    Flex,
    Scale,
    Priority,
    Fast,
}

impl ResponsesServiceTier {
    /// Wire string for the `service_tier` request field.
    pub fn wire_name(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Default => "default",
            Self::Flex => "flex",
            Self::Scale => "scale",
            Self::Priority => "priority",
            Self::Fast => "fast",
        }
    }
}

/// Typed Responses-native parameters. Common temperature, transport, cache,
/// cancellation and transcript inputs stay on [`Request`]. Construct with
/// [`Self::default`] and set the admitted fields; a native default does not
/// consume neutral `Request::thinking`, the simple entry supplies that
/// mapping separately.
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct OpenAiResponsesOptions {
    /// Full Responses `tool_choice` shape.
    pub tool_choice: Option<ResponsesToolChoice>,
    /// Native reasoning effort (`minimal`..`max`). Unsupported levels fail
    /// typed preflight; the neutral entry clamps instead.
    pub reasoning_effort: Option<crate::model_info::ThinkingLevel>,
    /// Reasoning summary selection; omission and explicit null both map to
    /// the wire default `auto`.
    pub reasoning_summary: Option<ResponsesReasoningSummary>,
    /// Requested service tier (source semantics; pricing multipliers are
    /// not computed on this wire).
    pub service_tier: Option<ResponsesServiceTier>,
    /// Dispatch provenance (task 20.38): `pub(crate)` so the azure
    /// dual-wire adapter can construct the mapped native options.
    pub(crate) simple: bool,
}

impl OpenAiResponsesOptions {
    /// Neutral mapping for the simple entry: only `Auto`/`None` tool choice
    /// and the request's neutral thinking intent are admitted.
    fn from_simple(options: &crate::models::SimpleOptions) -> Self {
        Self {
            tool_choice: options.tool_choice.map(|choice| match choice {
                crate::models::SimpleToolChoice::Auto => ResponsesToolChoice::Auto,
                crate::models::SimpleToolChoice::None => ResponsesToolChoice::None,
            }),
            reasoning_effort: None,
            reasoning_summary: None,
            service_tier: None,
            simple: true,
        }
    }

    fn is_simple(&self) -> bool {
        self.simple
    }
}

/// The public openai-responses direct adapter registered through
/// [`crate::Models::register_chat_api`]. Each dispatch clones its immutable
/// provider configuration; native options never mutate a registered
/// provider.
pub struct OpenAiResponsesChatApi {
    api_id: crate::models::ApiId,
    provider: Arc<OpenAiResponsesProvider>,
    model: ModelInfo,
}

impl std::fmt::Debug for OpenAiResponsesChatApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiResponsesChatApi")
            .field("api_id", &self.api_id)
            .finish_non_exhaustive()
    }
}

impl OpenAiResponsesChatApi {
    /// Bind a complete Responses record. Wrong-wire or invalid records fail
    /// before authentication or network activity.
    pub fn new(
        provider: Arc<OpenAiResponsesProvider>,
        model: ModelInfo,
    ) -> Result<Self, crate::provider::ProviderError> {
        validate_complete_responses_record(&model)?;
        Ok(Self {
            api_id: crate::models::ApiId::new("openai-responses"),
            provider,
            model,
        })
    }

    pub(crate) fn configured(
        &self,
        model: ModelInfo,
        options: OpenAiResponsesOptions,
    ) -> OpenAiResponsesProvider {
        self.provider.dispatch_configured(model, options)
    }
}

/// One complete, wire-valid Responses model record or a typed failure.
fn validate_complete_responses_record(
    model: &ModelInfo,
) -> Result<(), crate::provider::ProviderError> {
    if model.wire_api != crate::model_info::WireApi::OpenAiResponses
        || model.id.trim().is_empty()
        || model.validate().is_err()
    {
        return Err(crate::provider::ProviderError::Config(
            crate::provider::ProviderErrorSummary::attested_static(
                "openai-responses adapters require a valid complete Responses model record",
            ),
        ));
    }
    if let Some(base) = model.base_url.as_deref() {
        let valid = url::Url::parse(base)
            .is_ok_and(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some());
        if !valid {
            return Err(ProviderError::Config(
                ProviderErrorSummary::attested_static(
                    "The selected model requires an absolute HTTP or HTTPS API base URL",
                ),
            ));
        }
    }
    Ok(())
}

impl crate::models::ChatApi<OpenAiResponsesOptions> for OpenAiResponsesChatApi {
    fn validate_simple_options(
        &self,
        model: &ModelInfo,
        _options: &crate::models::SimpleOptions,
    ) -> Result<(), crate::provider::ProviderError> {
        validate_complete_responses_record(model)
    }

    fn api_id(&self) -> &crate::models::ApiId {
        &self.api_id
    }

    fn provider_id(&self) -> &str {
        &self.provider.provider_id
    }

    fn complete_record(&self) -> ModelInfo {
        self.model.clone()
    }

    fn simple_options(&self) -> OpenAiResponsesOptions {
        OpenAiResponsesOptions::from_simple(&crate::models::SimpleOptions::default())
    }

    fn simple_options_for(&self, options: &crate::models::SimpleOptions) -> OpenAiResponsesOptions {
        OpenAiResponsesOptions::from_simple(options)
    }

    fn session_resources(&self) -> Option<&dyn crate::codex_resources::SessionResourceOwner> {
        None
    }

    fn validate_native_request(
        &self,
        model: &ModelInfo,
        _request: &Request,
        options: &OpenAiResponsesOptions,
    ) -> Result<(), crate::provider::ProviderError> {
        validate_complete_responses_record(model)?;
        // Native/exact effort fails typed preflight on unsupported levels
        // (SP-03); the neutral entry clamps instead.
        if let Some(level) = options.reasoning_effort
            && model.thinking_level_map.resolve(level).is_err()
        {
            return Err(crate::provider::ProviderError::Config(
                crate::provider::ProviderErrorSummary::sanitized(format!(
                    "openai-responses native options reject unsupported thinking level {level:?}"
                )),
            ));
        }
        Ok(())
    }

    fn stream_prepared(
        &self,
        request: Request,
        options: OpenAiResponsesOptions,
        auth: ResolvedAuth,
    ) -> EventStream {
        self.stream_model_resource_prepared(self.model.clone(), request, options, auth, None)
    }

    fn stream_resource_prepared(
        &self,
        request: Request,
        options: OpenAiResponsesOptions,
        auth: ResolvedAuth,
        resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        self.stream_model_resource_prepared(self.model.clone(), request, options, auth, resources)
    }

    fn stream_model_resource_prepared(
        &self,
        model: ModelInfo,
        mut request: Request,
        options: OpenAiResponsesOptions,
        auth: ResolvedAuth,
        resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        if let Err(error) = self.validate_native_request(&model, &request, &options) {
            return Box::pin(futures_util::stream::once(async move { Err(error) }));
        }
        if !options.is_simple() {
            // Native effort is validated and projected independently of the
            // neutral thinking fields, which are unused on this path. The
            // neutral entry clamps at Models::stream_simple before dispatch.
            request.thinking = Default::default();
        }
        let provider = self.configured(model, options);
        if resources.is_none() {
            return provider.stream_prepared(request, auth);
        }
        let attempt = request.callbacks.inherited_attempt().unwrap_or_else(|| {
            crate::provider::AttemptContext {
                invocation_id: uuid::Uuid::new_v4().to_string(),
                attempt: 1,
                max_attempts: 1,
                payload_variant: crate::provider::PayloadVariant::Ordinary,
            }
        });
        provider.stream_resource_attempt(request, auth, attempt, resources)
    }
}
