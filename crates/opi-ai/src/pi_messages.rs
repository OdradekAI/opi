//! pi-messages direct adapter (task 20.44, DEL-11).
//!
//! Pins `.repo/pi-1.3.0/packages/ai/src/api/pi-messages.ts`: pi's own
//! message protocol spoken by the Radius gateway. One POST of
//! `{ model, context, options }` to `<base>/messages`, answered by an SSE
//! stream of serialized assistant-message events plus a terminal
//! `done`/`error` event. The options object is an exact six-field whitelist
//! (`temperature`, `maxTokens`, `reasoning`, `cacheRetention`, `sessionId`,
//! `toolChoice`) — omissions stay omissions (no `auto` fill-in, no default
//! maxTokens, no context clamp), and the simple entry forwards the requested
//! thinking level verbatim without consulting the model record's level map.
//! The context is the canonical chronology projected into pi's camelCase
//! wire JSON (not a Chat-serializer conversion); mid-conversation system
//! messages stay in place because the backend natively understands pi's
//! chronology.
//!
//! Authentication arrives per call via [`Provider::stream_prepared`] with a
//! Bearer secret; the adapter never reads credentials itself.

use std::collections::BTreeMap;
use std::sync::Arc;

use futures_util::StreamExt;
use secrecy::ExposeSecret;
use tokio_util::sync::CancellationToken;

use crate::message::{AssistantContent, Message, ToolCall, ToolDef};
use crate::model_info::{ThinkingLevel, WireApi};
use crate::provider::{
    CacheRetention, EventStream, Provider, ProviderError, ProviderErrorSummary, Request,
};
use crate::stream::{AssistantStreamEvent, StopReason, Usage};

/// Default Radius gateway (pi `DEFAULT_RADIUS_GATEWAY`).
pub const BASE_URL: &str = "https://radius.pi.dev";

/// Resolve the wire `options.cacheRetention` member (pi
/// `resolveCacheRetention`): an explicit request value wins and maps
/// one-to-one (`Disabled` → `none`), an unset request keeps the backend
/// default (omitted) except for the legacy `PI_CACHE_RETENTION=long`
/// opt-in, which maps to `long`. No other ambient value maps.
pub fn resolve_wire_cache_retention(
    explicit: CacheRetention,
    env: Option<&str>,
) -> Option<&'static str> {
    match explicit {
        CacheRetention::None => (env == Some("long")).then_some("long"),
        CacheRetention::Disabled => Some("none"),
        CacheRetention::Short => Some("short"),
        CacheRetention::Long => Some("long"),
    }
}

/// Native tool selection for the pi-messages wire (pi `PiMessagesOptions
/// .toolChoice`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PiMessagesToolChoice {
    Auto,
    None_,
    Required,
    Function { name: String },
}

impl PiMessagesToolChoice {
    fn wire_value(&self) -> serde_json::Value {
        match self {
            Self::Auto => serde_json::json!("auto"),
            Self::None_ => serde_json::json!("none"),
            Self::Required => serde_json::json!("required"),
            Self::Function { name } => serde_json::json!({
                "type": "function",
                "function": {"name": name}
            }),
        }
    }
}

/// Typed native options for the pi-messages adapter. The whitelist members
/// `temperature`, `maxTokens`, `sessionId` and `cacheRetention` flow from
/// the neutral request (they are neutral fields on every adapter); these
/// options own the pi-specific `reasoning`, `toolChoice` and `debug`
/// members plus the simple/native distinction.
#[derive(Debug, Clone)]
pub struct PiMessagesOptions {
    /// Requested thinking level; `ThinkingLevel::None` projects the wire
    /// string `"off"`.
    pub reasoning: Option<ThinkingLevel>,
    pub tool_choice: Option<PiMessagesToolChoice>,
    /// Ask the backend for debug metadata (adds `?debug=1`).
    pub debug: bool,
    simple: bool,
}

impl PiMessagesOptions {
    /// Native options: the pi-specific members ride the wire verbatim.
    pub fn native(
        reasoning: Option<ThinkingLevel>,
        tool_choice: Option<PiMessagesToolChoice>,
        debug: bool,
    ) -> Self {
        Self {
            reasoning,
            tool_choice,
            debug,
            simple: false,
        }
    }

    /// The neutral mapping (pi `streamSimple` base): only tool choice
    /// carries; reasoning resolves from the request's thinking config in
    /// [`PiMessagesProvider::stream_prepared`].
    pub fn from_simple(options: &crate::models::SimpleOptions) -> Self {
        Self {
            reasoning: None,
            tool_choice: options.tool_choice.map(|choice| match choice {
                crate::models::SimpleToolChoice::Auto => PiMessagesToolChoice::Auto,
                crate::models::SimpleToolChoice::None => PiMessagesToolChoice::None_,
            }),
            debug: false,
            simple: true,
        }
    }

    fn is_simple(&self) -> bool {
        self.simple
    }
}

/// The native `pi-messages` provider (Radius gateway).
#[derive(Clone)]
pub struct PiMessagesProvider {
    base_url: String,
    client: Arc<crate::http::HttpClient>,
    models: Vec<crate::provider::ModelInfo>,
    native_options: Option<PiMessagesOptions>,
}

impl std::fmt::Debug for PiMessagesProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PiMessagesProvider")
            .field("base_url", &self.base_url)
            .field("models", &self.models)
            .finish_non_exhaustive()
    }
}

impl PiMessagesProvider {
    pub(crate) fn with_client(mut self, client: Arc<crate::http::HttpClient>) -> Self {
        self.client = client;
        self
    }

    pub fn new(base_url: Option<String>) -> Self {
        Self {
            base_url: base_url.unwrap_or_else(|| BASE_URL.into()),
            client: Arc::new(crate::http::HttpClient::new()),
            models: model_catalog().to_vec(),
            native_options: None,
        }
    }

