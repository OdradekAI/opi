//! Google Vertex AI provider.
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

use crate::gemini::{
    GeminiMapper, GeminiProvider, GoogleGenerativeAiOptions, GoogleThinkingConfig,
    GoogleToolChoice, ParsedEvent, drain_sse_payloads, parse_sse_data,
};
use crate::http::HttpClient;
use crate::model_info::WireApi;
use crate::provider::{
    EventStream, ModelInfo, Provider, ProviderError, ProviderErrorSummary, Request,
};
use crate::provider_headers::ProviderHeaders;
use crate::registry::ModelCapabilities;
use crate::stream::{AssistantStreamEvent, CancelAwareReceiverStream, send_or_cancel};

/// Google Vertex AI provider.
///
/// Wraps a [`GeminiProvider`] for request body serialization and SSE parsing,
/// but overrides the HTTP transport layer (URL and auth header).
#[derive(Clone)]
pub struct VertexProvider {
    provider_identity: String,
    project: String,
    location: String,
    base_url: String,
    models: Vec<ModelInfo>,
    inner: GeminiProvider,
    client: Arc<HttpClient>,
    /// API-key Express mode: project/location-less wire routed to the global
    /// `aiplatform.googleapis.com` collection endpoint with
    /// `x-goog-api-key` (pinned `@google/genai` Express behavior).
    express: bool,
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
    /// Create a new Vertex AI provider (ADC/Bearer mode with project and
    /// location).
    pub fn new(project: String, location: String, base_url: Option<String>) -> Self {
        let base_url = base_url.unwrap_or_else(|| default_vertex_base_url(&location));
        let inner = GeminiProvider::new(None).with_wire(WireApi::GoogleVertex);
        let models = model_catalog();
        Self {
            provider_identity: "vertex".into(),
            project,
            location,
            base_url,
            models,
            inner,
            client: Arc::new(HttpClient::new()),
            express: false,
        }
    }

    /// Create the API-key Express provider (no project/location): requests
    /// go to `{base}/v1/publishers/google/models/{model}:streamGenerateContent`
    /// with the `x-goog-api-key` header.
    pub fn express(base_url: Option<String>) -> Self {
        let base_url = base_url.unwrap_or_else(|| "https://aiplatform.googleapis.com".to_owned());
        let inner = GeminiProvider::new(None).with_wire(WireApi::GoogleVertex);
        let models = model_catalog();
        Self {
            provider_identity: "vertex".into(),
            project: String::new(),
            location: String::new(),
            base_url,
            models,
            inner,
            client: Arc::new(HttpClient::new()),
            express: true,
        }
    }

    /// Whether this provider routes Express (API-key) requests.
    pub fn is_express(&self) -> bool {
        self.express
    }

