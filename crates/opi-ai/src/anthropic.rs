//! Anthropic Messages SSE provider (S8.1).

use std::sync::Arc;

use futures_util::{StreamExt, stream};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use tokio_util::sync::CancellationToken;

use crate::auth::{AuthResolver, AuthScheme, ResolvedAuth, StaticAuthResolver};
use crate::http::HttpClient;
use crate::message::{AssistantContent, AssistantMessage, ToolCall};
use crate::provider::{EventStream, ModelInfo, Provider, ProviderError, Request};
use crate::stream::{AssistantStreamEvent, StopReason, Usage};

// ---------------------------------------------------------------------------
// SSE line parser
// ---------------------------------------------------------------------------

/// Known Anthropic SSE event types.
static ANTHROPIC_EVENTS: &[&str] = &[
    "message_start",
    "content_block_start",
    "content_block_delta",
    "content_block_stop",
    "message_delta",
    "message_stop",
    "error",
];

/// A raw SSE frame extracted from the byte stream.
struct SseFrame {
    event: String,
    data: String,
}

/// Parsed result for a single SSE frame  - either a valid event or a parse error.
pub enum ParsedEvent {
    Valid(AnthropicEvent),
    Malformed {
        event_type: String,
        data: String,
        error: String,
    },
}

/// Parse SSE text into frames, then deserialize each frame as an AnthropicEvent.
/// Returns [`ParsedEvent`] so callers can decide how to handle malformed data.
pub fn parse_sse_events(input: &str) -> impl Iterator<Item = ParsedEvent> + '_ {
    parse_frames(input).filter_map(|frame| {
        if !ANTHROPIC_EVENTS.contains(&frame.event.as_str()) {
            return None;
        }
        match serde_json::from_str::<AnthropicRawEvent>(&frame.data) {
            Ok(raw) => Some(ParsedEvent::Valid(AnthropicEvent::from_raw(raw))),
            Err(e) => Some(ParsedEvent::Malformed {
                event_type: frame.event.clone(),
                data: frame.data.clone(),
                error: e.to_string(),
            }),
        }
    })
}

fn parse_frames(input: &str) -> impl Iterator<Item = SseFrame> + '_ {
    let mut lines = input.split('\n').peekable();
    std::iter::from_fn(move || {
        let mut event = None;
        let mut data_parts: Vec<&str> = Vec::new();

        loop {
            match lines.next() {
                Some(line) if line.starts_with(':') => continue,
                Some(line) if line.trim_end_matches('\r').is_empty() => {
                    if event.is_some() || !data_parts.is_empty() {
                        return Some(SseFrame {
                            event: event.take().unwrap_or_else(|| "message".into()),
                            data: data_parts.join("\n"),
                        });
                    }
                    continue;
                }
                Some(line) => {
                    let line = line.trim_end_matches('\r');
                    let (field, value) = if let Some(idx) = line.find(':') {
                        let v = if line.get(idx + 1..idx + 2) == Some(" ") {
                            &line[idx + 2..]
                        } else {
                            &line[idx + 1..]
                        };
                        (&line[..idx], v)
                    } else {
                        (line, "")
                    };
                    match field {
                        "event" => event = Some(value.to_string()),
                        "data" => data_parts.push(value),
                        _ => {}
                    }
                }
                None => {
                    if event.is_some() || !data_parts.is_empty() {
                        return Some(SseFrame {
                            event: event.take().unwrap_or_else(|| "message".into()),
                            data: data_parts.join("\n"),
                        });
                    }
                    return None;
                }
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Anthropic raw wire types (deserialized from SSE data payloads)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum AnthropicRawEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: RawMessage },
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: usize,
        content_block: RawContentBlock,
    },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: usize, delta: RawDelta },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop {
        #[allow(dead_code)]
        index: usize,
    },
    #[serde(rename = "message_delta")]
    MessageDelta {
        delta: RawMessageDelta,
        usage: RawUsage,
    },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(rename = "error")]
    Error { error: RawErrorBody },
}

