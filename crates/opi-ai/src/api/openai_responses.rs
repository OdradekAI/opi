//! OpenAI Responses wire adapter.
//!
//! This module owns Responses request projection, SSE/background decoding,
//! HTTP transport, status/error mapping, deferred operations and the typed
//! native Chat API. Provider identity and profile state live in
//! [`crate::providers::openai_responses`].

use std::sync::Arc;

use futures_util::{StreamExt, stream};
use secrecy::ExposeSecret;
use tokio_util::sync::CancellationToken;

use crate::api::contract::{DeferredApi, DeferredCancelCall, DeferredFetchCall};
use crate::api::shared::headers::ProviderHeaders;
use crate::api::shared::openai_responses::{
    ParsedEvent, ResponsesMapper, drain_sse_frames, parse_sse_frames,
};
use crate::auth::{AuthInvalidPolicy, AuthScheme, ResolvedAuth};
use crate::model_info::WireApi;
use crate::provider::{
    CacheRetention, EventStream, ModelInfo, Provider, ProviderError, ProviderErrorSummary, Request,
    github_copilot_initiator, github_copilot_route_headers,
};
use crate::providers::openai_responses::{
    OpenAiResponsesProvider, ResponsesAffinityPolicy, ResponsesConfig,
};
use crate::stream::{AssistantStreamEvent, CancelAwareReceiverStream, send_or_cancel};
use crate::types::deferred::DeferredHandle;

const DEFAULT_RESPONSES_PATH: &str = "/v1/responses";

impl OpenAiResponsesProvider {
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