    pub fn with_models(mut self, models: Vec<crate::provider::ModelInfo>) -> Self {
        self.models = models;
        self
    }

    pub fn with_native_options(mut self, options: PiMessagesOptions) -> Self {
        self.native_options = Some(options);
        self
    }

    fn resolve_model(&self, requested: &str) -> Option<&crate::provider::ModelInfo> {
        let id = requested.strip_prefix("radius:").unwrap_or(requested);
        self.models
            .iter()
            .find(|model| model.id == id)
            .or_else(|| self.models.iter().find(|model| model.id == requested))
    }

    /// The neutral entry (pi `streamSimple`): the simple bypass forwards the
    /// requested thinking level verbatim — no level-map clamp, no default
    /// maxTokens, no context clamp — and maps only the neutral tool choice.
    pub fn stream_simple(&self, request: Request, auth: crate::auth::ResolvedAuth) -> EventStream {
        let options = PiMessagesOptions::from_simple(&crate::models::SimpleOptions::default());
        let provider = self.clone().with_native_options(options);
        provider.stream_prepared(request, auth)
    }

    fn unknown_model_error(&self, requested: &str) -> ProviderError {
        ProviderError::Config(ProviderErrorSummary::sanitized(format!(
            "unknown radius model: {requested}"
        )))
    }

    fn build_request_body(&self, request: &Request) -> Result<serde_json::Value, ProviderError> {
        let model = self
            .resolve_model(&request.model)
            .ok_or_else(|| self.unknown_model_error(&request.model))?;
        let options = self.native_options.as_ref();

        // pi-messages natively understands pi's chronology: the transcript
        // rides as-is, mid-conversation system messages included.
        let context: Vec<serde_json::Value> = request
            .context
            .messages()
            .iter()
            .map(to_pi_wire_message)
            .collect();

        let mut wire_options = serde_json::Map::new();
        if let Some(temperature) = request.temperature {
            wire_options.insert("temperature".into(), serde_json::json!(temperature));
        }
        if let Some(max_tokens) = request.max_tokens {
            wire_options.insert("maxTokens".into(), serde_json::json!(max_tokens));
        }
        // Native options own the reasoning member; the simple entry derives
        // it from the request's thinking config (verbatim, unclamped).
        let reasoning = match options {
            Some(options) if !options.is_simple() => options.reasoning,
            Some(_) if request.thinking.enabled => Some(request.thinking.level),
            _ => None,
        };
        if let Some(level) = reasoning {
            wire_options.insert("reasoning".into(), serde_json::json!(reasoning_wire(level)));
        }
        let env_retention = crate::provider_env::provider_env_value("PI_CACHE_RETENTION", None);
        if let Some(retention) =
            resolve_wire_cache_retention(request.cache_retention, env_retention.as_deref())
        {
            wire_options.insert("cacheRetention".into(), serde_json::json!(retention));
        }
        if let Some(session) = &request.session_id {
            wire_options.insert("sessionId".into(), serde_json::json!(session));
        }
        if let Some(choice) = options.and_then(|options| options.tool_choice.as_ref()) {
            wire_options.insert("toolChoice".into(), choice.wire_value());
        }

        Ok(serde_json::json!({
            "model": model.id,
            "context": { "messages": context },
            "options": wire_options,
        }))
    }

    fn uses_debug(&self) -> bool {
        self.native_options
            .as_ref()
            .is_some_and(|options| options.debug)
    }
}

fn reasoning_wire(level: ThinkingLevel) -> &'static str {
    match level {
        ThinkingLevel::None => "off",
        ThinkingLevel::Minimal => "minimal",
        ThinkingLevel::Low => "low",
        ThinkingLevel::Medium => "medium",
        ThinkingLevel::High => "high",
        ThinkingLevel::XHigh => "xhigh",
        ThinkingLevel::Max => "max",
    }
}

fn tool_def_value(tool: &ToolDef) -> serde_json::Value {
    let mut value = serde_json::json!({
        "name": tool.name,
        "description": tool.description,
        "parameters": tool.input_schema,
    });
    if let Some(constrained) = &tool.constrained_sampling {
        value["constrainedSampling"] =
            serde_json::to_value(constrained).unwrap_or(serde_json::Value::Null);
    }
    value
}

fn input_content_value(content: &crate::message::InputContent) -> serde_json::Value {
    match content {
        crate::message::InputContent::Text { text } => {
            serde_json::json!({"type": "text", "text": text})
        }
        crate::message::InputContent::Image { source, media_type } => {
            serde_json::json!({
                "type": "image",
                "data": image_data(source),
                "mimeType": media_type.as_str(),
            })
        }
    }
}

fn image_data(source: &crate::message::ImageSource) -> String {
    match source {
        crate::message::ImageSource::Base64 { data } => data.clone(),
        crate::message::ImageSource::Url { url } => url.clone(),
        crate::message::ImageSource::Bytes { data } => {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.encode(data)
        }
    }
}

fn output_content_value(content: &crate::message::OutputContent) -> serde_json::Value {
    match content {
        crate::message::OutputContent::Text { text } => {
            serde_json::json!({"type": "text", "text": text})
        }
        crate::message::OutputContent::Image { source, media_type } => {
            serde_json::json!({
                "type": "image",
                "data": image_data(source),
                "mimeType": media_type.as_str(),
            })
        }
    }
}

fn stop_reason_wire(reason: StopReason) -> &'static str {
    match reason {
        StopReason::Stop => "stop",
        StopReason::Length => "length",
        StopReason::ToolUse => "toolUse",
        StopReason::Error => "error",
        StopReason::Aborted => "aborted",
        // Deferred pending turns serialize with pi's stopReason vocabulary;
        // radius wire support for pending handles is a future adapter slice.
        StopReason::Deferred => "deferred",
    }
}

