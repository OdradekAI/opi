//! Azure OpenAI provider profile (task 3.2).
//!
//! Routes through the OpenAI-compatible chat adapter with Azure-specific:
//! - URL: `{endpoint}/openai/deployments/{deployment}/chat/completions?api-version={version}`
//! - Auth: `api-key` header (not `Authorization: Bearer`)
//!
//! Reuses SSE parsing and event mapping from `openai_chat`.

use std::fmt;
use std::sync::Arc;

use futures_util::{StreamExt, stream};
use tokio_util::sync::CancellationToken;

use crate::http::HttpClient;
use crate::openai_chat::{
    CompatConfig, OpenAiChatMapper, OpenAiChatProvider, ParsedEvent, parse_sse_events,
};
use crate::provider::{EventStream, ModelInfo, Provider, ProviderError, Request};
use crate::stream::AssistantStreamEvent;

/// Default Azure OpenAI API version.
const DEFAULT_API_VERSION: &str = "2024-06-01";

/// Azure OpenAI provider.
///
/// Wraps an [`OpenAiChatProvider`] for request body serialization and SSE
/// parsing, but overrides the HTTP transport layer (URL and auth header).
pub struct AzureOpenAIProvider {
    api_key: String,
    endpoint: String,
    api_version: String,
    models: Vec<ModelInfo>,
    inner: OpenAiChatProvider,
    client: Arc<HttpClient>,
}

impl fmt::Debug for AzureOpenAIProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AzureOpenAIProvider")
            .field("endpoint", &self.endpoint)
            .field("api_version", &self.api_version)
            .field("api_key", &"***")
            .field("models", &self.models.len())
            .finish()
    }
}

impl AzureOpenAIProvider {
    /// Create a new Azure OpenAI provider.
    ///
    /// `deployment` is a default deployment name (used for model list display).
    /// The actual deployment is resolved from the model spec `azure:<deployment>`.
    pub fn new(
        api_key: String,
        endpoint: Option<String>,
        deployment: String,
        api_version: Option<String>,
    ) -> Result<Self, ProviderError> {
        let endpoint = endpoint.ok_or_else(|| {
            ProviderError::Config(
                "Azure OpenAI endpoint is required. Set it via config [providers.azure] endpoint or AZURE_OPENAI_ENDPOINT env var.".into()
            )
        })?;
        let api_version = api_version.unwrap_or_else(|| DEFAULT_API_VERSION.into());
        let inner = OpenAiChatProvider::new_for_profile(
            api_key.clone(),
            endpoint.clone(),
            "azure".into(),
            CompatConfig::default(),
            vec![],
            vec![],
        );
        let _ = deployment; // stored in models if needed
        Ok(Self {
            api_key,
            endpoint,
            api_version,
            models: vec![],
            inner,
            client: Arc::new(HttpClient::new()),
        })
    }

    /// Create from config with explicit deployment names for the model list.
    pub fn from_config(
        api_key: String,
        endpoint: Option<String>,
        deployments: Vec<String>,
        api_version: Option<String>,
    ) -> Result<Self, ProviderError> {
        let endpoint = endpoint.ok_or_else(|| {
            ProviderError::Config(
                "Azure OpenAI endpoint is required. Set it via config [providers.azure] endpoint or AZURE_OPENAI_ENDPOINT env var.".into()
            )
        })?;
        let api_version = api_version.unwrap_or_else(|| DEFAULT_API_VERSION.into());
        let models = deployments
            .iter()
            .map(|d| ModelInfo {
                id: d.clone(),
                display_name: d.clone(),
                context_window: 128000,
                max_output_tokens: 16384,
                supports_images: true,
                supports_streaming: true,
                supports_thinking: false,
            })
            .collect();
        let inner = OpenAiChatProvider::new_for_profile(
            api_key.clone(),
            endpoint.clone(),
            "azure".into(),
            CompatConfig::default(),
            vec![],
            vec![],
        );
        Ok(Self {
            api_key,
            endpoint,
            api_version,
            models,
            inner,
            client: Arc::new(HttpClient::new()),
        })
    }

    /// Replace the HTTP client (for shared connection pooling).
    pub fn with_client(self, client: Arc<HttpClient>) -> Self {
        Self { client, ..self }
    }

