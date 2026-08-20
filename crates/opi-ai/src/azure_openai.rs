//! Azure OpenAI provider profile.
//!
//! Routes through the OpenAI-compatible chat adapter with Azure-specific:
//! - URL: `{endpoint}/openai/deployments/{deployment}/chat/completions?api-version={version}`
//! - Auth: `api-key` header (not `Authorization: Bearer`)
//!
//! Reuses SSE parsing and event mapping from `openai_chat`.

use std::fmt;
use std::sync::Arc;

use futures_util::{StreamExt, stream};
use secrecy::ExposeSecret;
use tokio_util::sync::CancellationToken;

use crate::http::HttpClient;
use crate::model_info::WireApi;
use crate::openai_chat::{
    CompatConfig, OpenAiChatMapper, OpenAiChatProvider, ParsedEvent, parse_sse_events,
};
use crate::provider::{
    EventStream, ModelInfo, Provider, ProviderError, ProviderErrorSummary, Request,
};
use crate::provider_headers::ProviderHeaders;
use crate::registry::ModelCapabilities;
use crate::stream::{AssistantStreamEvent, CancelAwareReceiverStream, send_or_cancel};

/// Default Azure OpenAI API version.
const DEFAULT_API_VERSION: &str = "2024-06-01";

/// Azure OpenAI provider.
///
/// Wraps an [`OpenAiChatProvider`] for request body serialization and SSE
/// parsing, but overrides the HTTP transport layer (URL and auth header).
pub struct AzureOpenAIProvider {
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
        endpoint: Option<String>,
        deployment: String,
        api_version: Option<String>,
    ) -> Result<Self, ProviderError> {
        let endpoint = endpoint.ok_or_else(|| {
            ProviderError::Config(
                ProviderErrorSummary::attested_static(
                    "Azure OpenAI endpoint is required. Set it via config [providers.azure] endpoint or AZURE_OPENAI_ENDPOINT env var."
                )
            )
        })?;
        let api_version = api_version.unwrap_or_else(|| DEFAULT_API_VERSION.into());
        if deployment.trim().is_empty() {
            return Err(ProviderError::Config(
                ProviderErrorSummary::attested_static("Azure OpenAI deployment must not be empty"),
            ));
        }
        let models = vec![deployment_model(&deployment)];
        let inner = OpenAiChatProvider::new_for_profile(
            endpoint.clone(),
            "azure".into(),
            CompatConfig::default(),
            vec![],
            vec![],
        );
        Ok(Self {
            endpoint,
            api_version,
            models,
            inner,
            client: Arc::new(HttpClient::new()),
        })
    }

    /// Create from config with explicit deployment names for the model list.
    pub fn from_config(
        endpoint: Option<String>,
        deployments: Vec<String>,
        api_version: Option<String>,
    ) -> Result<Self, ProviderError> {
        let endpoint = endpoint.ok_or_else(|| {
            ProviderError::Config(
                ProviderErrorSummary::attested_static(
                    "Azure OpenAI endpoint is required. Set it via config [providers.azure] endpoint or AZURE_OPENAI_ENDPOINT env var."
                )
            )
        })?;
        let api_version = api_version.unwrap_or_else(|| DEFAULT_API_VERSION.into());
        if deployments.is_empty()
            || deployments
                .iter()
                .any(|deployment| deployment.trim().is_empty())
        {
            return Err(ProviderError::Config(
                ProviderErrorSummary::attested_static(
                    "Azure OpenAI deployments must contain only non-empty deployment names",
                ),
            ));
        }
        let models = deployments
            .iter()
            .map(|deployment| deployment_model(deployment))
            .collect();
        let inner = OpenAiChatProvider::new_for_profile(
            endpoint.clone(),
            "azure".into(),
            CompatConfig::default(),
            vec![],
            vec![],
        );
        Ok(Self {
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

    /// Apply an OpenAI-compatible profile compat config to the inner shared
    /// adapter. Azure is a first-class provider that
    /// routes request-body serialization through the shared OpenAI Chat
    /// adapter, so it honors the same compat flags (developer role, strict
    /// tool schema, max-tokens field, tool-result name) as config-driven
    /// profiles.
    pub fn with_compat(mut self, compat: CompatConfig) -> Self {
        self.inner = OpenAiChatProvider::new_for_profile(
            self.endpoint.clone(),
            "azure".into(),
            compat,
            vec![],
            vec![],
        );
        self
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
                ParsedEvent::UsageError(error) => stream_events.push(Err(error)),
                ParsedEvent::Malformed { .. } => {
                    stream_events.push(Err(ProviderError::StreamError(
                        ProviderErrorSummary::attested_static(
                            "Azure OpenAI returned a malformed streaming frame",
                        ),
                    )));
                }
            }
        }

        let _cancel = cancel;
        Box::pin(stream::iter(stream_events))
    }
}

fn deployment_model(deployment: &str) -> ModelInfo {
    ModelInfo::new(
        deployment,
        deployment,
        WireApi::AzureOpenAiCompletions,
        ModelCapabilities::new(128000, 16384)
            .with_images(true)
            .with_streaming(true),
    )
}

