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

/// Model families supported by this provider. `openai` exists for the
/// Bedrock-hosted GPT-OSS/GPT models (pi's effort engine carries dedicated
/// `gpt-oss`/`gpt-` arms keyed on the model id).
const SUPPORTED_FAMILIES: &[&str] = &["anthropic", "meta", "mistral", "amazon", "cohere", "openai"];

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
#[derive(Clone)]
pub struct BedrockProvider {
    base_url: Option<String>,
    client: Arc<HttpClient>,
    /// Dedicated HTTP/1.1 client for the explicit proxy-placeholder mode;
    /// placeholder-authenticated sends refuse to run without it.
    placeholder_client: Option<Arc<HttpClient>>,
    models: Vec<ModelInfo>,
    /// Typed native options for one direct/native dispatch (task 20.42);
    /// `None` keeps the plain provider path.
    native_options: Option<BedrockOptions>,
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
            native_options: None,
            retry: retry::SmithyRetryConfig::default(),
        }
    }

    /// Replace the model catalog (direct adapters bind their complete
    /// record; task 20.42).
    pub fn with_models(mut self, models: Vec<ModelInfo>) -> Self {
        self.models = models;
        self
    }

    /// Bind typed native options for one dispatch (task 20.42); the
    /// direct adapter clones this provider per call.
    pub fn with_native_options(mut self, options: BedrockOptions) -> Self {
        self.native_options = Some(options);
        self
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
        self.build_converse_body_inner(request, None)
    }

    /// Body assembly with the dispatch region (task 20.42): GovCloud
    /// regions omit Claude `thinking.display`/binding controls until the
    /// GovCloud Converse schema catches up (pi pins the same omissions).
    fn build_converse_body_inner(
        &self,
        request: &Request,
        gov_region: Option<&str>,
    ) -> Result<serde_json::Value, crate::provider::ProviderError> {
        let model_id = request
            .model
            .strip_prefix("bedrock:")
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
        let model = self.models.iter().find(|model| model.id == model_id);
        // Task 20.11/20.42 (AIR-040, fail-closed): a require intent on a
        // wire without strict projection fails typed before dispatch. The
        // compat flag carries pi `supportsStrictMode` per record.
        let supports_strict = model
            .and_then(|model| model.compat.as_bedrock_converse_stream())
            .is_some_and(|compat| compat.supports_strict_tools);
        crate::constrained_sampling::assert_supported_constraints(&current_tools, supports_strict)
            .map_err(|error| {
                crate::provider::ProviderError::Config(
                    crate::provider::ProviderErrorSummary::sanitized(error.detail()),
                )
            })?;

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
        let is_claude = model
            .map(|model| is_anthropic_claude_model(&model.id, &model.display_name))
            .unwrap_or_else(|| model_id.contains("anthropic.claude"));
        let mut body = serde_json::json!({
            "messages": serialize_converse_messages(&replay.messages, is_claude),
        });

        // Cache points (task 20.42, pi `supportsPromptCaching` +
        // `resolveCacheRetention`): the default retention keeps pi's
        // "short" default, so cache points ride supported Claude models
        // unless the request disables them.
        let retention = request.cache_retention;
        let caches = retention != crate::provider::CacheRetention::Disabled
            && model.is_some_and(|model| supports_prompt_caching(&model.id, &model.display_name));
        if !system_prompt.is_empty() {
            let mut blocks = vec![serde_json::json!({"text": system_prompt})];
            if caches {
                blocks.push(cache_point_block(retention));
            }
            body["system"] = serde_json::Value::Array(blocks);
        }

        if caches
            && let Some(last) = body["messages"]
                .as_array_mut()
                .and_then(|messages| messages.last_mut())
            && last.get("role").and_then(|role| role.as_str()) == Some("user")
            && let Some(content) = last.get_mut("content").and_then(|c| c.as_array_mut())
        {
            content.push(cache_point_block(retention));
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
            let tools = current_tools
                .iter()
                .map(|tool| {
                    let strict = crate::constrained_sampling::resolve_json_schema_strict_sampling(
                        tool,
                        supports_strict,
                        &crate::constrained_sampling::NoRejectedKeywords,
                    )
                    .map_err(|error| {
                        crate::provider::ProviderError::Config(
                            crate::provider::ProviderErrorSummary::sanitized(error.detail()),
                        )
                    })?;
                    let schema = if strict == Some(true) {
                        crate::constrained_sampling::make_strict_json_schema(
                            &tool.input_schema,
                            &crate::constrained_sampling::NoRejectedKeywords,
                        )
                        .expect("resolve succeeded so the conversion succeeds")
                    } else {
                        tool.input_schema.clone()
                    };
                    let mut spec = serde_json::json!({
                        "toolSpec": {
                            "name": tool.name,
                            "description": tool.description,
                            "inputSchema": {"json": schema},
                        }
                    });
                    if strict == Some(true) {
                        spec["toolSpec"]["strict"] = serde_json::Value::Bool(true);
                    }
                    Ok(spec)
                })
                .collect::<Result<Vec<_>, crate::provider::ProviderError>>()?;
            // Pi drops the whole toolConfig for `none`: the tools ride no
            // other member, so the request goes out without them.
            let tool_choice = self
                .native_options
                .as_ref()
                .and_then(|o| o.tool_choice.clone());
            if tool_choice != Some(BedrockToolChoice::None_) {
                let mut config = serde_json::json!({ "tools": tools });
                if let Some(choice) = tool_choice {
                    config["toolChoice"] = match choice {
                        BedrockToolChoice::Auto => serde_json::json!({"auto": {}}),
                        BedrockToolChoice::Any => serde_json::json!({"any": {}}),
                        BedrockToolChoice::None_ => unreachable!("excluded above"),
                        BedrockToolChoice::Named(name) => {
                            serde_json::json!({"tool": {"name": name}})
                        }
                    };
                }
                body["toolConfig"] = config;
            }
        }

        // Reasoning fields (task 20.42, pi `buildAdditionalModelRequestFields`):
        // the native `reasoning` level owns the projection; the neutral
        // entry consumes the compiled request thinking (budget table plus
        // answer-room clamp in `compile_prepared_request`).
        let model_thinking = model.is_some_and(|model| model.capabilities.supports_thinking);
        let native = self.native_options.as_ref();
        let simple = native.is_none_or(|options| options.simple);
        let reasoning = if simple {
            (request.thinking.enabled && model_thinking).then_some(request.thinking.level)
        } else {
            native
                .and_then(|options| options.reasoning)
                .filter(|_| model_thinking)
        };
        if let Some(level) = reasoning
            && let Some(fields) = build_additional_model_request_fields(
                model,
                level,
                native,
                simple,
                &request.thinking,
                gov_region,
            )
        {
            body["additionalModelRequestFields"] = fields;
        }

        // requestMetadata rides verbatim (annex row 51): the consumption
        // path forwards pairs without local preflight validation.
        if let Some(metadata) = native
            .filter(|_| !simple)
            .map(|options| &options.request_metadata)
            && !metadata.is_empty()
        {
            body["requestMetadata"] = serde_json::json!(metadata);
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

    fn stream_prepared(
        &self,
        mut request: Request,
        auth: crate::auth::ResolvedAuth,
    ) -> EventStream {
        request.callbacks = request.callbacks.ensure_call_scope();
        let callbacks = request.callbacks.clone();
        let cancel = request.cancel.clone();
        let model_id = request
            .model
            .strip_prefix("bedrock:")
            .map(str::to_owned)
            .unwrap_or_else(|| request.model.clone());

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
        // Body assembly needs the dispatch region (task 20.42): GovCloud
        // regions omit Claude thinking display/binding fields.
        let gov_region = credentials.as_ref().map(|c| c.region.as_str());
        let body = match self.build_converse_body_inner(&request, gov_region) {
            Ok(body) => body,
            Err(error) => return Box::pin(stream::once(async move { Err(error) })),
        };
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
        // Explicit frozen telemetry context (task 20.42, 20.54/AIR-055): the
        // producer task runs under the captured parent/Dispatch when
        // provided; omission observes nothing.
        let telemetry = request.telemetry_context.clone();
        let producer = async move {
            let run = async {
                let result = async {
                    let body = callbacks
                        .apply_payload_async(crate::callbacks::ORDINARY_PAYLOAD_VARIANT, body)
                        .await?;
                    Self::stream_http(
                        http_client,
                        credentials,
                        bearer_token,
                        base_url,
                        &model_id,
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
        callbacks: crate::callbacks::RequestCallbacks,
        snapshot: crate::stream::ProducerSnapshot,
        tx: &tokio::sync::mpsc::Sender<Result<AssistantStreamEvent, ProviderError>>,
    ) -> Result<(), ProviderError> {
        // Header transform (task 20.51) on the per-call set before signing;
        // the output is revalidated so no signing/identity name can enter.
        // Retry-observability headers belong to the attempt, not the frozen
        // functional transform. Preserve the freshly stamped send counter.
        let (attempt_headers, transformable): (Vec<_>, Vec<_>) = extra_headers
            .into_iter()
            .partition(|(name, _)| name.to_ascii_lowercase().starts_with("amz-sdk-"));
        let mut extra_headers = callbacks
            .apply_header_transform_async(&transformable)
            .await?;
        extra_headers.extend(attempt_headers);
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
        if !status.is_success() {
            let headers = response.headers().clone();
            return Err(map_bedrock_status(status, &headers));
        }
        // Response callback (V-36): fired exactly once on the successfully
        // acquired response; rejected dispatches never masquerade.
        snapshot.received_response();

        let mut byte_stream = response.bytes_stream();
        let mut buffer: Vec<u8> = Vec::new();
        let mut mapper = BedrockMapper::new(model_id);

        mapper.set_snapshot(snapshot);
        callbacks
            .notify_response_async(status.as_u16(), &observed_headers)
            .await
            .map_err(|error| mapper.failure(error))?;
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
                    // Raw observer (task 20.51): the parsed smithy frame as
                    // `{type, ...payload}` before normalization; failure
                    // preserves the partial.
                    let mut raw = serde_json::json!({"type": frame.event_type});
                    if let Ok(object) = serde_json::from_str::<
                        serde_json::Map<String, serde_json::Value>,
                    >(payload_str)
                    {
                        for (key, value) in object {
                            raw[key] = value;
                        }
                    }
                    if !(mapper.saw_done && mapper.pending_done.is_none())
                        && let Err(error) = callbacks.notify_raw_async(&raw).await
                    {
                        return Err(mapper.failure(error));
                    }
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
    ToolUse {
        tool_use_id: String,
        name: String,
    },
    /// `contentBlockStart.start.reasoningContent` (task 20.42): streaming
    /// thinking blocks with signature/redacted payload handling.
    Reasoning,
}

#[derive(Debug, Clone)]
enum BedrockDelta {
    Text {
        text: String,
    },
    ToolUse {
        input: String,
    },
    /// `delta.reasoningContent` (task 20.42): text, signature or opaque
    /// redacted bytes arrive as separate deltas; signatures follow text.
    Reasoning {
        text: Option<String>,
        signature: Option<String>,
        redacted_content: Option<String>,
    },
}

#[derive(Debug, Clone, Default)]
struct BedrockUsage {
    input_tokens: u32,
    output_tokens: u32,
    cache_read_tokens: u32,
    cache_write_tokens: u32,
    /// Subset of `cache_write_tokens` on one-hour TTL cache details
    /// (task 20.42, pi `cacheDetails` reduce).
    cache_write_1h_tokens: Option<u64>,
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
            } else if start.get("reasoningContent").is_some() {
                BedrockBlockType::Reasoning
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
            } else if delta.get("reasoningContent").is_some() {
                let reasoning = &delta["reasoningContent"];
                BedrockDelta::Reasoning {
                    text: reasoning
                        .get("text")
                        .and_then(|v| v.as_str())
                        .map(str::to_owned),
                    signature: reasoning
                        .get("signature")
                        .and_then(|v| v.as_str())
                        .map(str::to_owned),
                    redacted_content: reasoning
                        .get("redactedContent")
                        .and_then(|v| v.as_str())
                        .map(str::to_owned),
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
                .unwrap_or("")
                .to_string();
            vec![Ok(BedrockEvent::MessageStop { stop_reason })]
        }
        "metadata" => {
            let parsed: serde_json::Value = serde_json::from_str(payload).unwrap_or_default();
            let usage = &parsed["usage"];
            // One-hour cache details sum into the 1h subset of
            // cacheCreationInputTokens (task 20.42).
            let cache_write_1h = usage
                .get("cacheDetails")
                .and_then(|v| v.as_array())
                .map(|details| {
                    details
                        .iter()
                        .filter(|detail| detail.get("ttl").and_then(|v| v.as_str()) == Some("1h"))
                        .map(|detail| {
                            detail
                                .get("inputTokens")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0)
                        })
                        .sum::<u64>()
                })
                .filter(|total| *total > 0);
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
                    // pi reads `cacheWriteInputTokens`; the
                    // `cacheCreationInputTokens` name stays accepted as the
                    // established opi fixture key (phase 12.6).
                    cache_write_tokens: usage
                        .get("cacheWriteInputTokens")
                        .or_else(|| usage.get("cacheCreationInputTokens"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32,
                    cache_write_1h_tokens: cache_write_1h,
                },
            })]
        }
        "exception" => vec![Ok(BedrockEvent::Exception)],
        _ => Vec::new(),
    }
}

/// Map a known Converse stop reason; unmapped reasons (including the empty
/// reason) return `None` and fail typed carrying the raw value (task 20.42,
/// pi `mapStopReason` default arm plus the no-stop-reason throw).
fn map_bedrock_stop_reason(raw: &str) -> Option<StopReason> {
    match raw {
        "end_turn" | "stop_sequence" => Some(StopReason::Stop),
        "max_tokens" | "model_context_window_exceeded" => Some(StopReason::Length),
        "tool_use" => Some(StopReason::ToolUse),
        _ => None,
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
    Reasoning {
        thinking: String,
        signature: String,
        /// Decoded redacted chunks; a non-empty buffer turns the block into
        /// [`AssistantContent::RedactedThinking`] at stop.
        redacted: Vec<u8>,
        redacted_started: bool,
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
        self.snapshot
            .failure(cause.with_partial(self.partial.clone()))
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
                            signature: None,
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
                                signature: None,
                            },
                        });
                        vec![Ok(AssistantStreamEvent::ToolCallStart {
                            content_index,
                            partial: self.partial.clone(),
                        })]
                    }
                    BedrockBlockType::Reasoning => {
                        self.blocks.push(BlockState::Reasoning {
                            thinking: String::new(),
                            signature: String::new(),
                            redacted: Vec::new(),
                            redacted_started: false,
                        });
                        self.partial.content.push(AssistantContent::Thinking {
                            thinking: String::new(),
                            signature: None,
                        });
                        vec![Ok(AssistantStreamEvent::ThinkingStart {
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
                        if let Some(AssistantContent::Text { text: acc, .. }) =
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
                    BedrockDelta::Reasoning {
                        text,
                        signature,
                        redacted_content,
                    } => {
                        let mut events = Vec::new();
                        if let Some(BlockState::Reasoning {
                            thinking,
                            signature: signature_acc,
                            redacted,
                            redacted_started,
                        }) = self.blocks.last_mut()
                        {
                            if let Some(text) = text.filter(|text| !text.is_empty()) {
                                thinking.push_str(&text);
                                if let Some(AssistantContent::Thinking { thinking: acc, .. }) =
                                    self.partial.content.last_mut()
                                {
                                    acc.push_str(&text);
                                }
                                events.push(Ok(AssistantStreamEvent::ThinkingDelta {
                                    content_index,
                                    delta: text,
                                    partial: self.partial.clone(),
                                }));
                            }
                            // Signatures never mix with redacted payloads: an
                            // empty buffer guards the signature accumulator.
                            if let Some(signature) =
                                signature.filter(|signature| !signature.is_empty())
                                && redacted.is_empty()
                            {
                                signature_acc.push_str(&signature);
                            }
                            if let Some(payload) =
                                redacted_content.filter(|payload| !payload.is_empty())
                            {
                                use base64::Engine as _;
                                if let Ok(bytes) = base64::engine::general_purpose::STANDARD
                                    .decode(payload.as_bytes())
                                {
                                    redacted.extend_from_slice(&bytes);
                                }
                                if !*redacted_started {
                                    *redacted_started = true;
                                    // Placeholder mirrors the Anthropic path's
                                    // redacted display text.
                                    thinking.push_str("[Reasoning redacted]");
                                    if let Some(AssistantContent::Thinking {
                                        thinking: acc, ..
                                    }) = self.partial.content.last_mut()
                                    {
                                        acc.push_str("[Reasoning redacted]");
                                    }
                                    events.push(Ok(AssistantStreamEvent::ThinkingDelta {
                                        content_index,
                                        delta: "[Reasoning redacted]".to_owned(),
                                        partial: self.partial.clone(),
                                    }));
                                }
                            }
                        }
                        events
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
                            signature: None,
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
                    Some(BlockState::Reasoning {
                        thinking,
                        signature,
                        redacted,
                        redacted_started: _,
                    }) => {
                        // A buffered encrypted payload becomes an opaque
                        // redacted block; readable thinking keeps its
                        // accumulated signature.
                        if !redacted.is_empty() {
                            use base64::Engine as _;
                            if let Some(slot) = self.partial.content.last_mut() {
                                *slot = AssistantContent::RedactedThinking {
                                    data: base64::engine::general_purpose::STANDARD
                                        .encode(redacted.as_slice()),
                                };
                            }
                        } else if let Some(AssistantContent::Thinking {
                            signature: signature_slot,
                            ..
                        }) = self.partial.content.last_mut()
                            && !signature.is_empty()
                        {
                            *signature_slot = Some(signature.clone());
                        }
                        vec![Ok(AssistantStreamEvent::ThinkingEnd {
                            content_index,
                            content: thinking.clone(),
                            partial: self.partial.clone(),
                        })]
                    }
                    None => Vec::new(),
                }
            }
            BedrockEvent::MessageStop { stop_reason } => {
                match map_bedrock_stop_reason(&stop_reason) {
                    Some(reason) => {
                        self.partial.stop_reason = reason;
                        self.saw_done = true;
                        // Defer Done event  - metadata may follow with final usage
                        self.pending_done = Some(AssistantStreamEvent::Done {
                            reason: self.partial.stop_reason,
                            message: self.partial.clone(),
                        });
                        Vec::new()
                    }
                    // Unmapped stop reasons fail typed carrying the raw
                    // value; an empty reason means the stream settled
                    // without one (task 20.42, pi mapStopReason default
                    // arm and the no-stop-reason throw).
                    None => {
                        self.saw_done = true;
                        let detail = if stop_reason.is_empty() {
                            "Bedrock stream ended without a stop reason".to_owned()
                        } else {
                            format!("Provider stopped with: {stop_reason}")
                        };
                        vec![Err(self.failure(ProviderError::StreamError(
                            ProviderErrorSummary::sanitized(detail),
                        )))]
                    }
                }
            }
            BedrockEvent::Metadata { usage } => {
                self.usage = usage.clone();
                self.usage_reported = true;
                self.partial.usage = Usage::reported(
                    self.usage.input_tokens,
                    self.usage.output_tokens,
                    self.usage.cache_read_tokens,
                    self.usage.cache_write_tokens,
                    self.usage.cache_write_1h_tokens,
                    None,
                );
                // Flush pending Done with updated usage
                if let Some(AssistantStreamEvent::Done { message, .. }) = &mut self.pending_done {
                    message.usage = Usage::reported(
                        self.usage.input_tokens,
                        self.usage.output_tokens,
                        self.usage.cache_read_tokens,
                        self.usage.cache_write_tokens,
                        self.usage.cache_write_1h_tokens,
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
        diagnostics: Vec::new(),
        duration_ms: None,
        provider_thinking_level: None,
        deferred: None,
    }
}

// ---------------------------------------------------------------------------
// Converse API message serialization
// ---------------------------------------------------------------------------

/// Pi `<empty>`: Bedrock rejects empty content arrays and blank text blocks
/// (task 20.42, pi `EMPTY_TEXT_PLACEHOLDER`).
const EMPTY_TEXT_PLACEHOLDER: &str = "<empty>";
/// Placeholder replacing an image run inside an error tool result whose
/// images move after the turn's results (pi `ERROR_TOOL_IMAGE_PLACEHOLDER`).
const ERROR_TOOL_IMAGE_PLACEHOLDER: &str = "(image attached after the tool results)";

/// Lowercased id/name candidates with pi's separator normalization
/// (`[\s_.:]+` -> `-`) so application inference profile ARNs and catalog
/// names match the same substring rules as model ids.
fn model_candidates(id: &str, name: &str) -> Vec<String> {
    let mut candidates = Vec::new();
    for value in [id, name] {
        if value.is_empty() {
            continue;
        }
        let lower = value.to_lowercase();
        candidates.push(lower.clone());
        candidates.push(lower.replace([' ', '_', '.', ':'], "-"));
    }
    candidates
}

/// Claude models on Bedrock (pi `isAnthropicClaudeModel`): the model id
/// carries the anthropic prefix, or the user-controlled name references
/// Claude (application inference profiles).
fn is_anthropic_claude_model(id: &str, name: &str) -> bool {
    id.to_lowercase().contains("anthropic.claude")
        || id.to_lowercase().contains("anthropic/claude")
        || name.to_lowercase().contains("claude")
}

/// Adaptive-thinking Claude models (pi `supportsAdaptiveThinking`):
/// Opus 4.6+, Sonnet 4.6+, Claude 5.
pub(crate) fn supports_adaptive_thinking(id: &str, name: &str) -> bool {
    model_candidates(id, name).iter().any(|s| {
        s.contains("opus-4-6")
            || s.contains("opus-4-7")
            || s.contains("opus-4-8")
            || s.contains("opus-5")
            || s.contains("sonnet-4-6")
            || s.contains("sonnet-5")
            || s.contains("haiku-5")
            || s.contains("fable-5")
    })
}

/// Models accepting a native `xhigh` effort (pi `supportsNativeXhighEffort`).
fn supports_native_xhigh_effort(id: &str, name: &str) -> bool {
    model_candidates(id, name).iter().any(|s| {
        s.contains("opus-4-7")
            || s.contains("opus-4-8")
            || s.contains("opus-5")
            || s.contains("sonnet-5")
            || s.contains("haiku-5")
            || s.contains("fable-5")
    })
}

/// Models accepting `thinking.block_binding` (pi `supportsThinkingBlockBinding`):
/// Opus 4.6 and Sonnet 4.6 reject the field.
fn supports_thinking_block_binding(id: &str, name: &str) -> bool {
    supports_native_xhigh_effort(id, name)
}

/// Claude models with explicit cache points (pi `supportsPromptCaching`):
/// Claude 3.5 Haiku, 3.7 Sonnet, 4.x and 5 families.
fn supports_prompt_caching(id: &str, name: &str) -> bool {
    let candidates = model_candidates(id, name);
    if !candidates.iter().any(|s| s.contains("claude")) {
        return false;
    }
    candidates.iter().any(|s| {
        s.contains("fable-5")
            || s.contains("opus-5")
            || s.contains("sonnet-5")
            || s.contains("haiku-5")
            || s.contains("-4-")
            || s.contains("claude-3-7-sonnet")
            || s.contains("claude-3-5-haiku")
    })
}

/// One cache point block (pi `CachePointType.DEFAULT`, `CacheTTL.ONE_HOUR`).
fn cache_point_block(retention: crate::provider::CacheRetention) -> serde_json::Value {
    match retention {
        crate::provider::CacheRetention::Long => serde_json::json!({
            "cachePoint": {"type": "default", "ttl": "1h"}
        }),
        _ => serde_json::json!({"cachePoint": {"type": "default"}}),
    }
}

/// Pi default thinking budgets for budget-based Claude models
/// (`DEFAULT_THINKING_BUDGETS` plus the xhigh/max clamp to high).
fn bedrock_default_budget(level: crate::model_info::ThinkingLevel) -> u64 {
    use crate::model_info::ThinkingLevel;
    match level {
        ThinkingLevel::Minimal => 1_024,
        ThinkingLevel::Low => 2_048,
        ThinkingLevel::Medium => 8_192,
        ThinkingLevel::High | ThinkingLevel::XHigh | ThinkingLevel::Max => 16_384,
        ThinkingLevel::None => 1_024,
    }
}

/// Effort mapping (pi `mapThinkingLevelToEffort`): native xhigh passes
/// through on capable models, the thinking map resolves the level, then the
/// fixed minimal/low -> low, medium -> medium, else high.
fn map_thinking_level_to_effort(
    model: Option<&ModelInfo>,
    level: crate::model_info::ThinkingLevel,
    id: &str,
    name: &str,
) -> String {
    use crate::model_info::ThinkingLevel;
    if level == ThinkingLevel::XHigh && supports_native_xhigh_effort(id, name) {
        return "xhigh".to_owned();
    }
    // An identity mapping is the opi synthetic default, not a source-provided
    // level map: only an explicit mapped value overrides the fixed table.
    if let Some(mapped) = model
        .and_then(|model| model.thinking_level_map.resolve(level).ok().flatten())
        .filter(|mapped| Some(mapped.as_str()) != level.wire_name())
    {
        return mapped;
    }
    match level {
        ThinkingLevel::Minimal | ThinkingLevel::Low => "low".to_owned(),
        ThinkingLevel::Medium => "medium".to_owned(),
        _ => "high".to_owned(),
    }
}

/// `additionalModelRequestFields` assembly (pi
/// `buildAdditionalModelRequestFields`, task 20.42). Claude models carry the
/// adaptive or numeric thinking shapes; gpt-oss/gpt- models their effort
/// fields; everything else gets none.
fn build_additional_model_request_fields(
    model: Option<&ModelInfo>,
    level: crate::model_info::ThinkingLevel,
    native: Option<&BedrockOptions>,
    simple: bool,
    request_thinking: &crate::provider::ThinkingConfig,
    gov_region: Option<&str>,
) -> Option<serde_json::Value> {
    use crate::model_info::ThinkingLevel;
    let id = model.map(|model| model.id.as_str()).unwrap_or_default();
    let name = model
        .map(|model| model.display_name.as_str())
        .unwrap_or_default();
    if is_anthropic_claude_model(id, name) {
        // GovCloud Bedrock rejects the Claude thinking.display field and the
        // binding controls beta until its Converse schema catches up.
        let is_gov = gov_region.is_some_and(|region| region.starts_with("us-gov-"))
            || id.to_lowercase().starts_with("us-gov.")
            || id.to_lowercase().starts_with("arn:aws-us-gov:");
        let display = (!is_gov).then(|| {
            native
                .and_then(|options| options.thinking_display)
                .unwrap_or(BedrockThinkingDisplay::Summarized)
        });
        let use_block_binding = !is_gov && supports_thinking_block_binding(id, name);
        if supports_adaptive_thinking(id, name) {
            let mut thinking = serde_json::json!({"type": "adaptive"});
            if let Some(display) = display {
                thinking["display"] = serde_json::json!(display.wire_str());
            }
            if use_block_binding {
                thinking["block_binding"] =
                    serde_json::json!({"prefix_mismatch_behavior": "drop_block"});
            }
            let mut fields = serde_json::json!({
                "thinking": thinking,
                "output_config": {"effort": map_thinking_level_to_effort(model, level, id, name)},
            });
            if use_block_binding {
                fields["anthropic_beta"] =
                    serde_json::json!(["thinking-binding-controls-2026-08-01"]);
            }
            return Some(fields);
        }
        // Budget-based Claude: custom budgets cover token-based levels
        // through high; the clamped level keys the override while the source
        // default table reads the raw level (xhigh/max clamp to high).
        let clamped = match level {
            ThinkingLevel::XHigh | ThinkingLevel::Max => ThinkingLevel::High,
            other => other,
        };
        let budget = native
            .and_then(|options| options.thinking_budgets.get(&clamped).copied())
            .or(if simple {
                request_thinking.budget_tokens
            } else {
                None
            })
            .unwrap_or_else(|| bedrock_default_budget(level));
        let mut thinking = serde_json::json!({"type": "enabled", "budget_tokens": budget});
        if let Some(display) = display {
            thinking["display"] = serde_json::json!(display.wire_str());
        }
        let mut fields = serde_json::json!({"thinking": thinking});
        // Interleaved thinking defaults on for budget-based models
        // (pi `options.interleavedThinking ?? true`).
        if native.and_then(|options| options.interleaved_thinking) != Some(false) {
            fields["anthropic_beta"] = serde_json::json!(["interleaved-thinking-2025-05-14"]);
        }
        return Some(fields);
    }
    let candidates = model_candidates(id, name);
    if candidates.iter().any(|s| s.contains("gpt-oss")) {
        // gpt-oss takes a flat reasoning_effort limited to low/medium/high.
        let effort = match level {
            ThinkingLevel::Minimal | ThinkingLevel::Low => "low",
            ThinkingLevel::Medium => "medium",
            _ => "high",
        };
        return Some(serde_json::json!({"reasoning_effort": effort}));
    }
    if candidates.iter().any(|s| s.contains("gpt-")) {
        let effort = model
            .and_then(|model| model.thinking_level_map.resolve(level).ok().flatten())
            .filter(|mapped| Some(mapped.as_str()) != level.wire_name())
            .unwrap_or_else(|| match level {
                ThinkingLevel::Minimal | ThinkingLevel::Low => "low".to_owned(),
                ThinkingLevel::Medium => "medium".to_owned(),
                ThinkingLevel::XHigh => "xhigh".to_owned(),
                ThinkingLevel::Max => "max".to_owned(),
                _ => "high".to_owned(),
            });
        return Some(serde_json::json!({"reasoning": {"effort": effort}}));
    }
    None
}

/// Serialize replayed messages onto the Converse wire (task 20.42): blank
/// text drops, `<empty>` placeholders keep required content arrays non-empty,
/// thinking blocks replay as `reasoningContent` (signed Claude, signature-less
/// text fallback, or opaque `redactedContent`), consecutive tool results
/// share one user message, and Claude error-result images follow the turn's
/// results with a caption (pi `splitErrorToolResultImages`).
fn serialize_converse_messages(
    messages: &[crate::message::Message],
    is_claude: bool,
) -> serde_json::Value {
    fn non_blank_text(text: &str) -> Option<serde_json::Value> {
        (!text.trim().is_empty()).then(|| serde_json::json!({"text": text}))
    }

    fn image_block(
        source: &crate::message::ImageSource,
        media_type: &crate::message::MediaType,
    ) -> serde_json::Value {
        let data = match source {
            crate::message::ImageSource::Base64 { data } => data.clone(),
            crate::message::ImageSource::Bytes { data } => {
                use base64::Engine as _;
                base64::engine::general_purpose::STANDARD.encode(data)
            }
            // URL sources fail typed before body assembly.
            crate::message::ImageSource::Url { .. } => String::new(),
        };
        serde_json::json!({
            "image": {
                "format": media_type.as_str().split('/').next_back().unwrap_or("png"),
                "source": {"bytes": data}
            }
        })
    }

    fn sanitize_document(value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Array(items) => {
                serde_json::Value::Array(items.iter().map(sanitize_document).collect())
            }
            serde_json::Value::Object(map) => serde_json::Value::Object(
                map.iter()
                    .filter(|(key, _)| !key.is_empty())
                    .map(|(key, nested)| (key.clone(), sanitize_document(nested)))
                    .collect(),
            ),
            other => other.clone(),
        }
    }

    fn tool_result_content(
        content: &[crate::message::OutputContent],
        error_split: bool,
    ) -> (Vec<serde_json::Value>, Vec<serde_json::Value>) {
        // Returns the in-result content blocks plus the deferred image
        // blocks for an error result about to be split.
        let mut blocks = Vec::new();
        let mut deferred_images = Vec::new();
        let mut previous_was_placeholder = false;
        for item in content {
            match item {
                crate::message::OutputContent::Text { text } => {
                    previous_was_placeholder = text == ERROR_TOOL_IMAGE_PLACEHOLDER;
                    if let Some(block) = non_blank_text(text) {
                        blocks.push(block);
                    }
                }
                crate::message::OutputContent::Image { source, media_type } => {
                    if error_split {
                        if !previous_was_placeholder {
                            blocks.push(serde_json::json!({
                                "text": ERROR_TOOL_IMAGE_PLACEHOLDER
                            }));
                        }
                        previous_was_placeholder = true;
                    } else {
                        previous_was_placeholder = false;
                    }
                    deferred_images.push(image_block(source, media_type));
                }
            }
        }
        if !error_split {
            blocks.append(&mut deferred_images);
        }
        (blocks, deferred_images)
    }

    let mut result: Vec<serde_json::Value> = Vec::new();
    // Consecutive tool results flush as one user message; Claude error
    // images follow all of the turn's results.
    let mut turn_results: Vec<serde_json::Value> = Vec::new();
    let mut turn_error_payloads: Vec<(String, Vec<serde_json::Value>)> = Vec::new();
    let flush_turn =
        |result: &mut Vec<serde_json::Value>,
         turn_results: &mut Vec<serde_json::Value>,
         turn_error_payloads: &mut Vec<(String, Vec<serde_json::Value>)>| {
            if turn_results.is_empty() && turn_error_payloads.is_empty() {
                return;
            }
            let mut content = std::mem::take(turn_results);
            for (caption, images) in turn_error_payloads.drain(..) {
                content.push(serde_json::json!({"text": caption}));
                content.extend(images);
            }
            result.push(serde_json::json!({"role": "user", "content": content}));
        };

    for msg in messages {
        match msg {
            crate::message::Message::System(_) => {
                // Unreachable after collapse + without_initial_system_message.
                unreachable!("system messages are projected out of the body list")
            }
            crate::message::Message::User(u) => {
                flush_turn(&mut result, &mut turn_results, &mut turn_error_payloads);
                let mut content: Vec<serde_json::Value> = Vec::new();
                for item in &u.content {
                    match item {
                        crate::message::InputContent::Text { text } => {
                            if let Some(block) = non_blank_text(text) {
                                content.push(block);
                            }
                        }
                        crate::message::InputContent::Image {
                            source, media_type, ..
                        } => content.push(image_block(source, media_type)),
                    }
                }
                if content.is_empty() {
                    content.push(serde_json::json!({"text": EMPTY_TEXT_PLACEHOLDER}));
                }
                result.push(serde_json::json!({"role": "user", "content": content}));
            }
            crate::message::Message::Assistant(a) => {
                flush_turn(&mut result, &mut turn_results, &mut turn_error_payloads);
                let mut content: Vec<serde_json::Value> = Vec::new();
                for item in &a.content {
                    match item {
                        AssistantContent::Text { text, .. } => {
                            if let Some(block) = non_blank_text(text) {
                                content.push(block);
                            }
                        }
                        AssistantContent::ToolCall { tool_call } => {
                            let input: serde_json::Value =
                                serde_json::from_str(&tool_call.arguments)
                                    .ok()
                                    .filter(|v: &serde_json::Value| v.is_object())
                                    .map(|v| sanitize_document(&v))
                                    .unwrap_or_else(|| serde_json::json!({}));
                            content.push(serde_json::json!({
                                "toolUse": {
                                    "toolUseId": tool_call.id,
                                    "name": tool_call.name,
                                    "input": input,
                                }
                            }));
                        }
                        AssistantContent::Thinking {
                            thinking,
                            signature,
                        } => {
                            if thinking.trim().is_empty() {
                                continue;
                            }
                            content.push(if is_claude {
                                match signature.as_deref().filter(|s| !s.trim().is_empty()) {
                                    // Signatures arrive after thinking deltas;
                                    // a partial or externally persisted message
                                    // without one replays plain text (Bedrock
                                    // rejects the unsigned reasoning block).
                                    Some(signature) => serde_json::json!({
                                        "reasoningContent": {"reasoningText": {
                                            "text": thinking,
                                            "signature": signature,
                                        }}
                                    }),
                                    None => serde_json::json!({"text": thinking}),
                                }
                            } else {
                                // Non-Anthropic models reject the signature
                                // member outright.
                                serde_json::json!({
                                    "reasoningContent": {"reasoningText": {"text": thinking}}
                                })
                            });
                        }
                        AssistantContent::RedactedThinking { data } => {
                            // Opaque payload replays as bytes; a payload that
                            // is not base64 drops instead of failing.
                            use base64::Engine as _;
                            if let Ok(bytes) =
                                base64::engine::general_purpose::STANDARD.decode(data.as_bytes())
                            {
                                content.push(serde_json::json!({
                                    "reasoningContent": {"redactedContent":
                                        base64::engine::general_purpose::STANDARD.encode(bytes)}
                                }));
                            }
                        }
                    }
                }
                // Bedrock rejects empty assistant content arrays.
                if !content.is_empty() {
                    result.push(serde_json::json!({"role": "assistant", "content": content}));
                }
            }
            crate::message::Message::ToolResult(t) => {
                let error_split = is_claude
                    && t.is_error
                    && t.content
                        .iter()
                        .any(|c| matches!(c, crate::message::OutputContent::Image { .. }));
                let (mut blocks, deferred) = tool_result_content(&t.content, error_split);
                if blocks.is_empty() {
                    blocks.push(serde_json::json!({"text": EMPTY_TEXT_PLACEHOLDER}));
                }
                turn_results.push(serde_json::json!({
                    "toolResult": {
                        "toolUseId": t.tool_call_id,
                        "content": blocks,
                        "status": if t.is_error { "error" } else { "success" },
                    }
                }));
                if error_split && !deferred.is_empty() {
                    turn_error_payloads.push((
                        format!(
                            "Images from the error result of tool call {} ({}):",
                            t.tool_call_id, t.tool_name
                        ),
                        deferred,
                    ));
                }
            }
        }
    }
    flush_turn(&mut result, &mut turn_results, &mut turn_error_payloads);
    serde_json::Value::Array(result)
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

// ---------------------------------------------------------------------------
// Task 20.42: typed native options and the direct chat adapter
// ---------------------------------------------------------------------------

/// Native Converse `toolConfig.toolChoice` (pi `BedrockOptions.toolChoice`):
/// the Google-style named-tool mode exists on this wire.
#[derive(Clone, Debug, PartialEq)]
pub enum BedrockToolChoice {
    Auto,
    Any,
    None_,
    Named(String),
}

/// Claude thinking display controls (pi `BedrockThinkingDisplay`):
/// `summarized` keeps readable thinking text, `omitted` redacts it while the
/// signature still travels back for multi-turn continuity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BedrockThinkingDisplay {
    #[default]
    Summarized,
    Omitted,
}

impl BedrockThinkingDisplay {
    fn wire_str(self) -> &'static str {
        match self {
            Self::Summarized => "summarized",
            Self::Omitted => "omitted",
        }
    }
}

/// Typed native options for the bedrock-converse-stream wire (task 20.42,
/// annex row 51): the level-based reasoning entry with custom budget
/// overrides, the interleaved/display thinking controls, and the verbatim
/// `requestMetadata` passthrough. `region`/`profile`/`bearerToken` stay
/// resolver-owned inputs (annex row 111, task 20.20 chain) rather than
/// options on this struct.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BedrockOptions {
    pub tool_choice: Option<BedrockToolChoice>,
    /// Native reasoning level; budget-based Claude models map it through
    /// the budget table (custom `thinking_budgets` override, `xhigh`/`max`
    /// clamp to `high`), adaptive models map it to `output_config.effort`.
    pub reasoning: Option<crate::model_info::ThinkingLevel>,
    /// Custom per-level token budgets; cover token-based levels through
    /// high (pi `ThinkingBudgets`).
    pub thinking_budgets: std::collections::BTreeMap<crate::model_info::ThinkingLevel, u64>,
    /// Interleaved-thinking beta for budget-based Claude models; defaults
    /// to enabled.
    pub interleaved_thinking: Option<bool>,
    pub thinking_display: Option<BedrockThinkingDisplay>,
    /// Key-value pairs attached to the inference request for cost
    /// allocation tagging. Rides verbatim: pi documents the AWS limits in
    /// comments only, and the SDK owns validation.
    pub request_metadata: std::collections::BTreeMap<String, String>,
    simple: bool,
}

impl BedrockOptions {
    /// The neutral simple entry (task 20.42): only the neutral tool choice
    /// maps; the request thinking intent keeps driving the reasoning
    /// projection (compiled budgets for budget models, the level for
    /// adaptive ones).
    pub fn from_simple(simple: &crate::models::SimpleOptions) -> Self {
        Self {
            tool_choice: simple.tool_choice.map(|choice| match choice {
                crate::models::SimpleToolChoice::Auto => BedrockToolChoice::Auto,
                crate::models::SimpleToolChoice::None => BedrockToolChoice::None_,
            }),
            reasoning: None,
            thinking_budgets: Default::default(),
            interleaved_thinking: None,
            thinking_display: None,
            request_metadata: Default::default(),
            simple: true,
        }
    }

    pub fn is_simple(&self) -> bool {
        self.simple
    }
}

impl crate::model_info::WireCompat {
    /// The bedrock-converse-stream compat flags, when this record carries
    /// them (task 20.42).
    pub fn as_bedrock_converse_stream(
        &self,
    ) -> Option<&crate::model_info::BedrockConverseStreamCompat> {
        match self {
            crate::model_info::WireCompat::BedrockConverseStream(compat) => Some(compat),
            _ => None,
        }
    }
}

/// One complete, wire-valid Chat model record or a typed failure.
fn validate_complete_bedrock_record(
    model: &ModelInfo,
) -> Result<(), crate::provider::ProviderError> {
    if model.wire_api != crate::model_info::WireApi::BedrockConverseStream
        || model.id.trim().is_empty()
        || model.validate().is_err()
    {
        return Err(crate::provider::ProviderError::Config(
            crate::provider::ProviderErrorSummary::attested_static(
                "bedrock-converse-stream adapters require a valid complete Chat model record",
            ),
        ));
    }
    Ok(())
}

/// The public bedrock-converse-stream direct adapter registered through
/// [`crate::Models::register_chat_api`] (task 20.42). Each dispatch clones
/// its immutable provider configuration; native options never mutate a
/// registered provider.
pub struct BedrockConverseStreamChatApi {
    api_id: crate::models::ApiId,
    provider: Arc<BedrockProvider>,
    model: ModelInfo,
}

impl std::fmt::Debug for BedrockConverseStreamChatApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BedrockConverseStreamChatApi")
            .field("api_id", &self.api_id)
            .finish_non_exhaustive()
    }
}

