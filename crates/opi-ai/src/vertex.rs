//! Google Vertex AI provider (task 3.3).
//!
//! Routes through the Gemini `streamGenerateContent` adapter with
//! Vertex-specific URL and auth:
//! - URL: `https://{location}-aiplatform.googleapis.com/v1/projects/{project}/locations/{location}/publishers/google/models/{model}:streamGenerateContent?alt=sse`
//! - Auth: `Authorization: Bearer {access_token}` (OAuth2)
//!
//! Reuses Gemini SSE parsing and event mapping from the `gemini` module.

use std::fmt;
use std::sync::Arc;

use futures_util::{StreamExt, stream};
use secrecy::ExposeSecret;
use tokio_util::sync::CancellationToken;

use crate::gemini::{GeminiMapper, GeminiProvider, ParsedEvent, drain_sse_data, parse_sse_data};
use crate::http::HttpClient;
use crate::model_info::WireApi;
use crate::provider::{EventStream, ModelInfo, Provider, ProviderError, Request};
use crate::registry::ModelCapabilities;
use crate::stream::AssistantStreamEvent;

/// Google Vertex AI provider.
///
/// Wraps a [`GeminiProvider`] for request body serialization and SSE parsing,
/// but overrides the HTTP transport layer (URL and auth header).
pub struct VertexProvider {
    project: String,
    location: String,
    base_url: String,
    models: Vec<ModelInfo>,
    inner: GeminiProvider,
    client: Arc<HttpClient>,
}

impl fmt::Debug for VertexProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VertexProvider")
            .field("project", &self.project)
            .field("location", &self.location)
            .field("models", &self.models.len())
            .finish()
    }
}

impl VertexProvider {
    /// Create a new Vertex AI provider.
    pub fn new(project: String, location: String, base_url: Option<String>) -> Self {
        let base_url =
            base_url.unwrap_or_else(|| format!("https://{location}-aiplatform.googleapis.com"));
        let inner = GeminiProvider::new(None);
        let models = model_catalog();
        Self {
            project,
            location,
            base_url,
            models,
            inner,
            client: Arc::new(HttpClient::new()),
        }
    }

    /// Create from config with explicit model list.
    pub fn from_config(
        project: String,
        location: String,
        models: Vec<String>,
        base_url: Option<String>,
    ) -> Self {
        let base_url =
            base_url.unwrap_or_else(|| format!("https://{location}-aiplatform.googleapis.com"));
        let inner = GeminiProvider::new(None);
        let model_list = models
            .iter()
            .map(|id| {
                ModelInfo::new(
                    id,
                    id,
                    WireApi::GoogleVertex,
                    ModelCapabilities::new(1_000_000, 65536)
                        .with_images(true)
                        .with_streaming(true),
                )
            })
            .collect();
        Self {
            project,
            location,
            base_url,
            models: model_list,
            inner,
            client: Arc::new(HttpClient::new()),
        }
    }

    /// Replace the HTTP client (for shared connection pooling).
    pub fn with_client(self, client: Arc<HttpClient>) -> Self {
        Self { client, ..self }
    }

    /// Build the Vertex AI streaming URL for a given model.
    pub fn build_vertex_url(&self, model_id: &str) -> String {
        format!(
            "{base}/v1/projects/{project}/locations/{location}/publishers/google/models/{model}:streamGenerateContent?alt=sse",
            base = self.base_url,
            project = self.project,
            location = self.location,
            model = model_id,
        )
    }

    /// Build the request body (delegates to inner Gemini provider).
    pub fn build_request_body(&self, request: &Request) -> serde_json::Value {
        self.inner.build_request_body(request)
    }

    /// Stream events from a raw SSE response body (for testing).
    pub fn stream_from_sse(&self, sse_body: &str, cancel: CancellationToken) -> EventStream {
        let mut mapper = GeminiMapper::new("vertex");
        let mut stream_events: Vec<Result<AssistantStreamEvent, ProviderError>> = Vec::new();

        for data in parse_sse_data(sse_body) {
            for parsed in ParsedEvent::from_data(&data) {
                match parsed {
                    ParsedEvent::Valid(event) => {
                        stream_events.extend(mapper.process(event).into_iter().map(Ok));
                    }
                    ParsedEvent::Malformed => {
                        stream_events.push(Err(ProviderError::StreamError(
                            "Vertex returned a malformed streaming frame".to_owned(),
                        )));
                    }
                }
            }
        }

        let _cancel = cancel;
        Box::pin(stream::iter(stream_events))
    }
}

// Minimal ReceiverStream adapter for the tokio mpsc channel.
struct ReceiverStream {
    rx: tokio::sync::mpsc::Receiver<Result<AssistantStreamEvent, ProviderError>>,
}

