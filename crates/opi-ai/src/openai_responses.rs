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
    ParsedEvent, ResponsesEvent, ResponsesMapper, convert_messages, drain_sse_frames,
    parse_sse_frames,
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
    /// Emit the top-level `store` field.
    pub store: Option<bool>,
    /// Legacy profile metadata. Request/model thinking maps are authoritative
    /// for wire output.
    pub reasoning_effort: Option<String>,
    /// Emit `"strict": true` on function tool definitions.
    pub strict_tools: bool,
    /// Emit the `session_id` header when the selected affinity policy is active.
    /// Direct-route prompt cache keys and request IDs are independent of this.
    pub send_session_id_header: bool,
}

impl Default for ResponsesConfig {
    fn default() -> Self {
        Self {
            store: None,
            reasoning_effort: None,
            strict_tools: false,
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
            models: model_catalog(),
            config,
            provider_id,
            headers: ProviderHeaders::default(),
            route_headers: extra_headers,
            catalog_compat: false,
            auth_invalid_policy: AuthInvalidPolicy::Static,
            affinity_policy: ResponsesAffinityPolicy::CustomDisabled,
            copilot_headers: false,
            client,
        }
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

    fn resolve_config(&self, model_id: &str) -> ResponsesConfig {
        if self.catalog_compat
            && let Some(ModelInfo {
                compat: crate::model_info::WireCompat::OpenAiResponses(compat),
                ..
            }) = self.models.iter().find(|model| model.id == model_id)
        {
            return ResponsesConfig {
                store: compat.store,
                reasoning_effort: compat.reasoning_effort.clone(),
                strict_tools: compat.strict_tools,
                send_session_id_header: compat.send_session_id_header,
            };
        }
        self.config.clone()
    }

    fn resolve_route_path(&self, model_id: &str) -> String {
        if self.catalog_compat
            && let Some(ModelInfo {
                compat: crate::model_info::WireCompat::OpenAiResponses(compat),
                ..
            }) = self.models.iter().find(|model| model.id == model_id)
        {
            return compat.responses_path.clone();
        }
        DEFAULT_RESPONSES_PATH.into()
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
        let model_id = request
            .model
            .split_once(':')
            .map(|(_, id)| id)
            .unwrap_or(&request.model);
        let config = self.resolve_config(model_id);
        // Canonical transcript consumption (AIR-020); this wire currently
        // projects by collapse.
        const SUPPORTS_MID_CONVO_SYSTEM_MESSAGES: bool = false;
        let transcript = crate::transcript::resolve_transcript(
            &request.context,
            SUPPORTS_MID_CONVO_SYSTEM_MESSAGES,
            crate::transcript::TranscriptProjection::NativeOrCollapse,
        )
        .expect("native-or-collapse projection never fails");
        let transcript = transcript.as_ref();
        let system_prompt = crate::transcript::get_current_system_prompt(transcript.messages());
        let current_tools = crate::transcript::get_current_tools(transcript.messages());

        // Canonical replay projection (task 20.7) before body building.
        let replay_route = crate::replay::ReplayRoute {
            wire: crate::model_info::WireApi::OpenAiResponses,
            provider: self.provider_id.clone(),
            model: model_id.to_owned(),
            supports_images: crate::replay::supports_images_in_catalog(&self.models, model_id),
        };
        let replay = crate::replay::project_replay(
            &crate::transcript::without_initial_system_message(transcript.messages()),
            &replay_route,
            crate::replay::ReplayProjection::NativeOrCollapse,
        )?;
        let mut body = serde_json::json!({
            "model": model_id,
            "stream": true,
            "input": convert_messages(&replay.messages, &replay_route),
        });
        if !system_prompt.is_empty() {
            body["instructions"] = serde_json::Value::String(system_prompt.clone());
        }
        if let Some(max_tokens) = request.max_tokens {
            body["max_output_tokens"] = serde_json::Value::Number(max_tokens.into());
        }
        if let Some(temperature) = request.temperature
            && let Some(number) = serde_json::Number::from_f64(temperature)
        {
            body["temperature"] = serde_json::Value::Number(number);
        }
        if !current_tools.is_empty() {
            body["tools"] = serde_json::Value::Array(
                current_tools
                    .iter()
                    .map(|tool| {
                        let mut value = serde_json::json!({
                            "type": "function",
                            "name": tool.name,
                            "description": tool.description,
                            "parameters": tool.input_schema,
                        });
                        if config.strict_tools {
                            value["strict"] = serde_json::Value::Bool(true);
                        }
                        value
                    })
                    .collect(),
            );
        }
        if let Some(store) = config.store {
            body["store"] = serde_json::Value::Bool(store);
        }
        if request.thinking.enabled
            && let Some(model) = self.models.iter().find(|model| model.id == model_id)
            && let Ok(Some(effort)) = model.thinking_level_map.resolve(request.thinking.level)
        {
            body["reasoning"] = serde_json::json!({"effort": effort});
        }
        // Last so model and request sampling parameters override named
        // request fields (model base < effective level < request, per key).
        let effective_level = if request.thinking.enabled {
            request.thinking.level
        } else {
            crate::model_info::ThinkingLevel::None
        };
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
        Ok(body)
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

    #[allow(clippy::too_many_arguments)]
    async fn stream_http(
        client: reqwest::Client,
        resolved: ResolvedAuth,
        base_url: String,
        route_path: String,
        config: ResponsesConfig,
        auth_invalid_policy: AuthInvalidPolicy,
        extra_headers: Vec<(String, String)>,
        provider_id: String,
        requested_model: String,
        body: serde_json::Value,
        cancel: CancellationToken,
        timeout: Option<std::time::Duration>,
        session_id: Option<String>,
        request_id: Option<String>,
        callbacks: crate::callbacks::RequestCallbacks,
        snapshot: crate::stream::ProducerSnapshot,
        tx: tokio::sync::mpsc::Sender<Result<AssistantStreamEvent, ProviderError>>,
    ) -> Result<(), ProviderError> {
        let secret = resolved.secret.expose_secret();
        let bearer = match resolved.scheme {
            AuthScheme::Bearer => format!("Bearer {secret}"),
            AuthScheme::ApiKey | AuthScheme::AwsSigV4(_) => {
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
            .header("authorization", bearer)
            .header("content-type", "application/json");
        if let Some(timeout) = timeout {
            request = request.timeout(timeout);
        }
        for (name, value) in extra_headers {
            request = request.header(name, value);
        }
        if let Some(request_id) = request_id {
            request = request.header("x-client-request-id", request_id);
        }
        if config.send_session_id_header
            && let Some(session_id) = session_id
        {
            let header_value =
                reqwest::header::HeaderValue::from_str(&session_id).map_err(|_| {
                    ProviderError::RequestFailed(ProviderErrorSummary::attested_static(
                        "invalid session-id header value",
                    ))
                })?;
            request = request.header("session_id", header_value);
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
        callbacks.notify_response(response.status().as_u16(), &observed_headers)?;

        let status = response.status();
        if !status.is_success() {
            let headers = response.headers().clone();
            return Err(map_http_status(
                status,
                &headers,
                auth_invalid_policy,
                &provider_id,
            ));
        }

        let mut bytes = response.bytes_stream();
        let mut buffer = String::new();
        let mut decoder = crate::text_decode::StreamTextDecoder::new();
        let mut mapper = ResponsesMapper::new(
            &provider_id,
            crate::model_info::WireApi::OpenAiResponses,
            &requested_model,
        );
        mapper.set_snapshot(snapshot);
        let result: Result<(), ProviderError> = async {
            loop {
                let chunk = tokio::select! {
                    _ = cancel.cancelled() => return Err(mapper.failure(ProviderError::Cancelled)),
                    chunk = bytes.next() => match chunk {
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
                        && let Err(error) = callbacks.notify_raw(&raw)
                    {
                        return Err(mapper.failure(error));
                    }
                    match ResponsesEvent::try_from_frame(&frame) {
                        ParsedEvent::Valid(event) => {
                            for item in mapper.process(event) {
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
    fn id(&self) -> &str {
        &self.provider_id
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    fn stream_prepared(
        &self,
        request: Request,
        resolved: crate::auth::ResolvedAuth,
    ) -> EventStream {
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
        let extra_headers = copilot_headers
            .and_then(|copilot_headers| {
                let mut route_headers = self.route_headers.clone();
                route_headers.extend(copilot_headers);
                self.headers
                    .merge_request(&route_headers, &request.extra_headers)
            })
            // Header transform (task 20.51): applied once to the merged
            // per-call set, revalidated against protected names.
            .and_then(|merged| request.callbacks.apply_header_transform(&merged));
        let session_id = if request.cache_retention != CacheRetention::Disabled {
            request.session_id.clone().filter(|id| !id.is_empty())
        } else {
            None
        };
        let affinity_enabled = session_id.is_some()
            && match affinity_policy {
                ResponsesAffinityPolicy::Direct => true,
                ResponsesAffinityPolicy::CustomDisabled => false,
                ResponsesAffinityPolicy::CustomOptIn => config.send_session_id_header,
            };
        let request_id = affinity_enabled.then(|| uuid::Uuid::now_v7().to_string());
        let requested_model = request
            .model
            .split_once(':')
            .map(|(_, id)| id)
            .unwrap_or(&request.model)
            .to_owned();
        let mut body = match self.build_request_body(&request) {
            Ok(body) => body,
            Err(error) => return Box::pin(stream::once(async move { Err(error) })),
        };
        if affinity_enabled && let Some(session_id) = session_id.as_deref() {
            body["prompt_cache_key"] =
                serde_json::Value::String(session_id.chars().take(64).collect());
        }
        // Payload callback (task 20.51): after sampling/compat/wire projection
        // (including the affinity stamp above), once per variant, frozen
        // across ordinary retries.
        let body = match request
            .callbacks
            .apply_payload(crate::callbacks::ORDINARY_PAYLOAD_VARIANT, body)
        {
            Ok(body) => body,
            Err(error) => return Box::pin(stream::once(async move { Err(error) })),
        };
        let callbacks = request.callbacks.clone();
        let cancel = request.cancel.clone();
        let timeout = request.timeout;
        let client = self.client.client().clone();
        let snapshot = crate::stream::ProducerSnapshot::default();
        let producer_snapshot = snapshot.clone();
        let (tx, rx) = tokio::sync::mpsc::channel(64);
        let tx_closed = tx.clone();
        let producer_cancel = cancel.clone();
        let stream_cancel = cancel.clone();

        tokio::spawn(async move {
            let work = async {
                let extra_headers = match extra_headers {
                    Ok(headers) => headers,
                    Err(error) => {
                        let _ = send_or_cancel(&producer_cancel, &tx, Err(error)).await;
                        return;
                    }
                };
                let base_url = resolved
                    .base_url
                    .clone()
                    .or(model_base_url)
                    .unwrap_or(default_base_url);
                if let Err(error) = Self::stream_http(
                    client,
                    resolved,
                    base_url,
                    route_path,
                    config,
                    auth_invalid_policy,
                    extra_headers,
                    provider_id,
                    requested_model,
                    body,
                    cancel,
                    timeout,
                    affinity_enabled.then_some(session_id).flatten(),
                    request_id,
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
                _ = tx_closed.closed() => (),
                _ = work => {},
            }
        });
        Box::pin(CancelAwareReceiverStream::with_snapshot(
            rx,
            stream_cancel,
            snapshot,
        ))
    }

    fn replace_model_catalog(&mut self, models: Vec<ModelInfo>) -> Result<(), ProviderError> {
        self.models = models;
        Ok(())
    }
}