#[derive(Debug, Deserialize)]
struct RawMessage {
    id: Option<String>,
    model: Option<String>,
    usage: Option<RawUsage>,
    #[allow(dead_code)]
    content: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct RawUsage {
    input_tokens: Option<u32>,
    output_tokens: Option<u32>,
    cache_read_input_tokens: Option<u32>,
    cache_creation_input_tokens: Option<u32>,
}

impl RawUsage {
    fn into_usage(self) -> Usage {
        if self.input_tokens.is_some()
            || self.output_tokens.is_some()
            || self.cache_read_input_tokens.is_some()
            || self.cache_creation_input_tokens.is_some()
        {
            Usage::reported(
                self.input_tokens.unwrap_or(0),
                self.output_tokens.unwrap_or(0),
                self.cache_read_input_tokens.unwrap_or(0),
                self.cache_creation_input_tokens.unwrap_or(0),
            )
        } else {
            Usage::unknown()
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum RawContentBlock {
    #[serde(rename = "text")]
    Text {
        #[allow(dead_code)]
        text: Option<String>,
    },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        #[serde(default)]
        #[allow(dead_code)]
        input: serde_json::Value,
    },
    #[serde(rename = "thinking")]
    Thinking {
        #[allow(dead_code)]
        thinking: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[allow(clippy::enum_variant_names)] // names mirror Anthropic API delta types
enum RawDelta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    #[serde(rename = "input_json_delta")]
    InputJsonDelta { partial_json: String },
    #[serde(rename = "thinking_delta")]
    ThinkingDelta { thinking: String },
}

#[derive(Debug, Deserialize)]
struct RawMessageDelta {
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawErrorBody {
    #[allow(dead_code)]
    r#type: Option<String>,
    message: Option<String>,
}

// ---------------------------------------------------------------------------
// Public AnthropicEvent enum
// ---------------------------------------------------------------------------

/// A parsed Anthropic SSE event.
#[derive(Debug, Clone)]
pub enum AnthropicEvent {
    MessageStart {
        id: Option<String>,
        model: Option<String>,
        usage: Usage,
    },
    ContentBlockStart {
        index: usize,
        block_type: ContentBlockType,
    },
    ContentBlockDelta {
        index: usize,
        delta: DeltaData,
    },
    ContentBlockStop {
        index: usize,
    },
    MessageDelta {
        stop_reason: Option<String>,
        usage: Usage,
    },
    MessageStop,
    Error {
        message: Option<String>,
    },
}

/// Type of content block started.
#[derive(Debug, Clone)]
pub enum ContentBlockType {
    Text,
    ToolUse { id: String, name: String },
    Thinking,
}

/// Delta data from content_block_delta.
#[derive(Debug, Clone)]
pub enum DeltaData {
    Text { text: String },
    InputJson { partial_json: String },
    Thinking { thinking: String },
}

impl AnthropicEvent {
    fn from_raw(raw: AnthropicRawEvent) -> Self {
        match raw {
            AnthropicRawEvent::MessageStart { message } => {
                let usage = message
                    .usage
                    .map(RawUsage::into_usage)
                    .unwrap_or_else(Usage::unknown);
                AnthropicEvent::MessageStart {
                    id: message.id,
                    model: message.model,
                    usage,
                }
            }
            AnthropicRawEvent::ContentBlockStart {
                index,
                content_block,
            } => {
                let block_type = match content_block {
                    RawContentBlock::Text { .. } => ContentBlockType::Text,
                    RawContentBlock::ToolUse { id, name, .. } => {
                        ContentBlockType::ToolUse { id, name }
                    }
                    RawContentBlock::Thinking { .. } => ContentBlockType::Thinking,
                };
                AnthropicEvent::ContentBlockStart { index, block_type }
            }
            AnthropicRawEvent::ContentBlockDelta { index, delta } => {
                let delta_data = match delta {
                    RawDelta::TextDelta { text } => DeltaData::Text { text },
                    RawDelta::InputJsonDelta { partial_json } => {
                        DeltaData::InputJson { partial_json }
                    }
                    RawDelta::ThinkingDelta { thinking } => DeltaData::Thinking { thinking },
                };
                AnthropicEvent::ContentBlockDelta {
                    index,
                    delta: delta_data,
                }
            }
            AnthropicRawEvent::ContentBlockStop { index } => {
                AnthropicEvent::ContentBlockStop { index }
            }
            AnthropicRawEvent::MessageDelta { delta, usage } => AnthropicEvent::MessageDelta {
                stop_reason: delta.stop_reason,
                usage: usage.into_usage(),
            },
            AnthropicRawEvent::MessageStop => AnthropicEvent::MessageStop,
            AnthropicRawEvent::Error { error } => AnthropicEvent::Error {
                message: error.message,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Stateful event mapper: AnthropicEvent ->AssistantStreamEvent
// ---------------------------------------------------------------------------

/// Tracks content block state and accumulates the final message.
pub struct AnthropicMapper {
    partial: AssistantMessage,
    blocks: Vec<BlockState>,
    saw_done: bool,
}

enum BlockState {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        partial_json: String,
    },
    Thinking {
        thinking: String,
    },
}

impl Default for AnthropicMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl AnthropicMapper {
    pub fn new() -> Self {
        Self {
            partial: empty_assistant_message(),
            blocks: Vec::new(),
            saw_done: false,
        }
    }

    /// Process one Anthropic event, returning zero or more stream events.
    pub fn process(&mut self, event: AnthropicEvent) -> Vec<AssistantStreamEvent> {
        if self.saw_done {
            return Vec::new();
        }
        match event {
            AnthropicEvent::MessageStart { id, model, usage } => {
                self.partial.response_id = id;
                if let Some(m) = model {
                    self.partial.model = m;
                }
                self.partial.usage = usage;
                let start = self.partial.clone();
                vec![AssistantStreamEvent::Start { partial: start }]
            }
            AnthropicEvent::ContentBlockStart {
                index: _,
                block_type,
            } => {
                let content_index = self.blocks.len();
                match block_type {
                    ContentBlockType::Text => {
                        self.blocks.push(BlockState::Text {
                            text: String::new(),
                        });
                        self.partial.content.push(AssistantContent::Text {
                            text: String::new(),
                        });
                        vec![AssistantStreamEvent::TextStart {
                            content_index,
                            partial: self.partial.clone(),
                        }]
                    }
                    ContentBlockType::ToolUse { id, name } => {
                        self.blocks.push(BlockState::ToolUse {
                            id: id.clone(),
                            name: name.clone(),
                            partial_json: String::new(),
                        });
                        self.partial.content.push(AssistantContent::ToolCall {
                            tool_call: ToolCall {
                                id,
                                name,
                                arguments: String::new(),
                            },
                        });
                        vec![AssistantStreamEvent::ToolCallStart {
                            content_index,
                            partial: self.partial.clone(),
                        }]
                    }
                    ContentBlockType::Thinking => {
                        self.blocks.push(BlockState::Thinking {
                            thinking: String::new(),
                        });
                        self.partial.content.push(AssistantContent::Thinking {
                            thinking: String::new(),
                        });
                        vec![AssistantStreamEvent::ThinkingStart {
                            content_index,
                            partial: self.partial.clone(),
                        }]
                    }
                }
            }
            AnthropicEvent::ContentBlockDelta { index: _, delta } => {
                let content_index = self.blocks.len() - 1;
                match delta {
                    DeltaData::Text { text } => {
                        if let Some(BlockState::Text { text: acc }) = self.blocks.last_mut() {
                            acc.push_str(&text);
                        }
                        if let Some(AssistantContent::Text { text: acc }) =
                            self.partial.content.last_mut()
                        {
                            acc.push_str(&text);
                        }
                        vec![AssistantStreamEvent::TextDelta {
                            content_index,
                            delta: text,
                            partial: self.partial.clone(),
                        }]
                    }
                    DeltaData::InputJson { partial_json } => {
                        if let Some(BlockState::ToolUse {
                            partial_json: acc, ..
                        }) = self.blocks.last_mut()
                        {
                            acc.push_str(&partial_json);
                        }
                        vec![AssistantStreamEvent::ToolCallDelta {
                            content_index,
                            delta: partial_json,
                            partial: self.partial.clone(),
                        }]
                    }
                    DeltaData::Thinking { thinking } => {
                        if let Some(BlockState::Thinking { thinking: acc }) = self.blocks.last_mut()
                        {
                            acc.push_str(&thinking);
                        }
                        if let Some(AssistantContent::Thinking { thinking: acc }) =
                            self.partial.content.last_mut()
                        {
                            acc.push_str(&thinking);
                        }
                        vec![AssistantStreamEvent::ThinkingDelta {
                            content_index,
                            delta: thinking,
                            partial: self.partial.clone(),
                        }]
                    }
                }
            }
            AnthropicEvent::ContentBlockStop { index: _ } => {
                let content_index = self.blocks.len() - 1;
                match self.blocks.last() {
                    Some(BlockState::Text { text }) => {
                        let content = text.clone();
                        vec![AssistantStreamEvent::TextEnd {
                            content_index,
                            content,
                            partial: self.partial.clone(),
                        }]
                    }
                    Some(BlockState::ToolUse {
                        id,
                        name,
                        partial_json,
                    }) => {
                        let tool_call = ToolCall {
                            id: id.clone(),
                            name: name.clone(),
                            arguments: partial_json.clone(),
                        };
                        // Update the partial message's tool call with final arguments
                        if let Some(AssistantContent::ToolCall { tool_call: tc }) =
                            self.partial.content.last_mut()
                        {
                            tc.arguments = partial_json.clone();
                        }
                        vec![AssistantStreamEvent::ToolCallEnd {
                            content_index,
                            tool_call,
                            partial: self.partial.clone(),
                        }]
                    }
                    Some(BlockState::Thinking { thinking }) => {
                        let content = thinking.clone();
                        vec![AssistantStreamEvent::ThinkingEnd {
                            content_index,
                            content,
                            partial: self.partial.clone(),
                        }]
                    }
                    None => Vec::new(),
                }
            }
            AnthropicEvent::MessageDelta { stop_reason, usage } => {
                self.partial.stop_reason = map_stop_reason(stop_reason.as_deref());
                if usage.is_reported() {
                    self.partial.usage.reported = true;
                }
                if usage.input_tokens > 0 {
                    self.partial.usage.input_tokens = usage.input_tokens;
                }
                if usage.output_tokens > 0 {
                    self.partial.usage.output_tokens = usage.output_tokens;
                }
                if usage.cache_read_tokens > 0 {
                    self.partial.usage.cache_read_tokens = usage.cache_read_tokens;
                }
                if usage.cache_write_tokens > 0 {
                    self.partial.usage.cache_write_tokens = usage.cache_write_tokens;
                }
                // message_delta doesn't emit a stream event; Done comes from message_stop
                Vec::new()
            }
            AnthropicEvent::MessageStop => {
                self.saw_done = true;
                vec![AssistantStreamEvent::Done {
                    reason: self.partial.stop_reason,
                    message: self.partial.clone(),
                }]
            }
            AnthropicEvent::Error { message } => {
                self.saw_done = true;
                let mut err_msg = self.partial.clone();
                err_msg.error_message = message;
                vec![AssistantStreamEvent::Error {
                    reason: StopReason::Error,
                    message: err_msg,
                }]
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Stop reason mapping
// ---------------------------------------------------------------------------

fn map_stop_reason(raw: Option<&str>) -> StopReason {
    match raw {
        Some("end_turn") | Some("stop_sequence") | Some("pause_turn") => StopReason::Stop,
        Some("max_tokens") => StopReason::Length,
        Some("tool_use") => StopReason::ToolUse,
        Some("refusal") | Some("sensitive") => StopReason::Error,
        _ => StopReason::Error,
    }
}

fn empty_assistant_message() -> AssistantMessage {
    AssistantMessage {
        content: Vec::new(),
        api: crate::ApiKind::Anthropic,
        provider: "anthropic".into(),
        model: String::new(),
        response_model: None,
        response_id: None,
        usage: Usage::default(),
        stop_reason: StopReason::Stop,
        error_message: None,
        timestamp_ms: crate::time::now_ms(),
    }
}

// ---------------------------------------------------------------------------
// AnthropicProvider
// ---------------------------------------------------------------------------

/// The `anthropic-beta` tag Anthropic OAuth requires on every request issued
/// under a Bearer (OAuth) credential. API-key requests do not send it. Gated on
/// `AuthScheme::Bearer` in `stream_http` — the only Bearer path for Anthropic —
/// so no extra-headers field or flag is needed. RESIDUAL: the exact tag value
/// was set from the upstream pi source at slice-5 time; it must be re-confirmed
/// against a current Anthropic OAuth reference before a production login is
/// advertised, since beta tags rotate.
const ANTHROPIC_OAUTH_BETA_HEADER: &str = "oauth-2025-04-20";

/// Concrete Anthropic Messages API provider.
pub struct AnthropicProvider {
    auth: Arc<dyn AuthResolver>,
    base_url: String,
    models: Vec<ModelInfo>,
    client: Arc<HttpClient>,
}

impl AnthropicProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self::with_client(api_key, base_url, Arc::new(HttpClient::new()))
    }

    /// Create with a shared HTTP client for connection pooling.
    pub fn with_client(api_key: String, base_url: Option<String>, client: Arc<HttpClient>) -> Self {
        let auth = Arc::new(StaticAuthResolver::new(
            AuthScheme::ApiKey,
            SecretString::from(api_key),
        ));
        Self::with_auth(auth, base_url, client)
    }

    /// Build with an injected per-request auth resolver (Phase 14.2). The
    /// resolver is consulted inside `Provider::stream` immediately before each
    /// HTTP request; `new`/`with_client` wrap a fixed key in a
    /// [`StaticAuthResolver`]. OAuth/env-backed resolution is supplied through
    /// this entry point by `opi-coding-agent`.
    pub fn with_auth(
        auth: Arc<dyn AuthResolver>,
        base_url: Option<String>,
        client: Arc<HttpClient>,
    ) -> Self {
        let base_url = base_url.unwrap_or_else(|| "https://api.anthropic.com".into());
        let models = vec![
            ModelInfo {
                id: "claude-sonnet-4-5-20250514".into(),
                display_name: "Claude Sonnet 4.5".into(),
                context_window: 200000,
                max_output_tokens: 8192,
                supports_images: true,
                supports_streaming: true,
                supports_thinking: true,
            },
            ModelInfo {
                id: "claude-opus-4-20250514".into(),
                display_name: "Claude Opus 4".into(),
                context_window: 200000,
                max_output_tokens: 8192,
                supports_images: true,
                supports_streaming: true,
                supports_thinking: true,
            },
            ModelInfo {
                id: "claude-haiku-4-5-20250514".into(),
                display_name: "Claude Haiku 4.5".into(),
                context_window: 200000,
                max_output_tokens: 8192,
                supports_images: true,
                supports_streaming: true,
                supports_thinking: true,
            },
        ];
        Self {
            auth,
            base_url,
            models,
            client,
        }
    }

    /// Access the shared HTTP client (for testing client reuse).
    pub fn http_client(&self) -> &Arc<HttpClient> {
        &self.client
    }

    /// Build the Anthropic Messages API request body.
    pub fn build_request_body(&self, request: &Request) -> serde_json::Value {
        let model_id = request
            .model
            .split_once(':')
            .map(|(_, id)| id)
            .unwrap_or(&request.model);
        let mut body = serde_json::json!({
            "model": model_id,
            "stream": true,
            "messages": serialize_messages(&request.messages),
        });

        if let Some(ref system) = request.system {
            body["system"] = serde_json::Value::String(system.clone());
        }
        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = serde_json::Value::Number(max_tokens.into());
        } else {
            body["max_tokens"] = serde_json::Value::Number(8192.into());
        }
        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::Number::from_f64(temp)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null);
        }
        if !request.tools.is_empty() {
            body["tools"] = serde_json::Value::Array(
                request
                    .tools
                    .iter()
                    .map(|t| {
                        serde_json::json!({
                            "name": t.name,
                            "description": t.description,
                            "input_schema": t.input_schema,
                        })
                    })
                    .collect(),
            );
        }
        if !request.stop_sequences.is_empty() {
            body["stop_sequences"] = serde_json::Value::Array(
                request
                    .stop_sequences
                    .iter()
                    .map(|s| serde_json::Value::String(s.clone()))
                    .collect(),
            );
        }
        if request.thinking.enabled {
            body["thinking"] = serde_json::json!({
                "type": "enabled",
                "budget_tokens": request.thinking.budget_tokens.unwrap_or(10000),
            });
        }
        body
    }

    /// Stream events from a raw SSE response body.
    pub fn stream_from_sse(&self, sse_body: &str, cancel: CancellationToken) -> EventStream {
        let mut mapper = AnthropicMapper::new();
        let mut stream_events: Vec<Result<AssistantStreamEvent, ProviderError>> = Vec::new();
        for parsed in parse_sse_events(sse_body) {
            match parsed {
                ParsedEvent::Valid(event) => {
                    stream_events.extend(mapper.process(event).into_iter().map(Ok));
                }
                ParsedEvent::Malformed {
                    event_type, error, ..
                } => {
                    stream_events.push(Err(ProviderError::StreamError(format!(
                        "malformed SSE event '{event_type}': {error}"
                    ))));
                }
            }
        }

        let _cancel = cancel; // used by the real HTTP path
        Box::pin(stream::iter(stream_events))
    }

    /// Real HTTP streaming: POST to Anthropic Messages API and parse SSE from the byte stream.
    #[allow(clippy::too_many_arguments)]
    async fn stream_http(
        client: reqwest::Client,
        resolved: ResolvedAuth,
        base_url: String,
        body: &serde_json::Value,
        cancel: CancellationToken,
        timeout: Option<std::time::Duration>,
        extra_headers: Vec<(String, String)>,
        tx: &tokio::sync::mpsc::Sender<Result<AssistantStreamEvent, ProviderError>>,
    ) -> Result<(), ProviderError> {
        let secret = resolved.secret.expose_secret();
        let mut request = client
            .post(format!("{base_url}/v1/messages"))
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .body(serde_json::to_string(body).unwrap_or_default());
        // Apply extra headers from Request (validated by caller).
        for (name, value) in &extra_headers {
            request = request.header(name.as_str(), value.as_str());
        }
        // Apply per-request timeout.
        if let Some(d) = timeout {
            request = request.timeout(d);
        }
        let request = match resolved.scheme {
            AuthScheme::ApiKey => request.header("x-api-key", secret),
            // OAuth Bearer credential: the required beta header rides the same
            // arm. API-key construction never enters this arm.
            AuthScheme::Bearer => request
                .header("authorization", format!("Bearer {secret}"))
                .header("anthropic-beta", ANTHROPIC_OAUTH_BETA_HEADER),
        };
        let response = request
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    ProviderError::Timeout
                } else {
                    ProviderError::Network(e.to_string())
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let headers = response.headers().clone();
            let error_body = response.text().await.unwrap_or_default();
            return Err(map_http_status(
                status,
                &error_body,
                &headers,
                resolved.scheme,
            ));
        }

        let mut byte_stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut mapper = AnthropicMapper::new();

        loop {
            let chunk = tokio::select! {
                _ = cancel.cancelled() => {
                    return Ok(());
                }
                chunk = byte_stream.next() => {
                    match chunk {
                        Some(c) => c,
                        None => break, // stream ended
                    }
                }
            };

            let chunk = chunk.map_err(|e| ProviderError::StreamError(e.to_string()))?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            for parsed in drain_sse_events(&mut buffer) {
                match parsed {
                    ParsedEvent::Valid(event) => {
                        for stream_event in mapper.process(event) {
                            if tx.send(Ok(stream_event)).await.is_err() {
                                return Ok(()); // receiver dropped
                            }
                        }
                    }
                    ParsedEvent::Malformed {
                        event_type, error, ..
                    } => {
                        let err = ProviderError::StreamError(format!(
                            "malformed SSE event '{event_type}': {error}"
                        ));
                        if tx.send(Err(err)).await.is_err() {
                            return Ok(());
                        }
                    }
                }
            }
        }

        // Stream ended without a terminal event  - surface as provider protocol error
        if !mapper.saw_done {
            let err = ProviderError::StreamError(
                "stream ended without a terminal event (message_stop or error)".into(),
            );
            let _ = tx.send(Err(err)).await;
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Streaming helpers
// ---------------------------------------------------------------------------

/// Wrapper to adapt `tokio::sync::mpsc::Receiver` to `futures_core::Stream`.
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

/// Drain complete SSE events from a growing buffer.
/// Returns parsed [`ParsedEvent`]s and leaves incomplete data in the buffer.
/// Normalizes CRLF (`\r\n`) to LF (`\n`) to handle real-world HTTP SSE streams.
fn drain_sse_events(buffer: &mut String) -> Vec<ParsedEvent> {
    // Normalize CRLF to LF for consistent delimiter detection
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

/// Map an HTTP status code + body + headers to a `ProviderError`.
///
/// `scheme` makes the 401 path scheme-aware: a 401 on a Bearer (OAuth)
/// credential is typed non-retryable `CredentialRevoked` with the body DROPPED
/// (an enterprise proxy may echo the submitted Bearer in a 401 body, so the body
/// never reaches a Display string), while an API-key 401 stays `AuthFailed` with
/// a redacted excerpt. 403 is `AuthFailed` for both schemes.
fn map_http_status(
    status: reqwest::StatusCode,
    body: &str,
    headers: &reqwest::header::HeaderMap,
    scheme: AuthScheme,
) -> ProviderError {
    match status.as_u16() {
        401 if scheme == AuthScheme::Bearer => ProviderError::CredentialRevoked {
            // Body intentionally dropped.
            provider_id: "anthropic".to_owned(),
        },
        401 => ProviderError::AuthFailed(format!(
            "authentication failed: {}",
            crate::http::safe_excerpt(body)
        )),
        403 => ProviderError::AuthFailed(format!(
            "access denied: {}",
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

fn serialize_messages(messages: &[crate::message::Message]) -> serde_json::Value {
    serde_json::Value::Array(
        messages
            .iter()
            .map(|msg| match msg {
                crate::message::Message::User(u) => {
                    let content: Vec<serde_json::Value> = u
                        .content
                        .iter()
                        .map(|c| match c {
                            crate::message::InputContent::Text { text } => {
                                serde_json::json!({"type": "text", "text": text})
                            }
                            crate::message::InputContent::Image { source, media_type } => {
                                match source {
                                    crate::message::ImageSource::Url { url } => {
                                        serde_json::json!({
                                            "type": "image",
                                            "source": {
                                                "type": "url",
                                                "url": url,
                                            }
                                        })
                                    }
                                    crate::message::ImageSource::Base64 { data } => {
                                        serde_json::json!({
                                            "type": "image",
                                            "source": {
                                                "type": "base64",
                                                "media_type": media_type.as_str(),
                                                "data": data,
                                            }
                                        })
                                    }
                                    crate::message::ImageSource::Bytes { data } => {
                                        serde_json::json!({
                                            "type": "image",
                                            "source": {
                                                "type": "base64",
                                                "media_type": media_type.as_str(),
                                                "data": base64::Engine::encode(
                                                    &base64::engine::general_purpose::STANDARD,
                                                    data,
                                                ),
                                            }
                                        })
                                    }
                                }
                            }
                        })
                        .collect();
                    serde_json::json!({"role": "user", "content": content})
                }
                crate::message::Message::Assistant(a) => {
                    let content: Vec<serde_json::Value> = a
                        .content
                        .iter()
                        .map(|c| match c {
                            AssistantContent::Text { text } => {
                                serde_json::json!({"type": "text", "text": text})
                            }
                            AssistantContent::ToolCall { tool_call } => {
                                let input: serde_json::Value =
                                    serde_json::from_str(&tool_call.arguments)
                                        .ok()
                                        .filter(|v: &serde_json::Value| v.is_object())
                                        .unwrap_or(serde_json::json!({}));
                                serde_json::json!({
                                    "type": "tool_use",
                                    "id": tool_call.id,
                                    "name": tool_call.name,
                                    "input": input,
                                })
                            }
                            AssistantContent::Thinking { thinking } => {
                                serde_json::json!({"type": "thinking", "thinking": thinking})
                            }
                        })
                        .collect();
                    serde_json::json!({"role": "assistant", "content": content})
                }
                crate::message::Message::ToolResult(t) => {
                    let text = t
                        .content
                        .iter()
                        .map(|c| match c {
                            crate::message::OutputContent::Text { text } => text.clone(),
                            crate::message::OutputContent::Image { media_type, .. } => {
                                format!("[image: {}]", media_type.as_str())
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("");
                    let mut block = serde_json::json!({
                        "type": "tool_result",
                        "tool_use_id": t.tool_call_id,
                        "content": text,
                    });
                    // Phase 11.9: the Anthropic Messages API documents `is_error` on
                    // the tool_result content block as the failure signal. Emit it
                    // only on failure so the is_error:false body stays byte-identical
                    // to the pre-fix shape.
                    if t.is_error {
                        block["is_error"] = serde_json::Value::Bool(true);
                    }
                    serde_json::json!({
                        "role": "user",
                        "content": [block],
                    })
                }
            })
            .collect(),
    )
}

impl Provider for AnthropicProvider {
    fn stream(&self, request: Request) -> EventStream {
        let auth = self.auth.clone();
        let base_url = self.base_url.clone();
        let body = self.build_request_body(&request);
        let cancel = request.cancel.clone();
        let timeout = request.timeout;
        let extra_headers = request.extra_headers.clone();
        let http_client = self.client.client().clone();

        let (tx, rx) = tokio::sync::mpsc::channel(64);

        tokio::spawn(async move {
            // Validate extra headers before any network call.
            if let Err(e) = crate::provider::validate_extra_headers(&extra_headers) {
                let _ = tx.send(Err(e)).await;
                return;
            }
            let resolved = match auth.resolve().await {
                Ok(resolved) => resolved,
                Err(e) => {
                    let _ = tx.send(Err(e)).await;
                    return;
                }
            };
            if let Err(e) =
                Self::stream_http(http_client, resolved, base_url, &body, cancel, timeout, extra_headers, &tx).await
            {
                let _ = tx.send(Err(e)).await;
            }
        });

        Box::pin(ReceiverStream { rx })
    }

    fn id(&self) -> &str {
        "anthropic"
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{AssistantContent, AssistantMessage, Message, ToolCall};
    use crate::stream::{StopReason, Usage};

    fn test_assistant_msg(content: Vec<AssistantContent>) -> Message {
        Message::Assistant(AssistantMessage {
            content,
            api: crate::ApiKind::Anthropic,
            provider: String::new(),
            model: String::new(),
            response_model: None,
            response_id: None,
            usage: Usage::default(),
            stop_reason: StopReason::Stop,
            error_message: None,
            timestamp_ms: 0,
        })
    }

    #[test]
    fn serialize_tool_call_input_is_json_object() {
        let msg = test_assistant_msg(vec![AssistantContent::ToolCall {
            tool_call: ToolCall {
                id: "tc_1".into(),
                name: "read".into(),
                arguments: r#"{"path":"/tmp/foo.txt"}"#.into(),
            },
        }]);

        let serialized = serialize_messages(&[msg]);
        let input = &serialized[0]["content"][0]["input"];
        assert!(input.is_object(), "input must be JSON object, got: {input}");
        assert_eq!(input["path"], "/tmp/foo.txt");
    }

    #[test]
    fn serialize_tool_call_malformed_args_defaults_to_empty_object() {
        let msg = test_assistant_msg(vec![AssistantContent::ToolCall {
            tool_call: ToolCall {
                id: "tc_2".into(),
                name: "bash".into(),
                arguments: "not valid json".into(),
            },
        }]);

        let serialized = serialize_messages(&[msg]);
        let input = &serialized[0]["content"][0]["input"];
        assert!(input.is_object());
        assert_eq!(input.as_object().unwrap().len(), 0);
    }

    #[test]
    fn serialize_tool_call_non_object_json_defaults_to_empty_object() {
        for (label, args) in [
            ("null", "null"),
            ("array", "[1,2]"),
            ("string", r#""hello""#),
            ("number", "42"),
            ("boolean", "true"),
        ] {
            let msg = test_assistant_msg(vec![AssistantContent::ToolCall {
                tool_call: ToolCall {
                    id: "tc".into(),
                    name: "test".into(),
                    arguments: args.into(),
                },
            }]);

            let serialized = serialize_messages(&[msg]);
            let input = &serialized[0]["content"][0]["input"];
            assert!(
                input.is_object(),
                "{label}: input must be JSON object, got: {input}"
            );
            assert_eq!(
                input.as_object().unwrap().len(),
                0,
                "{label}: input should be empty object"
            );
        }
    }
}