    /// Build the Azure deployment URL for a given deployment name.
    pub fn build_azure_url(&self, deployment: &str) -> String {
        format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            self.endpoint, deployment, self.api_version
        )
    }

    /// Build the request body (delegates to inner OpenAI chat provider).
    pub fn build_request_body(&self, request: &Request) -> serde_json::Value {
        self.inner.build_request_body(request)
    }

    /// Stream events from a raw SSE response body (for testing).
    pub fn stream_from_sse(&self, sse_body: &str, cancel: CancellationToken) -> EventStream {
        let mut mapper = OpenAiChatMapper::new(crate::ApiKind::OpenAi, "azure");
        let mut stream_events: Vec<Result<AssistantStreamEvent, ProviderError>> = Vec::new();
        for parsed in parse_sse_events(sse_body) {
            match parsed {
                ParsedEvent::Valid(events) => {
                    for event in events {
                        stream_events.extend(mapper.process(event).into_iter().map(Ok));
                    }
                }
                ParsedEvent::Malformed { data, error } => {
                    stream_events.push(Err(ProviderError::StreamError(format!(
                        "malformed SSE data: {error} (data: {data:.80})"
                    ))));
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

impl Provider for AzureOpenAIProvider {
    fn id(&self) -> &str {
        "azure"
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    fn stream(&self, request: Request) -> EventStream {
        let model_id = request
            .model
            .split_once(':')
            .map(|(_, id)| id)
            .unwrap_or(&request.model)
            .to_string();

        let url = self.build_azure_url(&model_id);
        let body = self.inner.build_request_body(&request);
        let cancel = request.cancel;
        let http_client = self.client.client().clone();
        let api_key = self.api_key.clone();

        let (tx, rx) = tokio::sync::mpsc::channel(64);

        tokio::spawn(async move {
            if let Err(e) = stream_azure_http(http_client, api_key, &url, &body, cancel, &tx).await
            {
                let _ = tx.send(Err(e)).await;
            }
        });

        Box::pin(ReceiverStream { rx })
    }
}

/// HTTP streaming with Azure-specific URL and `api-key` header.
async fn stream_azure_http(
    http_client: reqwest::Client,
    api_key: String,
    url: &str,
    body: &serde_json::Value,
    cancel: CancellationToken,
    tx: &tokio::sync::mpsc::Sender<Result<AssistantStreamEvent, ProviderError>>,
) -> Result<(), ProviderError> {
    let req = http_client
        .post(url)
        .header("api-key", &api_key)
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
        return Err(map_azure_status(status, &error_body, &headers));
    }

    let mut byte_stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut mapper = OpenAiChatMapper::new(crate::ApiKind::OpenAi, "azure");
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

        for parsed in drain_sse_events(&mut buffer) {
            match parsed {
                ParsedEvent::Valid(events) => {
                    for event in events {
                        for stream_event in mapper.process(event) {
                            let is_terminal = matches!(
                                stream_event,
                                AssistantStreamEvent::Done { .. }
                                    | AssistantStreamEvent::Error { .. }
                            );
                            if tx.send(Ok(stream_event)).await.is_err() {
                                return Ok(());
                            }
                            if is_terminal {
                                saw_done = true;
                            }
                        }
                    }
                }
                ParsedEvent::Malformed { data, error } => {
                    let err = ProviderError::StreamError(format!(
                        "malformed SSE data: {error} (data: {data:.80})"
                    ));
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

fn drain_sse_events(buffer: &mut String) -> Vec<ParsedEvent> {
    if buffer.contains('\r') {
        *buffer = buffer.replace("\r\n", "\n").replace('\r', "\n");
    }

    let mut events = Vec::new();
    while let Some(idx) = buffer.find("\n\n") {
        let end = idx + 2;
        let chunk: String = buffer.drain(..end).collect();
        events.extend(parse_sse_events(&chunk));
    }
    events
}

fn map_azure_status(
    status: reqwest::StatusCode,
    body: &str,
    headers: &reqwest::header::HeaderMap,
) -> ProviderError {
    match status.as_u16() {
        401 => ProviderError::AuthFailed(format!(
            "authentication failed: {}",
            crate::http::safe_excerpt(body)
        )),
        403 => ProviderError::AuthFailed(format!(
            "access denied: {}",
            crate::http::safe_excerpt(body)
        )),
        404 => ProviderError::Config(format!(
            "deployment not found: {}",
            crate::http::safe_excerpt(body)
        )),
        429 => ProviderError::RateLimited {
            retry_after_ms: crate::retry::parse_retry_after(headers),
        },
        408 | 504 => ProviderError::Timeout,
        code => {
            ProviderError::ProviderSide(format!("HTTP {code}: {}", crate::http::safe_excerpt(body)))
        }
    }
}
