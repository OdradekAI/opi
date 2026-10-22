//! AWS Bedrock provider.
//!
//! Implements the Bedrock Converse API with SigV4 or Bearer authentication,
//! event-stream parsing, and the explicit proxy-placeholder signing mode.
//! Local credential resolution lives in [`crate::auth::aws`]. No live AWS
//! calls.

pub mod event_stream;
pub mod retry;
pub mod sigv4;

use std::sync::Arc;

use futures_util::{StreamExt, stream};
use secrecy::{ExposeSecret, SecretString};
use tokio_util::sync::CancellationToken;

use crate::bedrock::sigv4::{AwsCredentials, sign_request};
use crate::http::HttpClient;
use crate::message::{AssistantContent, AssistantMessage, ToolCall};
use crate::model_info::WireApi;
use crate::provider::{
    AttemptContext, EventStream, ModelInfo, Provider, ProviderError, ProviderErrorSummary, Request,
};
use crate::provider_headers::ProviderHeaders;
use crate::registry::ModelCapabilities;
use crate::stream::{
    AssistantStreamEvent, CancelAwareReceiverStream, StopReason, Usage, send_or_cancel,
};

/// Model families supported by this provider.
const SUPPORTED_FAMILIES: &[&str] = &["anthropic", "meta", "mistral", "amazon", "cohere"];

/// Fixed nonsecret placeholder identity for the explicit
/// proxy-placeholder-signing mode. These well-known values carry no
/// AWS account authority; the mode exists to reach a caller-supplied local
/// proxy without reading any real credential source, and it keeps a real
/// SigV4 signature instead of claiming unsigned `NoAuth`.
pub const PROXY_PLACEHOLDER_ACCESS_KEY_ID: &str = "dummy-access-key";
/// Fixed nonsecret placeholder secret key (see
/// [`PROXY_PLACEHOLDER_ACCESS_KEY_ID`]).
pub const PROXY_PLACEHOLDER_SECRET_ACCESS_KEY: &str = "dummy-secret-key";

/// Fixed nonsecret placeholder SigV4 credentials for a frozen region.
///
/// Part of the explicit proxy-placeholder-signing mode (see
/// [`proxy_placeholder_auth`]).
pub fn proxy_placeholder_credentials(region: &str) -> AwsCredentials {
    AwsCredentials {
        access_key_id: SecretString::from(PROXY_PLACEHOLDER_ACCESS_KEY_ID),
        secret_access_key: SecretString::from(PROXY_PLACEHOLDER_SECRET_ACCESS_KEY),
        session_token: None,
        region: region.to_owned(),
        expires_at: None,
    }
}

/// Whether `base_url` is a standard AWS Bedrock runtime endpoint
/// (`bedrock-runtime[.-fips].<region>.amazonaws.com[.cn]`).
pub fn is_standard_bedrock_endpoint(base_url: &str) -> bool {
    let Ok(url) = url::Url::parse(base_url) else {
        return false;
    };
    let Some(host) = url.host_str() else {
        return false;
    };
    let host = host.to_ascii_lowercase();
    let Some(rest) = host.strip_prefix("bedrock-runtime") else {
        return false;
    };
    let rest = rest.strip_prefix("-fips").unwrap_or(rest);
    let Some(rest) = rest.strip_prefix('.') else {
        return false;
    };
    let Some((region, suffix)) = rest.split_once('.') else {
        return false;
    };
    !region.is_empty()
        && !region.contains('.')
        && (suffix == "amazonaws.com" || suffix == "amazonaws.com.cn")
}

/// Reject the explicit proxy-placeholder mode for standard AWS endpoints.
///
/// The mode is only lawful on a caller-supplied nonstandard endpoint: an
/// ordinary AWS endpoint must never receive placeholder-signed traffic.
fn validate_proxy_placeholder_endpoint(base_url: &str) -> Result<(), ProviderError> {
    if is_standard_bedrock_endpoint(base_url) {
        return Err(ProviderError::Config(ProviderErrorSummary::sanitized(
            "Bedrock ProxyPlaceholderSigning requires a caller-supplied nonstandard endpoint; \
             standard AWS endpoints reject this mode",
        )));
    }
    Ok(())
}

/// Build the explicit no-real-credential proxy-placeholder prepared
/// authentication for a caller-supplied nonstandard endpoint and frozen
/// region.
///
/// The result carries fixed nonsecret placeholder SigV4 credentials (not
/// unsigned `NoAuth` and no authenticated AWS identity), performs zero
/// environment/store/profile/process/metadata/bearer credential discovery,
/// and is rejected for standard AWS endpoints. This is a deliberate safety
/// difference from raw ambient `AWS_BEDROCK_SKIP_AUTH` overrides.
pub fn proxy_placeholder_auth(
    endpoint: &str,
    region: &str,
) -> Result<crate::auth::ResolvedAuth, ProviderError> {
    validate_proxy_placeholder_endpoint(endpoint)?;
    Ok(crate::auth::ResolvedAuth {
        scheme: crate::auth::AuthScheme::AwsSigV4(proxy_placeholder_credentials(region)),
        secret: SecretString::from(""),
        base_url: Some(endpoint.to_owned()),
        account_id: None,
        provenance: crate::auth::AuthProvenance {
            source: crate::auth::AuthProvenanceSource::AwsSigV4 {
                source: crate::auth::AwsCredentialSource::ProxyPlaceholder,
            },
            fallback: crate::auth::AuthFallback::NotAttempted,
        },
    })
}

