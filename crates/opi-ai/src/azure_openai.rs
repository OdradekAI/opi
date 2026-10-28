//! Azure OpenAI provider profile (task 20.38: explicit dual wires).
//!
//! - Chat wire: `{endpoint}/openai/deployments/{deployment}/chat/completions?api-version={version}`
//! - Responses wire: `{endpoint}/openai/deployments/{deployment}/responses?api-version={version}`
//! - Auth: `api-key` header (not `Authorization: Bearer`)
//! - Endpoint/deployment/api-version resolution follows the pinned source:
//!   explicit base URL or resource name, then the model base URL; the
//!   api-version default is `v1` (pi 1.3.0 `azure-openai-config.ts`).
//!
//! The Responses wire reuses the exact 20.37 parameter assembly and shared
//! Responses mapper/replay machinery; Azure omits `service_tier` (source
//! omission; no Responses service-tier multiplier on this wire).

use std::fmt;
use std::sync::Arc;

use futures_util::{StreamExt, stream};
use secrecy::ExposeSecret;
use tokio_util::sync::CancellationToken;

use crate::http::HttpClient;
use crate::model_info::WireApi;
use crate::openai_chat::{CompatConfig, OpenAiChatMapper, OpenAiChatProvider, ParsedEvent};
use crate::openai_responses::{
    OpenAiResponsesOptions, OpenAiResponsesProvider, ResponsesReasoningSummary, ResponsesToolChoice,
};
use crate::openai_responses_shared::{ResponsesEvent, ResponsesMapper, drain_sse_frames};
use crate::provider::{
    EventStream, ModelInfo, Provider, ProviderError, ProviderErrorSummary, Request,
};
use crate::provider_headers::ProviderHeaders;
use crate::registry::ModelCapabilities;
use crate::stream::{AssistantStreamEvent, CancelAwareReceiverStream, send_or_cancel};

/// Default Azure OpenAI API version (source: pi 1.3.0
/// `azure-openai-config.ts` `DEFAULT_AZURE_API_VERSION`).
const DEFAULT_API_VERSION: &str = "v1";

/// Azure hosts whose configured base URL is normalized to the resource root
/// (opi's builder appends the `/openai/...` path segments itself).
const AZURE_HOST_SUFFIXES: [&str; 3] = [
    ".openai.azure.com",
    ".cognitiveservices.azure.com",
    ".ai.azure.com",
];

/// Base-path shapes treated as "the resource root" on an Azure host.
const AZURE_ROOT_PATHS: [&str; 5] = ["", "/", "/openai", "/openai/v1", "/openai/v1/responses"];

/// Resolve the Azure base URL (task 20.38, pinned source precedence):
/// explicit base URL, then resource name, then the model's base URL; a
/// typed failure when none is configured. Azure hosts carrying one of the
/// known `/openai[/v1][/responses]` path shapes normalize to the resource
/// root; non-Azure hosts keep their configured path.
pub fn resolve_azure_base_url(
    explicit_base_url: Option<&str>,
    resource_name: Option<&str>,
    model_base_url: Option<&str>,
) -> Result<String, ProviderError> {
    let explicit = explicit_base_url
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let resolved = explicit
        .map(str::to_owned)
        .or_else(|| {
            resource_name
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|resource| format!("https://{resource}.openai.azure.com"))
        })
        .or_else(|| {
            model_base_url
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
        })
        .ok_or_else(|| {
            azure_config_error(
                "Azure OpenAI base URL is required. Set it via config [providers.azure] endpoint or AZURE_OPENAI_ENDPOINT env var.",
            )
        })?;

    normalize_azure_base_url(&resolved)
}

