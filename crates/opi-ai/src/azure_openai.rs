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
use crate::openai_chat::{CompatConfig, OpenAiChatMapper, OpenAiChatProvider, ParsedEvent};
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
    pub fn build_request_body(
        &self,
        request: &Request,
    ) -> Result<serde_json::Value, crate::provider::ProviderError> {
        self.inner.build_request_body(request)
    }

    /// Stream events from a raw SSE response body (for testing).
    pub fn stream_from_sse(&self, sse_body: &str, cancel: CancellationToken) -> EventStream {
        let mut mapper = OpenAiChatMapper::new(
            crate::ApiKind::OpenAi,
            "azure",
            crate::model_info::WireApi::AzureOpenAiCompletions,
            "",
        );
        let mut stream_events: Vec<Result<AssistantStreamEvent, ProviderError>> = Vec::new();
        for (parsed, _raw) in crate::openai_chat::parse_sse_events_with_raw(sse_body) {
            match parsed {
                ParsedEvent::Valid(events) => {
                    for event in events {
                        stream_events.extend(mapper.process(event));
                    }
                }
                ParsedEvent::UsageError(error) => stream_events.push(Err(mapper.failure(error))),
                ParsedEvent::Malformed { .. } => {
                    stream_events.push(Err(mapper.failure(ProviderError::StreamError(
                        ProviderErrorSummary::attested_static(
                            "Azure OpenAI returned a malformed streaming frame",
                        ),
                    ))));
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
        let secret = auth.secret.expose_secret();
        let api_key = match auth.scheme {
            crate::auth::AuthScheme::ApiKey => secret.to_string(),
            crate::auth::AuthScheme::Bearer | crate::auth::AuthScheme::AwsSigV4(_) => {
                return Box::pin(stream::iter(vec![Err(ProviderError::Config(
                    ProviderErrorSummary::attested_static(
                        "Azure OpenAI requires prepared API-key authentication",
                    ),
                ))]));
            }
        };
        let model_id = request
            .model
            .split_once(':')
            .map(|(_, id)| id)
            .unwrap_or(&request.model)
            .to_string();

        let url = self.build_azure_url(&model_id);
        let requested_model = model_id.clone();
        let body = match self.inner.build_request_body(&request) {
            Ok(body) => body,
            Err(error) => return Box::pin(stream::once(async move { Err(error) })),
        };
        // Payload callback (task 20.51): once per variant, frozen across
        // ordinary retries, on the inner chat projection.
        let body = match request
            .callbacks
            .apply_payload(crate::callbacks::ORDINARY_PAYLOAD_VARIANT, body)
        {
            Ok(body) => body,
            Err(error) => return Box::pin(stream::once(async move { Err(error) })),
        };
        let cancel = request.cancel;
        let timeout = request.timeout;
        let extra_headers = request.extra_headers;
        let callbacks = request.callbacks;
        let http_client = self.client.client().clone();

        let snapshot = crate::stream::ProducerSnapshot::default();
        let producer_snapshot = snapshot.clone();
        let (tx, rx) = tokio::sync::mpsc::channel(64);
        let producer_cancel = cancel.clone();

        let tx_closed = tx.clone();
        tokio::spawn(async move {
            let run = async {
                let result = stream_azure_http(
                    http_client,
                    api_key,
                    &url,
                    requested_model,
                    &body,
                    producer_cancel.clone(),
                    timeout,
                    extra_headers,
                    callbacks,
                    producer_snapshot,
                    &tx,
                )
                .await;
                match result {
                    Ok(()) | Err(ProviderError::Cancelled) => {}
                    Err(error) => {
                        let _ = send_or_cancel(&producer_cancel, &tx, Err(error)).await;
                    }
                }
            };
            tokio::select! {
                biased;
                _ = producer_cancel.cancelled() => (),
                _ = tx_closed.closed() => (),
                _ = run => (),
            }
        });

        Box::pin(CancelAwareReceiverStream::with_snapshot(
            rx, cancel, snapshot,
        ))
    }
}

/// HTTP streaming with Azure-specific URL and `api-key` header.
#[allow(clippy::too_many_arguments)]
async fn stream_azure_http(
    http_client: reqwest::Client,
    api_key: String,
    url: &str,
    requested_model: String,
    body: &serde_json::Value,
    cancel: CancellationToken,
    timeout: Option<std::time::Duration>,
    extra_headers: Vec<(String, String)>,
    callbacks: crate::callbacks::RequestCallbacks,
    snapshot: crate::stream::ProducerSnapshot,
    tx: &tokio::sync::mpsc::Sender<Result<AssistantStreamEvent, ProviderError>>,
) -> Result<(), ProviderError> {
    let route_headers = vec![
        ("api-key".to_owned(), api_key),
        ("content-type".to_owned(), "application/json".to_owned()),
    ];
    let merged = ProviderHeaders::default().merge_request(&route_headers, &extra_headers)?;
    let transformable: Vec<(String, String)> = merged
        .iter()
        .filter(|(name, _)| {
            // The protected route headers (api-key identity, content-type
            // wire type) are excluded from the transformable per-call set.
            !route_headers
                .iter()
                .any(|(managed, _)| name.eq_ignore_ascii_case(managed))
        })
        .cloned()
        .collect();
    // Header transform (task 20.51) on the merged per-call set; the output is
    // revalidated against protected identity/auth/signing names.
    let transformed = callbacks.apply_header_transform(&transformable)?;
    let mut headers = route_headers.clone();
    headers.extend(transformed);
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

    // Response observer (task 20.51): status and headers exist, before any
    // body processing or error mapping.
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
        return Err(map_azure_status(status, &headers));
    }
    // Response callback (V-36): fired exactly once on the successfully
    // acquired response; rejected dispatches never masquerade.
    callbacks.notify_response(status.as_u16(), &observed_headers)?;

    let mut byte_stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut decoder = crate::text_decode::StreamTextDecoder::new();
    let mut mapper = OpenAiChatMapper::new(
        crate::ApiKind::OpenAi,
        "azure",
        crate::model_info::WireApi::AzureOpenAiCompletions,
        &requested_model,
    );
    let mut saw_done = false;

    mapper.set_snapshot(snapshot);
    let result: Result<(), ProviderError> = async {
        loop {
            let chunk = tokio::select! {
                biased;
                _ = cancel.cancelled() => {
                    return Err(mapper.failure(ProviderError::Cancelled));
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
            buffer.push_str(&decoder.push(&chunk));

            for (parsed, raw) in drain_sse_events(&mut buffer) {
                // Raw observer (task 20.51): parsed wire JSON before
                // normalization; failure preserves the partial.
                if let Some(raw) = raw.as_ref()
                    && let Err(error) = callbacks.notify_raw(raw)
                {
                    return Err(mapper.failure(error));
                }
                match parsed {
                    ParsedEvent::Valid(events) => {
                        for event in events {
                            for item in mapper.process(event) {
                                let is_terminal = match &item {
                                    Ok(terminal_event) => terminal_event.is_terminal(),
                                    Err(_) => true,
                                };
                                if !send_or_cancel(&cancel, tx, item).await? {
                                    return Ok(());
                                }
                                if is_terminal {
                                    saw_done = true;
                                }
                            }
                        }
                    }
                    ParsedEvent::UsageError(error) => {
                        if !send_or_cancel(&cancel, tx, Err(mapper.failure(error))).await? {
                            return Ok(());
                        }
                    }
                    ParsedEvent::Malformed { .. } => {
                        let err =
                            ProviderError::StreamError(ProviderErrorSummary::attested_static(
                                "Azure OpenAI returned a malformed streaming frame",
                            ));
                        if !send_or_cancel(&cancel, tx, Err(mapper.failure(err))).await? {
                            return Ok(());
                        }
                    }
                }
            }
        }

        let eof_tail = decoder.finish();
        buffer.push_str(&eof_tail);
        if !saw_done {
            let err = ProviderError::StreamError(ProviderErrorSummary::attested_static(
                "stream ended without a terminal event",
            ));
            let _ = send_or_cancel(&cancel, tx, Err(mapper.failure(err))).await?;
        }

        Ok(())
    }
    .await;
    result.map_err(|cause| mapper.failure(cause))
}

fn drain_sse_events(buffer: &mut String) -> Vec<(ParsedEvent, Option<serde_json::Value>)> {
    if buffer.contains('\r') {
        *buffer = buffer.replace("\r\n", "\n").replace('\r', "\n");
    }

    let mut events = Vec::new();
    while let Some(idx) = buffer.find("\n\n") {
        let end = idx + 2;
        let chunk: String = buffer.drain(..end).collect();
        events.extend(crate::openai_chat::parse_sse_events_with_raw(&chunk));
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
