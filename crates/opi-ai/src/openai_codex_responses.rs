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

use crate::auth::{AuthResolver, ResolvedAuth};
use crate::http::HttpClient;
use crate::openai_responses_shared::{
    ParsedEvent, ResponsesEvent, ResponsesMapper, convert_messages, drain_sse_frames,
};
use crate::provider::{CacheRetention, EventStream, ModelInfo, Provider, ProviderError, Request};
use crate::stream::AssistantStreamEvent;

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
/// Authentication is resolved lazily for every stream and must include the
/// non-secret ChatGPT account id. Provider-managed authorization, account,
/// originator, beta, content, accept, and session headers are reserved from
/// `Request::extra_headers`.
pub struct OpenAiCodexResponsesProvider {
    auth: Arc<dyn AuthResolver>,
    base_url: String,
    models: Vec<ModelInfo>,
    client: Arc<HttpClient>,
}

impl OpenAiCodexResponsesProvider {
    pub fn new(
        auth: Arc<dyn AuthResolver>,
        base_url: Option<String>,
        models: Vec<ModelInfo>,
        client: Arc<HttpClient>,
    ) -> Self {
        Self {
            auth,
            base_url: base_url.unwrap_or_else(|| DEFAULT_BASE_URL.into()),
            models,
            client,
        }
    }