/// Concrete AWS Bedrock provider using the Converse API.
pub struct BedrockProvider {
    base_url: Option<String>,
    client: Arc<HttpClient>,
    /// Dedicated HTTP/1.1 client for the explicit proxy-placeholder mode;
    /// placeholder-authenticated sends refuse to run without it.
    placeholder_client: Option<Arc<HttpClient>>,
    models: Vec<ModelInfo>,
    /// The frozen AWS retry configuration (task 20.20): standard/adaptive
    /// mode, the Smithy2026 constants branch, and the total attempt budget.
    /// Frozen at construction; never resampled from ambient state.
    retry: retry::SmithyRetryConfig,
}

impl BedrockProvider {
    /// Construct a credential-free Bedrock wire adapter.
    ///
    /// AWS credentials arrive only through per-call prepared authentication.
    /// Retry defaults to the pinned protocol default (standard mode, legacy
    /// constants, 3 total attempts).
    pub fn new(base_url: Option<String>, client: Arc<HttpClient>) -> Self {
        let models = model_catalog();
        Self {
            base_url,
            models,
            client,
            placeholder_client: None,
            retry: retry::SmithyRetryConfig::default(),
        }
    }

    /// Freeze an explicit AWS retry configuration
    /// (see [`SmithyRetryConfig`](retry::SmithyRetryConfig)). Construction
    /// never samples ambient process state: resolve scoped inputs through
    /// [`retry::resolve_smithy_retry_config`] first.
    pub fn with_retry_config(mut self, retry: retry::SmithyRetryConfig) -> Self {
        self.retry = retry;
        self
    }

    /// Attach the dedicated HTTP/1.1 client used only by the explicit
    /// proxy-placeholder-signing mode (see [`proxy_placeholder_auth`]).
    /// Build it with
    /// [`TlsOptions::http1_only`](crate::http::TlsOptions::http1_only):
    /// placeholder-authenticated sends fail typed when this client is
    /// absent, and never fall back to the ordinary (H2-capable) client.
    pub fn with_placeholder_client(mut self, client: Arc<HttpClient>) -> Self {
        self.placeholder_client = Some(client);
        self
    }

    /// Replace the HTTP client with a shared one (for proxy configuration
    /// and connection pooling).
    pub fn with_client(self, client: Arc<HttpClient>) -> Self {
        Self { client, ..self }
    }

    /// Access the shared HTTP client (for testing client reuse).
    pub fn http_client(&self) -> &Arc<HttpClient> {
        &self.client
    }

    /// Return supported model families.
    pub fn supported_model_families(&self) -> Vec<&str> {
        SUPPORTED_FAMILIES.to_vec()
    }

    /// Validate that a model ID belongs to a supported family.
    pub fn validate_model_id(&self, model_id: &str) -> Result<(), ProviderError> {
        let family = model_id.split('.').next().unwrap_or("");
        if SUPPORTED_FAMILIES.contains(&family) {
            Ok(())
        } else {
            Err(ProviderError::Config(ProviderErrorSummary::sanitized(
                format!(
                    "unsupported Bedrock model family '{family}' in model ID '{model_id}'; supported families: {}",
                    SUPPORTED_FAMILIES.join(", ")
                ),
            )))
        }
    }

    /// Build the Converse API request body.
    pub fn build_converse_body(
        &self,
        request: &Request,
    ) -> Result<serde_json::Value, crate::provider::ProviderError> {
        let model_id = request
            .model
            .split_once(':')
            .map(|(_, id)| id)
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

        // Canonical replay projection (task 20.7) before body building.
        let replay = crate::replay::project_replay(
            &crate::transcript::without_initial_system_message(transcript.messages()),
            &crate::replay::ReplayRoute {
                wire: crate::model_info::WireApi::BedrockConverseStream,
                provider: "bedrock".to_owned(),
                model: model_id.to_owned(),
                supports_images: crate::replay::supports_images_in_catalog(&self.models, model_id),
            },
            crate::replay::ReplayProjection::NativeOrCollapse,
        )?;
        let mut body = serde_json::json!({
            "messages": serialize_converse_messages(&replay.messages),
        });

        if !system_prompt.is_empty() {
            body["system"] = serde_json::json!([{"text": system_prompt}]);
        }

        let mut inference_config = serde_json::json!({});
        if let Some(max_tokens) = request.max_tokens {
            inference_config["maxTokens"] = serde_json::Value::Number(max_tokens.into());
        }
        if let Some(temp) = request.temperature
            && let Some(n) = serde_json::Number::from_f64(temp)
        {
            inference_config["temperature"] = serde_json::Value::Number(n);
        }
        if !request.stop_sequences.is_empty() {
            inference_config["stopSequences"] = serde_json::Value::Array(
                request
                    .stop_sequences
                    .iter()
                    .map(|s| serde_json::Value::String(s.clone()))
                    .collect(),
            );
        }
        body["inferenceConfig"] = inference_config;

        if !current_tools.is_empty() {
            body["toolConfig"] = serde_json::json!({
                "tools": current_tools.iter().map(|t| {
                    serde_json::json!({
                        "toolSpec": {
                            "name": t.name,
                            "description": t.description,
                            "inputSchema": {"json": t.input_schema}
                        }
                    })
                }).collect::<Vec<_>>()
            });
        }

        Ok(body)
    }