    /// Create from config with explicit model list.
    pub fn from_config(
        project: String,
        location: String,
        models: Vec<String>,
        base_url: Option<String>,
    ) -> Self {
        let base_url = base_url.unwrap_or_else(|| default_vertex_base_url(&location));
        let inner = GeminiProvider::new(None).with_wire(WireApi::GoogleVertex);
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
            provider_identity: "vertex".into(),
            project,
            location,
            base_url,
            models: model_list,
            inner,
            client: Arc::new(HttpClient::new()),
            express: false,
        }
    }

    /// Replace the HTTP client (for shared connection pooling).
    pub fn with_client(self, client: Arc<HttpClient>) -> Self {
        Self { client, ..self }
    }

    /// Bind capture and replay to the registered canonical provider identity.
    pub(crate) fn with_provider_identity(mut self, provider: &str) -> Self {
        self.provider_identity = provider.to_owned();
        self.inner = self.inner.with_provider_identity(provider);
        self
    }

    /// Override the model records (task 20.41): direct adapters and tests
    /// bind complete custom records; the builtin catalog stays untouched.
    pub fn with_models(mut self, models: Vec<ModelInfo>) -> Self {
        self.inner = self.inner.with_models(models);
        self.models = self.inner.models().to_vec();
        self
    }

    /// Bind typed native options for one dispatch (task 20.41): the tool and
    /// thinking intents project onto the shared Google body engine and an
    /// explicit `project`/`location` re-routes the ADC URL (inert under the
    /// Express API-key route, which carries neither).
    pub fn with_native_options(mut self, options: GoogleVertexOptions) -> Self {
        if let Some(project) = options.project.as_ref() {
            self.project = project.clone();
        }
        if let Some(location) = options.location.as_ref() {
            self.location = location.clone();
        }
        self.inner = self.inner.with_native_options(options.google());
        self
    }

    /// Build the Vertex AI streaming URL for a given model.
    ///
    /// Express (API-key) mode drops the project/location resource prefix and
    /// routes to the collection endpoint; `location = "global"` keeps the
    /// project prefix on the global domain.
    pub fn build_vertex_url(&self, model_id: &str) -> String {
        if self.express {
            return format!(
                "{base}/v1/publishers/google/models/{model}:streamGenerateContent?alt=sse",
                base = self.base_url,
                model = model_id,
            );
        }
        format!(
            "{base}/v1/projects/{project}/locations/{location}/publishers/google/models/{model}:streamGenerateContent?alt=sse",
            base = self.base_url,
            project = self.project,
            location = self.location,
            model = model_id,
        )
    }

    /// Build the request body (delegates to inner Gemini provider).
    pub fn build_request_body(
        &self,
        request: &Request,
    ) -> Result<serde_json::Value, crate::provider::ProviderError> {
        self.inner.build_request_body(request)
    }

    /// Stream events from a raw SSE response body (for testing).
    pub fn stream_from_sse(&self, sse_body: &str, cancel: CancellationToken) -> EventStream {
        let mut mapper = GeminiMapper::new("vertex", crate::model_info::WireApi::GoogleVertex, "");
        let mut stream_events: Vec<Result<AssistantStreamEvent, ProviderError>> = Vec::new();

        for data in parse_sse_data(sse_body) {
            for parsed in ParsedEvent::from_data(&data) {
                match parsed {
                    ParsedEvent::Valid(event) => {
                        stream_events.extend(mapper.process(event));
                    }
                    ParsedEvent::Malformed => {
                        stream_events.push(Err(mapper.failure(ProviderError::StreamError(
                            ProviderErrorSummary::attested_static(
                                "Vertex returned a malformed streaming frame",
                            ),
                        ))));
                    }
                }
            }
        }

        let _cancel = cancel;
        Box::pin(stream::iter(stream_events))
    }
}

impl Provider for VertexProvider {
    fn validate_simple_options(
        &self,
        model: &ModelInfo,
        _options: &crate::models::SimpleOptions,
    ) -> Result<(), ProviderError> {
        validate_complete_vertex_record(model)
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
        self.clone()
            .with_models(vec![model])
            .with_native_options(GoogleVertexOptions::from_simple(&options))
            .stream_resource_attempt(request, auth, attempt, resources)
    }