fn usage_wire(usage: &Usage) -> serde_json::Value {
    let mut value = serde_json::json!({
        "input": usage.input_tokens,
        "output": usage.output_tokens,
        "cacheRead": usage.cache_read_tokens,
        "cacheWrite": usage.cache_write_tokens,
    });
    if let Some(cache_write_1h) = usage.cache_write_1h_tokens {
        value["cacheWrite1h"] = serde_json::json!(cache_write_1h);
    }
    if let Some(reasoning) = usage.reasoning_tokens {
        value["reasoning"] = serde_json::json!(reasoning);
    }
    if let Some(total) = usage.total_tokens {
        value["totalTokens"] = serde_json::json!(total);
    }
    value
}

fn assistant_content_value(content: &AssistantContent) -> serde_json::Value {
    match content {
        AssistantContent::Text { text, signature } => {
            let mut value = serde_json::json!({"type": "text", "text": text});
            if let Some(signature) = signature {
                value["textSignature"] = serde_json::json!(signature);
            }
            value
        }
        AssistantContent::Thinking {
            thinking,
            signature,
        } => {
            let mut value = serde_json::json!({"type": "thinking", "thinking": thinking});
            if let Some(signature) = signature {
                value["thinkingSignature"] = serde_json::json!(signature);
            }
            value
        }
        AssistantContent::RedactedThinking { data } => serde_json::json!({
            "type": "thinking",
            "thinking": "",
            "thinkingSignature": data,
            "redacted": true,
        }),
        AssistantContent::ToolCall { tool_call } => {
            let (arguments, arguments_error) = tool_arguments_value(tool_call);
            let mut value = serde_json::json!({
                "type": "toolCall",
                "id": tool_call.id,
                "name": tool_call.name,
                "arguments": arguments,
            });
            if let Some(signature) = &tool_call.signature {
                value["thoughtSignature"] = serde_json::json!(signature);
            }
            if let Some(error) = arguments_error {
                value["argumentsError"] = serde_json::json!(error);
            }
            value
        }
    }
}

fn tool_arguments_value(tool_call: &ToolCall) -> (serde_json::Value, Option<String>) {
    match serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&tool_call.arguments) {
        Ok(map) => (serde_json::Value::Object(map), None),
        Err(_) => (
            serde_json::json!({}),
            tool_call
                .arguments_error
                .clone()
                .or_else(|| Some("tool arguments were not a JSON object".to_owned())),
        ),
    }
}

fn assistant_api_wire(message: &crate::message::AssistantMessage) -> String {
    if let Some(wire) = &message.wire {
        return wire.as_str().to_owned();
    }
    match message.api {
        crate::ApiKind::Anthropic => "anthropic-messages".to_owned(),
        crate::ApiKind::OpenAi => "openai-completions".to_owned(),
        crate::ApiKind::Google => "google-generative-ai".to_owned(),
        crate::ApiKind::Mistral => "mistral-conversations".to_owned(),
    }
}

/// Project one canonical chronology message into pi's camelCase wire JSON.
fn to_pi_wire_message(message: &Message) -> serde_json::Value {
    match message {
        Message::System(system) => {
            let mut value = serde_json::json!({
                "role": "system",
                "content": system.content,
            });
            if !system.sections.is_empty() {
                let mut sections = serde_json::Map::new();
                for section in &system.sections {
                    sections.insert(
                        section.name.clone(),
                        section
                            .value
                            .clone()
                            .map(serde_json::Value::String)
                            .unwrap_or(serde_json::Value::Null),
                    );
                }
                value["sections"] = serde_json::Value::Object(sections);
            }
            if !system.tools_added.is_empty() {
                value["toolsAdded"] = serde_json::Value::Array(
                    system.tools_added.iter().map(tool_def_value).collect(),
                );
            }
            if !system.tools_removed.is_empty() {
                value["toolsRemoved"] = serde_json::Value::Array(
                    system
                        .tools_removed
                        .iter()
                        .map(|tool| serde_json::json!(tool.name))
                        .collect(),
                );
            }
            value["timestamp"] = serde_json::json!(system.timestamp_ms);
            value
        }
        Message::User(user) => {
            let content = if user.content.len() == 1
                && matches!(user.content[0], crate::message::InputContent::Text { .. })
            {
                let crate::message::InputContent::Text { text } = &user.content[0] else {
                    unreachable!("single text content checked above");
                };
                serde_json::json!(text)
            } else {
                serde_json::Value::Array(user.content.iter().map(input_content_value).collect())
            };
            serde_json::json!({
                "role": "user",
                "content": content,
                "timestamp": user.timestamp_ms,
            })
        }
        Message::Assistant(assistant) => {
            let mut value = serde_json::json!({
                "role": "assistant",
                "content": assistant
                    .content
                    .iter()
                    .map(assistant_content_value)
                    .collect::<Vec<_>>(),
                "api": assistant_api_wire(assistant),
                "provider": assistant.provider,
                "model": assistant.model,
                "usage": usage_wire(&assistant.usage),
                "stopReason": stop_reason_wire(assistant.stop_reason),
                "timestamp": assistant.timestamp_ms,
            });
            if let Some(model) = &assistant.response_model {
                value["responseModel"] = serde_json::json!(model);
            }
            if let Some(id) = &assistant.response_id {
                value["responseId"] = serde_json::json!(id);
            }
            if let Some(level) = &assistant.provider_thinking_level {
                value["providerThinkingLevel"] = serde_json::json!(level);
            }
            if let Some(error) = &assistant.error_message {
                value["errorMessage"] = serde_json::json!(error);
            }
            value
        }
        Message::ToolResult(tool) => {
            let mut value = serde_json::json!({
                "role": "toolResult",
                "toolCallId": tool.tool_call_id,
                "toolName": tool.tool_name,
                "content": tool
                    .content
                    .iter()
                    .map(output_content_value)
                    .collect::<Vec<_>>(),
                "isError": tool.is_error,
                "timestamp": tool.timestamp_ms,
            });
            if let Some(details) = &tool.details {
                value["details"] = details.clone();
            }
            value
        }
    }
}

