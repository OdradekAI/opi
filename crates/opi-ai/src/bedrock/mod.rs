//! AWS Bedrock provider (task 3.1).
//!
//! Implements the Bedrock Converse API with SigV4 signing, event-stream
//! parsing, and credential resolution. No live AWS calls.

pub mod credentials;
pub mod event_stream;
pub mod sigv4;

use std::sync::Arc;

use futures_util::{StreamExt, stream};
use tokio_util::sync::CancellationToken;

use crate::bedrock::credentials::BedrockCredentials;
use crate::bedrock::sigv4::{AwsCredentials, sign_request};
use crate::http::HttpClient;
use crate::message::{AssistantContent, AssistantMessage, ToolCall};
use crate::provider::{EventStream, ModelInfo, Provider, ProviderError, Request};
use crate::stream::{AssistantStreamEvent, StopReason, Usage};

/// Model families supported by this provider.
const SUPPORTED_FAMILIES: &[&str] = &["anthropic", "meta", "mistral", "amazon", "cohere"];

/// Concrete AWS Bedrock provider using the Converse API.
pub struct BedrockProvider {
    credentials: AwsCredentials,
    base_url: Option<String>,
    models: Vec<ModelInfo>,
    client: Arc<HttpClient>,
}

impl BedrockProvider {
    pub fn new(
        credentials: AwsCredentials,
        base_url: Option<String>,
        client: Arc<HttpClient>,
    ) -> Self {
        let models = default_bedrock_models();
        Self {
            credentials,
            base_url,
            models,
            client,
        }
    }