    /// Build the subscription-specific Codex request body.
    pub fn build_request_body(&self, request: &Request) -> serde_json::Value {
        let model_id = request
            .model
            .strip_prefix("openai-codex:")
            .unwrap_or(&request.model);
        let input = convert_messages(request);
        let session_id = if request.cache_retention != CacheRetention::Disabled {
            request.session_id.as_deref().filter(|id| !id.is_empty())
        } else {
            None
        };
        let mut body = serde_json::json!({
            "model": model_id,
            "store": false,
            "stream": true,
            "instructions": request.system.as_deref().unwrap_or("You are a helpful assistant."),
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
        if !request.tools.is_empty() {
            body["tools"] = serde_json::Value::Array(
                request
                    .tools
                    .iter()
                    .map(|tool| {
                        serde_json::json!({
                            "type": "function",
                            "name": tool.name,
                            "description": tool.description,
                            "parameters": tool.input_schema,
                            "strict": null,
                        })
                    })
                    .collect(),
            );
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
        body
    }

    #[allow(clippy::too_many_arguments)]
    async fn stream_http(
        client: reqwest::Client,
        resolved: ResolvedAuth,
        base_url: String,
        extra_headers: Vec<(String, String)>,
        body: serde_json::Value,
        cancel: CancellationToken,
        timeout: Option<std::time::Duration>,
        session_id: Option<String>,
        request_id: Option<String>,
        tx: tokio::sync::mpsc::Sender<Result<AssistantStreamEvent, ProviderError>>,
    ) -> Result<(), ProviderError> {
        let account_id = resolved
            .account_id
            .as_deref()
            .filter(|account_id| !account_id.trim().is_empty())
            .ok_or_else(|| ProviderError::AccountIdMissing {
                provider_id: PROVIDER_ID.into(),
            })?;
        let url = crate::endpoint::join_endpoint(&base_url, RESPONSES_PATH);
        let mut request = client
            .post(url)
            .header(
                "authorization",
                format!("Bearer {}", resolved.secret.expose_secret()),
            )
            .header("chatgpt-account-id", account_id)
            .header("originator", "opi")
            .header("OpenAI-Beta", "responses=experimental")
            .header("accept", "text/event-stream")
            .header("content-type", "application/json");
        // C7: omit affinity headers when CacheRetention::Disabled.
        // C11: validate session-id before sending.
        if let Some(value) = session_id {
            let header_value = reqwest::header::HeaderValue::from_str(&value).map_err(|_| {
                ProviderError::RequestFailed("invalid session-id header value".into())
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
                    ProviderError::Network(error.to_string())
                }
            })?;
        let status = response.status();
        if !status.is_success() {
            let headers = response.headers().clone();
            let body = response.text().await.unwrap_or_default();
            return Err(map_http_status(status, &body, &headers));
        }

        let mut bytes = response.bytes_stream();
        let mut buffer = String::new();
        let mut mapper = ResponsesMapper::new(PROVIDER_ID);
        loop {
            let chunk = tokio::select! {
                _ = cancel.cancelled() => return Err(ProviderError::Cancelled),
                chunk = bytes.next() => match chunk {
                    Some(chunk) => chunk,
                    None => break,
                },
            };
            let chunk = chunk.map_err(|error| {
                if error.is_timeout() {
                    ProviderError::Timeout
                } else if error.is_connect() {
                    ProviderError::Network(error.to_string())
                } else {
                    ProviderError::StreamError(error.to_string())
                }
            })?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));
            for frame in drain_sse_frames(&mut buffer) {
                match ResponsesEvent::try_from_frame(&frame) {
                    ParsedEvent::Valid(event) => {
                        let event = match event {
                            ResponsesEvent::Error { .. } => ResponsesEvent::Error {
                                message: UPSTREAM_STREAM_ERROR.to_owned(),
                            },
                            event => event,
                        };
                        for event in mapper.process(event) {
                            if tx.send(Ok(event)).await.is_err() {
                                return Ok(());
                            }
                        }
                    }
                    ParsedEvent::UsageError(error) => {
                        return Err(ProviderError::StreamError(error));
                    }
                    ParsedEvent::Malformed { .. } => {
                        return Err(ProviderError::StreamError(
                            MALFORMED_STREAM_ERROR.to_owned(),
                        ));
                    }
                }
            }
        }
        if !mapper.saw_done {
            let _ = tx
                .send(Err(ProviderError::StreamError(
                    "stream ended without a terminal event".into(),
                )))
                .await;
        }
        Ok(())
    }
}

impl Provider for OpenAiCodexResponsesProvider {
    fn id(&self) -> &str {
        PROVIDER_ID
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    fn stream(&self, request: Request) -> EventStream {
        if let Err(error) = crate::provider::validate_request_capabilities(self, &request) {
            return Box::pin(stream::once(async move { Err(error) }));
        }
        let auth = self.auth.clone();
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
        let extra_headers = validate_headers(&request.extra_headers);
        let body = self.build_request_body(&request);
        let timeout = request.timeout;
        // C7: derive affinity headers as None when caching is disabled; do not
        // synthesize UUID fallbacks for the Disabled case.
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
        let (tx, rx) = tokio::sync::mpsc::channel(64);

        // C9: UnknownModel pre-I/O guard. Surfaces the error through a tiny
        // spawned task (mirrors how other validation errors are surfaced via
        // the channel), and guarantees ZERO auth-resolver calls and ZERO HTTP
        // for unknown or cross-provider models.
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
            return Box::pin(ReceiverStream { rx });
        }

        // C1: observe receiver drop and abort credential resolution / HTTP
        // instead of running detached until the next send attempt.
        let tx_closed = tx.clone();
        tokio::spawn(async move {
            let work = async {
                let extra_headers = match extra_headers {
                    Ok(headers) => headers,
                    Err(error) => {
                        let _ = tx.send(Err(error)).await;
                        return;
                    }
                };
                let resolved = match auth.resolve().await {
                    Ok(resolved) => resolved,
                    Err(error) => {
                        let _ = tx.send(Err(error)).await;
                        return;
                    }
                };
                let base_url = model_base_url.unwrap_or(default_base_url);
                if let Err(error) = Self::stream_http(
                    client,
                    resolved,
                    base_url,
                    extra_headers,
                    body,
                    cancel,
                    timeout,
                    session_id,
                    request_id,
                    tx.clone(),
                )
                .await
                {
                    let _ = tx.send(Err(error)).await;
                }
            };

            tokio::select! {
                biased;
                _ = tx_closed.closed() => (),
                _ = work => {},
            }
        });
        Box::pin(ReceiverStream { rx })
    }
}

fn validate_headers(headers: &[(String, String)]) -> Result<Vec<(String, String)>, ProviderError> {
    crate::provider::validate_extra_headers(headers)?;
    if let Some((name, _)) = headers
        .iter()
        .find(|(name, _)| MANAGED_HEADERS.contains(&name.to_ascii_lowercase().as_str()))
    {
        return Err(ProviderError::RequestFailed(format!(
            "request header '{name}' is reserved for OpenAI Codex"
        )));
    }
    Ok(headers.to_vec())
}

fn map_http_status(
    status: reqwest::StatusCode,
    _body: &str,
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
        code => ProviderError::ProviderSide(format!("HTTP {code}")),
    }
}

struct ReceiverStream {
    rx: tokio::sync::mpsc::Receiver<Result<AssistantStreamEvent, ProviderError>>,
}

impl futures_core::Stream for ReceiverStream {
    type Item = Result<AssistantStreamEvent, ProviderError>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        context: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.rx.poll_recv(context)
    }
}
