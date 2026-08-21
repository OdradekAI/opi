//! Google Gemini `streamGenerateContent` SSE provider (S8.1).
//!
//! Implements streaming for the Gemini API using `?alt=sse` which returns
//! SSE-formatted responses with `data:` lines containing `GenerateContentResponse`
//! JSON objects.

use std::sync::Arc;

use futures_util::{StreamExt, stream};
use secrecy::ExposeSecret;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;

use crate::http::HttpClient;
use crate::message::{AssistantContent, AssistantMessage, OutputContent, ToolCall};
use crate::model_info::WireApi;
use crate::provider::{
    EventStream, ModelInfo, Provider, ProviderError, ProviderErrorSummary, Request,
};
use crate::provider_headers::ProviderHeaders;
use crate::registry::ModelCapabilities;
use crate::stream::{
    AssistantStreamEvent, CancelAwareReceiverStream, StopReason, Usage, send_or_cancel,
};

const UPSTREAM_STREAM_ERROR: &str = "Gemini returned a streaming error";
const MALFORMED_STREAM_FRAME: &str = "Gemini returned a malformed streaming frame";

// ---------------------------------------------------------------------------
// SSE line parser (Gemini uses simple data: lines, no event: types)
// ---------------------------------------------------------------------------

/// Parse SSE text into data payloads (just `data:` lines).
pub(crate) fn parse_sse_data(input: &str) -> impl Iterator<Item = String> + '_ {
    input.split('\n').filter_map(|line| {
        let line = line.trim_end_matches('\r');
        line.strip_prefix("data: ")
            .map(|s| s.to_string())
            .or_else(|| {
                // Handle "data:" without space
                line.strip_prefix("data:")
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
            })
    })
}