/// opi-curated baseline catalog. pi ships `data/radius.json` as a
/// pre-catalog baseline for the default gateway; the generated data file is
/// absent from the pinned source, so opi pins one conservative entry per
/// family. A configured gateway's dynamic catalog replaces the baseline at
/// registration time (pi `radiusProvider.refreshModels`).
fn model_catalog() -> &'static [crate::provider::ModelInfo] {
    static CATALOG: std::sync::OnceLock<Vec<crate::provider::ModelInfo>> =
        std::sync::OnceLock::new();
    CATALOG.get_or_init(|| {
        vec![crate::provider::ModelInfo::new(
            "claude-sonnet-4-5",
            "claude-sonnet-4-5",
            WireApi::PiMessages,
            crate::registry::ModelCapabilities::new(200_000, 16_384)
                .with_streaming(true)
                .with_thinking(true)
                .with_images(true),
        )]
    })
}

impl Provider for PiMessagesProvider {
    fn validate_simple_options(
        &self,
        model: &crate::provider::ModelInfo,
        _options: &crate::models::SimpleOptions,
    ) -> Result<(), ProviderError> {
        PiMessagesChatApi::new(self.clone(), model.clone()).map(|_| ())
    }

    fn stream_model_resource_attempt(
        &self,
        model: crate::provider::ModelInfo,
        options: crate::models::SimpleOptions,
        request: Request,
        auth: crate::auth::ResolvedAuth,
        attempt: crate::provider::AttemptContext,
        resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        let mut provider = self.clone();
        if let Some(base_url) = model.base_url.clone() {
            provider.base_url = base_url;
        }
        provider
            .with_models(vec![model])
            .with_native_options(PiMessagesOptions::from_simple(&options))
            .stream_resource_attempt(request, auth, attempt, resources)
    }

    fn id(&self) -> &str {
        "radius"
    }

    fn models(&self) -> &[crate::provider::ModelInfo] {
        &self.models
    }