    /// Build the standard Responses request body.
    pub fn build_request_body(
        &self,
        request: &Request,
    ) -> Result<serde_json::Value, crate::provider::ProviderError> {
        self.compile_request_body(request, None)
            .map(|(body, _, _)| body)
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
        native_options: Option<&OpenAiResponsesOptions>,
    ) -> Result<
        (
            serde_json::Value,
            Option<crate::api::shared::openai_responses::ResponsesWireTools>,
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
        let wire_facts = crate::api::shared::openai_responses::compile_projected_response_tools(
            transcript.messages(),
            &projection.tools.request_tools,
            crate::api::shared::openai_responses::ResponsesToolOptions {
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
        let native = native_options;
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
        let mut input = crate::api::shared::openai_responses::convert_projected_messages(
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
                request.sampling_params.as_ref(),
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
        )
        .with_deferred_terminal();
        let mut events = Vec::new();
        for frame in parse_sse_frames(sse_body) {
            match mapper.decode_frame(&frame) {
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
        if !events.iter().any(Result::is_err) {
            events.extend(mapper.flush_pending_terminal());
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
        wire_facts: Option<crate::api::shared::openai_responses::ResponsesWireTools>,
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
        let url = crate::api::shared::endpoint::join_endpoint(&base_url, &route_path);
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
        let mut decoder = crate::api::shared::text_decode::StreamTextDecoder::new();
        let mut response_activated_at: Option<std::time::Instant> = None;
        let mut mapper = ResponsesMapper::new(
            &provider_id,
            crate::model_info::WireApi::OpenAiResponses,
            &requested_model,
        )
        .with_deferred_terminal()
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
                    if let Ok(raw) = serde_json::from_str::<serde_json::Value>(&frame.data)
                        && let Err(error) = callbacks.notify_raw_async(&raw).await
                    {
                        return Err(mapper.failure(error));
                    }
                    match mapper.decode_frame(&frame) {
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
            if mapper.has_pending_terminal() {
                for item in mapper.flush_pending_terminal() {
                    let item = Self::stamp_done_duration(item, response_activated_at);
                    if !send_or_cancel(&cancel, &tx, item).await? {
                        return Ok(());
                    }
                }
            } else if !mapper.saw_done {
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

/// Explicit OpenAI Responses Background adapter for one catalog model.
///
/// Ordinary Responses calls remain on the existing streaming provider. This
/// adapter is registered separately through `Provider::deferred_api` and is
/// selected only by `Models::submit_deferred`/`fetch_deferred`/
/// `cancel_deferred`.
#[derive(Clone)]
pub struct OpenAiResponsesDeferredApi {
    provider: Arc<OpenAiResponsesProvider>,
    api_id: crate::types::ApiId,
    provider_id: String,
    model_id: String,
}

impl std::fmt::Debug for OpenAiResponsesDeferredApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiResponsesDeferredApi")
            .field("api_id", &self.api_id)
            .field("provider_id", &self.provider_id)
            .field("model_id", &self.model_id)
            .finish()
    }
}

impl OpenAiResponsesDeferredApi {
    fn new(provider: Arc<OpenAiResponsesProvider>, model_id: String) -> Self {
        Self {
            provider_id: provider.provider_id.clone(),
            provider,
            api_id: crate::types::ApiId::new("openai-responses"),
            model_id,
        }
    }

    fn auth_header(&self, auth: &ResolvedAuth) -> Result<(&'static str, String), ProviderError> {
        let secret = auth.secret.expose_secret();
        match auth.scheme {
            AuthScheme::Bearer => Ok(("authorization", format!("Bearer {secret}"))),
            AuthScheme::ApiKey if self.provider_id == "cloudflare-ai-gateway" => {
                Ok(("cf-aig-authorization", format!("Bearer {secret}")))
            }
            AuthScheme::ApiKey | AuthScheme::AwsSigV4(_) | AuthScheme::AnthropicFederation => Err(
                ProviderError::Config(ProviderErrorSummary::attested_static(
                    "OpenAI Responses accepts only Bearer authentication",
                )),
            ),
        }
    }

    async fn json_request(
        &self,
        method: reqwest::Method,
        path: String,
        auth: ResolvedAuth,
        body: Option<serde_json::Value>,
        timeout: Option<std::time::Duration>,
        cancel: CancellationToken,
        extra_headers: Vec<(String, String)>,
        callbacks: crate::callbacks::RequestCallbacks,
    ) -> Result<serde_json::Value, ProviderError> {
        let (header_name, header_value) = self.auth_header(&auth)?;
        let model_base = self
            .provider
            .models
            .iter()
            .find(|model| model.id == self.model_id)
            .and_then(|model| model.base_url.as_deref())
            .unwrap_or(&self.provider.base_url);
        let base_url =
            crate::auth::builtin::effective_base_url(&self.provider_id, model_base, &auth);
        let url = crate::api::shared::endpoint::join_endpoint(&base_url, &path);
        let mut request = self
            .provider
            .client
            .client()
            .request(method, url)
            .header(header_name, header_value)
            .header("content-type", "application/json");
        if let Some(timeout) = timeout {
            request = request.timeout(timeout);
        }
        let model_headers = self
            .provider
            .models
            .iter()
            .find(|model| model.id == self.model_id)
            .map(|model| model.headers.as_slice())
            .unwrap_or_default();
        let headers = self.provider.headers.merge_model_request(
            model_headers,
            &self.provider.route_headers,
            &extra_headers,
        )?;
        let headers = callbacks.apply_header_transform_async(&headers).await?;
        for (name, value) in headers {
            request = request.header(name, value);
        }
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(ProviderError::Cancelled),
            response = request.send() => response.map_err(|error| {
                if error.is_timeout() {
                    ProviderError::Timeout
                } else {
                    ProviderError::Network(ProviderErrorSummary::sanitized(error.to_string()))
                }
            })?,
        };
        let status = response.status();
        let headers = response.headers().clone();
        let observed_headers: Vec<(String, String)> = headers
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_ascii_lowercase(),
                    value.to_str().unwrap_or_default().to_owned(),
                )
            })
            .collect();
        callbacks
            .notify_response_async(status.as_u16(), &observed_headers)
            .await?;
        let bytes = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(ProviderError::Cancelled),
            bytes = response.bytes() => bytes.map_err(|error| {
                ProviderError::Network(ProviderErrorSummary::sanitized(error.to_string()))
            })?,
        };
        if !status.is_success() {
            return Err(map_http_status(
                status,
                &headers,
                self.provider.auth_invalid_policy,
                &self.provider_id,
            ));
        }
        let value = serde_json::from_slice(&bytes).map_err(|_| {
            ProviderError::StreamError(ProviderErrorSummary::attested_static(
                "OpenAI Responses Background returned a non-JSON body",
            ))
        })?;
        callbacks.notify_raw_async(&value).await?;
        Ok(value)
    }

    fn response_events(
        &self,
        response: serde_json::Value,
    ) -> Vec<Result<AssistantStreamEvent, ProviderError>> {
        let status = response
            .get("status")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");
        let id = response
            .get("id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        let Some(id) = id else {
            return vec![Err(ProviderError::StreamError(
                ProviderErrorSummary::attested_static(
                    "OpenAI Responses Background response omitted its id",
                ),
            ))];
        };
        if matches!(status, "queued" | "in_progress") {
            let message = crate::message::AssistantMessage {
                projection: None,
                content: Vec::new(),
                api: crate::ApiKind::OpenAi,
                provider: self.provider_id.clone(),
                model: self.model_id.clone(),
                response_model: response
                    .get("model")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
                wire: Some(WireApi::OpenAiResponses),
                response_id: Some(id.clone()),
                usage: crate::stream::Usage::default(),
                stop_reason: crate::stream::StopReason::Deferred,
                error_message: None,
                timestamp_ms: crate::time::now_ms(),
                diagnostics: Vec::new(),
                duration_ms: None,
                raw_stop_reason: Some(status.to_owned()),
                provider_thinking_level: None,
                deferred: Some(Box::new(DeferredHandle {
                    provider: self.provider_id.clone(),
                    model_id: self.model_id.clone(),
                    api: self.api_id.as_str().to_owned(),
                    id,
                    expires_at_ms: response
                        .get("expires_at")
                        .and_then(serde_json::Value::as_u64)
                        .map(|seconds| seconds.saturating_mul(1000)),
                    poll_after_ms: None,
                    data: None,
                })),
            };
            return vec![
                Ok(AssistantStreamEvent::Start {
                    partial: message.clone(),
                }),
                Ok(AssistantStreamEvent::Done {
                    reason: crate::stream::StopReason::Deferred,
                    message,
                }),
            ];
        }
        if matches!(status, "failed" | "cancelled" | "incomplete") {
            return vec![Err(ProviderError::StreamError(
                ProviderErrorSummary::sanitized(format!(
                    "OpenAI Responses Background response ended with status '{status}'"
                )),
            ))];
        }
        if status != "completed" {
            return vec![Err(ProviderError::StreamError(
                ProviderErrorSummary::sanitized(format!(
                    "OpenAI Responses Background returned unknown status '{status}'"
                )),
            ))];
        }
        let mut mapper =
            ResponsesMapper::new(&self.provider_id, WireApi::OpenAiResponses, &self.model_id)
                .with_deferred_terminal();
        let mut events = Vec::new();
        let created = serde_json::json!({
            "type": "response.created",
            "response": {
                "id": id,
                "status": "in_progress",
                "model": response.get("model").cloned().unwrap_or_else(|| {
                    serde_json::Value::String(self.model_id.clone())
                }),
            },
        });
        let mut wire_events = vec![created];
        if let Some(output) = response.get("output").and_then(serde_json::Value::as_array) {
            for (output_index, item) in output.iter().enumerate() {
                if item.get("type").and_then(serde_json::Value::as_str) != Some("message") {
                    continue;
                }
                let mut added_item = item.clone();
                if let Some(object) = added_item.as_object_mut() {
                    object.insert("content".to_owned(), serde_json::json!([]));
                }
                wire_events.push(serde_json::json!({
                    "type": "response.output_item.added",
                    "output_index": output_index,
                    "item": added_item,
                }));
                if let Some(content) = item.get("content").and_then(serde_json::Value::as_array) {
                    for (content_index, part) in content.iter().enumerate() {
                        if part.get("type").and_then(serde_json::Value::as_str)
                            != Some("output_text")
                        {
                            continue;
                        }
                        let text = part
                            .get("text")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default();
                        wire_events.push(serde_json::json!({
                            "type": "response.content_part.added",
                            "output_index": output_index,
                            "content_index": content_index,
                            "part": {"type": "output_text", "text": ""},
                        }));
                        if !text.is_empty() {
                            wire_events.push(serde_json::json!({
                                "type": "response.output_text.delta",
                                "output_index": output_index,
                                "content_index": content_index,
                                "delta": text,
                            }));
                        }
                        wire_events.push(serde_json::json!({
                            "type": "response.output_text.done",
                            "output_index": output_index,
                            "content_index": content_index,
                            "text": text,
                        }));
                    }
                }
                wire_events.push(serde_json::json!({
                    "type": "response.output_item.done",
                    "output_index": output_index,
                    "item": item,
                }));
            }
        }
        wire_events.push(serde_json::json!({
            "type": "response.completed",
            "response": response,
        }));
        for event in wire_events {
            let parsed = mapper.decode_json(&event.to_string());
            let mut next = match parsed {
                ParsedEvent::Valid(event) => mapper.process(event),
                ParsedEvent::UsageError(error) => vec![Err(mapper.failure(
                    ProviderError::StreamError(ProviderErrorSummary::sanitized(error)),
                ))],
                ParsedEvent::Malformed { .. } => vec![Err(mapper.failure(
                    ProviderError::StreamError(ProviderErrorSummary::attested_static(
                        "malformed OpenAI Responses Background response",
                    )),
                ))],
            };
            let failed = next.iter().any(Result::is_err);
            events.append(&mut next);
            if failed {
                break;
            }
        }
        if !events.iter().any(Result::is_err) {
            events.extend(mapper.flush_pending_terminal());
        }
        events
    }
}

impl DeferredApi<()> for OpenAiResponsesDeferredApi {
    fn api_id(&self) -> &crate::types::ApiId {
        &self.api_id
    }

    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }

    fn default_options(&self) {}

    fn submit_prepared(&self, request: Request, _options: &(), auth: ResolvedAuth) -> EventStream {
        let this = self.clone();
        Box::pin(
            stream::once(async move {
                let Ok((mut body, _, _)) = this.provider.compile_request_body(&request, None)
                else {
                    return vec![Err(ProviderError::Config(
                        ProviderErrorSummary::attested_static(
                            "OpenAI Responses Background request compilation failed",
                        ),
                    ))];
                };
                body["background"] = serde_json::Value::Bool(true);
                body["store"] = serde_json::Value::Bool(true);
                body["stream"] = serde_json::Value::Bool(false);
                let callbacks = request.callbacks.ensure_call_scope();
                let body = match callbacks
                    .apply_payload_async(crate::callbacks::ORDINARY_PAYLOAD_VARIANT, body)
                    .await
                    .and_then(|body| {
                        crate::callbacks::validate_openai_payload_shape(
                            &body,
                            crate::callbacks::OpenAiPayloadShape::ResponsesBackground,
                        )?;
                        Ok(body)
                    }) {
                    Ok(body) => body,
                    Err(error) => return vec![Err(error)],
                };
                match this
                    .json_request(
                        reqwest::Method::POST,
                        this.provider.resolve_route_path(&this.model_id),
                        auth,
                        Some(body),
                        request.timeout,
                        request.cancel,
                        request.extra_headers,
                        callbacks,
                    )
                    .await
                {
                    Ok(response) => this.response_events(response),
                    Err(error) => vec![Err(error)],
                }
            })
            .flat_map(stream::iter),
        )
    }

    fn fetch_prepared(
        &self,
        handle: &DeferredHandle,
        _options: &(),
        auth: ResolvedAuth,
        call: DeferredFetchCall,
    ) -> EventStream {
        let this = self.clone();
        let id = handle.id.clone();
        Box::pin(
            stream::once(async move {
                match this
                    .json_request(
                        reqwest::Method::GET,
                        format!(
                            "{}/{}",
                            this.provider.resolve_route_path(&this.model_id),
                            id
                        ),
                        auth,
                        None,
                        call.timeout,
                        call.cancel,
                        call.extra_headers,
                        call.callbacks.ensure_call_scope(),
                    )
                    .await
                {
                    Ok(response) => this.response_events(response),
                    Err(error) => vec![Err(error)],
                }
            })
            .flat_map(stream::iter),
        )
    }

    fn cancel_prepared<'a>(
        &'a self,
        handle: &'a DeferredHandle,
        _options: &'a (),
        auth: ResolvedAuth,
        call: DeferredCancelCall,
    ) -> futures_util::future::BoxFuture<'a, Result<(), ProviderError>> {
        Box::pin(async move {
            self.json_request(
                reqwest::Method::POST,
                format!(
                    "{}/{}/cancel",
                    self.provider.resolve_route_path(&self.model_id),
                    handle.id
                ),
                auth,
                Some(serde_json::json!({})),
                call.timeout,
                call.cancel,
                call.extra_headers,
                call.callbacks.ensure_call_scope(),
            )
            .await
            .map(|_| ())
        })
    }
}