    fn id(&self) -> &str {
        "vertex"
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
        // Google exposes parsed raw chunks but has no HTTP response hook.
        if request.callbacks.has_response_observer() {
            return Box::pin(stream::iter(vec![Err(
                ProviderError::UnsupportedCapability(ProviderErrorSummary::attested_static(
                    "Google wires do not support response callbacks",
                )),
            )]));
        }
        // Two pinned wire modes: an API key selects Express (x-goog-api-key,
        // collection URL); a Bearer token rides the ADC/project/location URL
        // plus the ADC file's quota project header when present.
        let auth_mode = match auth.scheme {
            crate::auth::AuthScheme::Bearer => VertexAuthMode::Bearer {
                token: auth.secret.expose_secret().to_string(),
                quota_project: quota_project_of(&auth).map(str::to_owned),
            },
            crate::auth::AuthScheme::ApiKey => VertexAuthMode::Express {
                api_key: auth.secret.expose_secret().to_string(),
            },
            crate::auth::AuthScheme::AwsSigV4(_) | crate::auth::AuthScheme::AnthropicFederation => {
                return Box::pin(stream::iter(vec![Err(ProviderError::Config(
                    ProviderErrorSummary::attested_static(
                        "Vertex accepts Bearer or API-key authentication",
                    ),
                ))]));
            }
        };
        // ADC routing owns the project/location resource prefix: a Bearer
        // dispatch without either is a typed setup failure, never a silently
        // degraded URL (pi resolveProject/resolveLocation throw).
        if !self.express
            && auth.base_url.is_none()
            && matches!(auth_mode, VertexAuthMode::Bearer { .. })
            && self.project.trim().is_empty()
        {
            return Box::pin(stream::iter(vec![Err(ProviderError::Config(
                ProviderErrorSummary::attested_static(
                    "Vertex AI requires a project ID for ADC routing; pass one in the native options or configure the provider",
                ),
            ))]));
        }
        if !self.express
            && auth.base_url.is_none()
            && matches!(auth_mode, VertexAuthMode::Bearer { .. })
            && self.location.trim().is_empty()
        {
            return Box::pin(stream::iter(vec![Err(ProviderError::Config(
                ProviderErrorSummary::attested_static(
                    "Vertex AI requires a location for ADC routing; pass one in the native options or configure the provider",
                ),
            ))]));
        }
        let model_id = request
            .model
            .split_once(':')
            .map(|(_, id)| id)
            .unwrap_or(&request.model)
            .to_string();

        // An explicit complete model base owns the collection scope. The
        // catalog's location template remains an instruction to use the
        // resolved Express/ADC endpoint instead.
        let model_base = self
            .models
            .iter()
            .find(|model| model.id == model_id)
            .and_then(|model| model.base_url.as_deref())
            .map(str::trim)
            .filter(|base| !base.is_empty() && !base.contains("{location}"));
        let url = if let Some(base) = model_base {
            let version = if vertex_base_has_api_version(base) {
                ""
            } else {
                "/v1"
            };
            format!(
                "{}{version}/publishers/google/models/{model_id}:streamGenerateContent?alt=sse",
                base.strip_suffix('/').unwrap_or(base)
            )
        } else if let Some(collection) = auth.base_url.as_deref() {
            format!(
                "{}/{model_id}:streamGenerateContent?alt=sse",
                collection.trim_end_matches('/')
            )
        } else if self.express {
            self.build_vertex_url(&model_id)
        } else {
            match &auth_mode {
                VertexAuthMode::Express { .. } => format!(
                    "{base}/v1/publishers/google/models/{model}:streamGenerateContent?alt=sse",
                    base = self.base_url,
                    model = model_id,
                ),
                VertexAuthMode::Bearer { .. } => self.build_vertex_url(&model_id),
            }
        };
        let requested_model = model_id;
        let body = match self.inner.build_request_body(&request) {
            Ok(body) => body,
            Err(error) => return Box::pin(stream::once(async move { Err(error) })),
        };
        let callbacks = request.callbacks.clone();
        let cancel = request.cancel;
        let timeout = request.timeout;
        let model_headers = self
            .models
            .iter()
            .find(|model| model.id == requested_model)
            .map(|model| model.headers.clone())
            .unwrap_or_default();
        let extra_headers = request.extra_headers;
        let http_client = self.client.client().clone();
        let provider_identity = self.provider_identity.clone();

        let snapshot = crate::stream::ProducerSnapshot::default();
        let producer_snapshot = snapshot.clone();
        let (tx, rx) = tokio::sync::mpsc::channel(64);
        let producer_cancel = cancel.clone();
        // Explicit frozen telemetry context (task 20.41, 20.54/AIR-055): the
        // producer task runs under the captured parent/Dispatch when
        // provided; omission observes nothing.
        let telemetry = request.telemetry_context.clone();

        let tx_closed = tx.clone();
        let producer = async move {
            let run = async {
                let result = async {
                    let body = callbacks
                        .apply_payload_async(crate::callbacks::ORDINARY_PAYLOAD_VARIANT, body)
                        .await?;
                    stream_vertex_http(
                        http_client,
                        provider_identity,
                        auth_mode,
                        &url,
                        requested_model,
                        &body,
                        producer_cancel.clone(),
                        timeout,
                        model_headers,
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

/// One pinned Vertex wire-auth mode.
#[derive(Clone)]
enum VertexAuthMode {
    /// ADC/Bearer route token plus the ADC file's non-secret quota project.
    Bearer {
        token: String,
        quota_project: Option<String>,
    },
    /// API-key Express mode (`x-goog-api-key`).
    Express { api_key: String },
}

/// The non-secret quota project carried by an ADC provenance source, when the
/// selected family reported one.
fn quota_project_of(auth: &crate::auth::ResolvedAuth) -> Option<&str> {
    match &auth.provenance.source {
        crate::auth::AuthProvenanceSource::Adc {
            source:
                crate::auth::AdcCredentialSource::AuthorizedUser { quota_project }
                | crate::auth::AdcCredentialSource::ExternalAccountAuthorizedUser {
                    quota_project, ..
                }
                | crate::auth::AdcCredentialSource::Impersonated { quota_project, .. },
        } => quota_project.as_deref(),
        _ => None,
    }
}

/// HTTP streaming with Vertex-specific URL and auth headers.
#[allow(clippy::too_many_arguments)]
async fn stream_vertex_http(
    http_client: reqwest::Client,
    provider_identity: String,
    auth_mode: VertexAuthMode,
    url: &str,
    requested_model: String,
    body: &serde_json::Value,
    cancel: CancellationToken,
    timeout: Option<std::time::Duration>,
    model_headers: Vec<(String, Option<String>)>,
    extra_headers: Vec<(String, String)>,
    callbacks: crate::callbacks::RequestCallbacks,
    snapshot: crate::stream::ProducerSnapshot,
    tx: &tokio::sync::mpsc::Sender<Result<AssistantStreamEvent, ProviderError>>,
) -> Result<(), ProviderError> {
    let mut route_headers = Vec::with_capacity(3);
    match &auth_mode {
        VertexAuthMode::Bearer {
            token,
            quota_project,
        } => {
            route_headers.push(("authorization".to_owned(), format!("Bearer {token}")));
            // The quota header is provider-managed when the ADC source owns
            // it; callers cannot override a provider-managed route header.
            if let Some(quota_project) = quota_project {
                route_headers.push(("x-goog-user-project".to_owned(), quota_project.clone()));
            }
        }
        VertexAuthMode::Express { api_key } => {
            route_headers.push(("x-goog-api-key".to_owned(), api_key.clone()));
        }
    }
    route_headers.push(("content-type".to_owned(), "application/json".to_owned()));
    let merged = ProviderHeaders::default().merge_model_request(
        &model_headers,
        &route_headers,
        &extra_headers,
    )?;
    // Header transform (task 20.51) on the transformable per-call set; the
    // protected auth/quota identity headers are stamped back after the
    // transform and never enter it.
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
        if status == reqwest::StatusCode::BAD_REQUEST {
            let body = crate::http::read_bounded_error_body(response, &cancel, timeout).await?;
            return Err(map_vertex_status(status, body.as_deref(), &headers));
        }
        return Err(map_vertex_status(status, None, &headers));
    }

    snapshot.received_response();
    let mut byte_stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut decoder = crate::text_decode::StreamTextDecoder::new();
    let mut mapper = GeminiMapper::new(
        &provider_identity,
        crate::model_info::WireApi::GoogleVertex,
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

            for data in drain_sse_payloads(&mut buffer) {
                if let Ok(raw) = serde_json::from_str::<serde_json::Value>(&data) {
                    callbacks.notify_raw_async(&raw).await?;
                }
                for parsed in ParsedEvent::from_data(&data) {
                    match parsed {
                        ParsedEvent::Valid(event) => {
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
                        ParsedEvent::Malformed => {
                            let err =
                                ProviderError::StreamError(ProviderErrorSummary::attested_static(
                                    "Vertex returned a malformed streaming frame",
                                ));
                            if !send_or_cancel(&cancel, tx, Err(mapper.failure(err))).await? {
                                return Ok(());
                            }
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

/// Default base URL for a location: `global` pins the shared domain, every
/// other location gets its regional subdomain.
fn vertex_base_has_api_version(base: &str) -> bool {
    let path = url::Url::parse(base)
        .map(|url| url.path().to_owned())
        .unwrap_or_else(|_| base.to_owned());
    path.split('/').any(|part| {
        let Some(version) = part.strip_prefix('v') else {
            return false;
        };
        let (digits, beta) = version
            .split_once("beta")
            .map_or((version, None), |(digits, beta)| (digits, Some(beta)));
        !digits.is_empty()
            && digits.bytes().all(|byte| byte.is_ascii_digit())
            && beta.is_none_or(|suffix| suffix.bytes().all(|byte| byte.is_ascii_digit()))
    })
}

fn default_vertex_base_url(location: &str) -> String {
    if location == "global" {
        "https://aiplatform.googleapis.com".to_owned()
    } else {
        format!("https://{location}-aiplatform.googleapis.com")
    }
}

fn map_vertex_status(
    status: reqwest::StatusCode,
    body: Option<&[u8]>,
    headers: &reqwest::header::HeaderMap,
) -> ProviderError {
    match status.as_u16() {
        401 | 403 => ProviderError::AuthFailed(ProviderErrorSummary::authentication_rejected()),
        429 => ProviderError::RateLimited {
            retry_after_ms: crate::retry::parse_retry_after(headers),
        },
        408 | 504 => ProviderError::Timeout,
        400 => {
            // Vertex/Gemini may return auth errors with HTTP 400 but code 401/403 in body
            if let Some(body) = body
                && let Ok(err_body) = serde_json::from_slice::<serde_json::Value>(body)
                && let Some(code) = err_body
                    .get("error")
                    .and_then(|e| e.get("code"))
                    .and_then(|c| c.as_i64())
                && (code == 401 || code == 403)
            {
                return ProviderError::AuthFailed(ProviderErrorSummary::authentication_rejected());
            }
            ProviderError::ProviderSide(ProviderErrorSummary::from_http_response(400))
        }
        code => ProviderError::ProviderSide(ProviderErrorSummary::from_http_response(code)),
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

// ---------------------------------------------------------------------------
// Public direct adapter (task 20.41)
// ---------------------------------------------------------------------------

/// Typed native options for one `google-vertex` dispatch (task 20.41, pi
/// `GoogleVertexOptions`).
///
/// The tool-choice and thinking intents share the Google shapes (no named
/// tool mode; `budget_tokens` keeps the source semantics — `-1` is the
/// dynamic default, `0` disables). `project` and `location` are Vertex-own:
/// they re-route the ADC URL and are inert under the Express API-key route.
/// The neutral/simple entry maps only the tool choice; thinking keeps the
/// neutral request intent (level policy + the vertex budget table).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GoogleVertexOptions {
    pub tool_choice: Option<GoogleToolChoice>,
    pub thinking: Option<GoogleThinkingConfig>,
    pub project: Option<String>,
    pub location: Option<String>,
    simple: bool,
}

impl GoogleVertexOptions {
    /// The neutral simple entry (task 20.41): only the neutral tool choice
    /// maps (auto/none); the neutral request intent drives thinking, and the
    /// routing identity stays provider-configured.
    pub fn from_simple(simple: &crate::models::SimpleOptions) -> Self {
        Self::native(
            None,
            simple.tool_choice.map(|choice| match choice {
                crate::models::SimpleToolChoice::Auto => GoogleToolChoice::Auto,
                crate::models::SimpleToolChoice::None => GoogleToolChoice::None_,
            }),
            None,
            None,
        )
        .marked_simple()
    }

    pub fn is_simple(&self) -> bool {
        self.simple
    }

    /// Construct a native (non-simple) options value: `thinking: None` keeps
    /// the neutral request intent on the wire; `project`/`location: None`
    /// keep the provider-configured route.
    pub fn native(
        thinking: Option<GoogleThinkingConfig>,
        tool_choice: Option<GoogleToolChoice>,
        project: Option<String>,
        location: Option<String>,
    ) -> Self {
        Self {
            thinking,
            tool_choice,
            project,
            location,
            simple: false,
        }
    }

    fn marked_simple(mut self) -> Self {
        self.simple = true;
        self
    }

    /// Project onto the shared body-engine options; the simple marker rides
    /// along so the engine keeps the neutral thinking intent.
    fn google(&self) -> GoogleGenerativeAiOptions {
        let mut options =
            GoogleGenerativeAiOptions::native(self.thinking.clone(), self.tool_choice.clone());
        if self.simple {
            options = options.marked_simple();
        }
        options
    }
}

/// The `google-vertex` direct Chat adapter (task 20.41, DEL-11).
///
/// Binds one complete Chat model record and serves the typed native
/// [`GoogleVertexOptions`] inventory through the four `Models` entries
/// (`register_chat_api` / native complete-stream / neutral simple
/// complete-stream), routing Express (API key) or ADC (Bearer plus
/// project/location) through the shared Vertex transport.
pub struct GoogleVertexChatApi {
    api_id: crate::models::ApiId,
    provider: Arc<VertexProvider>,
    model: ModelInfo,
}

impl std::fmt::Debug for GoogleVertexChatApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GoogleVertexChatApi")
            .field("api_id", &self.api_id)
            .finish_non_exhaustive()
    }
}

impl GoogleVertexChatApi {
    /// Bind a complete Chat record. Wrong-wire or invalid records fail
    /// before authentication or network activity.
    pub fn new(
        provider: Arc<VertexProvider>,
        model: ModelInfo,
    ) -> Result<Self, crate::provider::ProviderError> {
        validate_complete_vertex_record(&model)?;
        Ok(Self {
            api_id: crate::models::ApiId::new("google-vertex"),
            provider,
            model,
        })
    }

    fn configured(&self, model: ModelInfo, options: GoogleVertexOptions) -> VertexProvider {
        let mut provider = (*self.provider).clone();
        provider.inner = provider.inner.with_models(vec![model]);
        provider.models = provider.inner.models().to_vec();
        if let Some(project) = options.project.clone() {
            provider.project = project;
        }
        if let Some(location) = options.location.clone() {
            provider.location = location;
        }
        provider.inner = provider.inner.with_native_options(options.google());
        provider
    }
}

/// One complete, wire-valid Chat model record or a typed failure.
fn validate_complete_vertex_record(
    model: &ModelInfo,
) -> Result<(), crate::provider::ProviderError> {
    if model.wire_api != crate::model_info::WireApi::GoogleVertex
        || model.id.trim().is_empty()
        || model.validate().is_err()
    {
        return Err(crate::provider::ProviderError::Config(
            crate::provider::ProviderErrorSummary::attested_static(
                "google-vertex adapters require a valid complete Chat model record",
            ),
        ));
    }
    Ok(())
}

impl crate::models::ChatApi<GoogleVertexOptions> for GoogleVertexChatApi {
    fn validate_simple_options(
        &self,
        model: &ModelInfo,
        _options: &crate::models::SimpleOptions,
    ) -> Result<(), crate::provider::ProviderError> {
        validate_complete_vertex_record(model)
    }

    fn api_id(&self) -> &crate::models::ApiId {
        &self.api_id
    }

    fn provider_id(&self) -> &str {
        "vertex"
    }

    fn complete_record(&self) -> ModelInfo {
        self.model.clone()
    }

    fn simple_options(&self) -> GoogleVertexOptions {
        GoogleVertexOptions::from_simple(&crate::models::SimpleOptions::default())
    }

    fn simple_options_for(&self, options: &crate::models::SimpleOptions) -> GoogleVertexOptions {
        GoogleVertexOptions::from_simple(options)
    }

    fn session_resources(&self) -> Option<&dyn crate::codex_resources::SessionResourceOwner> {
        None
    }

    fn validate_native_request(
        &self,
        model: &ModelInfo,
        request: &Request,
        _options: &GoogleVertexOptions,
    ) -> Result<(), crate::provider::ProviderError> {
        validate_complete_vertex_record(model)?;
        crate::provider::validate_request_for_model(self.provider_id(), Some(model), request)
    }

    fn stream_prepared(
        &self,
        request: Request,
        options: GoogleVertexOptions,
        auth: crate::auth::ResolvedAuth,
    ) -> EventStream {
        self.stream_model_resource_prepared(self.model.clone(), request, options, auth, None)
    }

    fn stream_resource_prepared(
        &self,
        request: Request,
        options: GoogleVertexOptions,
        auth: crate::auth::ResolvedAuth,
        resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        self.stream_model_resource_prepared(self.model.clone(), request, options, auth, resources)
    }

    fn stream_model_resource_prepared(
        &self,
        model: ModelInfo,
        mut request: Request,
        options: GoogleVertexOptions,
        auth: crate::auth::ResolvedAuth,
        resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        if let Err(error) = self.validate_native_request(&model, &request, &options) {
            return Box::pin(futures_util::stream::once(async move { Err(error) }));
        }
        if !options.is_simple() {
            // Native thinking owns the wire projection; the neutral fields
            // are unused on this path (the simple entry clamps at
            // `Models::stream_simple` before dispatch).
            request.thinking = Default::default();
        }
        let provider = self.configured(model, options);
        // The Google wire holds no pooled session resources today; a
        // resource lease still dispatches through the ordinary producer.
        let _ = resources;
        provider.stream_prepared(request, auth)
    }
}