    /// Parse event-stream bytes and emit stream events from fixture data.
    pub fn stream_from_fixture(&self, data: &[u8], cancel: CancellationToken) -> EventStream {
        let mut buffer = data.to_vec();
        let frames = match event_stream::parse_frames(&mut buffer) {
            Ok(frames) => frames,
            Err(_) => {
                return Box::pin(stream::iter(vec![Err(ProviderError::StreamError(
                    ProviderErrorSummary::attested_static(
                        "Bedrock stream contains a malformed complete frame",
                    ),
                ))]));
            }
        };
        let mut mapper = BedrockMapper::new("");

        let mut stream_events: Vec<Result<AssistantStreamEvent, ProviderError>> = Vec::new();
        for frame in frames {
            // Payloads decode with replacement semantics (AIR-006): an invalid
            // byte becomes U+FFFD instead of silently dropping the
            // event text.
            let payload = String::from_utf8_lossy(&frame.payload);
            let payload_str = payload.as_ref();
            let parsed = parse_bedrock_event(&frame.event_type, payload_str);
            for event in parsed {
                match event {
                    Ok(bedrock_event) => {
                        stream_events.extend(mapper.process(bedrock_event));
                    }
                    Err(e) => {
                        stream_events.push(Err(mapper.failure(e)));
                    }
                }
            }
        }

        // Flush pending Done if metadata never arrived
        if let Some(pending) = mapper.flush_pending() {
            stream_events.push(Ok(pending));
        }

        let _cancel = cancel;
        Box::pin(stream::iter(stream_events))
    }

    /// Get the base URL for Bedrock runtime API.
    fn runtime_url(&self, region: &str) -> String {
        self.base_url
            .clone()
            .unwrap_or_else(|| format!("https://bedrock-runtime.{region}.amazonaws.com"))
    }
}