impl OpenAiResponsesProvider {
    pub(crate) fn stream_prepared_with_options(
        &self,
        mut request: Request,
        resolved: crate::auth::ResolvedAuth,
        native_options: Option<OpenAiResponsesOptions>,
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
                return configured.stream_prepared_with_options(request, resolved, native_options);
            }
            if self.chatgpt_signin.is_none() {
                let mut configured = self.clone();
                configured.chatgpt_signin = Some(self.prepared_signin(&effective_base, &resolved));
                return configured.stream_prepared_with_options(request, resolved, native_options);
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
            let affinity_enabled = request.cache_retention != CacheRetention::Disabled
                && session_id.is_some()
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
                            if config.send_session_id_header {
                                route_headers.push(("session_id".into(), session_id.to_owned()));
                            }
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
            let (body, wire_facts, effort_stamp) =
                match self.compile_request_body(&request, native_options.as_ref()) {
                    Ok(compiled) => compiled,
                    Err(error) => return Box::pin(stream::once(async move { Err(error) })),
                };

            let callbacks = request.callbacks.clone();
            let cancel = request.cancel.clone();
            // Explicit frozen telemetry context (task 20.54/AIR-055): the producer
            // task runs under the captured parent/Dispatch when provided; omission
            // observes nothing. Full cross-path proof closes in 20.55.
            let requested_service_tier = native_options
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
        _options: &crate::api::contract::SimpleOptions,
    ) -> Result<(), ProviderError> {
        validate_complete_responses_record(model)
    }