impl futures_core::Stream for ReceiverStream {
    type Item = Result<AssistantStreamEvent, ProviderError>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

impl Provider for VertexProvider {
    fn id(&self) -> &str {
        "vertex"
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    fn stream_prepared(&self, request: Request, auth: crate::auth::ResolvedAuth) -> EventStream {
        let access_token = auth.secret.expose_secret().to_string();
        let model_id = request
            .model
            .split_once(':')
            .map(|(_, id)| id)
            .unwrap_or(&request.model)
            .to_string();

        let url = self.build_vertex_url(&model_id);
        let body = self.inner.build_request_body(&request);
        let cancel = request.cancel;
        let http_client = self.client.client().clone();

        let (tx, rx) = tokio::sync::mpsc::channel(64);

        tokio::spawn(async move {
            if let Err(e) =
                stream_vertex_http(http_client, access_token, &url, &body, cancel, &tx).await
            {
                let _ = tx.send(Err(e)).await;
            }
        });

        Box::pin(ReceiverStream { rx })
    }
}

/// HTTP streaming with Vertex-specific URL and `Authorization: Bearer` header.
async fn stream_vertex_http(
    http_client: reqwest::Client,
    access_token: String,
    url: &str,
    body: &serde_json::Value,
    cancel: CancellationToken,
    tx: &tokio::sync::mpsc::Sender<Result<AssistantStreamEvent, ProviderError>>,
) -> Result<(), ProviderError> {
    let req = http_client
        .post(url)
        .header("authorization", format!("Bearer {access_token}"))
        .header("content-type", "application/json");

    let response = req
        .body(serde_json::to_string(body).unwrap_or_default())
        .send()
        .await
        .map_err(|e| ProviderError::Network(e.to_string()))?;

    let status = response.status();
    if !status.is_success() {
        let headers = response.headers().clone();
        let error_body = response.text().await.unwrap_or_default();
        return Err(map_vertex_status(status, &error_body, &headers));
    }

    let mut byte_stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut mapper = GeminiMapper::new("vertex");
    let mut saw_done = false;

    loop {
        let chunk = tokio::select! {
            _ = cancel.cancelled() => {
                return Ok(());
            }
            chunk = byte_stream.next() => {
                match chunk {
                    Some(c) => c,
                    None => break,
                }
            }
        };

        let chunk = chunk.map_err(|e| ProviderError::StreamError(e.to_string()))?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        for parsed in drain_sse_data(&mut buffer) {
            match parsed {
                ParsedEvent::Valid(event) => {
                    for stream_event in mapper.process(event) {
                        let is_terminal = matches!(
                            stream_event,
                            AssistantStreamEvent::Done { .. } | AssistantStreamEvent::Error { .. }
                        );
                        if tx.send(Ok(stream_event)).await.is_err() {
                            return Ok(());
                        }
                        if is_terminal {
                            saw_done = true;
                        }
                    }
                }
                ParsedEvent::Malformed => {
                    let err = ProviderError::StreamError(
                        "Vertex returned a malformed streaming frame".to_owned(),
                    );
                    if tx.send(Err(err)).await.is_err() {
                        return Ok(());
                    }
                }
            }
        }
    }

    if !saw_done {
        let err = ProviderError::StreamError("stream ended without a terminal event".into());
        let _ = tx.send(Err(err)).await;
    }

    Ok(())
}

fn map_vertex_status(
    status: reqwest::StatusCode,
    body: &str,
    headers: &reqwest::header::HeaderMap,
) -> ProviderError {
    match status.as_u16() {
        401 | 403 => ProviderError::AuthFailed(format!(
            "authentication failed: {}",
            crate::http::safe_excerpt(body)
        )),
        429 => ProviderError::RateLimited {
            retry_after_ms: crate::retry::parse_retry_after(headers),
        },
        408 | 504 => ProviderError::Timeout,
        _ => {
            // Vertex/Gemini may return auth errors with HTTP 400 but code 401/403 in body
            if let Ok(err_body) = serde_json::from_str::<serde_json::Value>(body)
                && let Some(code) = err_body
                    .get("error")
                    .and_then(|e| e.get("code"))
                    .and_then(|c| c.as_i64())
                && (code == 401 || code == 403)
            {
                return ProviderError::AuthFailed(format!(
                    "authentication failed: {}",
                    crate::http::safe_excerpt(body)
                ));
            }
            ProviderError::ProviderSide(format!(
                "HTTP {}: {}",
                status.as_u16(),
                crate::http::safe_excerpt(body)
            ))
        }
    }
}

/// Built-in Vertex model metadata without credentials or HTTP construction.
pub fn model_catalog() -> Vec<ModelInfo> {
    vec![
        ModelInfo::new(
            "gemini-2.5-flash",
            "Gemini 2.5 Flash (Vertex)",
            WireApi::GoogleVertex,
            ModelCapabilities::new(1_000_000, 65536)
                .with_images(true)
                .with_streaming(true),
        ),
        ModelInfo::new(
            "gemini-2.5-pro",
            "Gemini 2.5 Pro (Vertex)",
            WireApi::GoogleVertex,
            ModelCapabilities::new(1_000_000, 65536)
                .with_images(true)
                .with_streaming(true),
        ),
    ]
}
