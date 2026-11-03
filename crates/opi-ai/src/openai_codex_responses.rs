//! Dedicated OpenAI Codex Responses wire.
//!
//! This subscription-specific provider is separate from standard
//! [`crate::openai_responses::OpenAiResponsesProvider`]. It owns the
//! `/codex/responses` body, account-id and managed-header contract, session
//! affinity, error classification, and SSE mapping.

use std::sync::Arc;

use futures_util::{StreamExt, stream};
use secrecy::ExposeSecret;
use tokio_util::sync::CancellationToken;

use crate::auth::{AuthScheme, ResolvedAuth};
use crate::http::HttpClient;
use crate::openai_responses_shared::{
    ParsedEvent, ResponsesEvent, ResponsesMapper, convert_projected_messages, drain_sse_frames,
};
use crate::provider::{
    CacheRetention, EventStream, ModelInfo, Provider, ProviderError, ProviderErrorSummary, Request,
    TransportPreference,
};

mod native;
pub(crate) mod ws_transport;
use crate::stream::{AssistantStreamEvent, CancelAwareReceiverStream, send_or_cancel};
pub use native::{
    CodexChatApi, CodexOptions, CodexReasoningEffort, CodexReasoningSummary, CodexServiceTier,
    CodexTextVerbosity, CodexToolChoice,
};

const PROVIDER_ID: &str = "openai-codex";
const DEFAULT_BASE_URL: &str = "https://chatgpt.com/backend-api";
const RESPONSES_PATH: &str = "/codex/responses";
const MALFORMED_STREAM_ERROR: &str = "OpenAI Codex returned malformed streaming data";

fn validate_simple_model(model: &ModelInfo) -> Result<(), ProviderError> {
    if model.id.trim().is_empty()
        || model.wire_api != crate::model_info::WireApi::OpenAiCodexResponses
        || model.validate().is_err()
    {
        return Err(ProviderError::Config(
            ProviderErrorSummary::attested_static(
                "Codex calls require a valid complete Codex model record",
            ),
        ));
    }
    Ok(())
}
const UPSTREAM_STREAM_ERROR: &str = "OpenAI Codex returned a streaming error";
const MANAGED_HEADERS: &[&str] = &[
    "authorization",
    "chatgpt-account-id",
    "originator",
    "openai-beta",
    "accept",
    "content-type",
    "session-id",
    "x-client-request-id",
];

/// OpenAI Codex provider using the subscription-specific Responses wire.
///
/// Authentication is supplied per-call by the provider collection via
/// [`crate::auth::ResolvedAuth`] on `Provider::stream_prepared` and must
/// include the non-secret ChatGPT account id. Provider-managed authorization,
/// account, originator, beta, content, accept, and session headers are
/// reserved from `Request::extra_headers`.
#[derive(Clone)]
pub struct OpenAiCodexResponsesProvider {
    base_url: String,
    models: Vec<ModelInfo>,
    client: Arc<HttpClient>,
    ws_state: Arc<ws_transport::WsTransportState>,
    native_options: Option<CodexOptions>,
}

impl OpenAiCodexResponsesProvider {
    pub fn new(base_url: Option<String>, models: Vec<ModelInfo>, client: Arc<HttpClient>) -> Self {
        Self {
            base_url: base_url.unwrap_or_else(|| DEFAULT_BASE_URL.into()),
            models,
            client,
            ws_state: Arc::default(),
            native_options: None,
        }
    }

    /// Build the subscription-specific Codex request body.
    pub fn build_request_body(
        &self,
        request: &Request,
    ) -> Result<serde_json::Value, crate::provider::ProviderError> {
        self.compile_request_body(request).map(|(body, _)| body)
    }