    fn stream_model_resource_attempt(
        &self,
        model: ModelInfo,
        options: crate::api::contract::SimpleOptions,
        request: Request,
        auth: crate::auth::ResolvedAuth,
        _attempt: crate::provider::AttemptContext,
        _resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        let mut configured = self.clone();
        configured.models = vec![model];
        configured.catalog_compat = true;
        configured.stream_prepared_with_options(
            request,
            auth,
            Some(OpenAiResponsesOptions::from_simple(&options)),
        )
    }

    fn id(&self) -> &str {
        &self.provider_id
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    fn deferred_api(&self) -> Option<Arc<dyn DeferredApi<()>>> {
        self.models
            .iter()
            .find(|model| model.id == "o3")
            .map(|model| {
                Arc::new(OpenAiResponsesDeferredApi::new(
                    Arc::new(self.clone()),
                    model.id.clone(),
                )) as Arc<dyn DeferredApi<()>>
            })
    }

    fn stream_prepared(
        &self,
        request: Request,
        resolved: crate::auth::ResolvedAuth,
    ) -> EventStream {
        self.stream_prepared_with_options(request, resolved, None)
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
    fn from_simple(options: &crate::api::contract::SimpleOptions) -> Self {
        Self {
            tool_choice: options.tool_choice.map(|choice| match choice {
                crate::api::contract::SimpleToolChoice::Auto => ResponsesToolChoice::Auto,
                crate::api::contract::SimpleToolChoice::None => ResponsesToolChoice::None,
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
    api_id: crate::types::ApiId,
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
            api_id: crate::types::ApiId::new("openai-responses"),
            provider,
            model,
        })
    }

    pub(crate) fn configured(&self, model: ModelInfo) -> OpenAiResponsesProvider {
        self.provider.dispatch_configured(model)
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

impl crate::api::contract::ChatApi<OpenAiResponsesOptions> for OpenAiResponsesChatApi {
    fn validate_simple_options(
        &self,
        model: &ModelInfo,
        _options: &crate::api::contract::SimpleOptions,
    ) -> Result<(), crate::provider::ProviderError> {
        validate_complete_responses_record(model)
    }

    fn api_id(&self) -> &crate::types::ApiId {
        &self.api_id
    }

    fn provider_id(&self) -> &str {
        &self.provider.provider_id
    }

    fn complete_record(&self) -> ModelInfo {
        self.model.clone()
    }

    fn simple_options(&self) -> OpenAiResponsesOptions {
        OpenAiResponsesOptions::from_simple(&crate::api::contract::SimpleOptions::default())
    }

    fn simple_options_for(
        &self,
        options: &crate::api::contract::SimpleOptions,
    ) -> OpenAiResponsesOptions {
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
        let _resources = resources;
        let provider = self.configured(model);
        provider.stream_prepared_with_options(request, auth, Some(options))
    }
}