// ---------------------------------------------------------------------------
// Gemini raw wire types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct GenerateContentResponse {
    candidates: Option<Vec<Candidate>>,
    #[serde(default)]
    error: Option<GeminiError>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: Option<Content>,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
    #[allow(dead_code)]
    index: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct Content {
    #[allow(dead_code)]
    role: Option<String>,
    parts: Option<Vec<Part>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Part {
    Text {
        text: String,
    },
    FunctionCall {
        #[serde(rename = "functionCall")]
        function_call: FunctionCallPart,
    },
}

#[derive(Debug, Deserialize)]
struct FunctionCallPart {
    name: String,
    #[serde(default)]
    args: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct UsageMetadata {
    #[serde(rename = "promptTokenCount")]
    prompt_token_count: Option<u32>,
    #[serde(rename = "candidatesTokenCount")]
    candidates_token_count: Option<u32>,
    #[allow(dead_code)]
    #[serde(rename = "totalTokenCount")]
    total_token_count: Option<u32>,
    #[serde(rename = "cachedContentTokenCount")]
    cached_content_token_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct GeminiError {
    #[allow(dead_code)]
    code: Option<i32>,
    #[serde(rename = "message")]
    _message: Option<String>,
    #[allow(dead_code)]
    status: Option<String>,
}

// ---------------------------------------------------------------------------
// Parsed event type
// ---------------------------------------------------------------------------

pub(crate) enum ParsedEvent {
    Valid(GeminiEvent),
    Malformed,
}

#[derive(Debug, Clone)]
pub(crate) enum GeminiEvent {
    TextDelta {
        text: String,
    },
    FunctionCall {
        name: String,
        args: serde_json::Value,
    },
    Finish {
        reason: String,
        usage: Option<Usage>,
    },
    Error {
        message: String,
    },
}

impl ParsedEvent {
    pub(crate) fn from_data(data: &str) -> Vec<Self> {
        let resp: GenerateContentResponse = match serde_json::from_str(data) {
            Ok(r) => r,
            Err(_) => return vec![ParsedEvent::Malformed],
        };

        // Check for error first
        if let Some(err) = resp.error {
            let _ = err;
            return vec![ParsedEvent::Valid(GeminiEvent::Error {
                message: UPSTREAM_STREAM_ERROR.to_owned(),
            })];
        }

        let mut events = Vec::new();

        // Check for usage/finish in this chunk
        let usage = resp.usage_metadata.map(|u| {
            Usage::reported(
                u.prompt_token_count.unwrap_or(0),
                u.candidates_token_count.unwrap_or(0),
                u.cached_content_token_count.unwrap_or(0),
                0,
                None, // cache_write_1h_tokens
                None, // reasoning_tokens
            )
        });

        if let Some(candidates) = &resp.candidates
            && let Some(candidate) = candidates.first()
        {
            let finish_reason = candidate.finish_reason.clone();

            if let Some(content) = &candidate.content
                && let Some(parts) = &content.parts
            {
                // Collect function calls
                let mut has_function_calls = false;
                for part in parts {
                    if let Part::FunctionCall { function_call } = part {
                        has_function_calls = true;
                        events.push(ParsedEvent::Valid(GeminiEvent::FunctionCall {
                            name: function_call.name.clone(),
                            args: function_call.args.clone(),
                        }));
                    }
                }

                if has_function_calls {
                    // Emit Finish after all function calls if we have usage/finish reason
                    if finish_reason.is_some() || usage.is_some() {
                        events.push(ParsedEvent::Valid(GeminiEvent::Finish {
                            reason: finish_reason.unwrap_or_else(|| "STOP".into()),
                            usage,
                        }));
                    }
                    return events;
                }

                // Check for text content
                let texts: Vec<&str> = parts
                    .iter()
                    .filter_map(|p| match p {
                        Part::Text { text } if !text.is_empty() => Some(text.as_str()),
                        _ => None,
                    })
                    .collect();

                if !texts.is_empty() {
                    let combined: String = texts.into_iter().collect();
                    events.push(ParsedEvent::Valid(GeminiEvent::TextDelta {
                        text: combined,
                    }));
                }

                // Finish event
                if let Some(ref reason) = finish_reason {
                    events.push(ParsedEvent::Valid(GeminiEvent::Finish {
                        reason: reason.clone(),
                        usage: usage.clone(),
                    }));
                }

                if !events.is_empty() {
                    return events;
                }
            }

            // Finish reason without content
            if let Some(reason) = finish_reason {
                return vec![ParsedEvent::Valid(GeminiEvent::Finish { reason, usage })];
            }
        }

        // No useful data  - return empty (silently skip)
        Vec::new()
    }
}

// ---------------------------------------------------------------------------
// Stateful event mapper: GeminiEvent -> AssistantStreamEvent
// ---------------------------------------------------------------------------

struct ToolCallState {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    arguments: String,
}

pub(crate) struct GeminiMapper {
    partial: AssistantMessage,
    saw_done: bool,
    text_started: bool,
    tool_calls: Vec<ToolCallState>,
}

impl GeminiMapper {
    pub(crate) fn new(provider: &str) -> Self {
        Self {
            partial: empty_assistant_message(provider),
            saw_done: false,
            text_started: false,
            tool_calls: Vec::new(),
        }
    }

    pub(crate) fn process(&mut self, event: GeminiEvent) -> Vec<AssistantStreamEvent> {
        if self.saw_done {
            return Vec::new();
        }
        match event {
            GeminiEvent::TextDelta { text } => {
                let mut events = Vec::new();
                if !self.text_started {
                    self.text_started = true;
                    self.partial.content.push(AssistantContent::Text {
                        text: String::new(),
                    });
                    events.push(AssistantStreamEvent::Start {
                        partial: self.partial.clone(),
                    });
                    events.push(AssistantStreamEvent::TextStart {
                        content_index: 0,
                        partial: self.partial.clone(),
                    });
                }
                if let Some(AssistantContent::Text { text: accumulated }) =
                    self.partial.content.last_mut()
                {
                    accumulated.push_str(&text);
                }
                events.push(AssistantStreamEvent::TextDelta {
                    content_index: 0,
                    delta: text,
                    partial: self.partial.clone(),
                });
                events
            }
            GeminiEvent::FunctionCall { name, args } => {
                let mut events = Vec::new();

                // End any open text block
                if self.text_started {
                    self.text_started = false;
                    if let Some(AssistantContent::Text { text }) = self.partial.content.last() {
                        events.push(AssistantStreamEvent::TextEnd {
                            content_index: 0,
                            content: text.clone(),
                            partial: self.partial.clone(),
                        });
                    }
                }

                // If this is the first content, emit Start
                if self.partial.content.is_empty() {
                    events.push(AssistantStreamEvent::Start {
                        partial: self.partial.clone(),
                    });
                }

                let id = format!("fc_{}", self.tool_calls.len());
                let args_str = serde_json::to_string(&args).unwrap_or_default();
                let content_index = self.partial.content.len();

                self.partial.content.push(AssistantContent::ToolCall {
                    tool_call: ToolCall {
                        id: id.clone(),
                        name: name.clone(),
                        arguments: args_str.clone(),
                    },
                });

                self.tool_calls.push(ToolCallState {
                    id: id.clone(),
                    name: name.clone(),
                    arguments: args_str.clone(),
                });

                events.push(AssistantStreamEvent::ToolCallStart {
                    content_index,
                    partial: self.partial.clone(),
                });
                events.push(AssistantStreamEvent::ToolCallEnd {
                    content_index,
                    tool_call: ToolCall {
                        id,
                        name,
                        arguments: args_str,
                    },
                    partial: self.partial.clone(),
                });
                events
            }
            GeminiEvent::Finish { reason, usage } => {
                let mut events = Vec::new();

                // End any open text block
                if self.text_started {
                    self.text_started = false;
                    if let Some(AssistantContent::Text { text }) = self.partial.content.last() {
                        events.push(AssistantStreamEvent::TextEnd {
                            content_index: 0,
                            content: text.clone(),
                            partial: self.partial.clone(),
                        });
                    }
                }

                // If no content at all, emit Start
                if !self.saw_done && self.partial.content.is_empty() {
                    events.push(AssistantStreamEvent::Start {
                        partial: self.partial.clone(),
                    });
                }

                if let Some(u) = usage {
                    self.partial.usage = u;
                }

                let has_tool_calls = self
                    .partial
                    .content
                    .iter()
                    .any(|c| matches!(c, AssistantContent::ToolCall { .. }));

                self.partial.stop_reason = match reason.as_str() {
                    "STOP" => {
                        if has_tool_calls {
                            StopReason::ToolUse
                        } else {
                            StopReason::Stop
                        }
                    }
                    "MAX_TOKENS" => StopReason::Length,
                    _ => StopReason::Stop,
                };
                self.saw_done = true;

                events.push(AssistantStreamEvent::Done {
                    reason: self.partial.stop_reason,
                    message: self.partial.clone(),
                });
                events
            }
            GeminiEvent::Error { message } => {
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

fn empty_assistant_message(provider: &str) -> AssistantMessage {
    AssistantMessage {
        content: Vec::new(),
        api: crate::ApiKind::Google,
        provider: provider.into(),
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
// GeminiProvider
// ---------------------------------------------------------------------------

pub struct GeminiProvider {
    base_url: String,
    models: Vec<ModelInfo>,
    client: Arc<HttpClient>,
}

/// Built-in Gemini model metadata without credentials or HTTP construction.
pub fn model_catalog() -> Vec<ModelInfo> {
    vec![
        ModelInfo::new(
            "gemini-2.5-flash",
            "Gemini 2.5 Flash",
            WireApi::GoogleGenerativeAi,
            ModelCapabilities::new(1_000_000, 65536)
                .with_images(true)
                .with_streaming(true),
        ),
        ModelInfo::new(
            "gemini-2.5-pro",
            "Gemini 2.5 Pro",
            WireApi::GoogleGenerativeAi,
            ModelCapabilities::new(1_000_000, 65536)
                .with_images(true)
                .with_streaming(true),
        ),
        ModelInfo::new(
            "gemini-2.0-flash",
            "Gemini 2.0 Flash",
            WireApi::GoogleGenerativeAi,
            ModelCapabilities::new(1_000_000, 8192)
                .with_images(true)
                .with_streaming(true),
        ),
    ]
}

impl GeminiProvider {
    pub fn new(base_url: Option<String>) -> Self {
        Self::with_client(base_url, Arc::new(HttpClient::new()))
    }

    /// Create with a shared HTTP client.
    pub fn with_client(base_url: Option<String>, client: Arc<HttpClient>) -> Self {
        let base_url =
            base_url.unwrap_or_else(|| "https://generativelanguage.googleapis.com".into());
        let models = model_catalog();
        Self {
            base_url,
            models,
            client,
        }
    }

    /// Access the shared HTTP client.
    pub fn http_client(&self) -> &Arc<HttpClient> {
        &self.client
    }

    /// Build the Gemini `generateContent` request body.
    /// The model ID goes in the URL path, not the body.
    pub fn build_request_body(&self, request: &Request) -> serde_json::Value {
        let _model_id = request
            .model
            .split_once(':')
            .map(|(_, id)| id)
            .unwrap_or(&request.model);

        let mut contents = Vec::new();

        for msg in &request.messages {
            match msg {
                crate::message::Message::User(u) => {
                    let parts: Vec<serde_json::Value> = u
                        .content
                        .iter()
                        .map(|c| match c {
                            crate::message::InputContent::Text { text } => {
                                serde_json::json!({"text": text})
                            }
                            crate::message::InputContent::Image { source, media_type } => {
                                match source {
                                    crate::message::ImageSource::Url { url } => {
                                        serde_json::json!({
                                            "file_data": {
                                                "file_uri": url,
                                                "mime_type": media_type.as_str(),
                                            }
                                        })
                                    }
                                    crate::message::ImageSource::Base64 { data } => {
                                        serde_json::json!({
                                            "inline_data": {
                                                "mime_type": media_type.as_str(),
                                                "data": data,
                                            }
                                        })
                                    }
                                    crate::message::ImageSource::Bytes { data } => {
                                        serde_json::json!({
                                            "inline_data": {
                                                "mime_type": media_type.as_str(),
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
                    contents.push(serde_json::json!({
                        "role": "user",
                        "parts": parts,
                    }));
                }
                crate::message::Message::Assistant(a) => {
                    let parts: Vec<serde_json::Value> = a
                        .content
                        .iter()
                        .filter_map(|c| match c {
                            AssistantContent::Text { text } => {
                                Some(serde_json::json!({"text": text}))
                            }
                            AssistantContent::ToolCall { tool_call } => {
                                // Convert to functionCall response part
                                let args: serde_json::Value =
                                    serde_json::from_str(&tool_call.arguments)
                                        .unwrap_or(serde_json::Value::Null);
                                Some(serde_json::json!({
                                    "functionCall": {
                                        "name": tool_call.name,
                                        "args": args,
                                    }
                                }))
                            }
                            AssistantContent::Thinking { .. } => None,
                        })
                        .collect();
                    if !parts.is_empty() {
                        contents.push(serde_json::json!({
                            "role": "model",
                            "parts": parts,
                        }));
                    }
                }
                crate::message::Message::ToolResult(t) => {
                    let response_text: String = t
                        .content
                        .iter()
                        .map(|c| match c {
                            OutputContent::Text { text } => text.clone(),
                            OutputContent::Image { media_type, .. } => {
                                format!("[image: {}]", media_type.as_str())
                            }
                        })
                        .collect();
                    let mut response = serde_json::json!({
                        "content": response_text,
                    });
                    // The Gemini REST API documents an `error` key inside
                    // functionResponse.response as the failure signal ("if the
                    // function call failed to execute, the response can have an
                    // 'error' key"). Emit it only on failure so the success body
                    // stays byte-identical. Vertex inherits this via the shared adapter.
                    if t.is_error {
                        response["error"] = serde_json::Value::Bool(true);
                    }
                    contents.push(serde_json::json!({
                        "role": "user",
                        "parts": [{
                            "functionResponse": {
                                "name": t.tool_name,
                                "id": t.tool_call_id,
                                "response": response,
                            }
                        }],
                    }));
                }
            }
        }

        let mut body = serde_json::json!({
            "contents": contents,
        });

        // System instruction is a separate object
        if let Some(sys) = &request.system {
            body["systemInstruction"] = serde_json::json!({
                "parts": [{"text": sys}]
            });
        }

        // Generation config
        let mut gen_config = serde_json::json!({});
        if let Some(max_tokens) = request.max_tokens {
            gen_config["maxOutputTokens"] = serde_json::Value::Number(max_tokens.into());
        }
        if let Some(temp) = request.temperature
            && let Some(n) = serde_json::Number::from_f64(temp)
        {
            gen_config["temperature"] = serde_json::Value::Number(n);
        }
        if !gen_config.as_object().map(|o| o.is_empty()).unwrap_or(true) {
            body["generationConfig"] = gen_config;
        }

        // Tools
        if !request.tools.is_empty() {
            body["tools"] = serde_json::Value::Array(vec![serde_json::json!({
                "functionDeclarations": request.tools.iter().map(|t| {
                    serde_json::json!({
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.input_schema,
                    })
                }).collect::<Vec<_>>()
            })]);
        }

        body
    }

    /// Stream events from a raw SSE response body.
    pub fn stream_from_sse(&self, sse_body: &str, cancel: CancellationToken) -> EventStream {
        let mut mapper = GeminiMapper::new("gemini");
        let mut stream_events: Vec<Result<AssistantStreamEvent, ProviderError>> = Vec::new();

        for data in parse_sse_data(sse_body) {
            for parsed in ParsedEvent::from_data(&data) {
                match parsed {
                    ParsedEvent::Valid(event) => {
                        stream_events.extend(mapper.process(event).into_iter().map(Ok));
                    }
                    ParsedEvent::Malformed => {
                        stream_events.push(Err(ProviderError::StreamError(
                            ProviderErrorSummary::attested_static(MALFORMED_STREAM_FRAME),
                        )));
                    }
                }
            }
        }

        let _cancel = cancel;
        Box::pin(stream::iter(stream_events))
    }

    /// Real HTTP streaming: POST to Gemini streamGenerateContent API with ?alt=sse.
    #[allow(clippy::too_many_arguments)]
    async fn stream_http(
        http_client: reqwest::Client,
        api_key: String,
        base_url: String,
        model_id: String,
        body: &serde_json::Value,
        cancel: CancellationToken,
        timeout: Option<std::time::Duration>,
        extra_headers: Vec<(String, String)>,
        tx: &tokio::sync::mpsc::Sender<Result<AssistantStreamEvent, ProviderError>>,
    ) -> Result<(), ProviderError> {
        let url = format!("{base_url}/v1beta/models/{model_id}:streamGenerateContent?alt=sse");
        let route_headers = vec![
            ("x-goog-api-key".to_owned(), api_key),
            ("content-type".to_owned(), "application/json".to_owned()),
        ];
        let headers = ProviderHeaders::default().merge_request(&route_headers, &extra_headers)?;
        let mut request = http_client.post(&url);
        for (name, value) in headers {
            request = request.header(name, value);
        }
        if let Some(timeout) = timeout {
            request = request.timeout(timeout);
        }
        let response = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(ProviderError::Cancelled),
            response = request
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
                return Err(map_gemini_error(status, body.as_deref(), &headers));
            }
            return Err(map_gemini_error(status, None, &headers));
        }

        let mut byte_stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut mapper = GeminiMapper::new("gemini");

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

            for parsed in drain_sse_data(&mut buffer) {
                match parsed {
                    ParsedEvent::Valid(event) => {
                        for stream_event in mapper.process(event) {
                            if !send_or_cancel(&cancel, tx, Ok(stream_event)).await? {
                                return Ok(());
                            }
                        }
                    }
                    ParsedEvent::Malformed => {
                        let err = ProviderError::StreamError(
                            ProviderErrorSummary::attested_static(MALFORMED_STREAM_FRAME),
                        );
                        if !send_or_cancel(&cancel, tx, Err(err)).await? {
                            return Ok(());
                        }
                    }
                }
            }
        }

        if !mapper.saw_done {
            let err = ProviderError::StreamError(ProviderErrorSummary::attested_static(
                "stream ended without a terminal event",
            ));
            let _ = send_or_cancel(&cancel, tx, Err(err)).await?;
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Streaming helpers
// ---------------------------------------------------------------------------

/// Drain complete SSE events from the buffer (delimited by `\n\n`).
pub(crate) fn drain_sse_data(buffer: &mut String) -> Vec<ParsedEvent> {
    if buffer.contains('\r') {
        *buffer = buffer.replace("\r\n", "\n").replace('\r', "\n");
    }

    let mut events = Vec::new();
    while let Some(idx) = buffer.find("\n\n") {
        let end = idx + 2;
        let chunk: String = buffer.drain(..end).collect();
        for data in parse_sse_data(&chunk) {
            events.extend(ParsedEvent::from_data(&data));
        }
    }
    events
}

/// Map Gemini HTTP error responses to ProviderError variants.
///
/// Gemini sometimes returns auth errors with HTTP 400 but a JSON body containing
/// `"code":401` or `"code":403`, so we inspect the body for those codes as well.
fn map_gemini_error(
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
            // Gemini may return auth errors with HTTP 400 but code 401/403 in the body
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

impl Provider for GeminiProvider {
    fn stream_prepared(&self, request: Request, auth: crate::auth::ResolvedAuth) -> EventStream {
        let secret = auth.secret.expose_secret();
        let api_key = match auth.scheme {
            crate::auth::AuthScheme::ApiKey => secret.to_string(),
            crate::auth::AuthScheme::Bearer | crate::auth::AuthScheme::AwsSigV4(_) => {
                return Box::pin(stream::iter(vec![Err(ProviderError::Config(
                    ProviderErrorSummary::attested_static(
                        "Gemini requires prepared API-key authentication",
                    ),
                ))]));
            }
        };
        let base_url = self.base_url.clone();
        let model_id = request
            .model
            .split_once(':')
            .map(|(_, id)| id.to_string())
            .unwrap_or(request.model.clone());
        let body = self.build_request_body(&request);
        let cancel = request.cancel.clone();
        let timeout = request.timeout;
        let extra_headers = request.extra_headers;
        let http_client = self.client.client().clone();

        let (tx, rx) = tokio::sync::mpsc::channel(64);
        let producer_cancel = cancel.clone();

        tokio::spawn(async move {
            let result = Self::stream_http(
                http_client,
                api_key,
                base_url,
                model_id,
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

    fn id(&self) -> &str {
        "gemini"
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }
}
