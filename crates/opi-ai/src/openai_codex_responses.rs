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
    ParsedEvent, ResponsesEvent, ResponsesMapper, convert_messages, drain_sse_frames,
};
use crate::provider::{
    CacheRetention, EventStream, ModelInfo, Provider, ProviderError, ProviderErrorSummary, Request,
};
use crate::stream::{AssistantStreamEvent, CancelAwareReceiverStream, send_or_cancel};

const PROVIDER_ID: &str = "openai-codex";
const DEFAULT_BASE_URL: &str = "https://chatgpt.com/backend-api";
const RESPONSES_PATH: &str = "/codex/responses";
const MALFORMED_STREAM_ERROR: &str = "OpenAI Codex returned malformed streaming data";
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
pub struct OpenAiCodexResponsesProvider {
    base_url: String,
    models: Vec<ModelInfo>,
    client: Arc<HttpClient>,
}

impl OpenAiCodexResponsesProvider {
    pub fn new(base_url: Option<String>, models: Vec<ModelInfo>, client: Arc<HttpClient>) -> Self {
        Self {
            base_url: base_url.unwrap_or_else(|| DEFAULT_BASE_URL.into()),
            models,
            client,
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
        let replay_route = crate::replay::ReplayRoute {
            wire: crate::model_info::WireApi::OpenAiCodexResponses,
            provider: PROVIDER_ID.to_owned(),
            model: model_id.to_owned(),
            supports_images: crate::replay::supports_images_in_catalog(&self.models, model_id),
        };
        let replay = crate::replay::project_replay(
            &crate::transcript::without_initial_system_message(transcript.messages()),
            &replay_route,
            crate::replay::ReplayProjection::NativeOrCollapse,
        )?;
        // Task 20.11: compile the tools first so grammar replay items know
        // their custom shape.
        let mut wire_facts: Option<crate::openai_responses_shared::ResponsesWireTools> = None;
        if !current_tools.is_empty() {
            let wire = crate::openai_responses_shared::convert_responses_tools(
                &current_tools,
                crate::openai_responses_shared::ResponsesToolOptions {
                    default_strict: None,
                    supports_strict_mode: true,
                    supports_grammar_tools: false,
                },
            )?;
            wire_facts = Some(wire);
        }
        let grammar_map = wire_facts
            .as_ref()
            .map(|wire| wire.grammar_input_properties.clone())
            .unwrap_or_default();
        let input = convert_messages(&replay.messages, &replay_route, &grammar_map);
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
        if let Some(wire) = &wire_facts {
            body["tools"] = serde_json::Value::Array(wire.tools.clone());
        }
        if request.thinking.enabled {
            let effort = self
                .models
                .iter()
                .find(|model| model.id == model_id)
                .and_then(|model| {
                    model
                        .thinking_level_map
                        .resolve(request.thinking.level)
                        .ok()
                })
                .flatten();
            if let Some(effort) = effort {
                body["reasoning"] = serde_json::json!({
                    "effort": effort,
                    "summary": "auto",
                });
            }
        }
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
            AuthScheme::ApiKey | AuthScheme::AwsSigV4(_) => {
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
        callbacks.notify_response(response.status().as_u16(), &observed_headers)?;

        let status = response.status();
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
        );
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
                        && let Err(error) = callbacks.notify_raw(&raw)
                    {
                        return Err(mapper.failure(error));
                    }
                    match ResponsesEvent::try_from_frame(&frame) {
                        ParsedEvent::Valid(event) => {
                            let event = match event {
                                ResponsesEvent::Error { .. } => ResponsesEvent::Error {
                                    message: UPSTREAM_STREAM_ERROR.to_owned(),
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

impl Provider for OpenAiCodexResponsesProvider {
    fn id(&self) -> &str {
        PROVIDER_ID
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
        let extra_headers = validate_headers(&request.extra_headers)
            // Header transform (task 20.51) on the per-call set, revalidated
            // against protected/managed names before any dispatch.
            .and_then(|merged| request.callbacks.apply_header_transform(&merged));
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
        let body = match request
            .callbacks
            .apply_payload(crate::callbacks::ORDINARY_PAYLOAD_VARIANT, body)
        {
            Ok(body) => body,
            Err(error) => return Box::pin(stream::once(async move { Err(error) })),
        };
        let callbacks = request.callbacks.clone();
        let timeout = request.timeout;
        let session_id: Option<String> = if request.cache_retention != CacheRetention::Disabled {
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
        let request_id: Option<String> = if request.cache_retention != CacheRetention::Disabled {
            Some(uuid::Uuid::now_v7().to_string())
        } else {
            None
        };
        let cancel = request.cancel.clone();
        let client = self.client.client().clone();
        let snapshot = crate::stream::ProducerSnapshot::default();
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
        tokio::spawn(async move {
            let work = async {
                let extra_headers = match extra_headers {
                    Ok(headers) => headers,
                    Err(error) => {
                        let _ = send_or_cancel(&producer_cancel, &tx, Err(error)).await;
                        return;
                    }
                };
                let base_url = model_base_url.unwrap_or(default_base_url);
                if let Err(error) = Self::stream_http(
                    client,
                    resolved,
                    wire_facts,
                    base_url,
                    requested_model,
                    extra_headers,
                    body,
                    cancel,
                    timeout,
                    session_id,
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