    fn stream_prepared(
        &self,
        mut request: Request,
        auth: crate::auth::ResolvedAuth,
    ) -> EventStream {
        request.callbacks = request.callbacks.ensure_call_scope();
        let bearer = match auth.scheme {
            crate::auth::AuthScheme::Bearer => {
                format!("Bearer {}", auth.secret.expose_secret())
            }
            crate::auth::AuthScheme::ApiKey
            | crate::auth::AuthScheme::AwsSigV4(_)
            | crate::auth::AuthScheme::AnthropicFederation => {
                return Box::pin(futures_util::stream::iter(vec![Err(
                    ProviderError::Config(ProviderErrorSummary::attested_static(
                        "pi-messages accepts only Bearer authentication",
                    )),
                )]));
            }
        };
        let model = match self.resolve_model(&request.model).cloned() {
            Some(model) => model,
            None => {
                let error = self.unknown_model_error(&request.model);
                return Box::pin(futures_util::stream::iter(vec![Err(error)]));
            }
        };
        let body = match self.build_request_body(&request) {
            Ok(body) => body,
            Err(error) => return Box::pin(futures_util::stream::iter(vec![Err(error)])),
        };

        let base = model
            .base_url
            .as_deref()
            .unwrap_or(&self.base_url)
            .trim_end_matches('/');
        let path = if self.uses_debug() {
            format!("{base}/messages?debug=1")
        } else {
            format!("{base}/messages")
        };
        let headers = match crate::provider_headers::ProviderHeaders::default().merge_model_request(
            &model.headers,
            &[
                ("authorization".into(), bearer),
                ("accept".into(), "text/event-stream".into()),
                ("content-type".into(), "application/json".into()),
            ],
            &request.extra_headers,
        ) {
            Ok(headers) => headers,
            Err(error) => return Box::pin(futures_util::stream::iter(vec![Err(error)])),
        };
        let mut req = self.client.client().post(&path);
        for (name, value) in &headers {
            req = req.header(name.as_str(), value.as_str());
        }

        let requested_model = model.id.clone();
        let callbacks = request.callbacks.clone();
        let cancel = request.cancel;
        let telemetry = request.telemetry_context.clone();
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
                    stream_pi_messages_http(
                        req,
                        body,
                        &requested_model,
                        producer_cancel.clone(),
                        callbacks,
                        producer_snapshot.clone(),
                        &tx,
                    )
                    .await
                    .map_err(|error| producer_snapshot.failure(error))
                }
                .await;
                match result {
                    Ok(()) | Err(ProviderError::Cancelled) => {}
                    Err(error) => {
                        let _ =
                            crate::stream::send_or_cancel(&producer_cancel, &tx, Err(error)).await;
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
        Box::pin(crate::stream::CancelAwareReceiverStream::with_snapshot(
            rx, cancel, snapshot,
        ))
    }
}

#[allow(clippy::too_many_arguments)]
async fn stream_pi_messages_http(
    req: reqwest::RequestBuilder,
    body: serde_json::Value,
    requested_model: &str,
    cancel: CancellationToken,
    callbacks: crate::callbacks::RequestCallbacks,
    snapshot: crate::stream::ProducerSnapshot,
    tx: &tokio::sync::mpsc::Sender<Result<AssistantStreamEvent, ProviderError>>,
) -> Result<(), ProviderError> {
    let response = req.json(&body).send().await.map_err(|error| {
        if error.is_timeout() {
            ProviderError::Timeout
        } else if error.is_redirect() {
            ProviderError::Network(ProviderErrorSummary::attested_static("too many redirects"))
        } else {
            ProviderError::Network(ProviderErrorSummary::sanitized(error.to_string()))
        }
    })?;

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
    // The response observer fires before the success check and before any
    // body processing (pi onResponse ordering).
    callbacks
        .notify_response_async(status.as_u16(), &observed_headers)
        .await?;
    if !status.is_success() {
        let body_text = response.text().await.unwrap_or_default();
        return Err(map_pi_messages_status(status, &body_text));
    }
    snapshot.received_response();

    let mut mapper = PiMessagesSseMapper::new(requested_model);
    snapshot.record(&mapper.partial());
    let mut byte_stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut decoder = crate::text_decode::StreamTextDecoder::new();
    let mut received_bytes = false;
    loop {
        let chunk = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(ProviderError::Cancelled),
            chunk = byte_stream.next() => chunk,
        };
        let Some(chunk) = chunk else { break };
        let chunk = chunk.map_err(|error| {
            ProviderError::Network(ProviderErrorSummary::sanitized(error.to_string()))
        })?;
        received_bytes = !chunk.is_empty() || received_bytes;
        buffer.push_str(&decoder.push(&chunk));
        // pi's reader normalizes CRLF before splitting frames; a trailing
        // lone CR waits for its LF in the next chunk.
        buffer = buffer.replace("\r\n", "\n");
        for frame in extract_sse_frames(&mut buffer) {
            if frame.trim().is_empty() {
                continue;
            }
            let event = parse_pi_messages_frame(&frame)?;
            if let Some(event) = event {
                // Raw observer: each parsed provider event, before
                // normalization, read-only (annex raw-observer row).
                callbacks.notify_raw_async(&event).await?;
                let outgoing = mapper.convert(event);
                snapshot.record(&mapper.partial());
                for outgoing in outgoing? {
                    crate::stream::send_or_cancel(&cancel, tx, Ok(outgoing)).await?;
                }
                if let Some(error) = mapper.terminal_error.take() {
                    crate::stream::send_or_cancel(&cancel, tx, Err(snapshot.failure(error)))
                        .await?;
                }
                if mapper.finished {
                    return Ok(());
                }
            }
        }
    }
    buffer.push_str(&decoder.finish());
    buffer = buffer.replace("\r\n", "\n");
    if !buffer.trim().is_empty() {
        let event = parse_pi_messages_frame(&buffer)?;
        if let Some(event) = event {
            callbacks.notify_raw_async(&event).await?;
            let outgoing = mapper.convert(event);
            snapshot.record(&mapper.partial());
            for outgoing in outgoing? {
                crate::stream::send_or_cancel(&cancel, tx, Ok(outgoing)).await?;
            }
            if let Some(error) = mapper.terminal_error.take() {
                crate::stream::send_or_cancel(&cancel, tx, Err(snapshot.failure(error))).await?;
            }
        }
    }
    mapper.received_bytes = received_bytes || !buffer.trim().is_empty();
    if !mapper.finished {
        return Err(ProviderError::StreamError(
            ProviderErrorSummary::attested_static(if mapper.received_bytes {
                "pi-messages stream ended without a terminal event"
            } else {
                "pi-messages response has no body"
            }),
        ));
    }
    Ok(())
}

/// Split complete `\n\n`-delimited frames off the buffer (CRLF was already
/// normalized, matching pi's stream reader).
fn extract_sse_frames(buffer: &mut String) -> Vec<String> {
    let mut frames = Vec::new();
    while let Some(index) = buffer.find("\n\n") {
        let frame: String = buffer.drain(..index + 2).collect();
        frames.push(frame);
    }
    frames
}

fn invalid_event(reason: &str) -> ProviderError {
    ProviderError::StreamError(ProviderErrorSummary::sanitized(format!(
        "Invalid pi-messages streaming event: {reason}"
    )))
}

/// Parse one SSE frame into its `data:` JSON payload. `data: [DONE]` and
/// frames without a data line are ignored (`None`); malformed JSON is a
/// typed failure (pi's reader would throw into an error event).
fn parse_pi_messages_frame(frame: &str) -> Result<Option<serde_json::Value>, ProviderError> {
    let data = frame
        .split('\n')
        .find_map(|line| line.strip_prefix("data:"))
        .map(str::trim);
    let Some(data) = data else {
        return Ok(None);
    };
    if data == "[DONE]" {
        return Ok(None);
    }
    serde_json::from_str::<serde_json::Value>(data)
        .map(Some)
        .map_err(|_| invalid_event("frame payload is not JSON"))
}

fn map_pi_messages_status(status: reqwest::StatusCode, body: &str) -> ProviderError {
    let message = format_pi_messages_http_error(status, body);
    match status.as_u16() {
        401 | 403 => ProviderError::AuthFailed(ProviderErrorSummary::sanitized(message)),
        429 => ProviderError::RateLimited {
            retry_after_ms: None,
        },
        408 | 504 => ProviderError::Timeout,
        code => ProviderError::ServiceFault {
            status: code,
            error_name: None,
            retry_after_ms: None,
            detail: ProviderErrorSummary::sanitized(message),
        },
    }
}