    /// The body plus this call's constrained-sampling facts (task 20.11).
    fn compile_request_body(
        &self,
        request: &Request,
    ) -> Result<
        (
            serde_json::Value,
            Option<crate::openai_responses_shared::ResponsesWireTools>,
        ),
        crate::provider::ProviderError,
    > {
        let model_id = request
            .model
            .strip_prefix("openai-codex:")
            .unwrap_or(&request.model);
        let selected_model = self.models.iter().find(|model| model.id == model_id);
        let projection = crate::transcript::compile_request_projection(
            request,
            PROVIDER_ID,
            selected_model,
            crate::WireApi::OpenAiCodexResponses,
        )?;
        let transcript = projection.transcript.as_ref();
        let system_prompt = crate::transcript::get_initial_system_message(transcript.messages())
            .map(crate::transcript::system_message_text)
            .unwrap_or_default();
        let source = selected_model
            .and_then(|model| match &model.compat {
                crate::WireCompat::OpenAiCodexResponses(compat) => Some(&compat.source),
                _ => None,
            })
            .cloned()
            .unwrap_or_default();
        let replay_route = crate::replay::ReplayRoute {
            wire: crate::model_info::WireApi::OpenAiCodexResponses,
            provider: PROVIDER_ID.to_owned(),
            model: model_id.to_owned(),
            supports_images: crate::replay::supports_images_in_catalog(&self.models, model_id),
        };
        let replay = &projection.replay;
        let wire_facts = crate::openai_responses_shared::compile_projected_response_tools(
            transcript.messages(),
            &projection.tools.request_tools,
            crate::openai_responses_shared::ResponsesToolOptions {
                default_strict: None,
                supports_strict_mode: source.supports_strict_mode.unwrap_or(true),
                supports_grammar_tools: source.supports_grammar_tools.unwrap_or(false),
            },
        )?;
        let grammar_map = wire_facts
            .as_ref()
            .map(|wire| wire.grammar_input_properties.clone())
            .unwrap_or_default();
        let input = convert_projected_messages(
            &replay.messages,
            &replay_route,
            &grammar_map,
            None,
            &source,
            if source.supports_developer_role.unwrap_or(true)
                && selected_model.is_some_and(|model| model.capabilities.supports_thinking)
            {
                "developer"
            } else {
                "system"
            },
        )?;
        let session_id = if request.cache_retention != CacheRetention::Disabled {
            request.session_id.as_deref().filter(|id| !id.is_empty())
        } else {
            None
        };
        let mut body = serde_json::json!({
            "model": model_id,
            "store": false,
            "stream": true,
            "instructions": if system_prompt.is_empty() {
                "You are a helpful assistant."
            } else {
                &system_prompt
            },
            "input": input,
            "tool_choice": "auto",
            "parallel_tool_calls": true,
            "text": {"verbosity": "low"},
            "include": ["reasoning.encrypted_content"],
        });
        if let Some(session_id) = session_id {
            body["prompt_cache_key"] =
                serde_json::Value::String(session_id.chars().take(64).collect());
        }
        if let Some(wire) = &wire_facts
            && !wire.tools.is_empty()
        {
            body["tools"] = serde_json::Value::Array(wire.tools.clone());
        }
        native::apply_native_options(
            &mut body,
            request,
            self.native_options.as_ref(),
            self.models.iter().find(|model| model.id == model_id),
        )?;
        if let Some(temperature) = request.temperature
            && let Some(number) = serde_json::Number::from_f64(temperature)
        {
            body["temperature"] = serde_json::Value::Number(number);
        }
        Ok((body, wire_facts))
    }