impl BedrockConverseStreamChatApi {
    /// Bind a complete Chat record. Wrong-wire or invalid records fail
    /// before authentication or network activity.
    pub fn new(
        provider: Arc<BedrockProvider>,
        model: ModelInfo,
    ) -> Result<Self, crate::provider::ProviderError> {
        validate_complete_bedrock_record(&model)?;
        Ok(Self {
            api_id: crate::models::ApiId::new("bedrock-converse-stream"),
            provider,
            model,
        })
    }

    fn configured(&self, model: ModelInfo, options: BedrockOptions) -> BedrockProvider {
        let mut provider = (*self.provider).clone();
        provider.models = vec![model];
        provider.native_options = Some(options);
        provider
    }
}

impl crate::models::ChatApi<BedrockOptions> for BedrockConverseStreamChatApi {
    fn validate_simple_options(
        &self,
        model: &ModelInfo,
        _options: &crate::models::SimpleOptions,
    ) -> Result<(), crate::provider::ProviderError> {
        validate_complete_bedrock_record(model)
    }

    fn api_id(&self) -> &crate::models::ApiId {
        &self.api_id
    }

    fn provider_id(&self) -> &str {
        "bedrock"
    }

    fn complete_record(&self) -> ModelInfo {
        self.model.clone()
    }