/// pi `formatPiMessagesResponseError`: `"{status} {statusText}: {suffix}"`
/// where the suffix prefers the JSON `error.message` and appends
/// `" ({code})"` when the JSON error carries a string code; the raw body is
/// the fallback suffix.
fn format_pi_messages_http_error(status: reqwest::StatusCode, body: &str) -> String {
    let status_text = status.canonical_reason().unwrap_or_default();
    let mut suffix = body.to_owned();
    let mut code_suffix = String::new();
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(body)
        && let Some(error) = parsed.get("error").filter(|error| error.is_object())
        && let Some(message) = error.get("message").and_then(|value| value.as_str())
    {
        suffix = message.to_owned();
        if let Some(code) = error.get("code").and_then(|value| value.as_str()) {
            code_suffix = format!(" ({code})");
        }
    }
    format!("{} {status_text}: {suffix}{code_suffix}", status.as_u16())
}

/// pi `createEmptyUsage`.
fn empty_usage() -> Usage {
    Usage::default()
}

/// Stateful event converter (pi `createEventConverter`): accumulates the
/// partial assistant message, validating the typed event shapes
/// fail-closed (a delta without its start, an unknown member or a
/// malformed terminal never trusts the server).
struct PiMessagesSseMapper {
    message: crate::message::AssistantMessage,
    /// Sparse content blocks keyed by wire index; finalized in index order
    /// with unstarted holes dropped (pi's sparse array keeps holes, which
    /// are not content).
    blocks: BTreeMap<usize, AssistantContent>,
    tool_json: std::collections::HashMap<usize, String>,
    finished: bool,
    /// Terminal failure carried by an `error` event: sent after the
    /// converted events so the stream's last item is the typed error.
    terminal_error: Option<ProviderError>,
    received_bytes: bool,
}

impl PiMessagesSseMapper {
    fn new(requested_model: &str) -> Self {
        let message = crate::message::AssistantMessage {
            content: Vec::new(),
            api: crate::ApiKind::OpenAi,
            provider: "radius".to_owned(),
            deferred: None,
            model: requested_model.to_owned(),
            response_model: None,
            wire: Some(WireApi::PiMessages),
            response_id: None,
            usage: empty_usage(),
            stop_reason: StopReason::Stop,
            error_message: None,
            timestamp_ms: crate::time::now_ms(),
            diagnostics: Vec::new(),
            duration_ms: None,
            provider_thinking_level: None,
        };
        Self {
            message,
            blocks: BTreeMap::new(),
            tool_json: std::collections::HashMap::new(),
            finished: false,
            terminal_error: None,
            received_bytes: false,
        }
    }

    fn partial(&self) -> crate::message::AssistantMessage {
        let mut partial = self.message.clone();
        partial.content = self.ordered_content();
        partial
    }

    fn ordered_content(&self) -> Vec<AssistantContent> {
        self.blocks.values().cloned().collect()
    }

    fn content_index(event: &serde_json::Value) -> Result<usize, ProviderError> {
        event
            .get("contentIndex")
            .and_then(|index| index.as_u64())
            .and_then(|index| usize::try_from(index).ok())
            .ok_or_else(|| invalid_event("event contentIndex is not a valid index"))
    }