    #[allow(clippy::too_many_arguments)]
    async fn stream_http(
        client: reqwest::Client,
        resolved: ResolvedAuth,
        wire_facts: Option<crate::openai_responses_shared::ResponsesWireTools>,
        base_url: String,
        requested_model: String,
        requested_service_tier: Option<crate::stream::ServiceTier>,
        extra_headers: Vec<(String, String)>,
        body: serde_json::Value,
        cancel: CancellationToken,
        timeout: Option<std::time::Duration>,
        session_id: Option<String>,
        request_id: Option<String>,
        callbacks: crate::callbacks::RequestCallbacks,
        snapshot: crate::stream::ProducerSnapshot,
        tx: tokio::sync::mpsc::Sender<Result<AssistantStreamEvent, ProviderError>>,
    ) -> Result<(), ProviderError> {
        let account_id = resolved
            .account_id
            .as_deref()
            .filter(|account_id| !account_id.trim().is_empty())
            .ok_or_else(|| ProviderError::AccountIdMissing {
                provider_id: PROVIDER_ID.into(),
            })?;
        let secret = resolved.secret.expose_secret();
        let bearer = match resolved.scheme {
            AuthScheme::Bearer => format!("Bearer {secret}"),
            AuthScheme::ApiKey | AuthScheme::AwsSigV4(_) | AuthScheme::AnthropicFederation => {
                return Err(ProviderError::Config(
                    ProviderErrorSummary::attested_static(
                        "OpenAI Codex Responses accepts only Bearer authentication",
                    ),
                ));
            }
        };
        let url = crate::endpoint::join_endpoint(&base_url, RESPONSES_PATH);
        let mut request = client
            .post(url)
            .header("authorization", bearer)
            .header("chatgpt-account-id", account_id)
            .header("originator", "opi")
            .header("OpenAI-Beta", "responses=experimental")
            .header("accept", "text/event-stream")
            .header("content-type", "application/json");
        // C7: omit affinity headers when CacheRetention::Disabled.
        // C11: validate session-id before sending.
        if let Some(value) = session_id {
            let header_value = reqwest::header::HeaderValue::from_str(&value).map_err(|_| {
                ProviderError::RequestFailed(ProviderErrorSummary::attested_static(
                    "invalid session-id header value",
                ))
            })?;
            request = request.header("session-id", header_value);
        }
        if let Some(value) = request_id {
            request = request.header("x-client-request-id", value);
        }
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
        // Response observer (task 20.51): status and headers exist, before
        // any body processing or error mapping.
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
        if status.is_success() {
            snapshot.received_response();
        }
        tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(snapshot.failure(ProviderError::Cancelled)),
            result = callbacks.notify_response_async(status.as_u16(), &observed_headers) => {
                result.map_err(|error| snapshot.failure(error))?;
            }
        }
        if !status.is_success() {
            let headers = response.headers().clone();
            return Err(map_http_status(status, &headers));
        }