impl Provider for BedrockProvider {
    fn id(&self) -> &str {
        "bedrock"
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    /// The frozen AWS retry protocol default for ordinary prepared calls
    /// (standard/legacy/3-total unless the bundle froze another
    /// configuration).
    fn acquisition_policy_default(&self) -> crate::retry::AcquisitionPolicy {
        self.retry.acquisition_policy()
    }

    fn stream_attempt(
        &self,
        mut request: Request,
        auth: crate::auth::ResolvedAuth,
        attempt: AttemptContext,
    ) -> EventStream {
        // The source stamps its retry-observability headers on every send:
        // one stable invocation id per logical call and the per-attempt
        // `attempt=N; max=M` counter. They join the signed header set like
        // any other request header.
        request
            .extra_headers
            .push(("amz-sdk-invocation-id".to_owned(), attempt.invocation_id));
        request.extra_headers.push((
            "amz-sdk-request".to_owned(),
            format!("attempt={}; max={}", attempt.attempt, attempt.max_attempts),
        ));
        self.stream_prepared(request, auth)
    }

    fn stream_prepared(&self, request: Request, auth: crate::auth::ResolvedAuth) -> EventStream {
        let body = match self.build_converse_body(&request) {
            Ok(body) => body,
            Err(error) => return Box::pin(stream::once(async move { Err(error) })),
        };
        let cancel = request.cancel.clone();
        let model_id = request
            .model
            .split_once(':')
            .map(|(_, id)| id.to_string())
            .unwrap_or(request.model.clone());

        // Validate model family
        if let Err(e) = self.validate_model_id(&model_id) {
            return Box::pin(stream::iter(vec![Err(e)]));
        }

        // Validate image sources -- Bedrock Converse does not support URL-sourced images
        for msg in request.context.messages() {
            if let crate::message::Message::User(user_msg) = msg {
                for content in &user_msg.content {
                    if let crate::message::InputContent::Image {
                        source: crate::message::ImageSource::Url { .. },
                        ..
                    } = content
                    {
                        return Box::pin(stream::iter(vec![Err(
                            ProviderError::UnsupportedCapability(
                                ProviderErrorSummary::attested_static(
                                    "URL-sourced images are not supported by Bedrock. Use base64 or bytes.",
                                ),
                            ),
                        )]));
                    }
                }
            }
        }

        let placeholder_mode = matches!(
            auth.provenance.source,
            crate::auth::AuthProvenanceSource::AwsSigV4 {
                source: crate::auth::AwsCredentialSource::ProxyPlaceholder,
            }
        );
        if placeholder_mode {
            // The explicit no-real-credential mode is lawful only on a
            // caller-supplied nonstandard endpoint and only over the
            // dedicated HTTP/1.1 placeholder client; ordinary AWS endpoints
            // reject it and no H2-capable fallback exists.
            let endpoint = self.base_url.clone().or_else(|| auth.base_url.clone());
            let Some(endpoint) = endpoint else {
                return Box::pin(stream::iter(vec![Err(ProviderError::Config(
                    ProviderErrorSummary::attested_static(
                        "Bedrock proxy-placeholder signing requires a caller-supplied nonstandard endpoint",
                    ),
                ))]));
            };
            if let Err(error) = validate_proxy_placeholder_endpoint(&endpoint) {
                return Box::pin(stream::iter(vec![Err(error)]));
            }
        }

        let mut credentials = None;
        let mut bearer_token = None;
        match auth.scheme {
            crate::auth::AuthScheme::AwsSigV4(creds) => credentials = Some(creds),
            crate::auth::AuthScheme::Bearer => bearer_token = Some(auth.secret.clone()),
            _ => {
                return Box::pin(stream::iter(vec![Err(ProviderError::Config(
                    ProviderErrorSummary::attested_static(
                        "Bedrock requires prepared AWS SigV4 or Bearer authentication",
                    ),
                ))]));
            }
        }
        // Bearer and SigV4 never attach together: exactly one attachment
        // path runs below.
        debug_assert!(credentials.is_some() != bearer_token.is_some());
        let base_url = match (&credentials, &bearer_token) {
            (Some(credentials), _) => self.runtime_url(&credentials.region),
            (None, Some(_)) => match self.base_url.clone().or_else(|| auth.base_url.clone()) {
                Some(base_url) => base_url,
                // Bearer auth carries no SigV4 region, so the endpoint must
                // come from the provider configuration or the resolver that
                // selected the token.
                None => {
                    return Box::pin(stream::iter(vec![Err(ProviderError::Config(
                        ProviderErrorSummary::attested_static(
                            "Bedrock Bearer authentication requires a configured or resolved endpoint",
                        ),
                    ))]));
                }
            },
            (None, None) => unreachable!("one attachment path is always selected"),
        };
        let timeout = request.timeout;
        let extra_headers = request.extra_headers;

        let http_client = if placeholder_mode {
            match &self.placeholder_client {
                Some(client) => client.client().clone(),
                None => {
                    return Box::pin(stream::iter(vec![Err(ProviderError::Config(
                        ProviderErrorSummary::attested_static(
                            "Bedrock proxy-placeholder signing requires its dedicated HTTP/1.1 client",
                        ),
                    ))]));
                }
            }
        } else {
            self.client.client().clone()
        };

        let snapshot = crate::stream::ProducerSnapshot::default();
        let producer_snapshot = snapshot.clone();
        let (tx, rx) = tokio::sync::mpsc::channel(64);
        let producer_cancel = cancel.clone();

        let tx_closed = tx.clone();
        tokio::spawn(async move {
            let run = async {
                let result = Self::stream_http(
                    http_client,
                    credentials,
                    bearer_token,
                    base_url,
                    &model_id,
                    &body,
                    producer_cancel.clone(),
                    timeout,
                    extra_headers,
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

// ---------------------------------------------------------------------------
// HTTP streaming
// ---------------------------------------------------------------------------

impl BedrockProvider {
    #[allow(clippy::too_many_arguments)]
    async fn stream_http(
        client: reqwest::Client,
        credentials: Option<AwsCredentials>,
        bearer_token: Option<SecretString>,
        base_url: String,
        model_id: &str,
        body: &serde_json::Value,
        cancel: CancellationToken,
        timeout: Option<std::time::Duration>,
        extra_headers: Vec<(String, String)>,
        snapshot: crate::stream::ProducerSnapshot,
        tx: &tokio::sync::mpsc::Sender<Result<AssistantStreamEvent, ProviderError>>,
    ) -> Result<(), ProviderError> {
        let path = format!("/model/{model_id}/converse-stream");
        let payload = serde_json::to_vec(body).unwrap_or_default();
        let host = base_url
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .to_string();

        // Generate time strings
        let now = std::time::SystemTime::now();
        let duration = now
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = duration.as_secs();
        let date_time = chrono_format(secs);
        let date_stamp = date_string(secs);

        if let Some((name, _)) = extra_headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("x-amz-security-token"))
        {
            return Err(ProviderError::RequestFailed(
                ProviderErrorSummary::sanitized(format!(
                    "request header '{name}' is reserved for Bedrock SigV4 authentication"
                )),
            ));
        }

        let headers = match (&credentials, &bearer_token) {
            (Some(credentials), _) => {
                // SigV4 attachment: headers are frozen before signing and
                // every actual send re-signs only the timestamp with the same
                // credentials.
                let mut route_headers = vec![
                    ("host".to_owned(), host),
                    ("content-type".to_owned(), "application/json".to_owned()),
                    ("x-amz-date".to_owned(), date_time.clone()),
                    (
                        "x-amz-content-sha256".to_owned(),
                        sigv4::sha256_hex(&payload),
                    ),
                ];
                if let Some(token) = &credentials.session_token {
                    route_headers.push((
                        "x-amz-security-token".to_owned(),
                        token.expose_secret().to_owned(),
                    ));
                }
                let mut headers =
                    ProviderHeaders::default().merge_request(&route_headers, &extra_headers)?;
                let signing_headers: Vec<_> = headers
                    .iter()
                    .filter(|(name, _)| !name.eq_ignore_ascii_case("x-amz-security-token"))
                    .map(|(name, value)| (name.as_str(), value.as_str()))
                    .collect();
                let signed = sign_request(
                    "POST",
                    &path,
                    "",
                    &signing_headers,
                    &payload,
                    credentials,
                    "bedrock",
                    &date_stamp,
                    &date_time,
                );
                headers.push((
                    "authorization".to_owned(),
                    signed.authorization.expose_secret().to_owned(),
                ));
                headers
            }
            (None, Some(token)) => {
                // Bearer attachment (Bedrock API key): Authorization: Bearer
                // only. No SigV4 header is attached, so the two schemes are
                // mutually exclusive at the wire.
                let route_headers = vec![
                    ("host".to_owned(), host),
                    ("content-type".to_owned(), "application/json".to_owned()),
                ];
                let mut headers =
                    ProviderHeaders::default().merge_request(&route_headers, &extra_headers)?;
                headers.push((
                    "authorization".to_owned(),
                    format!("Bearer {}", token.expose_secret()),
                ));
                headers
            }
            (None, None) => unreachable!("one attachment path is always selected"),
        };
        let url = format!("{base_url}{path}");
        let mut req = client.post(&url);
        for (name, value) in headers {
            req = req.header(name, value);
        }
        if let Some(timeout) = timeout {
            req = req.timeout(timeout);
        }
        let req = req.body(payload);

        let response = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(ProviderError::Cancelled),
            response = req.send() => response,
        }
        .map_err(|error| {
            if error.is_timeout() {
                ProviderError::Timeout
            } else {
                ProviderError::Network(ProviderErrorSummary::sanitized(format!(
                    "Bedrock request failed: {error}"
                )))
            }
        })?;

        let status = response.status();
        if !status.is_success() {
            let headers = response.headers().clone();
            return Err(map_bedrock_status(status, &headers));
        }

        let mut byte_stream = response.bytes_stream();
        let mut buffer: Vec<u8> = Vec::new();
        let mut mapper = BedrockMapper::new(model_id);

        mapper.set_snapshot(snapshot);
        let result: Result<(), ProviderError> = async {
            loop {
                let chunk = tokio::select! {
                    biased;
                    _ = cancel.cancelled() => return Err(mapper.failure(ProviderError::Cancelled)),
                    chunk = byte_stream.next() => match chunk {
                        Some(c) => c,
                        None => break,
                    },
                };

                let chunk = chunk.map_err(|error: reqwest::Error| {
                    if error.is_timeout() {
                        ProviderError::Timeout
                    } else {
                        ProviderError::StreamError(ProviderErrorSummary::sanitized(
                            error.to_string(),
                        ))
                    }
                })?;
                buffer.extend_from_slice(&chunk);

                let frames = match event_stream::parse_frames(&mut buffer) {
                    Ok(frames) => frames,
                    Err(_) => {
                        let _ = send_or_cancel(
                            &cancel,
                            tx,
                            Err(mapper.failure(ProviderError::StreamError(
                                ProviderErrorSummary::attested_static(
                                    "Bedrock stream contains a malformed complete frame",
                                ),
                            ))),
                        )
                        .await?;
                        return Ok(());
                    }
                };
                for frame in frames {
                    // Payloads decode with replacement semantics (AIR-006): an invalid
                    // byte becomes U+FFFD instead of silently dropping the
                    // event text.
                    let payload = String::from_utf8_lossy(&frame.payload);
                    let payload_str = payload.as_ref();
                    for event in parse_bedrock_event(&frame.event_type, payload_str) {
                        match event {
                            Ok(bedrock_event) => {
                                for item in mapper.process(bedrock_event) {
                                    if !send_or_cancel(&cancel, tx, item).await? {
                                        return Ok(());
                                    }
                                }
                            }
                            Err(e) => {
                                if !send_or_cancel(&cancel, tx, Err(mapper.failure(e))).await? {
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            }

            if !buffer.is_empty() {
                let _ = send_or_cancel(
                    &cancel,
                    tx,
                    Err(mapper.failure(ProviderError::StreamError(
                        ProviderErrorSummary::attested_static(
                            "Bedrock stream ended with an incomplete frame",
                        ),
                    ))),
                )
                .await?;
                return Ok(());
            }

            if let Some(pending) = mapper.flush_pending() {
                let _ = send_or_cancel(&cancel, tx, Ok(pending)).await?;
            }

            if !mapper.saw_done {
                let _ = send_or_cancel(
                    &cancel,
                    tx,
                    Err(mapper.failure(ProviderError::StreamError(
                        ProviderErrorSummary::attested_static(
                            "Bedrock stream ended without terminal event",
                        ),
                    ))),
                )
                .await?;
            }

            Ok(())
        }
        .await;
        result.map_err(|cause| mapper.failure(cause))
    }
}

// ---------------------------------------------------------------------------
// Time formatting helpers
// ---------------------------------------------------------------------------

fn chrono_format(secs: u64) -> String {
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Calculate date from days since epoch
    let (year, month, day) = days_to_date(days);
    format!(
        "{:04}{:02}{:02}T{:02}{:02}{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

fn date_string(secs: u64) -> String {
    let cf = chrono_format(secs);
    cf[..8].to_string()
}

fn days_to_date(days: u64) -> (u64, u64, u64) {
    // Simplified date calculation from days since 1970-01-01
    let mut y = 1970;
    let mut remaining = days;

    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }

    let leap = is_leap(y);
    let month_days: [u64; 12] = if leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut m = 0;
    for (i, &md) in month_days.iter().enumerate() {
        if remaining < md {
            m = i;
            break;
        }
        remaining -= md;
    }

    (y, (m + 1) as u64, remaining + 1)
}

fn is_leap(y: u64) -> bool {
    (y.is_multiple_of(4) && !y.is_multiple_of(100)) || y.is_multiple_of(400)
}

// ---------------------------------------------------------------------------
// Bedrock event parsing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum BedrockEvent {
    MessageStart,
    ContentBlockStart {
        _index: usize,
        block_type: BedrockBlockType,
    },
    ContentBlockDelta {
        _index: usize,
        delta: BedrockDelta,
    },
    ContentBlockStop,
    MessageStop {
        stop_reason: String,
    },
    Metadata {
        usage: BedrockUsage,
    },
    Exception,
}

#[derive(Debug, Clone)]
enum BedrockBlockType {
    Text,
    ToolUse { tool_use_id: String, name: String },
}

#[derive(Debug, Clone)]
enum BedrockDelta {
    Text { text: String },
    ToolUse { input: String },
}

#[derive(Debug, Clone, Default)]
struct BedrockUsage {
    input_tokens: u32,
    output_tokens: u32,
    cache_read_tokens: u32,
    cache_write_tokens: u32,
}

fn parse_bedrock_event(
    event_type: &str,
    payload: &str,
) -> Vec<Result<BedrockEvent, ProviderError>> {
    match event_type {
        "messageStart" => vec![Ok(BedrockEvent::MessageStart)],
        "contentBlockStart" => {
            let parsed: serde_json::Value = match serde_json::from_str(payload) {
                Ok(v) => v,
                Err(e) => {
                    return vec![Err(ProviderError::StreamError(
                        ProviderErrorSummary::sanitized(format!("invalid contentBlockStart: {e}")),
                    ))];
                }
            };
            let index = parsed
                .get("contentBlockIndex")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;
            let start = parsed
                .get("start")
                .cloned()
                .unwrap_or(serde_json::json!({}));

            let block_type = if start.get("toolUse").is_some() {
                let tu = &start["toolUse"];
                BedrockBlockType::ToolUse {
                    tool_use_id: tu
                        .get("toolUseId")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    name: tu
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                }
            } else {
                BedrockBlockType::Text
            };
            vec![Ok(BedrockEvent::ContentBlockStart {
                _index: index,
                block_type,
            })]
        }
        "contentBlockDelta" => {
            let parsed: serde_json::Value = match serde_json::from_str(payload) {
                Ok(v) => v,
                Err(e) => {
                    return vec![Err(ProviderError::StreamError(
                        ProviderErrorSummary::sanitized(format!("invalid contentBlockDelta: {e}")),
                    ))];
                }
            };
            let index = parsed
                .get("contentBlockIndex")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;
            let delta = parsed
                .get("delta")
                .cloned()
                .unwrap_or(serde_json::json!({}));

            let bedrock_delta = if delta.get("text").is_some() {
                BedrockDelta::Text {
                    text: delta["text"].as_str().unwrap_or("").to_string(),
                }
            } else if delta.get("toolUse").is_some() {
                BedrockDelta::ToolUse {
                    input: delta["toolUse"]
                        .get("input")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                }
            } else {
                BedrockDelta::Text {
                    text: String::new(),
                }
            };
            vec![Ok(BedrockEvent::ContentBlockDelta {
                _index: index,
                delta: bedrock_delta,
            })]
        }
        "contentBlockStop" => vec![Ok(BedrockEvent::ContentBlockStop)],
        "messageStop" => {
            let parsed: serde_json::Value = serde_json::from_str(payload).unwrap_or_default();
            let stop_reason = parsed
                .get("stopReason")
                .and_then(|v| v.as_str())
                .unwrap_or("end_turn")
                .to_string();
            vec![Ok(BedrockEvent::MessageStop { stop_reason })]
        }
        "metadata" => {
            let parsed: serde_json::Value = serde_json::from_str(payload).unwrap_or_default();
            let usage = &parsed["usage"];
            vec![Ok(BedrockEvent::Metadata {
                usage: BedrockUsage {
                    input_tokens: usage
                        .get("inputTokens")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32,
                    output_tokens: usage
                        .get("outputTokens")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32,
                    cache_read_tokens: usage
                        .get("cacheReadInputTokens")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32,
                    cache_write_tokens: usage
                        .get("cacheCreationInputTokens")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32,
                },
            })]
        }
        "exception" => vec![Ok(BedrockEvent::Exception)],
        _ => Vec::new(),
    }
}

fn map_bedrock_stop_reason(raw: &str) -> StopReason {
    match raw {
        "end_turn" | "stop_sequence" => StopReason::Stop,
        "max_tokens" => StopReason::Length,
        "tool_use" => StopReason::ToolUse,
        _ => StopReason::Error,
    }
}

// ---------------------------------------------------------------------------
// BedrockMapper: BedrockEvent ->AssistantStreamEvent
// ---------------------------------------------------------------------------

struct BedrockMapper {
    snapshot: crate::stream::ProducerSnapshot,
    partial: AssistantMessage,
    blocks: Vec<BlockState>,
    saw_done: bool,
    usage: BedrockUsage,
    usage_reported: bool,
    /// Pending Done event held until Metadata arrives (Bedrock sends metadata after messageStop).
    pending_done: Option<AssistantStreamEvent>,
}

enum BlockState {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        partial_input: String,
    },
}

impl BedrockMapper {
    pub(crate) fn set_snapshot(&mut self, snapshot: crate::stream::ProducerSnapshot) {
        self.snapshot = snapshot;
        self.snapshot.record(&self.partial);
    }

    fn process(&mut self, event: BedrockEvent) -> Vec<Result<AssistantStreamEvent, ProviderError>> {
        let events = self.process_inner(event);
        self.snapshot.record(&self.partial);
        events
    }

    pub(crate) fn failure(&self, cause: ProviderError) -> ProviderError {
        cause.with_partial(self.partial.clone())
    }

    fn new(model: &str) -> Self {
        Self {
            snapshot: crate::stream::ProducerSnapshot::default(),
            partial: empty_assistant_message(model),
            blocks: Vec::new(),
            saw_done: false,
            usage: BedrockUsage::default(),
            usage_reported: false,
            pending_done: None,
        }
    }

    /// Flush any pending Done event (when stream ends without metadata).
    pub fn flush_pending(&mut self) -> Option<AssistantStreamEvent> {
        if self.usage_reported
            && let Some(AssistantStreamEvent::Done { message, .. }) = &mut self.pending_done
        {
            message.usage = Usage::reported(
                self.usage.input_tokens,
                self.usage.output_tokens,
                self.usage.cache_read_tokens,
                self.usage.cache_write_tokens,
                None, // cache_write_1h_tokens
                None, // reasoning_tokens
            );
        }
        self.pending_done.take()
    }

    fn process_inner(
        &mut self,
        event: BedrockEvent,
    ) -> Vec<Result<AssistantStreamEvent, ProviderError>> {
        // Allow Metadata through even after saw_done
        if self.saw_done && !matches!(event, BedrockEvent::Metadata { .. }) {
            return Vec::new();
        }

        match event {
            BedrockEvent::MessageStart => {
                vec![Ok(AssistantStreamEvent::Start {
                    partial: self.partial.clone(),
                })]
            }
            BedrockEvent::ContentBlockStart {
                _index: _,
                block_type,
            } => {
                let content_index = self.blocks.len();
                match block_type {
                    BedrockBlockType::Text => {
                        self.blocks.push(BlockState::Text {
                            text: String::new(),
                        });
                        self.partial.content.push(AssistantContent::Text {
                            text: String::new(),
                        });
                        vec![Ok(AssistantStreamEvent::TextStart {
                            content_index,
                            partial: self.partial.clone(),
                        })]
                    }
                    BedrockBlockType::ToolUse { tool_use_id, name } => {
                        self.blocks.push(BlockState::ToolUse {
                            id: tool_use_id.clone(),
                            name: name.clone(),
                            partial_input: String::new(),
                        });
                        self.partial.content.push(AssistantContent::ToolCall {
                            tool_call: ToolCall {
                                id: tool_use_id,
                                name,
                                arguments: String::new(),
                                arguments_error: None,
                            },
                        });
                        vec![Ok(AssistantStreamEvent::ToolCallStart {
                            content_index,
                            partial: self.partial.clone(),
                        })]
                    }
                }
            }
            BedrockEvent::ContentBlockDelta { _index: _, delta } => {
                let content_index = self.blocks.len().saturating_sub(1);
                match delta {
                    BedrockDelta::Text { text } => {
                        if let Some(BlockState::Text { text: acc }) = self.blocks.last_mut() {
                            acc.push_str(&text);
                        }
                        if let Some(AssistantContent::Text { text: acc }) =
                            self.partial.content.last_mut()
                        {
                            acc.push_str(&text);
                        }
                        vec![Ok(AssistantStreamEvent::TextDelta {
                            content_index,
                            delta: text,
                            partial: self.partial.clone(),
                        })]
                    }
                    BedrockDelta::ToolUse { input } => {
                        if let Some(BlockState::ToolUse {
                            partial_input: acc, ..
                        }) = self.blocks.last_mut()
                        {
                            acc.push_str(&input);
                        }
                        if let Some(AssistantContent::ToolCall { tool_call }) =
                            self.partial.content.last_mut()
                            && let Some(BlockState::ToolUse { partial_input, .. }) =
                                self.blocks.last()
                        {
                            tool_call.arguments = partial_input.clone();
                        }
                        vec![Ok(AssistantStreamEvent::ToolCallDelta {
                            content_index,
                            delta: input,
                            partial: self.partial.clone(),
                        })]
                    }
                }
            }
            BedrockEvent::ContentBlockStop => {
                let content_index = self.blocks.len().saturating_sub(1);
                match self.blocks.last() {
                    Some(BlockState::Text { text }) => {
                        vec![Ok(AssistantStreamEvent::TextEnd {
                            content_index,
                            content: text.clone(),
                            partial: self.partial.clone(),
                        })]
                    }
                    Some(BlockState::ToolUse {
                        id,
                        name,
                        partial_input,
                    }) => {
                        // Final textual arguments are parsed once at block
                        // end (AIR-048).
                        let (arguments, arguments_error) =
                            ToolCall::finalize_text_arguments(partial_input.clone());
                        let tool_call = ToolCall {
                            id: id.clone(),
                            name: name.clone(),
                            arguments: arguments.clone(),
                            arguments_error,
                        };
                        if let Some(AssistantContent::ToolCall { tool_call: tc }) =
                            self.partial.content.last_mut()
                        {
                            tc.arguments = arguments;
                            tc.arguments_error = tool_call.arguments_error.clone();
                        }
                        vec![Ok(AssistantStreamEvent::ToolCallEnd {
                            content_index,
                            tool_call,
                            partial: self.partial.clone(),
                        })]
                    }
                    None => Vec::new(),
                }
            }
            BedrockEvent::MessageStop { stop_reason } => {
                self.partial.stop_reason = map_bedrock_stop_reason(&stop_reason);
                self.saw_done = true;
                // Defer Done event  - metadata may follow with final usage
                self.pending_done = Some(AssistantStreamEvent::Done {
                    reason: self.partial.stop_reason,
                    message: self.partial.clone(),
                });
                Vec::new()
            }
            BedrockEvent::Metadata { usage } => {
                self.usage = usage;
                self.usage_reported = true;
                self.partial.usage = Usage::reported(
                    self.usage.input_tokens,
                    self.usage.output_tokens,
                    self.usage.cache_read_tokens,
                    self.usage.cache_write_tokens,
                    None,
                    None,
                );
                // Flush pending Done with updated usage
                if let Some(AssistantStreamEvent::Done { message, .. }) = &mut self.pending_done {
                    message.usage = Usage::reported(
                        self.usage.input_tokens,
                        self.usage.output_tokens,
                        self.usage.cache_read_tokens,
                        self.usage.cache_write_tokens,
                        None, // cache_write_1h_tokens
                        None, // reasoning_tokens
                    );
                }
                self.pending_done.take().into_iter().map(Ok).collect()
            }
            BedrockEvent::Exception => {
                // Preserve state that did not produce a public event (usage,
                // opaque content or signatures), alongside the typed cause.
                self.saw_done = true;
                vec![Err(self.failure(ProviderError::StreamError(
                    ProviderErrorSummary::attested_static("Bedrock returned a streaming error"),
                )))]
            }
        }
    }
}

fn empty_assistant_message(model: &str) -> AssistantMessage {
    AssistantMessage {
        content: Vec::new(),
        api: crate::ApiKind::Anthropic, // Bedrock uses Anthropic-style content
        provider: "bedrock".into(),
        model: model.to_owned(),
        response_model: None,
        wire: Some(crate::model_info::WireApi::BedrockConverseStream),
        response_id: None,
        usage: Usage::default(),
        stop_reason: StopReason::Stop,
        error_message: None,
        timestamp_ms: crate::time::now_ms(),
    }
}

// ---------------------------------------------------------------------------
// Converse API message serialization
// ---------------------------------------------------------------------------

fn serialize_converse_messages(messages: &[crate::message::Message]) -> serde_json::Value {
    serde_json::Value::Array(
        messages
            .iter()
            .map(|msg| match msg {
                crate::message::Message::System(_) => {
                    // Unreachable after collapse + without_initial_system_message.
                    unreachable!("system messages are projected out of the body list")
                }
                crate::message::Message::User(u) => {
                    let content: Vec<serde_json::Value> = u
                        .content
                        .iter()
                        .map(|c| match c {
                            crate::message::InputContent::Text { text } => {
                                serde_json::json!({"text": text})
                            }
                            crate::message::InputContent::Image { source, media_type } => {
                                let data = match source {
                                    crate::message::ImageSource::Base64 { data } => data.clone(),
                                    crate::message::ImageSource::Bytes { data } => base64::Engine::encode(
                                        &base64::engine::general_purpose::STANDARD,
                                        data,
                                    ),
                                    crate::message::ImageSource::Url { .. } => String::new(),
                                };
                                serde_json::json!({
                                    "image": {
                                        "format": media_type.as_str().split('/').next_back().unwrap_or("png"),
                                        "source": {"bytes": data}
                                    }
                                })
                            }
                        })
                        .collect();
                    serde_json::json!({"role": "user", "content": content})
                }
                crate::message::Message::Assistant(a) => {
                    let content: Vec<serde_json::Value> = a
                        .content
                        .iter()
                        .filter_map(|c| Some(match c {
                            AssistantContent::Text { text } => {
                                serde_json::json!({"text": text})
                            }
                            AssistantContent::ToolCall { tool_call } => {
                                let input: serde_json::Value =
                                    serde_json::from_str(&tool_call.arguments)
                                        .ok()
                                        .filter(|v: &serde_json::Value| v.is_object())
                                        .unwrap_or(serde_json::json!({}));
                                serde_json::json!({
                                    "toolUse": {
                                        "toolUseId": tool_call.id,
                                        "name": tool_call.name,
                                        "input": input,
                                    }
                                })
                            }
                            AssistantContent::Thinking { thinking, .. } => {
                                serde_json::json!({"text": thinking})
                            }
                            AssistantContent::RedactedThinking { .. } => return None,
                        }))
                        .collect();
                    serde_json::json!({"role": "assistant", "content": content})
                }
                crate::message::Message::ToolResult(t) => {
                    let content: Vec<serde_json::Value> = t
                        .content
                        .iter()
                        .map(|c| match c {
                            crate::message::OutputContent::Text { text } => {
                                serde_json::json!({"text": text})
                            }
                            crate::message::OutputContent::Image { media_type, .. } => {
                                serde_json::json!({"text": format!("[image: {}]", media_type.as_str())})
                            }
                        })
                        .collect();
                    serde_json::json!({
                        "role": "user",
                        "content": vec![serde_json::json!({
                            "toolResult": {
                                "toolUseId": t.tool_call_id,
                                "content": content,
                                "status": if t.is_error { "error" } else { "success" },
                            }
                        })]
                    })
                }
            })
            .filter(|message| message["role"] != "assistant" || message["content"].as_array().is_none_or(|content| !content.is_empty()))
            .collect(),
    )
}

// ---------------------------------------------------------------------------
// Error mapping
// ---------------------------------------------------------------------------

/// Map an HTTP status + headers to a bodyless, credential-safe error.
///
/// Auth rejections (401/403) keep their typed authentication identity; every
/// other failure carries the structured service fault (status plus the
/// `x-amzn-errortype` wire name) so the acquisition owner can classify
/// retryability through the source's exact trait/name rules (task 20.20):
/// 429/throttling names are THROTTLING, the transient names/statuses and the
/// modeled `$retryable` trait (ModelNotReady) are TRANSIENT, bare 5xx are
/// SERVER_ERROR (never blanket-retried), and client faults like
/// ModelTimeout (408, no trait) are CLIENT_ERROR.
pub fn map_bedrock_status(
    status: reqwest::StatusCode,
    headers: &reqwest::header::HeaderMap,
) -> ProviderError {
    let code = status.as_u16();
    if code == 401 || code == 403 {
        return ProviderError::AuthFailed(ProviderErrorSummary::authentication_rejected());
    }
    let error_name = headers
        .get("x-amzn-errortype")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.split(':').next().unwrap_or(value).trim().to_owned())
        .filter(|name| !name.is_empty());
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();
    let retry_after_ms = retry::parse_smithy_retry_after(headers, now);
    let detail = match &error_name {
        Some(name) => ProviderErrorSummary::sanitized(format!("Bedrock {name} (HTTP {code})")),
        None => ProviderErrorSummary::sanitized(format!("Bedrock HTTP {code}")),
    };
    ProviderError::ServiceFault {
        status: code,
        error_name,
        retry_after_ms,
        detail,
    }
}

// ---------------------------------------------------------------------------
// Secret redaction
// ---------------------------------------------------------------------------

/// Redact AWS credentials for safe display.
pub fn redact_credentials(_access_key_id: &str, _secret_key: &str) -> String {
    "***".to_string()
}

// ---------------------------------------------------------------------------
// Default models
// ---------------------------------------------------------------------------

/// Built-in Bedrock model metadata without credentials or HTTP construction.
pub fn model_catalog() -> Vec<ModelInfo> {
    vec![
        ModelInfo::new(
            "anthropic.claude-sonnet-4-20250514-v2:0",
            "Claude Sonnet 4 (Bedrock)",
            WireApi::BedrockConverseStream,
            ModelCapabilities::new(200000, 8192)
                .with_images(true)
                .with_streaming(true)
                .with_thinking(true),
        ),
        ModelInfo::new(
            "anthropic.claude-opus-4-20250514-v1:0",
            "Claude Opus 4 (Bedrock)",
            WireApi::BedrockConverseStream,
            ModelCapabilities::new(200000, 8192)
                .with_images(true)
                .with_streaming(true)
                .with_thinking(true),
        ),
        ModelInfo::new(
            "anthropic.claude-haiku-4-5-20250514-v1:0",
            "Claude Haiku 4.5 (Bedrock)",
            WireApi::BedrockConverseStream,
            ModelCapabilities::new(200000, 8192)
                .with_images(true)
                .with_streaming(true)
                .with_thinking(true),
        ),
    ]
}