/// Normalize a resolved base URL: strip the trailing slash and, on Azure
/// hosts, collapse the known `/openai[/v1][/responses]` path shapes to the
/// resource root. Invalid URLs fail typed.
fn normalize_azure_base_url(base_url: &str) -> Result<String, ProviderError> {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(azure_config_error(
            "Azure OpenAI base URL must not be empty.",
        ));
    }
    let url = url::Url::parse(trimmed).map_err(|_| {
        ProviderError::Config(ProviderErrorSummary::sanitized(format!(
            "Invalid Azure OpenAI base URL: {trimmed}"
        )))
    })?;
    let host = url.host_str().unwrap_or_default();
    let is_azure_host = AZURE_HOST_SUFFIXES
        .iter()
        .any(|suffix| host.ends_with(suffix));
    if !is_azure_host {
        return Ok(trimmed.to_owned());
    }
    let path = url.path().trim_end_matches('/');
    if AZURE_ROOT_PATHS.contains(&path) {
        return Ok(format!("{}://{host}", url.scheme()));
    }
    Ok(trimmed.to_owned())
}

/// Resolve the request api-version: explicit value wins, else the pinned
/// source default `v1`.
pub fn resolve_azure_api_version(explicit: Option<&str>) -> String {
    explicit
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_API_VERSION)
        .to_owned()
}

/// Parse a `modelId=deployment,modelId2=deployment2` map (source:
/// `AZURE_OPENAI_DEPLOYMENT_NAME_MAP`).
fn parse_deployment_map(value: Option<&str>) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    let Some(value) = value else { return map };
    for entry in value.split(',') {
        let Some((model_id, deployment)) = entry.trim().split_once('=') else {
            continue;
        };
        let (model_id, deployment) = (model_id.trim(), deployment.trim());
        if model_id.is_empty() || deployment.is_empty() {
            continue;
        }
        map.insert(model_id.to_owned(), deployment.to_owned());
    }
    map
}

/// Resolve the deployment name for one model (pinned source precedence):
/// explicit deployment, then the configured deployment map, then the model
/// id. The deployment rides the request body's `model` field and the URL
/// path; the catalog id never changes.
pub fn resolve_deployment_name(
    explicit: Option<&str>,
    deployment_map: Option<&str>,
    model_id: &str,
) -> String {
    if let Some(explicit) = explicit.map(str::trim).filter(|value| !value.is_empty()) {
        return explicit.to_owned();
    }
    parse_deployment_map(deployment_map)
        .get(model_id)
        .cloned()
        .unwrap_or_else(|| model_id.to_owned())
}

fn azure_config_error(message: &'static str) -> ProviderError {
    ProviderError::Config(ProviderErrorSummary::attested_static(message))
}