        let mut bytes = response.bytes_stream();
        let mut buffer = String::new();
        let mut decoder = crate::text_decode::StreamTextDecoder::new();
        let mut mapper = ResponsesMapper::new(
            PROVIDER_ID,
            crate::model_info::WireApi::OpenAiCodexResponses,
            &requested_model,
        )
        .with_requested_service_tier(requested_service_tier);
        // Task 20.11: the call's constrained-sampling facts ride the mapper.
        if let Some(wire) = wire_facts {
            mapper.set_grammar_tools(wire.grammar_input_properties);
            mapper.set_strict_tools(wire.strict_tools);
            mapper.seed_diagnostics(wire.diagnostics);
        }
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
                    // Raw observer (task 20.51): the frame's parsed wire JSON
                    // before the codex mapper; failure preserves the partial.
                    if let Ok(raw) = serde_json::from_str::<serde_json::Value>(&frame.data)
                        && let Err(error) = tokio::select! {
                            biased;
                            _ = cancel.cancelled() => Err(ProviderError::Cancelled),
                            result = callbacks.notify_raw_async(&raw) => result,
                        }
                    {
                        return Err(mapper.failure(error));
                    }
                    match ResponsesEvent::try_from_frame(&frame) {
                        ParsedEvent::Valid(event) => {
                            let event = match event {
                                ResponsesEvent::Error {
                                    raw_stop_reason, ..
                                } => ResponsesEvent::Error {
                                    message: UPSTREAM_STREAM_ERROR.to_owned(),
                                    raw_stop_reason,
                                },
                                event => event,
                            };
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
                                ProviderErrorSummary::attested_static(MALFORMED_STREAM_ERROR),
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

impl crate::codex_resources::SessionResourceOwner for OpenAiCodexResponsesProvider {
    fn new_namespace(&self) -> Arc<dyn crate::codex_resources::SessionResourceOwner> {
        Arc::new(CodexResourceOwner {
            state: Arc::default(),
        })
    }

    fn register_call(
        &self,
        request: &Request,
    ) -> Option<crate::codex_resources::CallResourceLease> {
        (request.transport != TransportPreference::Sse)
            .then(|| crate::codex_resources::CallResourceLease::new(self.ws_state.clone(), request))
    }

    fn cleanup_session_resources(
        &self,
        session: Option<&str>,
    ) -> crate::codex_resources::CleanupHandle {
        cleanup_state(self.ws_state.clone(), session)
    }
}

struct CodexResourceOwner {
    state: Arc<ws_transport::WsTransportState>,
}

impl Drop for CodexResourceOwner {
    fn drop(&mut self) {
        self.state.registry().cleanup(None);
        // Drop initiates local IO release; an explicit cleanup owns the await.
        drop(self.state.take_cleanup_connections(None));
        self.state.drop_pending_closes();
    }
}

impl crate::codex_resources::SessionResourceOwner for CodexResourceOwner {
    fn new_namespace(&self) -> Arc<dyn crate::codex_resources::SessionResourceOwner> {
        Arc::new(Self {
            state: Arc::default(),
        })
    }

    fn register_call(
        &self,
        request: &Request,
    ) -> Option<crate::codex_resources::CallResourceLease> {
        (request.transport != TransportPreference::Sse)
            .then(|| crate::codex_resources::CallResourceLease::new(self.state.clone(), request))
    }

    fn cleanup_session_resources(
        &self,
        session: Option<&str>,
    ) -> crate::codex_resources::CleanupHandle {
        cleanup_state(self.state.clone(), session)
    }
}

fn cleanup_state(
    state: Arc<ws_transport::WsTransportState>,
    session: Option<&str>,
) -> crate::codex_resources::CleanupHandle {
    if session == Some("") {
        return crate::codex_resources::CleanupHandle::failed(
            crate::codex_resources::CleanupError::EmptySessionId,
        );
    }
    let (cancelled, pending, (close_jobs, cleared_markers)) = state
        .registry()
        .cleanup_with(session, || state.prepare_cleanup(session));
    crate::codex_resources::CleanupHandle::from_future(async move {
        state.registry().await_exits(&pending).await;
        let mut report = crate::codex_resources::CleanupReport {
            cancelled_generations: cancelled,
            closed_connections: 0,
            cleared_fallback_sessions: cleared_markers,
            unconfirmed_effects: Vec::new(),
        };
        let mut completed = Vec::with_capacity(close_jobs.len());
        for job in close_jobs {
            let id = job.id;
            match job.resolve().await {
                Ok(()) => report.closed_connections += 1,
                Err(detail) => report.unconfirmed_effects.push(detail),
            }
            completed.push(id);
        }
        // There is no await after consuming stored results. Dropping a
        // partially polled barrier therefore leaves every release observable.
        state.finish_cleanup(&completed);
        report
            .unconfirmed_effects
            .extend(state.registry().take_unconfirmed(&pending));
        if report.unconfirmed_effects.is_empty() {
            Ok(report)
        } else {
            Err(crate::codex_resources::CleanupError::Incomplete { report })
        }
    })
}

impl Provider for OpenAiCodexResponsesProvider {
    fn validate_simple_options(
        &self,
        model: &ModelInfo,
        _options: &crate::models::SimpleOptions,
    ) -> Result<(), ProviderError> {
        validate_simple_model(model)
    }
    fn id(&self) -> &str {
        PROVIDER_ID
    }

    fn session_resources(&self) -> Option<&dyn crate::codex_resources::SessionResourceOwner> {
        Some(self)
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    fn stream_prepared(
        &self,
        request: Request,
        resolved: crate::auth::ResolvedAuth,
    ) -> EventStream {
        crate::models::drive_resolved_call(Arc::new(self.clone()), request, resolved)
    }

    fn stream_attempt(
        &self,
        request: Request,
        resolved: crate::auth::ResolvedAuth,
        attempt: crate::provider::AttemptContext,
    ) -> EventStream {
        self.stream_resource_attempt(request, resolved, attempt, None)
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
        if let Err(error) = validate_simple_model(&model) {
            return Box::pin(stream::once(async move { Err(error) }));
        }
        let mut provider = self.clone();
        provider.models = vec![model];
        provider.native_options = Some(native::simple_options(&options));
        provider.stream_resource_attempt(request, auth, attempt, resources)
    }

    fn stream_resource_attempt(
        &self,
        request: Request,
        resolved: crate::auth::ResolvedAuth,
        _attempt: crate::provider::AttemptContext,
        resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        let projection = /* frozen call */ match crate::provider::request_projection(self, &request, crate::WireApi::OpenAiCodexResponses) {
            Ok(facts) => facts,
            Err(error) => return Box::pin(futures_util::stream::once(async move { Err(error) })),
        };
        let event_projection = projection.clone();
        crate::stream::project_call(event_projection, || -> EventStream {
            // Same as `stream` but skips per-call auth resolution: the collection
            // resolved and froze authentication once for the logical call.
            // Resource generation registration (task 20.17): every WS-family
            // call registers before any auth attach or header/body transform so
            // a cleanup cutoff can cancel it in every phase. Pure SSE-preference
            // calls are not part of the WebSocket resource family and stay
            // unregistered.
            let transport = request.transport;
            let session_key: Option<String> = if request.cache_retention != CacheRetention::Disabled
            {
                request.session_id.clone().filter(|id| !id.is_empty())
            } else {
                None
            };
            let lease = resources.or_else(|| {
                (transport != TransportPreference::Sse).then(|| {
                    crate::codex_resources::CallResourceLease::new(self.ws_state.clone(), &request)
                })
            });
            let activity = lease.as_ref().and_then(|lease| lease.activity());
            if lease.is_some() && activity.is_none() {
                return Box::pin(stream::once(async { Err(ProviderError::Cancelled) }));
            }
            if let Err(error) = crate::provider::validate_request_capabilities(self, &request) {
                return Box::pin(stream::once(async move { Err(error) }));
            }
            let default_base_url = self.base_url.clone();
            let requested_service_tier = self
                .native_options
                .as_ref()
                .and_then(|options| options.service_tier)
                .and_then(|tier| {
                    use crate::stream::ServiceTier;
                    match tier {
                        CodexServiceTier::Auto => Some(ServiceTier::Auto),
                        CodexServiceTier::Default => Some(ServiceTier::Default),
                        CodexServiceTier::Flex => Some(ServiceTier::Flex),
                        CodexServiceTier::Scale => Some(ServiceTier::Scale),
                        CodexServiceTier::Priority => Some(ServiceTier::Priority),
                        CodexServiceTier::Fast => Some(ServiceTier::Fast),
                        CodexServiceTier::Null => None,
                    }
                });
            // C9: strip ONLY the openai-codex: prefix; do not strip arbitrary prefixes.
            let model_id_owned = match request.model.strip_prefix("openai-codex:") {
                Some(rest) => rest.to_owned(),
                None => request.model.clone(),
            };
            let model_id = model_id_owned.as_str();
            let model_known = self.models.iter().any(|model| model.id == model_id);
            let model_base_url = self
                .models
                .iter()
                .find(|model| model.id == model_id)
                .and_then(|model| model.base_url.clone());
            let extra_headers = validate_headers(&request.extra_headers);
            let requested_model = request
                .model
                .split_once(':')
                .map(|(_, id)| id)
                .unwrap_or(&request.model)
                .to_owned();
            let (body, wire_facts) = match self.compile_request_body(&request) {
                Ok(compiled) => compiled,
                Err(error) => return Box::pin(stream::once(async move { Err(error) })),
            };

            // Payload callback (task 20.51): once per variant, frozen across
            // ordinary retries, on the codex mapper body.
            let callbacks = request.callbacks.clone();
            let telemetry = request.telemetry_context.clone();
            let timeout = request.timeout;
            let websocket_connect_timeout = self
                .native_options
                .as_ref()
                .and_then(|options| options.websocket_connect_timeout);
            let session_id: Option<String> = if request.cache_retention != CacheRetention::Disabled
            {
                Some(
                    request
                        .session_id
                        .clone()
                        .filter(|id| !id.is_empty())
                        .unwrap_or_else(|| uuid::Uuid::now_v7().to_string()),
                )
            } else {
                None
            };
            let request_id: Option<String> = if request.cache_retention != CacheRetention::Disabled
            {
                Some(uuid::Uuid::now_v7().to_string())
            } else {
                None
            };
            let cancel = lease
                .as_ref()
                .map_or_else(|| request.cancel.child_token(), |lease| lease.token());
            let client = self.client.client().clone();
            let http_client = self.client.clone();
            let ws_state = lease
                .as_ref()
                .map_or_else(|| self.ws_state.clone(), |lease| lease.state.clone());
            let cont_route = crate::replay::ReplayRoute {
                wire: crate::model_info::WireApi::OpenAiCodexResponses,
                provider: PROVIDER_ID.to_owned(),
                model: model_id_owned.clone(),
                supports_images: crate::replay::supports_images_in_catalog(&self.models, model_id),
            };
            let snapshot = crate::stream::ProducerSnapshot::with_projection(projection);
            let producer_snapshot = snapshot.clone();
            let (tx, rx) = tokio::sync::mpsc::channel(64);
            let stream_cancel = cancel.clone();

            // C9: UnknownModel pre-I/O guard — zero auth and zero HTTP for unknown or
            // cross-provider models (the prepared path already holds resolved auth,
            // so there is no resolver call to suppress here).
            if !model_known {
                let provider_id: String = PROVIDER_ID.into();
                tokio::spawn(async move {
                    let _ = tx
                        .send(Err(ProviderError::UnknownModel {
                            provider_id,
                            model_id: model_id_owned,
                        }))
                        .await;
                });
                return Box::pin(CancelAwareReceiverStream::with_snapshot(
                    rx,
                    stream_cancel,
                    snapshot,
                ));
            }

            let tx_closed = tx.clone();
            let producer_cancel = cancel.clone();
            // One-way link: caller-parent cancellation fires the generation
            // token (cleanup cutoffs fire it directly). The link exits when
            // either side fires, including the guard's drop-time cancel.
            let lease = lease;
            let producer = async move {
                let _activity = activity;
                let owner_cancel = producer_cancel.clone();
                let release_snapshot = producer_snapshot.clone();
                let work = async {
                    let extra_headers = match extra_headers {
                        Ok(headers) => headers,
                        Err(error) => {
                            let _ = send_or_cancel(&producer_cancel, &tx, Err(error)).await;
                            return;
                        }
                    };
                    let transformed = tokio::select! {
                        biased;
                        _ = cancel.cancelled() => Err(ProviderError::Cancelled),
                        result = callbacks.apply_header_transform_async(&extra_headers) => result,
                    };
                    let extra_headers = match transformed {
                        Ok(headers) => headers,
                        Err(error) => {
                            let _ = send_or_cancel(&producer_cancel, &tx, Err(error)).await;
                            return;
                        }
                    };
                    let transformed = tokio::select! {
                        biased;
                        _ = cancel.cancelled() => Err(ProviderError::Cancelled),
                        result = callbacks.apply_payload_async(crate::callbacks::ORDINARY_PAYLOAD_VARIANT, body) => result,
                    };
                    let body = match transformed.and_then(|body| {
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
                    let base_url = model_base_url.unwrap_or(default_base_url);
                    if transport == TransportPreference::Sse {
                        stream_sse(
                            client,
                            resolved,
                            wire_facts,
                            base_url,
                            requested_model,
                            requested_service_tier,
                            extra_headers,
                            body,
                            cancel,
                            timeout,
                            session_id,
                            request_id,
                            callbacks,
                            producer_snapshot,
                            tx.clone(),
                            producer_cancel,
                        )
                        .await;
                    } else {
                        let outcome = ws_transport::drive(
                            &ws_state,
                            &http_client,
                            transport,
                            session_key,
                            &base_url,
                            &resolved,
                            body.clone(),
                            requested_service_tier,
                            extra_headers.clone(),
                            wire_facts.clone(),
                            cont_route,
                            timeout,
                            websocket_connect_timeout,
                            &lease.as_ref().expect("ws-family call registered").guard,
                            callbacks.clone(),
                            producer_snapshot.clone(),
                            tx.clone(),
                        )
                        .await;
                        match outcome {
                            ws_transport::WsLoopOutcome::Done => {}
                            ws_transport::WsLoopOutcome::Fallback(reason) => {
                                let guard = lease.as_ref().expect("ws-family call registered");
                                if guard.token().is_cancelled() {
                                    // Cutoff forbids a late SSE leg.
                                    let _ = send_or_cancel(
                                        &guard.token(),
                                        &tx,
                                        Err(ProviderError::Cancelled),
                                    )
                                    .await;
                                    return;
                                }
                                guard.guard.set_phase(
                                    crate::codex_resources::GenerationPhase::ActiveFallback,
                                );
                                let cancel = guard.token();
                                let mut wire_facts = wire_facts.unwrap_or(
                                    crate::openai_responses_shared::ResponsesWireTools {
                                        tools: Vec::new(),
                                        grammar_input_properties: Default::default(),
                                        strict_tools: Vec::new(),
                                        diagnostics: Vec::new(),
                                    },
                                );
                                wire_facts
                                    .diagnostics
                                    .push(crate::stream::AssistantDiagnostic {
                                        kind: "codex_websocket_fallback".into(),
                                        reason: reason.into(),
                                        detail: None,
                                    });
                                stream_sse(
                                    client,
                                    resolved,
                                    Some(wire_facts),
                                    base_url,
                                    requested_model,
                                    requested_service_tier,
                                    extra_headers,
                                    body,
                                    cancel,
                                    timeout,
                                    session_id,
                                    request_id,
                                    callbacks,
                                    producer_snapshot,
                                    tx.clone(),
                                    producer_cancel,
                                )
                                .await;
                            }
                            ws_transport::WsLoopOutcome::Terminal(error) => {
                                let _ = send_or_cancel(&producer_cancel, &tx, Err(error)).await;
                            }
                        }
                    }
                };

                tokio::pin!(work);
                tokio::select! {
                    biased;
                    _ = &mut work => {},
                    _ = tx_closed.closed() => {
                        // A delivered terminal stays final and still owns bounded
                        // pool publication/Close. Active receiver drop cancels
                        // inward, then waits for that same local release path.
                        if !release_snapshot.terminal_delivered() { owner_cancel.cancel(); }
                        work.await;
                    }
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

/// Drive the SSE transport and publish any terminal error (shared by the
/// explicit SSE route and the bounded WS→SSE fallback).
#[allow(clippy::too_many_arguments)]
async fn stream_sse(
    client: reqwest::Client,
    resolved: crate::auth::ResolvedAuth,
    wire_facts: Option<crate::openai_responses_shared::ResponsesWireTools>,
    base_url: String,
    requested_model: String,
    requested_service_tier: Option<crate::stream::ServiceTier>,
    extra_headers: Vec<(String, String)>,
    body: serde_json::Value,
    cancel: CancellationToken,
    timeout: Option<std::time::Duration>,
    session_id: Option<String>,
    request_id: Option<String>,
    callbacks: crate::callbacks::RequestCallbacks,
    snapshot: crate::stream::ProducerSnapshot,
    tx: tokio::sync::mpsc::Sender<Result<AssistantStreamEvent, ProviderError>>,
    producer_cancel: CancellationToken,
) {
    let failure_snapshot = snapshot.clone();
    if let Err(error) = OpenAiCodexResponsesProvider::stream_http(
        client,
        resolved,
        wire_facts,
        base_url,
        requested_model,
        requested_service_tier,
        extra_headers,
        body,
        cancel,
        timeout,
        session_id,
        request_id,
        callbacks,
        snapshot,
        tx.clone(),
    )
    .await
    {
        let _ = send_or_cancel(&producer_cancel, &tx, Err(failure_snapshot.failure(error))).await;
    }
}

/// Map a non-101 WebSocket upgrade status through the same typed HTTP
/// classification as the SSE transport.
fn map_http_upgrade_status(status: u16) -> ProviderError {
    match status {
        401 | 403 => ProviderError::CredentialRevoked {
            provider_id: PROVIDER_ID.into(),
        },
        429 => ProviderError::RateLimited {
            retry_after_ms: None,
        },
        408 | 504 => ProviderError::Timeout,
        code => ProviderError::ProviderSide(ProviderErrorSummary::from_http_response(code)),
    }
}

fn validate_headers(headers: &[(String, String)]) -> Result<Vec<(String, String)>, ProviderError> {
    crate::provider::validate_extra_headers(headers)?;
    if let Some((name, _)) = headers
        .iter()
        .find(|(name, _)| MANAGED_HEADERS.contains(&name.to_ascii_lowercase().as_str()))
    {
        return Err(ProviderError::RequestFailed(
            ProviderErrorSummary::sanitized(format!(
                "request header '{name}' is reserved for OpenAI Codex"
            )),
        ));
    }
    Ok(headers.to_vec())
}

fn map_http_status(
    status: reqwest::StatusCode,
    headers: &reqwest::header::HeaderMap,
) -> ProviderError {
    match status.as_u16() {
        401 | 403 => ProviderError::CredentialRevoked {
            provider_id: PROVIDER_ID.into(),
        },
        429 => ProviderError::RateLimited {
            retry_after_ms: crate::retry::parse_retry_after(headers),
        },
        408 | 504 => ProviderError::Timeout,
        code => ProviderError::ProviderSide(ProviderErrorSummary::from_http_response(code)),
    }
}