    fn convert(
        &mut self,
        event: serde_json::Value,
    ) -> Result<Vec<AssistantStreamEvent>, ProviderError> {
        let kind = event
            .get("type")
            .and_then(|value| value.as_str())
            .ok_or_else(|| invalid_event("event has no type"))?;
        let mut outgoing = Vec::new();
        match kind {
            "start" => {
                outgoing.push(AssistantStreamEvent::Start {
                    partial: self.partial(),
                });
            }
            "text_start" => {
                let index = Self::content_index(&event)?;
                self.blocks.insert(
                    index,
                    AssistantContent::Text {
                        text: String::new(),
                        signature: None,
                    },
                );
                outgoing.push(AssistantStreamEvent::TextStart {
                    content_index: index,
                    partial: self.partial(),
                });
            }
            "text_delta" => {
                let index = Self::content_index(&event)?;
                let delta = string_member(&event, "delta")?;
                let block = self
                    .blocks
                    .get_mut(&index)
                    .ok_or_else(|| invalid_event("text_delta arrived before its text_start"))?;
                let AssistantContent::Text { text, .. } = block else {
                    return Err(invalid_event("text delta on a non-text block"));
                };
                text.push_str(&delta);
                outgoing.push(AssistantStreamEvent::TextDelta {
                    content_index: index,
                    delta,
                    partial: self.partial(),
                });
            }
            "text_end" => {
                let index = Self::content_index(&event)?;
                let content = string_member(&event, "content")?;
                let signature = optional_string_member(&event, "contentSignature");
                let block = self
                    .blocks
                    .get_mut(&index)
                    .ok_or_else(|| invalid_event("text_end arrived before its text_start"))?;
                *block = AssistantContent::Text {
                    text: content.clone(),
                    signature: signature.clone(),
                };
                outgoing.push(AssistantStreamEvent::TextEnd {
                    content_index: index,
                    content,
                    partial: self.partial(),
                });
            }
            "thinking_start" => {
                let index = Self::content_index(&event)?;
                self.blocks.insert(
                    index,
                    AssistantContent::Thinking {
                        thinking: String::new(),
                        signature: None,
                    },
                );
                outgoing.push(AssistantStreamEvent::ThinkingStart {
                    content_index: index,
                    partial: self.partial(),
                });
            }
            "thinking_delta" => {
                let index = Self::content_index(&event)?;
                let delta = string_member(&event, "delta")?;
                let block = self.blocks.get_mut(&index).ok_or_else(|| {
                    invalid_event("thinking_delta arrived before its thinking_start")
                })?;
                let AssistantContent::Thinking { thinking, .. } = block else {
                    return Err(invalid_event("thinking delta on a non-thinking block"));
                };
                thinking.push_str(&delta);
                outgoing.push(AssistantStreamEvent::ThinkingDelta {
                    content_index: index,
                    delta,
                    partial: self.partial(),
                });
            }
            "thinking_end" => {
                let index = Self::content_index(&event)?;
                let content = string_member(&event, "content")?;
                let signature = optional_string_member(&event, "contentSignature");
                let redacted = event
                    .get("redacted")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false);
                let block = self.blocks.get_mut(&index).ok_or_else(|| {
                    invalid_event("thinking_end arrived before its thinking_start")
                })?;
                *block = if redacted {
                    // pi stores the redacted payload in thinkingSignature and
                    // keeps an empty thinking string.
                    AssistantContent::RedactedThinking {
                        data: signature.clone().unwrap_or_default(),
                    }
                } else {
                    AssistantContent::Thinking {
                        thinking: content.clone(),
                        signature: signature.clone(),
                    }
                };
                outgoing.push(AssistantStreamEvent::ThinkingEnd {
                    content_index: index,
                    content,
                    partial: self.partial(),
                });
            }
            "toolcall_start" => {
                let index = Self::content_index(&event)?;
                let id = string_member(&event, "id")?;
                let tool_name = string_member(&event, "toolName")?;
                self.blocks.insert(
                    index,
                    AssistantContent::ToolCall {
                        tool_call: ToolCall {
                            id,
                            name: tool_name,
                            arguments: String::new(),
                            arguments_error: None,
                            signature: None,
                        },
                    },
                );
                self.tool_json.insert(index, String::new());
                outgoing.push(AssistantStreamEvent::ToolCallStart {
                    content_index: index,
                    partial: self.partial(),
                });
            }
            "toolcall_delta" => {
                let index = Self::content_index(&event)?;
                let delta = string_member(&event, "delta")?;
                let json = self
                    .tool_json
                    .get_mut(&index)
                    .ok_or_else(|| invalid_event("toolcall_delta arrived before its start"))?;
                json.push_str(&delta);
                let parsed = crate::message::parse_streaming_json(json);
                if let Some(AssistantContent::ToolCall { tool_call, .. }) =
                    self.blocks.get_mut(&index)
                {
                    tool_call.arguments = parsed.to_string();
                }
                outgoing.push(AssistantStreamEvent::ToolCallDelta {
                    content_index: index,
                    delta,
                    partial: self.partial(),
                });
            }
            "toolcall_end" => {
                let index = Self::content_index(&event)?;
                let tool_call = event
                    .get("toolCall")
                    .ok_or_else(|| invalid_event("toolcall_end has no toolCall"))?;
                let id = tool_call
                    .get("id")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| invalid_event("toolCall.id is not a string"))?
                    .to_owned();
                let name = tool_call
                    .get("name")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| invalid_event("toolCall.name is not a string"))?
                    .to_owned();
                let arguments = tool_call
                    .get("arguments")
                    .and_then(|value| value.as_object())
                    .ok_or_else(|| invalid_event("toolCall.arguments is not an object"))?;
                let signature = tool_call
                    .get("thoughtSignature")
                    .and_then(|value| value.as_str())
                    .map(str::to_owned);
                let finalized = ToolCall {
                    id,
                    name,
                    arguments: serde_json::Value::Object(arguments.clone()).to_string(),
                    arguments_error: None,
                    signature,
                };
                // The end event is authoritative, even when no deltas were
                // sent or its arguments differ from the streamed preview.
                self.tool_json.remove(&index);
                self.blocks.insert(
                    index,
                    AssistantContent::ToolCall {
                        tool_call: finalized.clone(),
                    },
                );
                outgoing.push(AssistantStreamEvent::ToolCallEnd {
                    content_index: index,
                    tool_call: finalized,
                    partial: self.partial(),
                });
            }
            "done" | "error" => {
                self.message.usage = parse_usage(&event)?;
                if let Some(id) = optional_string_member(&event, "responseId") {
                    self.message.response_id = Some(id);
                }
                if let Some(level) = optional_string_member(&event, "providerThinkingLevel") {
                    self.message.provider_thinking_level = Some(level);
                }
                if let Some(rewrite) = event.get("rewrite") {
                    self.append_rewrite_diagnostic(rewrite)?;
                }
                self.message.content = self.ordered_content();
                if kind == "done" {
                    let reason = match event
                        .get("reason")
                        .and_then(|value| value.as_str())
                        .ok_or_else(|| invalid_event("done event has no reason"))?
                    {
                        "stop" => StopReason::Stop,
                        "length" => StopReason::Length,
                        "toolUse" => StopReason::ToolUse,
                        other => {
                            return Err(invalid_event(&format!(
                                "done reason '{other}' is not a success stop reason"
                            )));
                        }
                    };
                    self.message.stop_reason = reason;
                    self.finished = true;
                    outgoing.push(AssistantStreamEvent::Done {
                        reason,
                        message: self.message.clone(),
                    });
                } else {
                    let aborted =
                        event.get("reason").and_then(|value| value.as_str()) == Some("aborted");
                    self.message.stop_reason = if aborted {
                        StopReason::Aborted
                    } else {
                        StopReason::Error
                    };
                    self.message.error_message = optional_string_member(&event, "errorMessage");
                    self.finished = true;
                    self.terminal_error = Some(if aborted {
                        ProviderError::Cancelled
                    } else {
                        ProviderError::StreamError(ProviderErrorSummary::sanitized(
                            self.message
                                .error_message
                                .clone()
                                .unwrap_or_else(|| "pi-messages error event".to_owned()),
                        ))
                    });
                }
            }
            other => {
                return Err(invalid_event(&format!("unknown event type '{other}'")));
            }
        }
        Ok(outgoing)
    }

    fn append_rewrite_diagnostic(
        &mut self,
        rewrite: &serde_json::Value,
    ) -> Result<(), ProviderError> {
        let policy_id = rewrite
            .get("policyId")
            .and_then(|value| value.as_str())
            .ok_or_else(|| invalid_event("rewrite has no policyId"))?;
        let detail = serde_json::to_string(rewrite)
            .map_err(|_| invalid_event("rewrite impact is not serializable"))?;
        self.message
            .diagnostics
            .push(crate::stream::AssistantDiagnostic {
                kind: "pi_messages_rewrite".to_owned(),
                reason: policy_id.to_owned(),
                detail: Some(detail),
            });
        Ok(())
    }
}