impl Provider for AzureOpenAIProvider {
    fn id(&self) -> &str {
        "azure"
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    fn stream_prepared(&self, request: Request, auth: crate::auth::ResolvedAuth) -> EventStream {
        let api_key = auth.secret.expose_secret().to_string();
        let model_id = request
            .model
            .split_once(':')
            .map(|(_, id)| id)
            .unwrap_or(&request.model)
            .to_string();

        let url = self.build_azure_url(&model_id);
        let body = self.inner.build_request_body(&request);
        let cancel = request.cancel;
        let timeout = request.timeout;
        let extra_headers = request.extra_headers;
        let http_client = self.client.client().clone();

        let (tx, rx) = tokio::sync::mpsc::channel(64);
        let producer_cancel = cancel.clone();

        tokio::spawn(async move {
            let result = stream_azure_http(
                http_client,
                api_key,
                &url,
                &body,
                producer_cancel.clone(),
                timeout,
                extra_headers,
                &tx,
            )
            .await;
            match result {
                Ok(()) | Err(ProviderError::Cancelled) => {}
                Err(error) => {
                    let _ = send_or_cancel(&producer_cancel, &tx, Err(error)).await;
                }
            }
        });

        Box::pin(CancelAwareReceiverStream::new(rx, cancel))
    }
}

/// HTTP streaming with Azure-specific URL and `api-key` header.
#[allow(clippy::too_many_arguments)]
async fn stream_azure_http(
    http_client: reqwest::Client,
    api_key: String,
    url: &str,
    body: &serde_json::Value,
    cancel: CancellationToken,
    timeout: Option<std::time::Duration>,
    extra_headers: Vec<(String, String)>,
    tx: &tokio::sync::mpsc::Sender<Result<AssistantStreamEvent, ProviderError>>,
) -> Result<(), ProviderError> {
    let route_headers = vec![
        ("api-key".to_owned(), api_key),
        ("content-type".to_owned(), "application/json".to_owned()),
    ];
    let headers = ProviderHeaders::default().merge_request(&route_headers, &extra_headers)?;
    let mut req = http_client.post(url);
    for (name, value) in headers {
        req = req.header(name, value);
    }
    if let Some(timeout) = timeout {
        req = req.timeout(timeout);
    }

    let response = tokio::select! {
        biased;
        _ = cancel.cancelled() => return Err(ProviderError::Cancelled),
        response = req
            .body(serde_json::to_string(body).unwrap_or_default())
            .send() => response,
    }
    .map_err(|error| {
        if error.is_timeout() {
            ProviderError::Timeout
        } else {
            ProviderError::Network(ProviderErrorSummary::sanitized(error.to_string()))
        }
    })?;

    let status = response.status();
    if !status.is_success() {
        let headers = response.headers().clone();
        return Err(map_azure_status(status, &headers));
    }

    let mut byte_stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut mapper = OpenAiChatMapper::new(crate::ApiKind::OpenAi, "azure");
    let mut saw_done = false;

    loop {
        let chunk = tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                return Err(ProviderError::Cancelled);
            }
            chunk = byte_stream.next() => {
                match chunk {
                    Some(c) => c,
                    None => break,
                }
            }
        };

        let chunk = chunk.map_err(|error| {
            if error.is_timeout() {
                ProviderError::Timeout
            } else {
                ProviderError::StreamError(ProviderErrorSummary::sanitized(error.to_string()))
            }
        })?;
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
                            if !send_or_cancel(&cancel, tx, Ok(stream_event)).await? {
                                return Ok(());
                            }
                            if is_terminal {
                                saw_done = true;
                            }
                        }
                    }
                }
                ParsedEvent::UsageError(error) => {
                    if !send_or_cancel(&cancel, tx, Err(error)).await? {
                        return Ok(());
                    }
                }
                ParsedEvent::Malformed { .. } => {
                    let err = ProviderError::StreamError(ProviderErrorSummary::attested_static(
                        "Azure OpenAI returned a malformed streaming frame",
                    ));
                    if !send_or_cancel(&cancel, tx, Err(err)).await? {
                        return Ok(());
                    }
                }
            }
        }
    }

    if !saw_done {
        let err = ProviderError::StreamError(ProviderErrorSummary::attested_static(
            "stream ended without a terminal event",
        ));
        let _ = send_or_cancel(&cancel, tx, Err(err)).await?;
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
    headers: &reqwest::header::HeaderMap,
) -> ProviderError {
    match status.as_u16() {
        401 | 403 => ProviderError::AuthFailed(ProviderErrorSummary::authentication_rejected()),
        404 => ProviderError::Config(ProviderErrorSummary::from_http_response(404)),
        429 => ProviderError::RateLimited {
            retry_after_ms: crate::retry::parse_retry_after(headers),
        },
        408 | 504 => ProviderError::Timeout,
        code => ProviderError::ProviderSide(ProviderErrorSummary::from_http_response(code)),
    }
}