    /// Create from resolved BedrockCredentials (credential resolution layer).
    pub fn from_credentials(
        creds: BedrockCredentials,
        base_url: Option<String>,
        client: Arc<HttpClient>,
    ) -> Self {
        Self::new(
            AwsCredentials {
                access_key_id: creds.access_key_id,
                secret_access_key: creds.secret_access_key,
                session_token: creds.session_token,
                region: creds.region,
            },
            base_url,
            client,
        )
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
            Err(ProviderError::Config(format!(
                "unsupported Bedrock model family '{family}' in model ID '{model_id}'; supported families: {}",
                SUPPORTED_FAMILIES.join(", ")
            )))
        }
    }

    /// Build the Converse API request body.
    pub fn build_converse_body(&self, request: &Request) -> serde_json::Value {
        let mut body = serde_json::json!({
            "messages": serialize_converse_messages(&request.messages),
        });

        if let Some(ref system) = request.system {
            body["system"] = serde_json::json!([{"text": system}]);
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

        if !request.tools.is_empty() {
            body["toolConfig"] = serde_json::json!({
                "tools": request.tools.iter().map(|t| {
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

        body
    }

    /// Parse event-stream bytes and emit stream events from fixture data.
    pub fn stream_from_fixture(&self, data: &[u8], cancel: CancellationToken) -> EventStream {
        let mut buffer = data.to_vec();
        let frames = event_stream::parse_frames(&mut buffer);
        let mut mapper = BedrockMapper::new();

        let mut stream_events: Vec<Result<AssistantStreamEvent, ProviderError>> = Vec::new();
        for frame in frames {
            let payload_str = std::str::from_utf8(&frame.payload).unwrap_or("");
            let parsed = parse_bedrock_event(&frame.event_type, payload_str);
            for event in parsed {
                match event {
                    Ok(bedrock_event) => {
                        stream_events.extend(mapper.process(bedrock_event).into_iter().map(Ok));
                    }
                    Err(e) => {
                        stream_events.push(Err(e));
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
    fn runtime_url(&self) -> String {
        self.base_url.clone().unwrap_or_else(|| {
            format!(
                "https://bedrock-runtime.{}.amazonaws.com",
                self.credentials.region
            )
        })
    }
}

impl Provider for BedrockProvider {
    fn id(&self) -> &str {
        "bedrock"
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    fn stream(&self, request: Request) -> EventStream {
        let credentials = self.credentials.clone();
        let base_url = self.runtime_url();
        let body = self.build_converse_body(&request);
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
        for msg in &request.messages {
            if let crate::message::Message::User(user_msg) = msg {
                for content in &user_msg.content {
                    if let crate::message::InputContent::Image {
                        source: crate::message::ImageSource::Url { .. },
                        ..
                    } = content
                    {
                        return Box::pin(stream::iter(vec![Err(
                            ProviderError::UnsupportedCapability(
                                "URL-sourced images are not supported by Bedrock. Use base64 or bytes."
                                    .into(),
                            )
                        )]));
                    }
                }
            }
        }

        let http_client = self.client.client().clone();

        let (tx, rx) = tokio::sync::mpsc::channel(64);

        tokio::spawn(async move {
            if let Err(e) = Self::stream_http(
                http_client,
                credentials,
                base_url,
                &model_id,
                &body,
                cancel,
                &tx,
            )
            .await
            {
                let _ = tx.send(Err(e)).await;
            }
        });

        Box::pin(ReceiverStream { rx })
    }
}

// ---------------------------------------------------------------------------
// HTTP streaming
// ---------------------------------------------------------------------------

impl BedrockProvider {
    async fn stream_http(
        client: reqwest::Client,
        credentials: AwsCredentials,
        base_url: String,
        model_id: &str,
        body: &serde_json::Value,
        cancel: CancellationToken,
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

        let signed = sign_request(
            "POST",
            &path,
            "",
            &[
                ("host", &host),
                ("content-type", "application/json"),
                ("x-amz-content-sha256", &sigv4::sha256_hex(&payload)),
                ("x-amz-date", &date_time),
            ],
            &payload,
            &credentials,
            "bedrock",
            &date_stamp,
            &date_time,
        );

        let url = format!("{base_url}{path}");
        let mut req = client
            .post(&url)
            .header("content-type", "application/json")
            .header("x-amz-date", &signed.x_amz_date)
            .header("x-amz-content-sha256", &signed.x_amz_content_sha256)
            .header("authorization", &signed.authorization)
            .body(payload);

        if let Some(token) = &signed.x_amz_security_token {
            req = req.header("x-amz-security-token", token);
        }

        let response = req
            .send()
            .await
            .map_err(|e| ProviderError::Network(format!("Bedrock request failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let headers = response.headers().clone();
            let error_body = response.text().await.unwrap_or_default();
            return Err(map_bedrock_status(status, &error_body, &headers));
        }

        let mut byte_stream = response.bytes_stream();
        let mut buffer: Vec<u8> = Vec::new();
        let mut mapper = BedrockMapper::new();

        loop {
            let chunk = tokio::select! {
                _ = cancel.cancelled() => return Ok(()),
                chunk = byte_stream.next() => match chunk {
                    Some(c) => c,
                    None => break,
                },
            };

            let chunk =
                chunk.map_err(|e: reqwest::Error| ProviderError::StreamError(e.to_string()))?;
            buffer.extend_from_slice(&chunk);

            let frames = event_stream::parse_frames(&mut buffer);
            for frame in frames {
                let payload_str = std::str::from_utf8(&frame.payload).unwrap_or("");
                for event in parse_bedrock_event(&frame.event_type, payload_str) {
                    match event {
                        Ok(bedrock_event) => {
                            for stream_event in mapper.process(bedrock_event) {
                                if tx.send(Ok(stream_event)).await.is_err() {
                                    return Ok(());
                                }
                            }
                        }
                        Err(e) => {
                            if tx.send(Err(e)).await.is_err() {
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }

        if let Some(pending) = mapper.flush_pending() {
            let _ = tx.send(Ok(pending)).await;
        }

        if !mapper.saw_done {
            let _ = tx
                .send(Err(ProviderError::StreamError(
                    "Bedrock stream ended without terminal event".into(),
                )))
                .await;
        }

        Ok(())
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
// ReceiverStream adapter
// ---------------------------------------------------------------------------

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
    Exception {
        message: String,
    },
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
                    return vec![Err(ProviderError::StreamError(format!(
                        "invalid contentBlockStart: {e}"
                    )))];
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
                    return vec![Err(ProviderError::StreamError(format!(
                        "invalid contentBlockDelta: {e}"
                    )))];
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
        "exception" => {
            let parsed: serde_json::Value = serde_json::from_str(payload).unwrap_or_default();
            vec![Ok(BedrockEvent::Exception {
                message: parsed
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown error")
                    .to_string(),
            })]
        }
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
    fn new() -> Self {
        Self {
            partial: empty_assistant_message(),
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
            );
        }
        self.pending_done.take()
    }

    fn process(&mut self, event: BedrockEvent) -> Vec<AssistantStreamEvent> {
        // Allow Metadata through even after saw_done
        if self.saw_done && !matches!(event, BedrockEvent::Metadata { .. }) {
            return Vec::new();
        }

        match event {
            BedrockEvent::MessageStart => {
                vec![AssistantStreamEvent::Start {
                    partial: self.partial.clone(),
                }]
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
                        vec![AssistantStreamEvent::TextStart {
                            content_index,
                            partial: self.partial.clone(),
                        }]
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
                            },
                        });
                        vec![AssistantStreamEvent::ToolCallStart {
                            content_index,
                            partial: self.partial.clone(),
                        }]
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
                        vec![AssistantStreamEvent::TextDelta {
                            content_index,
                            delta: text,
                            partial: self.partial.clone(),
                        }]
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
                        vec![AssistantStreamEvent::ToolCallDelta {
                            content_index,
                            delta: input,
                            partial: self.partial.clone(),
                        }]
                    }
                }
            }
            BedrockEvent::ContentBlockStop => {
                let content_index = self.blocks.len().saturating_sub(1);
                match self.blocks.last() {
                    Some(BlockState::Text { text }) => {
                        vec![AssistantStreamEvent::TextEnd {
                            content_index,
                            content: text.clone(),
                            partial: self.partial.clone(),
                        }]
                    }
                    Some(BlockState::ToolUse {
                        id,
                        name,
                        partial_input,
                    }) => {
                        let tool_call = ToolCall {
                            id: id.clone(),
                            name: name.clone(),
                            arguments: partial_input.clone(),
                        };
                        if let Some(AssistantContent::ToolCall { tool_call: tc }) =
                            self.partial.content.last_mut()
                        {
                            tc.arguments = partial_input.clone();
                        }
                        vec![AssistantStreamEvent::ToolCallEnd {
                            content_index,
                            tool_call,
                            partial: self.partial.clone(),
                        }]
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
                // Flush pending Done with updated usage
                if let Some(AssistantStreamEvent::Done { message, .. }) = &mut self.pending_done {
                    message.usage = Usage::reported(
                        self.usage.input_tokens,
                        self.usage.output_tokens,
                        self.usage.cache_read_tokens,
                        self.usage.cache_write_tokens,
                    );
                }
                self.pending_done.take().into_iter().collect()
            }
            BedrockEvent::Exception { message } => {
                self.saw_done = true;
                let mut err_msg = self.partial.clone();
                err_msg.error_message = Some(message);
                vec![AssistantStreamEvent::Error {
                    reason: StopReason::Error,
                    message: err_msg,
                }]
            }
        }
    }
}

fn empty_assistant_message() -> AssistantMessage {
    AssistantMessage {
        content: Vec::new(),
        api: crate::ApiKind::Anthropic, // Bedrock uses Anthropic-style content
        provider: "bedrock".into(),
        model: String::new(),
        response_model: None,
        response_id: None,
        usage: Usage::default(),
        stop_reason: StopReason::Stop,
        error_message: None,
        timestamp_ms: 0,
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
                        .map(|c| match c {
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
                            AssistantContent::Thinking { thinking } => {
                                serde_json::json!({"text": thinking})
                            }
                        })
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
            .collect(),
    )
}

// ---------------------------------------------------------------------------
// Error mapping
// ---------------------------------------------------------------------------

/// Map an HTTP status code + body + headers to a `ProviderError`.
pub fn map_bedrock_status(
    status: reqwest::StatusCode,
    body: &str,
    headers: &reqwest::header::HeaderMap,
) -> ProviderError {
    match status.as_u16() {
        401 | 403 => ProviderError::AuthFailed(format!(
            "Bedrock access denied: {}",
            crate::http::safe_excerpt(body)
        )),
        429 => ProviderError::RateLimited {
            retry_after_ms: crate::retry::parse_retry_after(headers),
        },
        408 | 504 => ProviderError::Timeout,
        code => ProviderError::ProviderSide(format!(
            "Bedrock HTTP {code}: {}",
            crate::http::safe_excerpt(body)
        )),
    }
}

// ---------------------------------------------------------------------------
// Secret redaction
// ---------------------------------------------------------------------------

/// Redact AWS credentials for safe display.
pub fn redact_credentials(access_key_id: &str, _secret_key: &str) -> String {
    if access_key_id.len() > 4 {
        format!("{}***", &access_key_id[..4])
    } else {
        "***".to_string()
    }
}

// ---------------------------------------------------------------------------
// Default models
// ---------------------------------------------------------------------------

fn default_bedrock_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "anthropic.claude-sonnet-4-20250514-v2:0".into(),
            display_name: "Claude Sonnet 4 (Bedrock)".into(),
            context_window: 200000,
            max_output_tokens: 8192,
            supports_images: true,
            supports_streaming: true,
            supports_thinking: true,
        },
        ModelInfo {
            id: "anthropic.claude-opus-4-20250514-v1:0".into(),
            display_name: "Claude Opus 4 (Bedrock)".into(),
            context_window: 200000,
            max_output_tokens: 8192,
            supports_images: true,
            supports_streaming: true,
            supports_thinking: true,
        },
        ModelInfo {
            id: "anthropic.claude-haiku-4-5-20250514-v1:0".into(),
            display_name: "Claude Haiku 4.5 (Bedrock)".into(),
            context_window: 200000,
            max_output_tokens: 8192,
            supports_images: true,
            supports_streaming: true,
            supports_thinking: true,
        },
    ]
}