fn string_member(event: &serde_json::Value, member: &str) -> Result<String, ProviderError> {
    event
        .get(member)
        .and_then(|value| value.as_str())
        .map(str::to_owned)
        .ok_or_else(|| invalid_event(&format!("event member '{member}' is not a string")))
}

fn optional_string_member(event: &serde_json::Value, member: &str) -> Option<String> {
    event
        .get(member)
        .and_then(|value| value.as_str())
        .map(str::to_owned)
}

fn parse_usage(event: &serde_json::Value) -> Result<Usage, ProviderError> {
    let usage = event
        .get("usage")
        .and_then(|value| value.as_object())
        .ok_or_else(|| invalid_event("terminal event has no usage object"))?;
    let number = |name: &str| -> Result<u32, ProviderError> {
        usage
            .get(name)
            .and_then(|value| value.as_u64())
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| invalid_event(&format!("usage member '{name}' is not a number")))
    };
    let optional = |name: &str| -> Result<Option<u64>, ProviderError> {
        match usage.get(name) {
            None | Some(serde_json::Value::Null) => Ok(None),
            Some(value) => value
                .as_u64()
                .map(Some)
                .ok_or_else(|| invalid_event(&format!("usage member '{name}' is not a number"))),
        }
    };
    Ok(Usage {
        input_tokens: number("input")?,
        output_tokens: number("output")?,
        cache_read_tokens: number("cacheRead")?,
        cache_write_tokens: number("cacheWrite")?,
        cache_write_1h_tokens: optional("cacheWrite1h")?,
        reasoning_tokens: optional("reasoning")?,
        total_tokens: optional("totalTokens")?,
        service_tier: None,
        effective_service_tier: None,
        reported: true,
    })
}

// ---------------------------------------------------------------------------
// Public complete-Chat registration (Models four entries)
// ---------------------------------------------------------------------------

/// The registered `pi-messages` complete-Chat adapter.
#[derive(Clone)]
pub struct PiMessagesChatApi {
    api_id: crate::models::ApiId,
    provider: PiMessagesProvider,
    model: crate::provider::ModelInfo,
}

impl std::fmt::Debug for PiMessagesChatApi {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PiMessagesChatApi")
            .field("api_id", &self.api_id)
            .finish_non_exhaustive()
    }
}

impl PiMessagesChatApi {
    /// Build the adapter over one complete Chat model record. The record
    /// must be a valid `pi-messages` wire record; anything else is a typed
    /// rejection.
    pub fn new(
        provider: PiMessagesProvider,
        model: crate::provider::ModelInfo,
    ) -> Result<Self, ProviderError> {
        if model.wire_api != WireApi::PiMessages
            || model.id.trim().is_empty()
            || model.validate().is_err()
        {
            return Err(ProviderError::Config(
                ProviderErrorSummary::attested_static(
                    "pi-messages adapters require a valid complete Chat model record",
                ),
            ));
        }
        Ok(Self {
            api_id: crate::models::ApiId::new("pi-messages"),
            provider,
            model,
        })
    }
}

impl crate::models::ChatApi<PiMessagesOptions> for PiMessagesChatApi {
    fn validate_simple_options(
        &self,
        model: &crate::provider::ModelInfo,
        options: &crate::models::SimpleOptions,
    ) -> Result<(), ProviderError> {
        Provider::validate_simple_options(&self.provider, model, options)
    }

    fn api_id(&self) -> &crate::models::ApiId {
        &self.api_id
    }

    fn provider_id(&self) -> &str {
        "radius"
    }

    fn complete_record(&self) -> crate::provider::ModelInfo {
        self.model.clone()
    }

    fn simple_options(&self) -> PiMessagesOptions {
        PiMessagesOptions::from_simple(&crate::models::SimpleOptions::default())
    }

    fn simple_options_for(&self, options: &crate::models::SimpleOptions) -> PiMessagesOptions {
        PiMessagesOptions::from_simple(options)
    }

    fn validate_native_request(
        &self,
        model: &crate::provider::ModelInfo,
        request: &Request,
        _options: &PiMessagesOptions,
    ) -> Result<(), ProviderError> {
        if model.wire_api != WireApi::PiMessages {
            return Err(ProviderError::Config(
                ProviderErrorSummary::attested_static(
                    "pi-messages adapters require a valid complete Chat model record",
                ),
            ));
        }
        crate::provider::validate_request_structure_for_model(
            self.provider_id(),
            Some(model),
            request,
        )
    }

    fn stream_prepared(
        &self,
        request: Request,
        options: PiMessagesOptions,
        auth: crate::auth::ResolvedAuth,
    ) -> EventStream {
        self.stream_model_resource_prepared(self.model.clone(), request, options, auth, None)
    }

    fn stream_model_resource_prepared(
        &self,
        model: crate::provider::ModelInfo,
        mut request: Request,
        options: PiMessagesOptions,
        auth: crate::auth::ResolvedAuth,
        _resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        if let Err(error) = self.validate_native_request(&model, &request, &options) {
            return Box::pin(futures_util::stream::once(async move { Err(error) }));
        }
        if !options.is_simple() {
            // Native reasoning owns the wire projection; neutral thinking
            // fields stay unused on this path.
            request.thinking = Default::default();
        }
        let provider = self
            .provider
            .clone()
            .with_models(vec![model])
            .with_native_options(options);
        provider.stream_prepared(request, auth)
    }
}