/// Azure OpenAI provider.
///
/// Wraps an [`OpenAiChatProvider`] for request body serialization and SSE
/// parsing, but overrides the HTTP transport layer (URL and auth header).
#[derive(Clone)]
pub struct AzureOpenAIProvider {
    endpoint: String,
    api_version: String,
    models: Vec<ModelInfo>,
    inner: OpenAiChatProvider,
    client: Arc<HttpClient>,
    /// Optional `modelId=deployment` map (task 20.38): explicit per-call
    /// deployment options win, then this map, then the model id.
    deployment_map: Option<String>,
    /// Typed dispatch record (task 20.38): a responses-wire record routes
    /// this dispatch through the Responses wire; `None` keeps the legacy
    /// chat transport.
    active_record: Option<ModelInfo>,
    /// Typed native options set per dispatch by the public adapter
    /// (task 20.38); `None` on the plain provider path.
    azure_native: Option<AzureOpenAIOptions>,
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
        let endpoint = resolve_azure_base_url(endpoint.as_deref(), None, None)?;
        let api_version = resolve_azure_api_version(api_version.as_deref());
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
            deployment_map: None,
            active_record: None,
            azure_native: None,
        })
    }

    /// Create from config with explicit deployment names for the model list.
    pub fn from_config(
        endpoint: Option<String>,
        deployments: Vec<String>,
        api_version: Option<String>,
    ) -> Result<Self, ProviderError> {
        let endpoint = resolve_azure_base_url(endpoint.as_deref(), None, None)?;
        let api_version = resolve_azure_api_version(api_version.as_deref());
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
            deployment_map: None,
            active_record: None,
            azure_native: None,
        })
    }

    /// Create from explicit model records (task 20.38): each record's own
    /// wire selects the transport (chat or responses). The deployment for
    /// one dispatch resolves from the per-call options, the deployment
    /// map, then the record id.
    pub fn for_records(
        endpoint: Option<String>,
        models: Vec<ModelInfo>,
        api_version: Option<String>,
    ) -> Result<Self, ProviderError> {
        let endpoint = resolve_azure_base_url(endpoint.as_deref(), None, None)?;
        let api_version = resolve_azure_api_version(api_version.as_deref());
        if models.is_empty()
            || models
                .iter()
                .any(|model| model.id.trim().is_empty() || model.validate().is_err())
        {
            return Err(ProviderError::Config(
                ProviderErrorSummary::attested_static(
                    "Azure OpenAI model records must be complete and non-empty",
                ),
            ));
        }
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
            deployment_map: None,
            active_record: None,
            azure_native: None,
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

    /// Build the Azure Responses deployment URL (task 20.38 dual wire).
    pub fn build_azure_responses_url(&self, deployment: &str) -> String {
        format!(
            "{}/openai/deployments/{}/responses?api-version={}",
            self.endpoint, deployment, self.api_version
        )
    }

    /// Set the optional `modelId=deployment` map used when a dispatch has
    /// no explicit deployment option (task 20.38).
    pub fn with_deployment_map(mut self, map: Option<String>) -> Self {
        self.deployment_map = map;
        self
    }

    /// Clone with the typed dispatch state for one call (task 20.38): a
    /// responses-wire record routes through the Responses wire. The public
    /// adapter owns this transition; registered providers never mutate.
    pub(crate) fn dispatch_configured(
        &self,
        model: ModelInfo,
        options: AzureOpenAIOptions,
    ) -> AzureOpenAIProvider {
        let mut provider = self.clone();
        provider.active_record = Some(model);
        provider.azure_native = Some(options);
        provider
    }

    /// Stream one responses-wire dispatch (task 20.38): the shared 20.37
    /// parameter assembly under Azure transport (deployment URL, api-key
    /// header, azure status mapping, duration and telemetry).
    fn stream_responses_prepared(
        &self,
        record: ModelInfo,
        mut request: Request,
        auth: crate::auth::ResolvedAuth,
    ) -> EventStream {
        request.callbacks = request.callbacks.ensure_call_scope();
        let secret = auth.secret.expose_secret().to_string();
        let api_key = match auth.scheme {
            crate::auth::AuthScheme::ApiKey => secret,
            crate::auth::AuthScheme::Bearer | crate::auth::AuthScheme::AwsSigV4(_) => {
                return Box::pin(stream::once(async move {
                    Err(ProviderError::Config(
                        ProviderErrorSummary::attested_static(
                            "Azure OpenAI requires prepared API-key authentication",
                        ),
                    ))
                }));
            }
        };
        let requested_model = request
            .model
            .split_once(':')
            .map(|(_, id)| id)
            .unwrap_or(&request.model)
            .to_owned();
        let native = self.azure_native.clone().unwrap_or_default();
        let deployment = native
            .azure_deployment_name
            .clone()
            .or_else(|| {
                parse_deployment_map(self.deployment_map.as_deref())
                    .get(&requested_model)
                    .cloned()
            })
            .unwrap_or_else(|| requested_model.clone());
        let endpoint = native
            .azure_base_url
            .as_deref()
            .map(str::to_owned)
            .or_else(|| {
                native
                    .azure_resource_name
                    .as_deref()
                    .map(|resource| format!("https://{resource}.openai.azure.com"))
            })
            .and_then(|base| normalize_azure_base_url(&base).ok())
            .unwrap_or_else(|| self.endpoint.clone());
        let api_version = native
            .azure_api_version
            .clone()
            .unwrap_or_else(|| self.api_version.clone());
        let url = format!(
            "{endpoint}/openai/deployments/{deployment}/responses?api-version={api_version}"
        );

        // Shared 20.37 parameter assembly under the azure identity: the
        // record keeps its own wire so same-identity replay works
        // end-to-end; service_tier stays absent (source omission).
        let inner = OpenAiResponsesProvider::for_route(
            None,
            "azure".into(),
            ProviderHeaders::default(),
            vec![record.clone()],
            self.client.clone(),
        )
        .dispatch_configured(
            record.clone(),
            OpenAiResponsesOptions {
                tool_choice: native.tool_choice,
                reasoning_effort: native.reasoning_effort,
                reasoning_summary: native.reasoning_summary,
                service_tier: None,
                simple: native.simple,
            },
        );
        let (body, wire_facts, effort_stamp) = match inner.compile_request_body(&request) {
            Ok(compiled) => compiled,
            Err(error) => return Box::pin(stream::once(async move { Err(error) })),
        };
        let body = {
            let mut body = body;
            body["model"] = serde_json::Value::String(deployment.clone());
            body
        };

        let cancel = request.cancel;
        let timeout = request.timeout;
        let extra_headers = request.extra_headers;
        let callbacks = request.callbacks;
        // Explicit frozen telemetry context (task 20.54/AIR-055): the
        // producer task runs under the captured parent/Dispatch when
        // provided; omission observes nothing.
        let telemetry = request.telemetry_context.clone();
        let http_client = self.client.client().clone();
        let snapshot = crate::stream::ProducerSnapshot::default();
        let producer_snapshot = snapshot.clone();
        let (tx, rx) = tokio::sync::mpsc::channel(64);
        let tx_closed = tx.clone();
        let producer_cancel = cancel.clone();

        let producer = async move {
            let work = async {
                if let Err(error) = stream_azure_responses_http(
                    http_client,
                    api_key,
                    &url,
                    requested_model,
                    wire_facts,
                    effort_stamp,
                    &body,
                    producer_cancel.clone(),
                    timeout,
                    extra_headers,
                    callbacks,
                    producer_snapshot,
                    &tx,
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
            rx, cancel, snapshot,
        ))
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

    fn stream_prepared(
        &self,
        mut request: Request,
        auth: crate::auth::ResolvedAuth,
    ) -> EventStream {
        request.callbacks = request.callbacks.ensure_call_scope();
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

        // Typed dual-wire dispatch (task 20.38): the bound record (adapter
        // path) or the listed record for this model selects the wire; a
        // responses-wire record routes through the Responses transport,
        // every other record keeps the legacy chat transport.
        if let Some(record) = self
            .active_record
            .clone()
            .or_else(|| {
                self.models
                    .iter()
                    .find(|model| model.id == model_id)
                    .cloned()
            })
            .filter(|record| record.wire_api == WireApi::AzureOpenAiResponses)
        {
            return self.stream_responses_prepared(record, request, auth);
        }

        let url = self.build_azure_url(&model_id);
        let requested_model = model_id.clone();
        let body = match self.inner.build_request_body(&request) {
            Ok(body) => body,
            Err(error) => return Box::pin(stream::once(async move { Err(error) })),
        };
        let cancel = request.cancel;
        let timeout = request.timeout;
        let extra_headers = request.extra_headers;
        let callbacks = request.callbacks;
        // Explicit frozen telemetry context (task 20.54/AIR-055): the chat
        // producer runs under the captured parent/Dispatch when provided.
        let telemetry = request.telemetry_context.clone();
        let http_client = self.client.client().clone();

        let snapshot = crate::stream::ProducerSnapshot::default();
        let producer_snapshot = snapshot.clone();
        let (tx, rx) = tokio::sync::mpsc::channel(64);
        let producer_cancel = cancel.clone();

        let tx_closed = tx.clone();
        let producer = async move {
            let run = async {
                let result = async {
                    let body = callbacks
                        .apply_payload_async(crate::callbacks::ORDINARY_PAYLOAD_VARIANT, body)
                        .await?;
                    stream_azure_http(
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
                    .await
                }
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
        };
        if let Some(context) = telemetry {
            tokio::spawn(context.instrument(producer));
        } else {
            tokio::spawn(producer);
        }

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
    let transformed = callbacks
        .apply_header_transform_async(&transformable)
        .await?;
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
    snapshot.received_response();

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
    callbacks
        .notify_response_async(status.as_u16(), &observed_headers)
        .await
        .map_err(|error| mapper.failure(error))?;
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
                if !saw_done
                    && let Some(raw) = raw.as_ref()
                    && let Err(error) = callbacks.notify_raw_async(raw).await
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

/// HTTP streaming for the Azure Responses wire (task 20.38): Azure URL and
/// `api-key` header, shared Responses SSE mapping, duration stamping and
/// the ordinary payload/response/raw callbacks.
#[allow(clippy::too_many_arguments)]
async fn stream_azure_responses_http(
    http_client: reqwest::Client,
    api_key: String,
    url: &str,
    requested_model: String,
    wire_facts: Option<crate::openai_responses_shared::ResponsesWireTools>,
    effort_stamp: Option<String>,
    body: &serde_json::Value,
    cancel: CancellationToken,
    timeout: Option<std::time::Duration>,
    extra_headers: Vec<(String, String)>,
    callbacks: crate::callbacks::RequestCallbacks,
    snapshot: crate::stream::ProducerSnapshot,
    tx: &tokio::sync::mpsc::Sender<Result<AssistantStreamEvent, ProviderError>>,
) -> Result<(), ProviderError> {
    // Payload callback (task 20.51) then original-schema validation: the
    // transformed body must stay a valid Responses request.
    let body = callbacks
        .apply_payload_async(crate::callbacks::ORDINARY_PAYLOAD_VARIANT, body.clone())
        .await
        .and_then(|body| {
            crate::callbacks::validate_openai_payload_shape(
                &body,
                crate::callbacks::OpenAiPayloadShape::Responses,
            )?;
            Ok(body)
        })?;

    let route_headers = vec![
        ("api-key".to_owned(), api_key),
        ("content-type".to_owned(), "application/json".to_owned()),
    ];
    let merged = ProviderHeaders::default().merge_request(&route_headers, &extra_headers)?;
    let transformable: Vec<(String, String)> = merged
        .iter()
        .filter(|(name, _)| {
            !route_headers
                .iter()
                .any(|(managed, _)| name.eq_ignore_ascii_case(managed))
        })
        .cloned()
        .collect();
    let transformed = callbacks
        .apply_header_transform_async(&transformable)
        .await?;
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
            .body(serde_json::to_string(&body).unwrap_or_default())
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
    // Response callback (V-36): exactly once on the acquired response.
    snapshot.received_response();

    let mut byte_stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut decoder = crate::text_decode::StreamTextDecoder::new();
    let mut response_activated_at: Option<std::time::Instant> = None;
    let mut mapper = ResponsesMapper::new("azure", WireApi::AzureOpenAiResponses, &requested_model);
    if let Some(wire) = wire_facts {
        mapper.set_grammar_tools(wire.grammar_input_properties);
        mapper.set_strict_tools(wire.strict_tools);
        mapper.seed_diagnostics(wire.diagnostics);
    }
    mapper.set_provider_thinking_level(effort_stamp);
    mapper.set_snapshot(snapshot);
    callbacks
        .notify_response_async(status.as_u16(), &observed_headers)
        .await
        .map_err(|error| mapper.failure(error))?;
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
            if response_activated_at.is_none() {
                response_activated_at = Some(std::time::Instant::now());
            }
            for frame in drain_sse_frames(&mut buffer) {
                // Raw observer (task 20.51): parsed wire JSON before
                // normalization; failure preserves the partial.
                if !mapper.saw_done
                    && let Ok(raw) = serde_json::from_str::<serde_json::Value>(&frame.data)
                    && let Err(error) = callbacks.notify_raw_async(&raw).await
                {
                    return Err(mapper.failure(error));
                }
                match ResponsesEvent::try_from_frame(&frame) {
                    crate::openai_responses_shared::ParsedEvent::Valid(event) => {
                        for item in mapper.process(event) {
                            let item = OpenAiResponsesProvider::stamp_done_duration(
                                item,
                                response_activated_at,
                            );
                            let is_terminal = match &item {
                                Ok(terminal_event) => terminal_event.is_terminal(),
                                Err(_) => true,
                            };
                            if !send_or_cancel(&cancel, tx, item).await? {
                                return Ok(());
                            }
                            if is_terminal {
                                break;
                            }
                        }
                    }
                    crate::openai_responses_shared::ParsedEvent::UsageError(error) => {
                        return Err(mapper.failure(ProviderError::StreamError(
                            ProviderErrorSummary::sanitized(error),
                        )));
                    }
                    crate::openai_responses_shared::ParsedEvent::Malformed { .. } => {
                        return Err(mapper.failure(ProviderError::StreamError(
                            ProviderErrorSummary::attested_static(
                                "Azure OpenAI returned a malformed Responses SSE frame",
                            ),
                        )));
                    }
                }
            }
        }

        let eof_tail = decoder.finish();
        buffer.push_str(&eof_tail);
        if !mapper.saw_done {
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

/// Typed azure-openai-responses native parameters (task 20.38, SDK annex
/// §2 R/Azure row): the responses pair keeps `reasoning_effort`,
/// `reasoning_summary` (omission and explicit null both wire `auto`) and
/// `tool_choice`, plus the Azure endpoint mapping fields. `service_tier`
/// is deliberately absent (source omission; the plain OpenAI Responses
/// adapter owns that option).
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct AzureOpenAIOptions {
    /// Explicit deployment name; the deployment map then the model id are
    /// the fallbacks.
    pub azure_deployment_name: Option<String>,
    /// Resource name resolving to `https://{resource}.openai.azure.com`.
    pub azure_resource_name: Option<String>,
    /// Explicit base URL override (azure hosts normalize to the resource
    /// root).
    pub azure_base_url: Option<String>,
    /// Explicit api-version override (default `v1`).
    pub azure_api_version: Option<String>,
    /// Full Responses `tool_choice` shape.
    pub tool_choice: Option<ResponsesToolChoice>,
    /// Native reasoning effort; unsupported levels fail typed preflight,
    /// the simple entry clamps instead.
    pub reasoning_effort: Option<crate::model_info::ThinkingLevel>,
    /// Reasoning summary selection; omission and explicit null both map to
    /// the wire default `auto`.
    pub reasoning_summary: Option<ResponsesReasoningSummary>,
    simple: bool,
}

impl AzureOpenAIOptions {
    /// Neutral mapping for the simple entry: only neutral `Auto`/`None`
    /// tool choice and the request's neutral thinking intent.
    fn from_simple(options: &crate::models::SimpleOptions) -> Self {
        Self {
            azure_deployment_name: None,
            azure_resource_name: None,
            azure_base_url: None,
            azure_api_version: None,
            tool_choice: options.tool_choice.map(|choice| match choice {
                crate::models::SimpleToolChoice::Auto => ResponsesToolChoice::Auto,
                crate::models::SimpleToolChoice::None => ResponsesToolChoice::None,
            }),
            reasoning_effort: None,
            reasoning_summary: None,
            simple: true,
        }
    }

    fn is_simple(&self) -> bool {
        self.simple
    }
}

/// The public azure-openai-responses direct adapter registered through
/// [`crate::Models::register_chat_api`] (task 20.38). Each dispatch clones
/// the immutable provider configuration and routes through the Azure
/// responses wire; native options never mutate a registered provider.
pub struct AzureOpenAIResponsesChatApi {
    api_id: crate::models::ApiId,
    provider: Arc<AzureOpenAIProvider>,
    model: ModelInfo,
}

impl std::fmt::Debug for AzureOpenAIResponsesChatApi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AzureOpenAIResponsesChatApi")
            .field("api_id", &self.api_id)
            .finish_non_exhaustive()
    }
}

impl AzureOpenAIResponsesChatApi {
    /// Bind a complete azure responses record. Wrong-wire or invalid
    /// records fail before authentication or network activity.
    pub fn new(
        provider: Arc<AzureOpenAIProvider>,
        model: ModelInfo,
    ) -> Result<Self, ProviderError> {
        validate_complete_azure_responses_record(&model)?;
        Ok(Self {
            api_id: crate::models::ApiId::new("azure-openai-responses"),
            provider,
            model,
        })
    }
}

/// One complete, wire-valid azure responses model record or a typed
/// failure.
fn validate_complete_azure_responses_record(model: &ModelInfo) -> Result<(), ProviderError> {
    if model.wire_api != WireApi::AzureOpenAiResponses
        || model.id.trim().is_empty()
        || model.validate().is_err()
    {
        return Err(ProviderError::Config(
            ProviderErrorSummary::attested_static(
                "azure-openai-responses adapters require a valid complete azure responses model record",
            ),
        ));
    }
    Ok(())
}

impl crate::models::ChatApi<AzureOpenAIOptions> for AzureOpenAIResponsesChatApi {
    fn validate_simple_options(
        &self,
        model: &ModelInfo,
        _options: &crate::models::SimpleOptions,
    ) -> Result<(), ProviderError> {
        validate_complete_azure_responses_record(model)
    }

    fn api_id(&self) -> &crate::models::ApiId {
        &self.api_id
    }

    fn provider_id(&self) -> &str {
        self.provider.id()
    }

    fn complete_record(&self) -> ModelInfo {
        self.model.clone()
    }

    fn simple_options(&self) -> AzureOpenAIOptions {
        AzureOpenAIOptions::from_simple(&crate::models::SimpleOptions::default())
    }

    fn simple_options_for(&self, options: &crate::models::SimpleOptions) -> AzureOpenAIOptions {
        AzureOpenAIOptions::from_simple(options)
    }

    fn session_resources(&self) -> Option<&dyn crate::codex_resources::SessionResourceOwner> {
        None
    }

    fn validate_native_request(
        &self,
        model: &ModelInfo,
        _request: &Request,
        options: &AzureOpenAIOptions,
    ) -> Result<(), ProviderError> {
        validate_complete_azure_responses_record(model)?;
        // Native/exact effort fails typed preflight on unsupported levels;
        // the neutral entry clamps instead.
        if let Some(level) = options.reasoning_effort
            && !options.is_simple()
            && model.thinking_level_map.resolve(level).is_err()
        {
            return Err(ProviderError::Config(ProviderErrorSummary::sanitized(
                format!(
                    "azure-openai-responses native options reject unsupported thinking level {level:?}"
                ),
            )));
        }
        Ok(())
    }

    fn stream_prepared(
        &self,
        request: Request,
        options: AzureOpenAIOptions,
        auth: crate::auth::ResolvedAuth,
    ) -> EventStream {
        self.stream_model_resource_prepared(self.model.clone(), request, options, auth, None)
    }

    fn stream_resource_prepared(
        &self,
        request: Request,
        options: AzureOpenAIOptions,
        auth: crate::auth::ResolvedAuth,
        _resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        let provider = self
            .provider
            .dispatch_configured(self.model.clone(), options);
        provider.stream_prepared(request, auth)
    }
}