    fn simple_options(&self) -> BedrockOptions {
        BedrockOptions::from_simple(&crate::models::SimpleOptions::default())
    }

    fn simple_options_for(&self, options: &crate::models::SimpleOptions) -> BedrockOptions {
        BedrockOptions::from_simple(options)
    }

    fn session_resources(&self) -> Option<&dyn crate::codex_resources::SessionResourceOwner> {
        None
    }

    fn validate_native_request(
        &self,
        model: &ModelInfo,
        _request: &Request,
        _options: &BedrockOptions,
    ) -> Result<(), crate::provider::ProviderError> {
        validate_complete_bedrock_record(model)
    }

    fn stream_prepared(
        &self,
        request: Request,
        options: BedrockOptions,
        auth: crate::auth::ResolvedAuth,
    ) -> EventStream {
        self.stream_model_resource_prepared(self.model.clone(), request, options, auth, None)
    }

    fn stream_resource_prepared(
        &self,
        request: Request,
        options: BedrockOptions,
        auth: crate::auth::ResolvedAuth,
        resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        self.stream_model_resource_prepared(self.model.clone(), request, options, auth, resources)
    }

    fn stream_model_resource_prepared(
        &self,
        model: ModelInfo,
        mut request: Request,
        options: BedrockOptions,
        auth: crate::auth::ResolvedAuth,
        _resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        if let Err(error) = self.validate_native_request(&model, &request, &options) {
            return Box::pin(stream::once(async move { Err(error) }));
        }
        if !options.is_simple() {
            // Native reasoning owns the wire projection; the neutral fields
            // are unused on this path (the simple entry compiles its
            // budgets before dispatch).
            request.thinking = Default::default();
        }
        let provider = self.configured(model, options);
        let provider = Arc::new(provider);
        crate::models::drive_resolved_call(provider, request, auth)
    }
}
