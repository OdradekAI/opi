//! OpenAI Chat Completions wire adapter.
//!
//! This module owns the OpenAI-compatible SSE parser, event mapper, request
//! projection, streaming transport, status/error handling and typed Chat API.
//! Provider identity and profile state live in crate::providers::openai.

use std::collections::HashMap;
use std::sync::Arc;

use futures_util::{StreamExt, stream};
use secrecy::ExposeSecret;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;

use crate::auth::{AuthInvalidPolicy, AuthScheme, ResolvedAuth};
use crate::http::HttpClient;
use crate::message::{
    AssistantContent, AssistantMessage, OutputContent, TOOL_ERROR_MARKER, ToolCall,
};
use crate::model_info::OpenAiCompletionsCompat as CompatConfig;
use crate::provider::routes::OpenAiRouteContext;
use crate::provider::{
    CacheRetention, EventStream, ModelInfo, Provider, ProviderError, ProviderErrorSummary, Request,
    github_copilot_initiator, github_copilot_route_headers,
};
use crate::stream::{
    AssistantStreamEvent, CancelAwareReceiverStream, StopReason, Usage, send_or_cancel,
};

// ---------------------------------------------------------------------------
// SSE line parser
// ---------------------------------------------------------------------------

/// A raw SSE frame extracted from the byte stream.
struct SseFrame {
    data: String,
}

/// Parsed result for a single SSE frame.
pub enum ParsedEvent {
    Valid(Vec<OpenAiChatEvent>),
    /// The protocol sentinel; no raw JSON callback is associated with it.
    Done,
    UsageError(ProviderError),
    Malformed {
        data: String,
        error: String,
    },
}

/// Parse SSE text into frames, then deserialize each frame as an OpenAI chunk.
/// Returns [`ParsedEvent`] so callers can decide how to handle malformed data.
pub fn parse_sse_events(input: &str) -> impl Iterator<Item = ParsedEvent> + '_ {
    parse_sse_events_with_raw(input).map(|(parsed, _raw)| parsed)
}

/// Parse OpenRouter-compatible usage fields in addition to the standard
/// OpenAI Chat event shape. Generic parsing intentionally leaves those fields
/// as opaque extension data, matching Pi's provider-gated accounting.
pub fn parse_sse_events_for_openrouter(input: &str) -> impl Iterator<Item = ParsedEvent> + '_ {
    parse_sse_events_with_raw_for_model(input, true).map(|(parsed, _raw)| parsed)
}

/// [`parse_sse_events`] paired with each processed frame's raw wire JSON
/// (task 20.51): the raw-event callback observes exactly these values, before
/// normalization. The `[DONE]` sentinel and malformed frames carry no raw
/// value.
pub fn parse_sse_events_with_raw(
    input: &str,
) -> impl Iterator<Item = (ParsedEvent, Option<serde_json::Value>)> + '_ {
    parse_sse_events_with_raw_for_model(input, false)
}

/// [`parse_sse_events_with_raw`] with an explicit provider route flag.
pub fn parse_sse_events_with_raw_for_model(
    input: &str,
    is_openrouter: bool,
) -> impl Iterator<Item = (ParsedEvent, Option<serde_json::Value>)> + '_ {
    parse_frames(input).map(move |frame| {
        if frame.data == "[DONE]" {
            return (ParsedEvent::Done, None);
        }
        let raw = serde_json::from_str::<serde_json::Value>(&frame.data).ok();
        match serde_json::from_str::<OpenAiRawChunk>(&frame.data) {
            Ok(raw_chunk) => match validate_usage_subset(&raw_chunk) {
                Ok(()) => (
                    ParsedEvent::Valid(OpenAiChatEvent::from_raw_vec(raw_chunk, is_openrouter)),
                    raw,
                ),
                Err(error) => (ParsedEvent::UsageError(error), raw),
            },
            Err(e) => (
                ParsedEvent::Malformed {
                    data: frame.data.clone(),
                    error: e.to_string(),
                },
                raw,
            ),
        }
    })
}

fn parse_frames(input: &str) -> impl Iterator<Item = SseFrame> + '_ {
    let mut lines = input.split('\n').peekable();
    std::iter::from_fn(move || {
        let mut data_parts: Vec<&str> = Vec::new();

        loop {
            match lines.next() {
                Some(line) if line.starts_with(':') => continue,
                Some(line) if line.trim_end_matches('\r').is_empty() => {
                    if !data_parts.is_empty() {
                        return Some(SseFrame {
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
                    if field == "data" {
                        data_parts.push(value);
                    }
                }
                None => {
                    if !data_parts.is_empty() {
                        return Some(SseFrame {
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
// OpenAI raw wire types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct OpenAiRawChunk {
    id: Option<String>,
    #[allow(dead_code)]
    object: Option<String>,
    #[allow(dead_code)]
    created: Option<u64>,
    model: Option<String>,
    choices: Option<Vec<RawChoice>>,
    usage: Option<RawUsage>,
    error: Option<RawError>,
}

#[derive(Debug, Deserialize)]
struct RawChoice {
    #[allow(dead_code)]
    index: Option<usize>,
    delta: Option<RawDelta>,
    finish_reason: Option<String>,
    /// A few OpenAI-compatible endpoints (for example Moonshot) attach
    /// usage to the choice instead of the standard chunk-level member.
    #[serde(default)]
    usage: Option<RawUsage>,
}

#[derive(Debug, Deserialize)]
struct RawDelta {
    role: Option<String>,
    content: Option<String>,
    reasoning_content: Option<serde_json::Value>,
    reasoning: Option<serde_json::Value>,
    reasoning_text: Option<serde_json::Value>,
    reasoning_details: Option<serde_json::Value>,
    tool_calls: Option<Vec<RawToolCall>>,
}

#[derive(Debug, Deserialize)]
struct RawToolCall {
    index: usize,
    id: Option<String>,
    #[allow(dead_code)]
    r#type: Option<String>,
    function: Option<RawFunction>,
    custom: Option<RawCustom>,
}

#[derive(Debug, Deserialize)]
struct RawCustom {
    name: Option<String>,
    input: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawFunction {
    name: Option<String>,
    arguments: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct RawUsage {
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
    #[allow(dead_code)]
    total_tokens: Option<u32>,
    prompt_tokens_details: Option<RawPromptTokenDetails>,
    #[serde(default)]
    completion_tokens_details: Option<RawCompletionTokenDetails>,
    /// OpenRouter billing fields are outside the OpenAI schema but arrive on
    /// the same usage object for compatible routes.
    #[serde(default)]
    cost: Option<f64>,
    #[serde(default)]
    is_byok: Option<bool>,
    #[serde(default)]
    cost_details: Option<crate::api::shared::openrouter_usage::OpenRouterCostDetails>,
}

#[derive(Clone, Debug, Deserialize)]
struct RawPromptTokenDetails {
    cached_tokens: Option<u32>,
    #[serde(default)]
    cache_write_tokens: Option<u32>,
}

#[derive(Clone, Debug, Deserialize)]
struct RawCompletionTokenDetails {
    #[serde(default)]
    reasoning_tokens: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct RawError {
    message: Option<String>,
    #[allow(dead_code)]
    r#type: Option<String>,
}

fn validate_usage_subset(raw: &OpenAiRawChunk) -> Result<(), ProviderError> {
    let Some(usage) = raw.usage.as_ref().or_else(|| {
        raw.choices
            .as_ref()
            .and_then(|choices| choices.first())
            .and_then(|choice| choice.usage.as_ref())
    }) else {
        return Ok(());
    };
    let output = u64::from(usage.completion_tokens.unwrap_or(0));
    if let Some(reasoning) = usage
        .completion_tokens_details
        .as_ref()
        .and_then(|details| details.reasoning_tokens)
        && reasoning > output
    {
        return Err(ProviderError::StreamError(ProviderErrorSummary::sanitized(
            format!("reasoning_tokens ({reasoning}) exceeds completion_tokens ({output})"),
        )));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Public OpenAiChatEvent enum
// ---------------------------------------------------------------------------

/// A parsed OpenAI Chat Completions SSE event.
#[derive(Debug, Clone)]
pub enum OpenAiChatEvent {
    /// First chunk typically carries the role.
    RoleDelta {
        role: Option<String>,
        model: Option<String>,
        id: Option<String>,
        usage: Option<Usage>,
    },
    /// Text content delta.
    ContentDelta {
        content: String,
        id: Option<String>,
        model: Option<String>,
        usage: Option<Usage>,
    },
    /// Visible reasoning and source-qualified opaque replay metadata.
    ThinkingDelta {
        content: String,
        signature: String,
        details: Vec<serde_json::Value>,
        id: Option<String>,
        model: Option<String>,
        usage: Option<Usage>,
    },
    /// Tool call started (first appearance of a tool_calls entry with id+name).
    ToolCallStart {
        index: usize,
        id: String,
        name: String,
        response_id: Option<String>,
        model: Option<String>,
        usage: Option<Usage>,
    },
    /// Tool call argument delta.
    ToolCallDelta {
        index: usize,
        arguments: String,
        id: Option<String>,
        model: Option<String>,
        usage: Option<Usage>,
    },
    /// Finish reason received (stop, tool_calls, length).
    Finish {
        finish_reason: String,
        id: Option<String>,
        model: Option<String>,
        usage: Option<Usage>,
    },
    /// Error from the API.
    Error { message: Option<String> },
}

impl OpenAiChatEvent {
    fn from_raw_vec(raw: OpenAiRawChunk, is_openrouter: bool) -> Vec<Self> {
        // Check for top-level error
        if let Some(err) = raw.error {
            return vec![OpenAiChatEvent::Error {
                message: err.message,
            }];
        }

        let usage = raw
            .usage
            .as_ref()
            .or_else(|| {
                raw.choices
                    .as_ref()
                    .and_then(|choices| choices.first())
                    .and_then(|choice| choice.usage.as_ref())
            })
            .map(|u| {
                let cached = u
                    .prompt_tokens_details
                    .as_ref()
                    .and_then(|d| d.cached_tokens)
                    .unwrap_or(0);
                let cache_write = u
                    .prompt_tokens_details
                    .as_ref()
                    .and_then(|d| d.cache_write_tokens)
                    .unwrap_or(0);
                let output = u.completion_tokens.unwrap_or(0);
                let reasoning = u
                    .completion_tokens_details
                    .as_ref()
                    .and_then(|d| d.reasoning_tokens);
                let mut usage = Usage::reported(
                    u.prompt_tokens
                        .unwrap_or(0)
                        .saturating_sub(cached)
                        .saturating_sub(cache_write),
                    output,
                    cached,
                    cache_write,
                    None, // cache_write_1h_tokens — Chat doesn't write cache
                    reasoning,
                )
                .with_total_tokens(u.total_tokens.map(u64::from));
                if is_openrouter {
                    crate::api::shared::openrouter_usage::apply_openrouter_reported_cost(
                        &mut usage,
                        &crate::api::shared::openrouter_usage::OpenRouterUsageAccounting {
                            cost: u.cost,
                            is_byok: u.is_byok,
                            cost_details: u.cost_details.clone(),
                        },
                    );
                }
                usage
            });

        let response_id = raw.id.clone();
        let model = raw.model.clone();

        let choices = match raw.choices {
            Some(c) => c,
            None => {
                if let Some(u) = usage {
                    return vec![OpenAiChatEvent::Finish {
                        finish_reason: String::new(),
                        id: response_id,
                        model,
                        usage: Some(u),
                    }];
                }
                return vec![];
            }
        };

        if choices.is_empty() {
            if let Some(u) = usage {
                return vec![OpenAiChatEvent::Finish {
                    finish_reason: String::new(),
                    id: response_id,
                    model,
                    usage: Some(u),
                }];
            }
            return vec![];
        }

        let mut events = Vec::new();
        if let Some(choice) = choices.into_iter().next() {
            if let Some(delta) = choice.delta {
                if delta.role.is_some() {
                    events.push(OpenAiChatEvent::RoleDelta {
                        role: delta.role,
                        model: model.clone(),
                        id: response_id.clone(),
                        usage: usage.clone(),
                    });
                }
                let thinking = [
                    ("reasoning_content", delta.reasoning_content.as_ref()),
                    ("reasoning", delta.reasoning.as_ref()),
                    ("reasoning_text", delta.reasoning_text.as_ref()),
                ]
                .into_iter()
                .find_map(|(field, value)| {
                    value
                        .and_then(serde_json::Value::as_str)
                        .filter(|value| !value.is_empty())
                        .map(|value| (field, value))
                });
                let details: Vec<_> = delta
                    .reasoning_details
                    .as_ref()
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter(|detail| valid_reasoning_detail(detail))
                    .cloned()
                    .collect();
                if thinking.is_some() || !details.is_empty() {
                    let (signature, content) = thinking.unwrap_or(("", ""));
                    events.push(OpenAiChatEvent::ThinkingDelta {
                        content: content.into(),
                        signature: signature.into(),
                        details,
                        id: response_id.clone(),
                        model: model.clone(),
                        usage: usage.clone(),
                    });
                }
                if let Some(content) = delta.content.filter(|content| !content.is_empty()) {
                    events.push(OpenAiChatEvent::ContentDelta {
                        content,
                        id: response_id.clone(),
                        model: model.clone(),
                        usage: usage.clone(),
                    });
                }
                if let Some(tool_calls) = delta.tool_calls {
                    for tc in tool_calls {
                        let func = tc
                            .function
                            .or_else(|| {
                                tc.custom.map(|custom| RawFunction {
                                    name: custom.name,
                                    arguments: custom.input,
                                })
                            })
                            .unwrap_or(RawFunction {
                                name: None,
                                arguments: None,
                            });
                        if let Some(id) = tc.id {
                            events.push(OpenAiChatEvent::ToolCallStart {
                                index: tc.index,
                                id,
                                name: func.name.unwrap_or_default(),
                                response_id: response_id.clone(),
                                model: model.clone(),
                                usage: usage.clone(),
                            });
                        }
                        if let Some(arguments) =
                            func.arguments.filter(|arguments| !arguments.is_empty())
                        {
                            events.push(OpenAiChatEvent::ToolCallDelta {
                                index: tc.index,
                                arguments,
                                id: response_id.clone(),
                                model: model.clone(),
                                usage: usage.clone(),
                            });
                        }
                    }
                }
            }
            // A final chunk can also carry content; process its deltas before
            // closing the output so none of the acknowledged data is lost.
            if let Some(finish_reason) = choice.finish_reason {
                events.push(OpenAiChatEvent::Finish {
                    finish_reason,
                    id: response_id,
                    model,
                    usage,
                });
            }
        }

        events
    }
}

fn valid_reasoning_detail(value: &serde_json::Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    if object
        .get("id")
        .is_some_and(|value| !value.is_null() && !value.is_string())
        || object.get("format").is_some_and(|value| !value.is_string())
        || object.get("index").is_some_and(|value| !value.is_number())
    {
        return false;
    }
    match value.get("type").and_then(serde_json::Value::as_str) {
        Some("reasoning.summary") => value
            .get("summary")
            .is_some_and(serde_json::Value::is_string),
        Some("reasoning.encrypted") => value.get("data").is_some_and(serde_json::Value::is_string),
        Some("reasoning.text") => {
            value.get("text").is_some_and(serde_json::Value::is_string)
                && !object
                    .get("signature")
                    .is_some_and(|value| !value.is_null() && !value.is_string())
        }
        _ => false,
    }
}

fn append_reasoning_detail(details: &mut Vec<serde_json::Value>, detail: serde_json::Value) {
    let field = match detail["type"].as_str() {
        Some("reasoning.text") => Some("text"),
        Some("reasoning.summary") => Some("summary"),
        _ => None,
    };
    if let Some(field) = field
        && let Some(last) = details
            .last_mut()
            .filter(|last| last["type"] == detail["type"])
    {
        last[field] = format!(
            "{}{}",
            last[field].as_str().unwrap_or_default(),
            detail[field].as_str().unwrap_or_default()
        )
        .into();
        for key in ["id", "format", "index", "signature"] {
            let missing = last.get(key).is_none_or(|value| {
                value.is_null()
                    || (matches!(key, "format" | "signature") && value.as_str() == Some(""))
            });
            if missing && let Some(value) = detail.get(key) {
                last[key] = value.clone();
            }
        }
        return;
    }
    details.push(detail);
}

// ---------------------------------------------------------------------------
// Stateful event mapper: OpenAiChatEvent -> AssistantStreamEvent
// ---------------------------------------------------------------------------

/// Tracks tool call state and accumulates the final message.
pub struct OpenAiChatMapper {
    snapshot: crate::stream::ProducerSnapshot,
    partial: AssistantMessage,
    tool_calls: Vec<ToolCallState>,
    saw_done: bool,
    started: bool,
    text_started: bool,
    text_content_index: Option<usize>,
    thinking_content_index: Option<usize>,
    reasoning_details: Vec<serde_json::Value>,
    pending_stop_reason: Option<StopReason>,
    grammar_input_properties: HashMap<String, String>,
    strict_tools: HashMap<String, crate::message::ToolDef>,
    model_info: Option<crate::model_info::ModelInfo>,
}

struct ToolCallState {
    id: String,
    name: String,
    arguments: String,
    custom_input: Option<crate::constrained_sampling::GrammarInputBuffer>,
}

impl OpenAiChatMapper {
    pub(crate) fn set_snapshot(&mut self, snapshot: crate::stream::ProducerSnapshot) {
        self.snapshot = snapshot;
        self.snapshot.record(&self.partial);
    }

    pub fn process(
        &mut self,
        event: OpenAiChatEvent,
    ) -> Vec<Result<AssistantStreamEvent, ProviderError>> {
        let events = self.process_inner(event);
        self.snapshot.record(&self.partial);
        events
    }

    pub(crate) fn failure(&self, cause: ProviderError) -> ProviderError {
        self.snapshot.record(&self.partial);
        self.snapshot.failure(cause)
    }

    pub fn new(
        api: crate::ApiKind,
        provider: &str,
        wire: crate::model_info::WireApi,
        model: &str,
    ) -> Self {
        Self {
            snapshot: crate::stream::ProducerSnapshot::default(),
            partial: empty_assistant_message(api, provider, wire, model),
            tool_calls: Vec::new(),
            saw_done: false,
            started: false,
            text_started: false,
            text_content_index: None,
            thinking_content_index: None,
            reasoning_details: Vec::new(),
            pending_stop_reason: None,
            grammar_input_properties: HashMap::new(),
            strict_tools: HashMap::new(),
            model_info: None,
        }
    }

    pub(crate) fn set_model_info(&mut self, model_info: Option<crate::model_info::ModelInfo>) {
        self.model_info = model_info;
    }

    fn set_tool_metadata(
        &mut self,
        tools: Option<crate::api::shared::openai_responses::ResponsesWireTools>,
    ) {
        if let Some(tools) = tools {
            self.grammar_input_properties = tools.grammar_input_properties;
            self.strict_tools = tools
                .strict_tools
                .into_iter()
                .map(|tool| (tool.name.clone(), tool))
                .collect();
            self.partial.diagnostics.extend(tools.diagnostics);
        }
    }

    /// Complete a Chat response at `[DONE]` or natural HTTP EOF.
    /// Finish reasons and usage only accumulate metadata. This method closes
    /// blocks in content order, finalizes tool arguments, and emits one Done.
    /// Missing finish reasons fail unless the route explicitly disables them.
    /// Callers must stop consuming the response after any terminal or failure.
    pub fn finish(
        &mut self,
        supports_finish_reason: bool,
    ) -> Vec<Result<AssistantStreamEvent, ProviderError>> {
        if self.saw_done {
            return Vec::new();
        }
        let reason = match self.pending_stop_reason.take() {
            Some(reason) => reason,
            None if !supports_finish_reason => {
                if self.tool_calls.iter().any(|tool| !tool.id.is_empty()) {
                    StopReason::ToolUse
                } else {
                    StopReason::Stop
                }
            }
            None => {
                self.saw_done = true;
                return vec![Err(self.failure(ProviderError::StreamError(
                    ProviderErrorSummary::attested_static("stream ended without a terminal event"),
                )))];
            }
        };
        let mut events = Vec::new();
        self.ensure_start(&mut events);
        for content_index in 0..self.partial.content.len() {
            match &self.partial.content[content_index] {
                AssistantContent::Text { text, .. } => {
                    events.push(Ok(AssistantStreamEvent::TextEnd {
                        content_index,
                        content: text.clone(),
                        partial: self.partial.clone(),
                    }));
                }
                AssistantContent::Thinking { thinking, .. } => {
                    events.push(Ok(AssistantStreamEvent::ThinkingEnd {
                        content_index,
                        content: thinking.clone(),
                        partial: self.partial.clone(),
                    }));
                }
                AssistantContent::ToolCall { tool_call } => {
                    let Some(state) = self
                        .tool_calls
                        .iter_mut()
                        .find(|state| state.id == tool_call.id)
                    else {
                        continue;
                    };
                    // Grammar completion and strict normalization happen only
                    // at a successful protocol end, never on a failed partial.
                    if let Some(buffer) = &mut state.custom_input {
                        let input = buffer.input().to_owned();
                        if let Ok(Some(delta)) =
                            crate::constrained_sampling::append_grammar_tool_input_delta(
                                buffer, &input, true,
                            )
                        {
                            state.arguments.push_str(&delta);
                            if let AssistantContent::ToolCall { tool_call } =
                                &mut self.partial.content[content_index]
                            {
                                tool_call.arguments.push_str(&delta);
                            }
                            events.push(Ok(AssistantStreamEvent::ToolCallDelta {
                                content_index,
                                delta,
                                partial: self.partial.clone(),
                            }));
                        }
                    }
                    let (mut arguments, arguments_error) = if state.custom_input.is_some() {
                        (state.arguments.clone(), None)
                    } else {
                        ToolCall::finalize_text_arguments(state.arguments.clone())
                    };
                    if arguments_error.is_none()
                        && let Some(tool) = self.strict_tools.get(&state.name)
                        && let Ok(mut parsed) =
                            serde_json::from_str::<serde_json::Value>(&arguments)
                    {
                        crate::constrained_sampling::normalize_wire_arguments_nulls(
                            &tool.input_schema,
                            &mut parsed,
                        );
                        arguments = parsed.to_string();
                    }
                    if let AssistantContent::ToolCall { tool_call } =
                        &mut self.partial.content[content_index]
                    {
                        tool_call.arguments = arguments;
                        tool_call.arguments_error = arguments_error;
                        events.push(Ok(AssistantStreamEvent::ToolCallEnd {
                            content_index,
                            tool_call: tool_call.clone(),
                            partial: self.partial.clone(),
                        }));
                    }
                }
                _ => {}
            }
        }
        events.push(Ok(self.emit_done(reason)));
        self.snapshot.record(&self.partial);
        events
    }

    pub(crate) fn process_frame(
        &mut self,
        frame: ParsedEvent,
        supports_finish_reason: bool,
    ) -> Vec<Result<AssistantStreamEvent, ProviderError>> {
        if self.saw_done {
            return Vec::new();
        }
        match frame {
            ParsedEvent::Done => self.finish(supports_finish_reason),
            ParsedEvent::Valid(events) => events
                .into_iter()
                .flat_map(|event| self.process(event))
                .collect(),
            ParsedEvent::UsageError(error) => {
                self.saw_done = true;
                vec![Err(self.failure(error))]
            }
            ParsedEvent::Malformed { .. } => {
                self.saw_done = true;
                vec![Err(self.failure(ProviderError::StreamError(
                    ProviderErrorSummary::attested_static("malformed OpenAI Chat SSE frame"),
                )))]
            }
        }
    }

    fn update_metadata(&mut self, id: Option<String>, model: Option<String>, usage: Option<Usage>) {
        if let Some(response_id) = id {
            self.partial.response_id = Some(response_id);
        }
        if let Some(model) = model
            && !model.is_empty()
        {
            // The reported model is a separate fact from the requested
            // identity stored at construction (AIR-010/RP-02).
            self.partial.response_model = Some(model);
        }
        if let Some(usage) = usage {
            self.partial.usage = usage;
        }
    }

    fn emit_done(&mut self, reason: StopReason) -> AssistantStreamEvent {
        if let Some(model_info) = &self.model_info {
            model_info.apply_response_cost(&mut self.partial);
        }
        self.partial.stop_reason = reason;
        self.saw_done = true;
        AssistantStreamEvent::Done {
            reason,
            message: self.partial.clone(),
        }
    }

    fn ensure_start(&mut self, events: &mut Vec<Result<AssistantStreamEvent, ProviderError>>) {
        if !self.started {
            self.started = true;
            events.push(Ok(AssistantStreamEvent::Start {
                partial: self.partial.clone(),
            }));
        }
    }

    /// Process one OpenAI event, returning zero or more stream events.
    fn process_inner(
        &mut self,
        event: OpenAiChatEvent,
    ) -> Vec<Result<AssistantStreamEvent, ProviderError>> {
        if self.saw_done {
            return Vec::new();
        }
        match event {
            OpenAiChatEvent::RoleDelta {
                model, id, usage, ..
            } => {
                self.update_metadata(id, model, usage);
                let mut events = Vec::new();
                self.ensure_start(&mut events);
                events
            }
            OpenAiChatEvent::ThinkingDelta {
                content,
                mut signature,
                details,
                id,
                model,
                usage,
            } => {
                self.update_metadata(id, model, usage);
                let mut events = Vec::new();
                self.ensure_start(&mut events);
                if self.partial.provider == "opencode-go" && signature == "reasoning" {
                    signature = "reasoning_content".into();
                }
                let index = if let Some(index) = self.thinking_content_index {
                    index
                } else {
                    let index = self.partial.content.len();
                    self.thinking_content_index = Some(index);
                    self.partial.content.push(AssistantContent::Thinking {
                        thinking: String::new(),
                        signature: Some(signature),
                    });
                    events.push(Ok(AssistantStreamEvent::ThinkingStart {
                        content_index: index,
                        partial: self.partial.clone(),
                    }));
                    index
                };
                for detail in details {
                    append_reasoning_detail(&mut self.reasoning_details, detail);
                }
                if let Some(AssistantContent::Thinking {
                    thinking,
                    signature,
                }) = self.partial.content.get_mut(index)
                {
                    thinking.push_str(&content);
                    if !self.reasoning_details.is_empty() {
                        *signature = Some(
                            serde_json::to_string(&self.reasoning_details)
                                .expect("JSON reasoning details"),
                        );
                    }
                }
                if !content.is_empty() {
                    events.push(Ok(AssistantStreamEvent::ThinkingDelta {
                        content_index: index,
                        delta: content,
                        partial: self.partial.clone(),
                    }));
                }
                events
            }
            OpenAiChatEvent::ContentDelta {
                content,
                id,
                model,
                usage,
            } => {
                self.update_metadata(id, model, usage);
                if content.is_empty() {
                    return Vec::new();
                }
                let mut events = Vec::new();
                self.ensure_start(&mut events);
                // Chat content deltas describe one text block across interleaved tool deltas.
                if !self.text_started {
                    self.text_started = true;
                    self.text_content_index = Some(self.partial.content.len());
                    self.partial.content.push(AssistantContent::Text {
                        text: String::new(),
                        signature: None,
                    });
                    events.push(Ok(AssistantStreamEvent::TextStart {
                        content_index: self.text_content_index.expect("started text block"),
                        partial: self.partial.clone(),
                    }));
                }
                let index = self.text_content_index.expect("started text block");
                if let Some(AssistantContent::Text { text, .. }) =
                    self.partial.content.get_mut(index)
                {
                    text.push_str(&content);
                }
                events.push(Ok(AssistantStreamEvent::TextDelta {
                    content_index: index,
                    delta: content,
                    partial: self.partial.clone(),
                }));
                events
            }
            OpenAiChatEvent::ToolCallStart {
                index,
                id,
                name,
                response_id,
                model,
                usage,
            } => {
                self.update_metadata(response_id, model, usage);
                let mut events = Vec::new();
                self.ensure_start(&mut events);

                // Ensure we have room for this tool call
                while self.tool_calls.len() <= index {
                    self.tool_calls.push(ToolCallState {
                        id: String::new(),
                        name: String::new(),
                        arguments: String::new(),
                        custom_input: None,
                    });
                }

                let content_index = self.partial.content.len();
                self.tool_calls[index] = ToolCallState {
                    id: id.clone(),
                    name: name.clone(),
                    arguments: String::new(),
                    custom_input: self.grammar_input_properties.get(&name).map(|property| {
                        crate::constrained_sampling::GrammarInputBuffer::new(
                            property.clone(),
                            String::new(),
                        )
                    }),
                };
                self.partial.content.push(AssistantContent::ToolCall {
                    tool_call: ToolCall {
                        id,
                        name,
                        arguments: String::new(),
                        arguments_error: None,
                        signature: None,
                    },
                });

                events.push(Ok(AssistantStreamEvent::ToolCallStart {
                    content_index,
                    partial: self.partial.clone(),
                }));
                events
            }
            OpenAiChatEvent::ToolCallDelta {
                index,
                arguments,
                id,
                model,
                usage,
            } => {
                self.update_metadata(id, model, usage);
                if arguments.is_empty() || index >= self.tool_calls.len() {
                    return Vec::new();
                }
                let state = &mut self.tool_calls[index];
                let arguments = if let Some(buffer) = &mut state.custom_input {
                    let next = format!("{}{}", buffer.input(), arguments);
                    match crate::constrained_sampling::append_grammar_tool_input_delta(
                        buffer, &next, false,
                    ) {
                        Ok(delta) => delta.unwrap_or_default(),
                        Err(error) => {
                            return vec![Err(self.failure(ProviderError::StreamError(
                                ProviderErrorSummary::sanitized(error.detail()),
                            )))];
                        }
                    }
                } else {
                    arguments
                };
                state.arguments.push_str(&arguments);
                // Map tool_calls index to content index (skip non-tool-call entries)
                let mut tool_count = 0;
                let tool_content_index = self
                    .partial
                    .content
                    .iter()
                    .position(|c| {
                        if matches!(c, AssistantContent::ToolCall { .. }) {
                            if tool_count == index {
                                return true;
                            }
                            tool_count += 1;
                        }
                        false
                    })
                    .unwrap_or(0);
                if let Some(AssistantContent::ToolCall { tool_call }) =
                    self.partial.content.get_mut(tool_content_index)
                {
                    tool_call.arguments.push_str(&arguments);
                }
                vec![Ok(AssistantStreamEvent::ToolCallDelta {
                    content_index: tool_content_index,
                    delta: arguments,
                    partial: self.partial.clone(),
                })]
            }
            OpenAiChatEvent::Finish {
                finish_reason,
                id,
                model,
                usage,
            } => {
                self.update_metadata(id, model, usage);
                let mut events = Vec::new();
                if !finish_reason.is_empty() {
                    self.partial.raw_stop_reason = Some(finish_reason.clone());
                    self.pending_stop_reason = Some(map_stop_reason(&finish_reason));
                    self.ensure_start(&mut events);
                }
                events
            }
            OpenAiChatEvent::Error { message: _ } => {
                // Preserve state that did not produce a public event (usage,
                // opaque content or signatures), alongside the typed cause.
                self.saw_done = true;
                vec![Err(self.failure(ProviderError::StreamError(
                    ProviderErrorSummary::attested_static(OPENAI_CHAT_STREAM_ERROR),
                )))]
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Stop reason mapping
// ---------------------------------------------------------------------------

fn map_stop_reason(raw: &str) -> StopReason {
    match raw {
        "stop" => StopReason::Stop,
        "length" => StopReason::Length,
        "tool_calls" => StopReason::ToolUse,
        "content_filter" => StopReason::Error,
        _ => StopReason::Error,
    }
}

fn empty_assistant_message(
    api: crate::ApiKind,
    provider: &str,
    wire: crate::model_info::WireApi,
    model: &str,
) -> AssistantMessage {
    AssistantMessage {
        projection: None,
        content: Vec::new(),
        api,
        provider: provider.into(),
        model: model.to_owned(),
        response_model: None,
        wire: Some(wire),
        response_id: None,
        usage: Usage::default(),
        stop_reason: StopReason::Pending,
        error_message: None,
        timestamp_ms: crate::time::now_ms(),
        diagnostics: Vec::new(),
        duration_ms: None,
        raw_stop_reason: None,
        provider_thinking_level: None,
        deferred: None,
    }
}

/// Neutral public text substituted for the raw upstream OpenAI Chat
/// `error.message`. A proxy may echo credential material or request fragments
/// in an error frame, so the raw value is never placed in a persisted session
/// or a public event. Mirrors the Anthropic path.
const OPENAI_CHAT_STREAM_ERROR: &str = "openai chat stream error";

/// Apply optional model values without erasing inherited policy on `None`.
fn overlay_optional(
    base: &mut crate::model_info::OpenAiCompletionsCompat,
    explicit: &crate::model_info::OpenAiCompletionsCompat,
) {
    if explicit.supports_mid_convo_system_messages.is_some() {
        base.supports_mid_convo_system_messages = explicit.supports_mid_convo_system_messages;
    }
    if explicit.supports_mid_convo_tool_additions.is_some() {
        base.supports_mid_convo_tool_additions = explicit.supports_mid_convo_tool_additions;
    }
    if explicit.supports_finish_reason.is_some() {
        base.supports_finish_reason = explicit.supports_finish_reason;
    }
    if explicit.supports_grammar_tools.is_some() {
        base.supports_grammar_tools = explicit.supports_grammar_tools;
    }
    if explicit.open_router_routing.is_some() {
        base.open_router_routing = explicit.open_router_routing.clone();
    }
    if explicit.vercel_gateway_routing.is_some() {
        base.vercel_gateway_routing = explicit.vercel_gateway_routing.clone();
    }
    if explicit.cache_control_format.is_some() {
        base.cache_control_format = explicit.cache_control_format;
    }
    if explicit.session_affinity_format.is_some() {
        base.session_affinity_format = explicit.session_affinity_format;
    }
    if explicit.supports_long_cache_retention.is_some() {
        base.supports_long_cache_retention = explicit.supports_long_cache_retention;
    }
    if explicit.store.is_some() {
        base.store = explicit.store.clone();
    }
    if explicit.instruction_role.is_some() {
        base.instruction_role = explicit.instruction_role.clone();
    }
    if explicit.thinking_format.is_some() {
        base.thinking_format = explicit.thinking_format.clone();
    }
    if explicit.supports_reasoning_effort.is_some() {
        base.supports_reasoning_effort = explicit.supports_reasoning_effort;
    }
    if explicit.usage_in_stream.is_some() {
        base.usage_in_stream = explicit.usage_in_stream;
    }
    if explicit.max_tokens_field.is_some() {
        base.max_tokens_field = explicit.max_tokens_field.clone();
    }
    if explicit
        .requires_reasoning_content_on_assistant_messages
        .is_some()
    {
        base.requires_reasoning_content_on_assistant_messages =
            explicit.requires_reasoning_content_on_assistant_messages;
    }
    if explicit.send_session_affinity_headers.is_some() {
        base.send_session_affinity_headers = explicit.send_session_affinity_headers;
    }
    if explicit.vllm_priority.is_some() {
        base.vllm_priority = explicit.vllm_priority.clone();
    }
    if explicit.chat_template_kwargs.is_some() {
        base.chat_template_kwargs = explicit.chat_template_kwargs.clone();
    }
    if explicit.chat_template_args.is_some() {
        base.chat_template_args = explicit.chat_template_args.clone();
    }
    if explicit.cache_key.is_some() {
        base.cache_key = explicit.cache_key.clone();
    }
    if explicit.thinking_token_budget_field.is_some() {
        base.thinking_token_budget_field = explicit.thinking_token_budget_field.clone();
    }
}

/// A provider profile or complete catalog record also owns concrete values.
fn overlay_explicit(base: &mut CompatConfig, explicit: &CompatConfig) {
    overlay_optional(base, explicit);
    base.tool_result_name_field = explicit.tool_result_name_field;
    base.strict_tool_schema = explicit.strict_tool_schema;
    base.require_assistant_after_tool_result = explicit.require_assistant_after_tool_result;
    base.requires_thinking_as_text = explicit.requires_thinking_as_text;
    base.zai_tool_stream = explicit.zai_tool_stream;
    base.supports_thinking_token_budget = explicit.supports_thinking_token_budget;
    if !explicit.chat_completions_path.is_empty() {
        base.chat_completions_path = explicit.chat_completions_path.clone();
    }
}

/// The frozen SE-01 provider-inference discriminant set for the Chat wire.
/// Detection is by exact provider id or parsed base-URL host equality; this
/// replaces Pi's substring probe with known-host matching (recorded as an
/// explicit correction, not a verbatim detector copy).
struct ChatProviderClass {
    non_standard: bool,
    open_router: bool,
    open_router_developer_model: bool,
    legacy_max_tokens: bool,
    deep_seek: bool,
    thinking_format: crate::model_info::ChatThinkingFormat,
    sends_effort_field: bool,
    long_cache_retention: bool,
}

fn classify_chat_provider(provider_id: &str, base_url: &str, model_id: &str) -> ChatProviderClass {
    let host = base_url
        .split("://")
        .nth(1)
        .unwrap_or(base_url)
        .split('/')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    let host_is = |known: &str| host == known || host.ends_with(&format!(".{known}"));

    let zai = provider_id == "zai"
        || provider_id == "zai-coding-cn"
        || host_is("api.z.ai")
        || host_is("open.bigmodel.cn");
    let together =
        provider_id == "together" || host_is("api.together.ai") || host_is("api.together.xyz");
    let moonshot = provider_id == "moonshotai"
        || provider_id == "moonshotai-cn"
        || host_is("api.moonshot.ai")
        || host_is("api.moonshot.cn");
    let open_router = provider_id == "openrouter" || host_is("openrouter.ai");
    let cloudflare_workers_ai =
        provider_id == "cloudflare-workers-ai" || host_is("api.cloudflare.com");
    let cloudflare_ai_gateway =
        provider_id == "cloudflare-ai-gateway" || host_is("gateway.ai.cloudflare.com");
    let nvidia = provider_id == "nvidia" || host_is("integrate.api.nvidia.com");
    let ant_ling = provider_id == "ant-ling" || host_is("api.ant-ling.com");
    let cerebras = provider_id == "cerebras" || host_is("cerebras.ai");
    let deep_seek = provider_id == "deepseek" || host_is("deepseek.com");
    let chutes = host_is("chutes.ai");
    let opencode = provider_id == "opencode" || host_is("opencode.ai");
    let grok = provider_id == "xai" || host_is("api.x.ai");

    let non_standard = nvidia
        || cerebras
        || grok
        || together
        || chutes
        || deep_seek
        || zai
        || moonshot
        || opencode
        || cloudflare_workers_ai
        || cloudflare_ai_gateway
        || ant_ling;
    let open_router_developer_model =
        open_router && (model_id.starts_with("anthropic/") || model_id.starts_with("openai/"));
    let legacy_max_tokens = chutes
        || deep_seek
        || moonshot
        || cloudflare_ai_gateway
        || together
        || nvidia
        || ant_ling
        || zai;
    let thinking_format = if deep_seek {
        crate::model_info::ChatThinkingFormat::DeepSeek
    } else if zai {
        crate::model_info::ChatThinkingFormat::Zai
    } else if together {
        crate::model_info::ChatThinkingFormat::Together
    } else if ant_ling {
        crate::model_info::ChatThinkingFormat::AntLing
    } else if open_router {
        crate::model_info::ChatThinkingFormat::OpenRouter
    } else {
        crate::model_info::ChatThinkingFormat::OpenAi
    };
    let sends_effort_field =
        !(grok || zai || moonshot || together || cloudflare_ai_gateway || nvidia || ant_ling);

    ChatProviderClass {
        non_standard,
        open_router,
        open_router_developer_model,
        legacy_max_tokens,
        deep_seek,
        thinking_format,
        sends_effort_field,
        long_cache_retention: !(together
            || cloudflare_workers_ai
            || cloudflare_ai_gateway
            || nvidia
            || ant_ling),
    }
}

/// Adapter baseline inference (frozen SE-01 table): the pi-baseline defaults a
/// profile starts from before explicit provider/model values apply.
fn detect_chat_baseline(
    provider_id: &str,
    base_url: &str,
    model_id: &str,
) -> crate::model_info::OpenAiCompletionsCompat {
    use crate::model_info::{ChatInstructionRole, ChatStoreField};
    let class = classify_chat_provider(provider_id, base_url, model_id);
    crate::model_info::OpenAiCompletionsCompat {
        store: Some(if class.non_standard {
            ChatStoreField::Omit
        } else {
            ChatStoreField::SendFalse
        }),
        instruction_role: Some(
            if class.open_router_developer_model || (!class.non_standard && !class.open_router) {
                ChatInstructionRole::DeveloperForReasoning
            } else {
                ChatInstructionRole::System
            },
        ),
        thinking_format: Some(class.thinking_format),
        supports_reasoning_effort: Some(class.sends_effort_field),
        usage_in_stream: Some(true),
        supports_finish_reason: Some(true),
        supports_long_cache_retention: Some(class.long_cache_retention),
        cache_control_format: (provider_id == "openrouter" && model_id.starts_with("anthropic/"))
            .then_some(crate::model_info::CacheControlFormat::Anthropic),
        session_affinity_format: Some(if class.open_router {
            crate::model_info::SessionAffinityFormat::OpenRouter
        } else {
            crate::model_info::SessionAffinityFormat::OpenAi
        }),
        max_tokens_field: Some(
            if class.legacy_max_tokens {
                "max_tokens"
            } else {
                "max_completion_tokens"
            }
            .into(),
        ),
        requires_reasoning_content_on_assistant_messages: Some(class.deep_seek),
        send_session_affinity_headers: Some(class.open_router),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------

impl OpenAiRouteContext {
    fn resolve_compat(&self, model_id: &str) -> CompatConfig {
        let model = self.models.iter().find(|model| model.id == model_id);
        self.resolve_compat_for(model_id, model)
    }

    pub(crate) fn resolve_compat_for(
        &self,
        model_id: &str,
        model: Option<&crate::provider::ModelInfo>,
    ) -> CompatConfig {
        let inference_base_url = model
            .and_then(|model| model.base_url.as_deref())
            .unwrap_or(self.base_url.as_str());
        let mut resolved = detect_chat_baseline(&self.provider_id, inference_base_url, model_id);
        overlay_explicit(&mut resolved, &self.compat);
        if let Some(crate::provider::ModelInfo {
            compat: crate::model_info::WireCompat::OpenAiCompletions(compat),
            ..
        }) = model
        {
            if self.catalog_compat {
                overlay_explicit(&mut resolved, compat);
            } else {
                // Profile routes retain their concrete policies; optional model
                // values and the explicit overrides below refine that profile.
                overlay_optional(&mut resolved, compat);
            }
        }
        if let Some(ov) = self.model_overrides.get(model_id) {
            if let Some(role) = &ov.instruction_role {
                resolved.instruction_role = Some(role.clone());
            }
            if let Some(store) = &ov.store {
                resolved.store = Some(store.clone());
            }
            if let Some(format) = &ov.thinking_format {
                resolved.thinking_format = Some(format.clone());
            }
            if let Some(field) = &ov.max_tokens_field {
                resolved.max_tokens_field = Some(field.clone());
            }
        }
        if model.is_some_and(|model| model.base_url.is_some())
            && resolved.chat_completions_path == CompatConfig::default().chat_completions_path
        {
            resolved.chat_completions_path = "/chat/completions".into();
        }
        resolved
    }

    fn validate_tool_projection(
        &self,
        model: &ModelInfo,
        request: &Request,
    ) -> Result<(), ProviderError> {
        let compat = self.resolve_compat_for(&model.id, Some(model));
        let projection = crate::transcript::compile_request_projection(
            request,
            &self.provider_id,
            Some(model),
            self.wire,
        )?;
        compile_chat_tools(
            projection.transcript.messages(),
            &projection.tools.request_tools,
            &compat,
        )?;
        Ok(())
    }

    pub(crate) fn with_wire(mut self, wire: crate::WireApi) -> Self {
        self.wire = wire;
        self
    }

    /// Access the shared HTTP client (for testing client reuse).
    pub fn http_client(&self) -> &Arc<HttpClient> {
        &self.client
    }

    /// Build the OpenAI Chat Completions API request body.
    pub fn build_request_body(
        &self,
        request: &Request,
    ) -> Result<serde_json::Value, crate::provider::ProviderError> {
        self.build_request_body_with_options(request, None)
    }

    fn build_request_body_with_options(
        &self,
        request: &Request,
        native: Option<&OpenAiCompletionsOptions>,
    ) -> Result<serde_json::Value, crate::provider::ProviderError> {
        let model_id = request
            .model
            .split_once(':')
            .map(|(_, id)| id)
            .unwrap_or(&request.model);

        // Resolve the effective compat config for this model (inference ->
        // provider profile -> model override).
        let model = self.models.iter().find(|model| model.id == model_id);
        let compat = self.resolve_compat_for(model_id, model);
        let model_reasoning = model
            .map(|model| model.capabilities.supports_thinking)
            .unwrap_or(false);
        let instruction_role = match &compat.instruction_role {
            Some(crate::model_info::ChatInstructionRole::System) => "system",
            Some(crate::model_info::ChatInstructionRole::Developer) => "developer",
            Some(crate::model_info::ChatInstructionRole::DeveloperForReasoning)
            | Some(crate::model_info::ChatInstructionRole::Auto)
            | None => {
                if model_reasoning {
                    "developer"
                } else {
                    "system"
                }
            }
        };

        let selected_model = self.models.iter().find(|model| model.id == model_id);
        let projection = crate::transcript::compile_request_projection(
            request,
            &self.provider_id,
            selected_model,
            self.wire,
        )?;
        let transcript = projection.transcript.as_ref();
        let system_prompt = crate::transcript::get_initial_system_message(transcript.messages())
            .map(crate::transcript::system_message_text)
            .unwrap_or_default();
        let current_tools = projection.tools.request_tools.clone();
        let wire_tools = compile_chat_tools(transcript.messages(), &current_tools, &compat)?;
        let grammar_properties = wire_tools
            .as_ref()
            .map(|tools| tools.grammar_input_properties.clone())
            .unwrap_or_default();

        // Canonical replay projection (task 20.7) before body building.
        let replay = &projection.replay;
        let mut body = serde_json::json!({
            "model": self.wire_model_override.as_deref().unwrap_or(model_id),
            "stream": true,
            "messages": serialize_messages(
                &replay.messages,
                crate::replay::supports_images_in_catalog(&self.models, model_id),
                &system_prompt,
                instruction_role,
                &compat,
                model_reasoning,
                &grammar_properties,
            )?,
        });

        if compat.usage_in_stream.unwrap_or(true) {
            body["stream_options"] = serde_json::json!({ "include_usage": true });
        }

        // Store-field policy (AIR-022): SendFalse emits `store:false`, Omit
        // leaves the field absent; both differ from an unset policy.
        if matches!(
            compat.store,
            Some(crate::model_info::ChatStoreField::SendFalse)
        ) {
            body["store"] = serde_json::Value::Bool(false);
        }

        if let Some(max_tokens) = request.max_tokens
            && max_tokens != 0
        {
            let field = compat
                .max_tokens_field
                .as_deref()
                .unwrap_or("max_completion_tokens");
            body[field] = serde_json::Value::Number(max_tokens.into());
        }
        if let Some(temp) = request.temperature
            && let Some(n) = serde_json::Number::from_f64(temp)
        {
            body["temperature"] = serde_json::Value::Number(n);
        }
        if let Some(tools) = wire_tools
            && !tools.tools.is_empty()
        {
            body["tools"] = serde_json::Value::Array(tools.tools);
        }
        if !request.stop_sequences.is_empty() {
            body["stop"] = serde_json::Value::Array(
                request
                    .stop_sequences
                    .iter()
                    .map(|s| serde_json::Value::String(s.clone()))
                    .collect(),
            );
        }
        // Native options own effort and budgets; the simple entry clamps the
        // neutral intent before this point; the plain Provider path carries
        // the neutral intent unchanged.
        let (effort, budget_override) = match native {
            Some(options) if !options.simple => (
                options.reasoning_effort,
                options
                    .reasoning_effort
                    .and_then(|level| options.thinking_budgets.get(&level).copied()),
            ),
            _ => (
                (request.thinking.enabled
                    && request.thinking.level != crate::model_info::ThinkingLevel::None)
                    .then_some(request.thinking.level),
                request.thinking.budget_tokens,
            ),
        };
        apply_chat_thinking_projection(
            &mut body,
            &compat,
            model,
            effort,
            budget_override,
            request.max_tokens,
            !current_tools.is_empty(),
        );
        if let Some(tool_choice) = native.and_then(|options| options.tool_choice.as_ref()) {
            body["tool_choice"] = tool_choice.wire_value();
        }
        if request.cache_retention != CacheRetention::Disabled {
            if let Some(cache_key) = compat.cache_key.as_ref()
                && !cache_key.is_empty()
            {
                body["prompt_cache_key"] = serde_json::Value::String(cache_key.clone());
            }
            let long_cache = request.cache_retention == CacheRetention::Long
                && compat.supports_long_cache_retention.unwrap_or(true);
            if (self.direct_prompt_cache_key || long_cache)
                && let Some(session_id) = request.session_id.as_deref()
                && !session_id.is_empty()
            {
                body["prompt_cache_key"] =
                    serde_json::Value::String(session_id.chars().take(64).collect());
            }
            if long_cache {
                body["prompt_cache_retention"] = serde_json::json!("24h");
            }
            if compat.cache_control_format == Some(crate::model_info::CacheControlFormat::Anthropic)
            {
                let mut marker = serde_json::json!({"type":"ephemeral"});
                if long_cache {
                    marker["ttl"] = serde_json::json!("1h");
                }
                apply_chat_cache_control(&mut body, &marker);
            }
        }
        if let Some(routing) = &compat.open_router_routing {
            body["provider"] = serde_json::to_value(routing).expect("typed routing serializes");
        }
        if let Some(routing) = &compat.vercel_gateway_routing
            && (routing.only.is_some() || routing.order.is_some())
        {
            body["providerOptions"] = serde_json::json!({"gateway":routing});
        }
        // Last so model and request sampling parameters override named
        // request fields (model base < effective level < request, per key).
        let effective_level = effort.unwrap_or(crate::model_info::ThinkingLevel::None);
        if let Some(model) = self.models.iter().find(|model| model.id == model_id)
            && let Some(sampling) = crate::model_info::resolve_sampling_params(
                model,
                effective_level,
                request.sampling_params.as_ref(),
            )
        {
            for (key, value) in sampling {
                body[key.as_str()] = value;
            }
        }
        Ok(body)
    }

    /// Stream events from a raw SSE response body.
    pub fn stream_from_sse(&self, sse_body: &str, cancel: CancellationToken) -> EventStream {
        let mut mapper = OpenAiChatMapper::new(
            crate::ApiKind::OpenAi,
            &self.provider_id,
            crate::model_info::WireApi::OpenAiCompletions,
            "",
        );
        mapper.set_model_info(None);
        let mut stream_events: Vec<Result<AssistantStreamEvent, ProviderError>> = Vec::new();
        let is_openrouter =
            self.provider_id == "openrouter" || self.base_url.contains("openrouter.ai");
        for (parsed, _raw) in parse_sse_events_with_raw_for_model(sse_body, is_openrouter) {
            stream_events.extend(
                mapper.process_frame(parsed, self.compat.supports_finish_reason.unwrap_or(true)),
            );
            if mapper.saw_done {
                break;
            }
        }
        stream_events.extend(mapper.finish(self.compat.supports_finish_reason.unwrap_or(true)));

        let _cancel = cancel;
        Box::pin(stream::iter(stream_events))
    }

    /// Stamp the measured response duration onto a terminal Done message;
    /// already-known values and non-Done items pass through unchanged.
    fn stamp_chat_done_duration(
        item: Result<AssistantStreamEvent, crate::provider::ProviderError>,
        activated_at: Option<std::time::Instant>,
    ) -> Result<AssistantStreamEvent, crate::provider::ProviderError> {
        match (item, activated_at) {
            (
                Ok(AssistantStreamEvent::Done {
                    reason,
                    mut message,
                }),
                Some(activated_at),
            ) => {
                if message.duration_ms.is_none() {
                    message.duration_ms = Some(activated_at.elapsed().as_millis() as u64);
                }
                Ok(AssistantStreamEvent::Done { reason, message })
            }
            (item, _) => item,
        }
    }

    /// Real HTTP streaming: POST to OpenAI Chat Completions API.
    #[allow(clippy::too_many_arguments)]
    async fn stream_http(
        http_client: reqwest::Client,
        resolved: ResolvedAuth,
        base_url: String,
        provider_id: String,
        auth_invalid_policy: AuthInvalidPolicy,
        chat_completions_path: String,
        requested_model: String,
        model_info: Option<ModelInfo>,
        compat: CompatConfig,
        tool_metadata: Option<crate::api::shared::openai_responses::ResponsesWireTools>,
        extra_headers: Vec<(String, String)>,
        body: &serde_json::Value,
        cancel: CancellationToken,
        timeout: Option<std::time::Duration>,
        callbacks: crate::callbacks::RequestCallbacks,
        snapshot: crate::stream::ProducerSnapshot,
        tx: &tokio::sync::mpsc::Sender<Result<AssistantStreamEvent, ProviderError>>,
    ) -> Result<(), ProviderError> {
        let secret = resolved.secret.expose_secret();
        let bearer = match resolved.scheme {
            AuthScheme::ApiKey if provider_id == "cloudflare-ai-gateway" => {
                format!("Bearer {secret}")
            }
            AuthScheme::Bearer => format!("Bearer {secret}"),
            AuthScheme::ApiKey | AuthScheme::AwsSigV4(_) | AuthScheme::AnthropicFederation => {
                return Err(ProviderError::Config(
                    ProviderErrorSummary::attested_static(
                        "OpenAI Chat accepts only Bearer authentication",
                    ),
                ));
            }
        };
        let mut req = http_client
            .post(crate::api::shared::endpoint::join_endpoint(
                &base_url,
                &chat_completions_path,
            ))
            .header(
                if provider_id == "cloudflare-ai-gateway" {
                    "cf-aig-authorization"
                } else {
                    "authorization"
                },
                bearer,
            )
            .header("content-type", "application/json");
        // Apply per-request timeout.
        if let Some(d) = timeout {
            req = req.timeout(d);
        }
        for (name, value) in &extra_headers {
            req = req.header(name.as_str(), value.as_str());
        }
        let response = req
            .body(serde_json::to_string(body).unwrap_or_default())
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    ProviderError::Timeout
                } else if e.is_redirect() {
                    // Redirect limit exceeded at acquisition: the backend
                    // stops after the lawful hops. The cause is reported as
                    // a producer-attested static string so no redirect
                    // target or credential material leaks into diagnostics.
                    ProviderError::Network(ProviderErrorSummary::attested_static(
                        "too many redirects",
                    ))
                } else {
                    ProviderError::Network(ProviderErrorSummary::sanitized(e.to_string()))
                }
            })?;

        // Response observer (task 20.51): fires once the status and headers
        // exist, before any body processing or error mapping.
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
            let cause = map_http_status(status, &headers, auth_invalid_policy, &provider_id);
            return Err(if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                crate::http::openai_rate_limit_detail(response, cause, &cancel).await
            } else {
                cause
            });
        }
        // Response callback (V-36): fired exactly once on the successfully
        // acquired response; rejected dispatches never masquerade.
        snapshot.received_response();

        let mut byte_stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut decoder = crate::api::shared::text_decode::StreamTextDecoder::new();
        let is_openrouter = provider_id == "openrouter" || base_url.contains("openrouter.ai");
        // AIR-061: monotonic duration from response-stream activation (first
        // body poll) to terminal; setup time before activation stays untimed.
        let mut response_activated_at: Option<std::time::Instant> = None;
        let mut mapper = OpenAiChatMapper::new(
            crate::ApiKind::OpenAi,
            &provider_id,
            crate::model_info::WireApi::OpenAiCompletions,
            &requested_model,
        );
        mapper.set_model_info(model_info);

        mapper.set_tool_metadata(tool_metadata);
        mapper.set_snapshot(snapshot);
        callbacks
            .notify_response_async(status.as_u16(), &observed_headers)
            .await
            .map_err(|error| mapper.failure(error))?;
        let result: Result<(), ProviderError> = async {
            loop {
                let chunk = tokio::select! {
                    _ = cancel.cancelled() => {
                        return Err(mapper.failure(ProviderError::Cancelled));
                    }
                    chunk = async {
                        response_activated_at.get_or_insert_with(std::time::Instant::now);
                        byte_stream.next().await
                    } => {
                        match chunk {
                            Some(c) => c,
                            None => break,
                        }
                    }
                };

                let chunk = chunk.map_err(|e| {
                    if e.is_timeout() {
                        ProviderError::Timeout
                    } else if e.is_connect() {
                        ProviderError::Network(ProviderErrorSummary::sanitized(e.to_string()))
                    } else {
                        ProviderError::StreamError(ProviderErrorSummary::sanitized(e.to_string()))
                    }
                })?;
                buffer.push_str(&decoder.push(&chunk));

                for (parsed, raw) in drain_sse_events(&mut buffer, is_openrouter) {
                    // Raw observer first: strictly before normalization, in
                    // wire order (task 20.51); failure ends the call with the
                    // partial preserved.
                    if !mapper.saw_done
                        && let Some(raw) = raw.as_ref()
                        && let Err(error) = callbacks.notify_raw_async(raw).await
                    {
                        return Err(mapper.failure(error));
                    }
                    for item in
                        mapper.process_frame(parsed, compat.supports_finish_reason.unwrap_or(true))
                    {
                        let item = Self::stamp_chat_done_duration(item, response_activated_at);
                        let terminal = item.as_ref().map_or(true, |event| event.is_terminal());
                        if !send_or_cancel(&cancel, tx, item).await? || terminal {
                            return Ok(());
                        }
                    }
                }
            }

            let eof_tail = decoder.finish();
            buffer.push_str(&eof_tail);
            for item in mapper.finish(compat.supports_finish_reason.unwrap_or(true)) {
                let item = Self::stamp_chat_done_duration(item, response_activated_at);
                if !send_or_cancel(&cancel, tx, item).await? {
                    return Ok(());
                }
            }

            Ok(())
        }
        .await;
        result.map_err(|cause| mapper.failure(cause))
    }
}

// ---------------------------------------------------------------------------
// Streaming helpers
// ---------------------------------------------------------------------------

pub(crate) fn drain_sse_events(
    buffer: &mut String,
    is_openrouter: bool,
) -> Vec<(ParsedEvent, Option<serde_json::Value>)> {
    if buffer.contains('\r') {
        // A trailing CR after data may be the first half of a split CRLF.
        // A CR ending a blank line already completes a frame; a later LF is
        // only an empty delimiter and cannot split that frame's data lines.
        let pending_cr = buffer
            .strip_suffix('\r')
            .is_some_and(|prefix| !prefix.ends_with(['\r', '\n']));
        if pending_cr {
            buffer.pop();
        }
        *buffer = buffer.replace("\r\n", "\n").replace('\r', "\n");
        if pending_cr {
            buffer.push('\r');
        }
    }

    let mut events = Vec::new();
    while let Some(idx) = buffer.find("\n\n") {
        let end = idx + 2;
        let chunk: String = buffer.drain(..end).collect();
        events.extend(parse_sse_events_with_raw_for_model(&chunk, is_openrouter));
    }
    events
}

fn map_http_status(
    status: reqwest::StatusCode,
    headers: &reqwest::header::HeaderMap,
    policy: AuthInvalidPolicy,
    provider_id: &str,
) -> ProviderError {
    match status.as_u16() {
        // Auth-invalid classification comes from route construction policy,
        // never from the syntax used for the Authorization header.
        401 | 403 => policy.error(provider_id),
        429 => ProviderError::RateLimited {
            retry_after_ms: crate::retry::parse_retry_after(headers),
        },
        408 | 504 => ProviderError::Timeout,
        code => ProviderError::ProviderSide(ProviderErrorSummary::from_http_response(code)),
    }
}

/// Pi-1.3.0 thinking-budget defaults per level; xhigh/max budget families
/// downgrade to high (AIR-053).
fn chat_thinking_budget_default(level: crate::model_info::ThinkingLevel) -> u64 {
    use crate::model_info::ThinkingLevel::*;
    match level {
        Minimal => 1024,
        Low => 2048,
        Medium => 8192,
        High | XHigh | Max => 16384,
        None => 1024,
    }
}

/// One declared thinking format's on/off/null/omission wire projection
/// (AIR-022 / V-11). `effort` is the per-call intent (None when disabled or
/// off); the model thinking-level map owns value mapping and off semantics:
/// `resolve(None)` erroring plays Pi's `off: null` (the wire cannot disable),
/// `Ok(None)` plays an unset off value and format defaults apply.
#[allow(clippy::too_many_arguments)]
fn apply_chat_thinking_projection(
    body: &mut serde_json::Value,
    compat: &CompatConfig,
    model: Option<&crate::provider::ModelInfo>,
    effort: Option<crate::model_info::ThinkingLevel>,
    budget_override: Option<u64>,
    max_tokens: Option<u64>,
    has_tools: bool,
) {
    use crate::model_info::ChatThinkingFormat as Format;

    let model_reasoning = model
        .map(|model| model.capabilities.supports_thinking)
        .unwrap_or(false);
    let thinking_map = model.map(|model| &model.thinking_level_map);
    let mapped = |level: crate::model_info::ThinkingLevel| -> Option<String> {
        thinking_map
            .and_then(|map| map.resolve(level).ok())
            .flatten()
    };
    let effort = effort.filter(|level| *level != crate::model_info::ThinkingLevel::None);
    let off_wire_value =
        || match thinking_map.map(|map| map.resolve(crate::model_info::ThinkingLevel::None)) {
            Some(Ok(value)) => Some(value),
            // Off not mappable: Pi's `off: null`; the wire cannot disable.
            Some(Err(_)) | None => None,
        };
    let supports_effort_field = compat.supports_reasoning_effort.unwrap_or(true);

    let budget_field = compat.thinking_token_budget_field.clone().or({
        if compat.supports_thinking_token_budget {
            Some(crate::model_info::ChatThinkingTokenBudgetField::ThinkingTokenBudget)
        } else {
            None
        }
    });
    // Budget only exists for an active effort on a reasoning model, clamped so
    // at least MIN_ANSWER_TOKENS remain under the shared response ceiling.
    let mut thinking_budget: Option<u64> = None;
    if let (Some(level), true) = (effort, model_reasoning) {
        let ceiling = max_tokens
            .or(model.map(|model| model.capabilities.max_output_tokens))
            .unwrap_or(0);
        let budget = budget_override.unwrap_or_else(|| chat_thinking_budget_default(level));
        let clamped = budget.min(ceiling.saturating_sub(1024));
        if clamped > 0 {
            thinking_budget = Some(clamped);
        }
    }

    let format = compat.thinking_format.clone().unwrap_or_default();
    match format {
        Format::Zai if model_reasoning => {
            body["thinking"] = if effort.is_some() {
                serde_json::json!({"type": "enabled", "clear_thinking": false})
            } else {
                serde_json::json!({"type": "disabled"})
            };
            if let Some(level) = effort
                && supports_effort_field
                && let Some(value) = mapped(level)
            {
                body["reasoning_effort"] = serde_json::Value::String(value);
            }
        }
        Format::Qwen if model_reasoning => {
            body["enable_thinking"] = serde_json::Value::Bool(effort.is_some());
            if let Some(level) = effort
                && supports_effort_field
                && let Some(value) = mapped(level)
            {
                body["reasoning_effort"] = serde_json::Value::String(value);
            }
        }
        Format::QwenChatTemplate if model_reasoning => {
            body["chat_template_kwargs"] = serde_json::json!({
                "enable_thinking": effort.is_some(),
                "preserve_thinking": true,
            });
        }
        Format::ChatTemplate if model_reasoning => {
            let values = resolve_chat_template_values(
                compat.chat_template_kwargs.as_ref(),
                effort,
                mapped,
                thinking_budget,
            );
            if !values.is_empty() {
                body["chat_template_kwargs"] = serde_json::Value::Object(values);
            }
        }
        Format::Baseten if model_reasoning => {
            let values = resolve_chat_template_values(
                compat.chat_template_args.as_ref(),
                effort,
                mapped,
                thinking_budget,
            );
            if !values.is_empty() {
                body["chat_template_args"] = serde_json::Value::Object(values);
            }
            if supports_effort_field {
                let value = match effort {
                    Some(level) => mapped(level),
                    None => off_wire_value().flatten(),
                };
                if let Some(value) = value {
                    body["reasoning_effort"] = serde_json::Value::String(value);
                }
            }
        }
        Format::DeepSeek if model_reasoning => {
            if effort.is_some() {
                body["thinking"] = serde_json::json!({"type": "enabled"});
            } else if off_wire_value().is_some() {
                // Pi gates on `map.off !== null`: any mappable off (explicit
                // value or unset) still sends the disable marker.
                body["thinking"] = serde_json::json!({"type": "disabled"});
            }
            if let Some(level) = effort
                && supports_effort_field
                && let Some(value) = mapped(level)
            {
                body["reasoning_effort"] = serde_json::Value::String(value);
            }
        }
        Format::OpenRouter if model_reasoning => {
            if let Some(level) = effort {
                body["reasoning"] = serde_json::json!({
                    "effort": mapped(level)
                        .unwrap_or_else(|| level.wire_name().unwrap_or("off").to_owned()),
                });
            } else if let Some(off) = off_wire_value() {
                body["reasoning"] = serde_json::json!({
                    "effort": off.unwrap_or_else(|| "none".to_owned()),
                });
            }
        }
        Format::AntLing if model_reasoning => {
            if let Some(level) = effort
                && let Some(value) = mapped(level)
            {
                body["reasoning"] = serde_json::json!({ "effort": value });
            }
        }
        Format::Together if model_reasoning => {
            body["reasoning"] = serde_json::json!({"enabled": effort.is_some()});
            if let Some(level) = effort
                && supports_effort_field
                && let Some(value) = mapped(level)
            {
                body["reasoning_effort"] = serde_json::Value::String(value);
            }
        }
        Format::StringThinking if model_reasoning => {
            if let Some(level) = effort {
                let value =
                    mapped(level).unwrap_or_else(|| level.wire_name().unwrap_or("off").to_owned());
                body["thinking"] = serde_json::Value::String(value);
            } else if let Some(off) = off_wire_value() {
                body["thinking"] =
                    serde_json::Value::String(off.unwrap_or_else(|| "none".to_owned()));
            }
        }
        Format::OpenAi
        | Format::Zai
        | Format::Qwen
        | Format::QwenChatTemplate
        | Format::ChatTemplate
        | Format::Baseten
        | Format::DeepSeek
        | Format::OpenRouter
        | Format::AntLing
        | Format::Together
        | Format::StringThinking => {
            // OpenAI-style `reasoning_effort`: on with a mapped value, off with
            // a declared off value; unsupported levels never invent a field.
            if model_reasoning && supports_effort_field {
                let value = match effort {
                    Some(level) => mapped(level),
                    None => off_wire_value().flatten(),
                };
                if let Some(value) = value {
                    body["reasoning_effort"] = serde_json::Value::String(value);
                }
            }
        }
    }

    // Top-level budget cap, independent of the declared format.
    if let Some(field) = budget_field
        && let Some(budget) = thinking_budget
    {
        body[field.wire_name()] = serde_json::Value::Number(budget.into());
    }
    if compat.zai_tool_stream && has_tools {
        body["tool_stream"] = serde_json::Value::Bool(true);
    }
    if let Some(priority) = &compat.vllm_priority {
        body["priority"] = serde_json::Value::Number(priority.clone());
    }
}

/// Resolve `chat_template_kwargs`/`args` values against the request intent;
/// unset resolutions (for example a missing budget) drop the key entirely.
fn resolve_chat_template_values(
    values: Option<&std::collections::BTreeMap<String, crate::model_info::ChatTemplateKwargValue>>,
    effort: Option<crate::model_info::ThinkingLevel>,
    mapped: impl Fn(crate::model_info::ThinkingLevel) -> Option<String>,
    thinking_budget: Option<u64>,
) -> serde_json::Map<String, serde_json::Value> {
    use crate::model_info::{ChatTemplateKwargValue, ChatTemplateKwargVar};
    let mut resolved = serde_json::Map::new();
    for (key, value) in values.into_iter().flatten() {
        let resolved_value = match value {
            ChatTemplateKwargValue::Literal(literal) => Some(literal.clone()),
            ChatTemplateKwargValue::Var { var, omit_when_off } => {
                if effort.is_none() && *omit_when_off {
                    continue;
                }
                match var {
                    ChatTemplateKwargVar::ThinkingEnabled => {
                        Some(serde_json::Value::Bool(effort.is_some()))
                    }
                    ChatTemplateKwargVar::ThinkingBudget => {
                        thinking_budget.map(|budget| serde_json::Value::Number(budget.into()))
                    }
                    ChatTemplateKwargVar::ThinkingEffort => {
                        let value = match effort {
                            Some(level) => {
                                mapped(level).or_else(|| level.wire_name().map(str::to_owned))
                            }
                            None => None,
                        };
                        value.map(serde_json::Value::String)
                    }
                }
            }
        };
        if let Some(resolved_value) = resolved_value {
            resolved.insert(key.clone(), resolved_value);
        }
    }
    resolved
}

fn serialize_messages(
    messages: &[crate::message::Message],
    supports_images: bool,
    system_prompt: &str,
    instruction_role: &str,
    compat: &CompatConfig,
    model_reasoning: bool,
    grammar_properties: &HashMap<String, String>,
) -> Result<serde_json::Value, ProviderError> {
    let mut result = Vec::new();

    let bridge_wire = compat.require_assistant_after_tool_result;
    let thinking_as_text = compat.requires_thinking_as_text;
    let reasoning_content_assistant = compat
        .requires_reasoning_content_on_assistant_messages
        .unwrap_or(false);

    // System message first
    if !system_prompt.is_empty() {
        result.push(serde_json::json!({
            "role": instruction_role,
            "content": system_prompt,
        }));
    }

    fn image_part_url(
        source: &crate::message::ImageSource,
        media_type: &crate::message::MediaType,
    ) -> String {
        match source {
            crate::message::ImageSource::Url { url } => url.clone(),
            crate::message::ImageSource::Base64 { data } => {
                format!("data:{};base64,{}", media_type.as_str(), data)
            }
            crate::message::ImageSource::Bytes { data } => format!(
                "data:{};base64,{}",
                media_type.as_str(),
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, data),
            ),
        }
    }

    fn push_assistant_bridge(result: &mut Vec<serde_json::Value>) {
        result.push(serde_json::json!({
            "role": "assistant",
            "content": "I have processed the tool results.",
        }));
    }

    // `Some("toolResult")` after a tool-result run without images; the next
    // user message on a bridge wire inserts the fixed assistant turn.
    let mut last_role: Option<&str> = None;
    let mut index = 0;
    while index < messages.len() {
        let msg = &messages[index];
        match msg {
            crate::message::Message::User(u) => {
                if bridge_wire && last_role == Some("toolResult") {
                    push_assistant_bridge(&mut result);
                }
                let content: Vec<serde_json::Value> = u
                    .content
                    .iter()
                    .map(|c| match c {
                        crate::message::InputContent::Text { text } => {
                            serde_json::json!({"type": "text", "text": text})
                        }
                        crate::message::InputContent::Image { source, media_type } => {
                            serde_json::json!({
                                "type": "image_url",
                                "image_url": {"url": image_part_url(source, media_type)}
                            })
                        }
                    })
                    .collect();
                // If single text content, flatten to string
                if content.len() == 1
                    && let Some(text_val) = content[0].get("text")
                {
                    result.push(serde_json::json!({
                        "role": "user",
                        "content": text_val,
                    }));
                } else {
                    result.push(serde_json::json!({
                        "role": "user",
                        "content": content,
                    }));
                }
                last_role = Some("user");
            }
            crate::message::Message::Assistant(a) => {
                let mut tool_calls_json = Vec::new();
                let mut text_parts: Vec<&str> = Vec::new();
                let mut thinking_blocks: Vec<(&str, Option<&str>)> = Vec::new();
                let mut reasoning_details = None;

                for c in &a.content {
                    match c {
                        AssistantContent::Text { text, .. } => {
                            if !text.trim().is_empty() {
                                text_parts.push(text);
                            }
                        }
                        AssistantContent::ToolCall { tool_call } => {
                            // The wire's arguments field is a raw JSON string:
                            // replayed proposals ride their original text
                            // (malformed ones included), never a re-encoded
                            // or fabricated substitute (RP-08).
                            if let Some(property) = grammar_properties.get(&tool_call.name) {
                                let input =
                                    serde_json::from_str::<serde_json::Value>(&tool_call.arguments)
                                        .ok()
                                        .and_then(|arguments| {
                                            arguments
                                                .get(property)
                                                .and_then(serde_json::Value::as_str)
                                                .map(str::to_owned)
                                        })
                                        .unwrap_or_default();
                                tool_calls_json.push(serde_json::json!({
                                    "id":tool_call.id,"type":"custom",
                                    "custom":{"name":tool_call.name,"input":input}
                                }));
                                continue;
                            }
                            tool_calls_json.push(serde_json::json!({
                                "id": tool_call.id,
                                "type": "function",
                                "function": {
                                    "name": tool_call.name,
                                    "arguments": tool_call.arguments,
                                }
                            }));
                        }
                        AssistantContent::Thinking {
                            thinking,
                            signature,
                        } => {
                            if reasoning_details.is_none()
                                && let Some(signature) = signature
                                && let Ok(details) =
                                    serde_json::from_str::<serde_json::Value>(signature)
                                && details.as_array().is_some_and(|items| {
                                    !items.is_empty() && items.iter().all(valid_reasoning_detail)
                                })
                            {
                                reasoning_details = Some(details);
                            }
                            if !thinking.trim().is_empty() {
                                thinking_blocks.push((thinking, signature.as_deref()));
                            }
                        }
                        AssistantContent::RedactedThinking { .. } => {}
                    }
                }

                let assistant_text = text_parts.join("");
                let mut assistant_msg = serde_json::json!({
                    "role": "assistant",
                });
                // Bridge wires never send null assistant content.
                assistant_msg["content"] = if bridge_wire {
                    serde_json::Value::String(String::new())
                } else {
                    serde_json::Value::Null
                };
                if !thinking_blocks.is_empty() && thinking_as_text {
                    // Convert thinking to plain text (no tags, so the model
                    // does not mimic markup).
                    let thinking_text = thinking_blocks
                        .iter()
                        .map(|(thinking, _)| *thinking)
                        .collect::<Vec<_>>()
                        .join("\n\n");
                    let mut parts = vec![serde_json::json!({
                        "type": "text",
                        "text": thinking_text,
                    })];
                    parts.extend(
                        text_parts
                            .iter()
                            .map(|text| serde_json::json!({"type": "text", "text": text})),
                    );
                    assistant_msg["content"] = serde_json::Value::Array(parts);
                } else if !assistant_text.is_empty() {
                    // Plain string content is the Chat Completions standard;
                    // block-array mirrors confuse some endpoints.
                    assistant_msg["content"] = serde_json::Value::String(assistant_text.clone());
                }

                if !thinking_as_text && !thinking_blocks.is_empty() && reasoning_details.is_none() {
                    let joined = thinking_blocks
                        .iter()
                        .map(|(thinking, _)| *thinking)
                        .collect::<Vec<_>>()
                        .join("\n");
                    let signature = thinking_blocks.iter().find_map(|&(_, signature)| signature);
                    if let Some(signature) = signature
                        && matches!(
                            signature,
                            "reasoning" | "reasoning_content" | "reasoning_text"
                        )
                    {
                        assistant_msg[signature] = serde_json::Value::String(joined);
                    }
                }
                if let Some(details) = reasoning_details {
                    // Structured replay data can be the only contents of a
                    // signed thinking block; it does not require visible text.
                    assistant_msg["reasoning_details"] = details;
                }

                if !tool_calls_json.is_empty() {
                    assistant_msg["tool_calls"] = serde_json::Value::Array(tool_calls_json);
                }

                if reasoning_content_assistant
                    && model_reasoning
                    && assistant_msg.get("reasoning_content").is_none()
                {
                    assistant_msg["reasoning_content"] = serde_json::Value::String(String::new());
                }

                // Skip assistant turns with no content and no tool calls:
                // aborted responses with nothing produced.
                let has_content = match assistant_msg.get("content") {
                    Some(serde_json::Value::String(text)) => !text.is_empty(),
                    Some(serde_json::Value::Array(parts)) => !parts.is_empty(),
                    _ => false,
                };
                if !has_content && assistant_msg.get("tool_calls").is_none() {
                    index += 1;
                    continue;
                }
                result.push(assistant_msg);
                last_role = Some("assistant");
            }
            crate::message::Message::System(system) => {
                if compat.supports_mid_convo_tool_additions.unwrap_or(false)
                    && !system.tools_added.is_empty()
                {
                    let tools = convert_chat_tools(&system.tools_added, compat)?.tools;
                    result.push(serde_json::json!({"role":"system","tools":tools}));
                }
                let text = crate::transcript::render_system_message_update(system);
                if !text.is_empty() {
                    result.push(serde_json::json!({"role":instruction_role,"content":text}));
                }
            }
            crate::message::Message::ToolResult(_) => {
                // Consecutive tool results serialize as one run: each keeps a
                // text tool message; extracted images ride one trailing user
                // message (AIR-022 tool-result image projection).
                let mut image_blocks: Vec<serde_json::Value> = Vec::new();
                while let Some(crate::message::Message::ToolResult(t)) = messages.get(index) {
                    let text_result: String = t
                        .content
                        .iter()
                        .filter_map(|c| match c {
                            OutputContent::Text { text } => Some(text.clone()),
                            OutputContent::Image { .. } => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    let has_images = t
                        .content
                        .iter()
                        .any(|c| matches!(c, OutputContent::Image { .. }));

                    // Source placeholders keep distinct absence semantics;
                    // this wire has no native error field, so the deterministic
                    // failure marker rides the leading text (AIR-014).
                    let tool_result_text = if !text_result.is_empty() {
                        text_result
                    } else if has_images {
                        "(see attached image)".to_owned()
                    } else {
                        "(no tool output)".to_owned()
                    };
                    let tool_result_text = if t.is_error {
                        format!("{TOOL_ERROR_MARKER}{tool_result_text}")
                    } else {
                        tool_result_text
                    };
                    let mut tool_msg = serde_json::json!({
                        "role": "tool",
                        "tool_call_id": t.tool_call_id,
                        "content": tool_result_text,
                    });
                    if compat.tool_result_name_field {
                        tool_msg["name"] = serde_json::Value::String(t.tool_name.clone());
                    }
                    result.push(tool_msg);

                    if supports_images {
                        for c in &t.content {
                            if let OutputContent::Image { source, media_type } = c {
                                image_blocks.push(serde_json::json!({
                                    "type": "image_url",
                                    "image_url": {"url": image_part_url(source, media_type)},
                                }));
                            }
                        }
                    }
                    index += 1;
                }

                if !image_blocks.is_empty() {
                    if bridge_wire {
                        push_assistant_bridge(&mut result);
                    }
                    let mut parts = vec![
                        serde_json::json!({"type": "text", "text": "Attached image(s) from tool result:"}),
                    ];
                    parts.extend(image_blocks);
                    result.push(serde_json::json!({
                        "role": "user",
                        "content": parts,
                    }));
                    last_role = Some("user");
                } else {
                    last_role = Some("toolResult");
                }
                continue;
            }
        }
        index += 1;
    }

    Ok(serde_json::Value::Array(result))
}

fn nest_chat_tool(mut tool: serde_json::Value) -> serde_json::Value {
    let kind = tool["type"].as_str().unwrap_or("function").to_owned();
    tool.as_object_mut()
        .expect("compiled tool object")
        .remove("type");
    let key = if kind == "custom" {
        "custom"
    } else {
        "function"
    };
    serde_json::json!({"type":kind,key:tool})
}

fn chat_tool_options(
    compat: &CompatConfig,
) -> crate::api::shared::openai_responses::ResponsesToolOptions {
    crate::api::shared::openai_responses::ResponsesToolOptions {
        default_strict: Some(false),
        supports_strict_mode: compat.strict_tool_schema,
        supports_grammar_tools: compat.supports_grammar_tools.unwrap_or(false),
    }
}

fn convert_chat_tools(
    tools: &[crate::message::ToolDef],
    compat: &CompatConfig,
) -> Result<crate::api::shared::openai_responses::ResponsesWireTools, ProviderError> {
    let mut compiled = crate::api::shared::openai_responses::convert_responses_tools(
        tools,
        chat_tool_options(compat),
    )?;
    compiled.tools = compiled.tools.into_iter().map(nest_chat_tool).collect();
    Ok(compiled)
}

fn compile_chat_tools(
    messages: &[crate::message::Message],
    request_tools: &[crate::message::ToolDef],
    compat: &CompatConfig,
) -> Result<Option<crate::api::shared::openai_responses::ResponsesWireTools>, ProviderError> {
    let compiled = crate::api::shared::openai_responses::compile_projected_response_tools(
        messages,
        request_tools,
        chat_tool_options(compat),
    )?;
    Ok(compiled.map(|mut tools| {
        tools.tools = tools.tools.into_iter().map(nest_chat_tool).collect();
        tools
    }))
}

fn apply_chat_cache_control(body: &mut serde_json::Value, marker: &serde_json::Value) {
    fn mark_text(message: &mut serde_json::Value, marker: &serde_json::Value) -> bool {
        if let Some(text) = message["content"].as_str() {
            if text.is_empty() {
                return false;
            }
            message["content"] =
                serde_json::json!([{"type":"text","text":text,"cache_control":marker}]);
            return true;
        }
        if let Some(parts) = message["content"].as_array_mut()
            && let Some(part) = parts.iter_mut().rev().find(|part| part["type"] == "text")
        {
            part["cache_control"] = marker.clone();
            return true;
        }
        false
    }
    if let Some(tools) = body["tools"].as_array_mut()
        && let Some(last) = tools.last_mut()
    {
        last["cache_control"] = marker.clone();
    }
    if let Some(messages) = body["messages"].as_array_mut() {
        if let Some(first) = messages
            .iter_mut()
            .find(|message| matches!(message["role"].as_str(), Some("system" | "developer")))
        {
            mark_text(first, marker);
        }
        for message in messages.iter_mut().rev() {
            if matches!(
                message["role"].as_str(),
                Some("user" | "assistant" | "tool")
            ) && mark_text(message, marker)
            {
                break;
            }
        }
    }
}

impl OpenAiRouteContext {
    fn stream_prepared_with_options(
        &self,
        mut request: Request,
        resolved: crate::auth::ResolvedAuth,
        native_options: Option<OpenAiCompletionsOptions>,
    ) -> EventStream {
        let projection = /* frozen call */ match crate::provider::request_projection(self, &request, self.wire) {
            Ok(facts) => facts,
            Err(error) => return Box::pin(futures_util::stream::once(async move { Err(error) })),
        };
        let event_projection = projection.clone();
        crate::stream::project_call(event_projection, || -> EventStream {
            let model_id = request
                .model
                .split_once(':')
                .map(|(_, id)| id)
                .unwrap_or(&request.model);
            let selected_base = self
                .models
                .iter()
                .find(|model| model.id == model_id)
                .and_then(|model| model.base_url.as_deref())
                .unwrap_or(&self.base_url);
            let effective_base = crate::auth::builtin::effective_base_url(
                &self.provider_id,
                selected_base,
                &resolved,
            );
            if effective_base != selected_base {
                let mut configured = self.clone();
                if let Some(model) = configured
                    .models
                    .iter_mut()
                    .find(|model| model.id == model_id)
                {
                    model.base_url = Some(effective_base);
                } else {
                    configured.base_url = effective_base;
                }
                return configured.stream_prepared_with_options(
                    request,
                    resolved,
                    native_options.clone(),
                );
            }
            request.callbacks = request.callbacks.ensure_call_scope();
            // Same as `stream` but skips per-call auth resolution: the collection
            // resolved and froze authentication once for the logical call.
            if let Err(error) = crate::provider::validate_request_capabilities(self, &request) {
                return Box::pin(stream::once(async move { Err(error) }));
            }
            let default_base_url = self.base_url.clone();
            let provider_id = self.provider_id.clone();
            let auth_invalid_policy = self.auth_invalid_policy;
            let model_id = request
                .model
                .split_once(':')
                .map(|(_, model_id)| model_id)
                .unwrap_or(&request.model);
            let model = self.models.iter().find(|model| model.id == model_id);
            let model_info = model.cloned();
            let model_base_url = model.and_then(|model| model.base_url.clone());
            let compat = self.resolve_compat(model_id);
            let chat_completions_path = compat.chat_completions_path.clone();
            let tool_metadata = match crate::transcript::compile_request_projection(
                &request,
                &self.provider_id,
                model,
                self.wire,
            )
            .and_then(|compiled| {
                compile_chat_tools(
                    compiled.transcript.messages(),
                    &compiled.tools.request_tools,
                    &compat,
                )
            }) {
                Ok(tools) => tools,
                Err(error) => return Box::pin(stream::once(async move { Err(error) })),
            };
            let mut extra_headers = self.route_headers.clone();
            let copilot_headers = if self.copilot_initiator {
                match github_copilot_route_headers(&request) {
                    Ok(mut extra_headers) => {
                        let initiator = github_copilot_initiator(&request);
                        extra_headers.push(("X-Initiator".into(), initiator.into()));
                        Ok(extra_headers)
                    }
                    Err(error) => Err(error),
                }
            } else {
                Ok(Vec::new())
            };
            if compat.send_session_affinity_headers.unwrap_or(false)
                && let Some(session_id) = request.session_id.as_deref()
                && !session_id.is_empty()
            {
                const AFFINITY_HEADERS: [&str; 4] = [
                    "session_id",
                    "x-client-request-id",
                    "x-session-affinity",
                    "x-session-id",
                ];
                extra_headers.retain(|(name, _)| {
                    !AFFINITY_HEADERS
                        .iter()
                        .any(|managed| name.eq_ignore_ascii_case(managed))
                });
                let selected: &[&str] = match compat
                    .session_affinity_format
                    .unwrap_or(crate::model_info::SessionAffinityFormat::OpenAi)
                {
                    crate::model_info::SessionAffinityFormat::OpenAi => {
                        &["session_id", "x-client-request-id", "x-session-affinity"]
                    }
                    crate::model_info::SessionAffinityFormat::OpenAiNoSession => {
                        &["x-client-request-id", "x-session-affinity"]
                    }
                    crate::model_info::SessionAffinityFormat::OpenRouter => &["x-session-id"],
                };
                extra_headers.extend(
                    selected
                        .iter()
                        .map(|name| ((*name).into(), session_id.to_owned())),
                );
            }
            let extra_headers = copilot_headers.and_then(|copilot_headers| {
                extra_headers.extend(copilot_headers);
                self.headers.merge_model_request(
                    self.models
                        .iter()
                        .find(|model| model.id == model_id)
                        .map(|model| model.headers.as_slice())
                        .unwrap_or_default(),
                    &extra_headers,
                    &request.extra_headers,
                )
            });
            let timeout = request.timeout;
            let body = match self.build_request_body_with_options(&request, native_options.as_ref())
            {
                Ok(body) => body,
                Err(error) => return Box::pin(stream::once(async move { Err(error) })),
            };

            let callbacks = request.callbacks.clone();
            let requested_model = request
                .model
                .split_once(':')
                .map(|(_, id)| id)
                .unwrap_or(&request.model)
                .to_owned();
            let cancel = request.cancel.clone();
            // Explicit frozen telemetry context (task 20.54/AIR-055): the producer
            // task runs under the captured parent/Dispatch when provided; omission
            // observes nothing. Full cross-path proof closes in 20.55.
            let telemetry = request.telemetry_context.clone();
            let http_client = self.client.client().clone();

            let snapshot = crate::stream::ProducerSnapshot::with_projection(projection);
            let producer_snapshot = snapshot.clone();
            let (tx, rx) = tokio::sync::mpsc::channel(64);
            let tx_closed = tx.clone();
            let producer_cancel = cancel.clone();
            let stream_cancel = cancel.clone();

            let producer = async move {
                let work = async {
                    let extra_headers = match extra_headers {
                        Ok(headers) => headers,
                        Err(error) => {
                            let _ = send_or_cancel(&producer_cancel, &tx, Err(error)).await;
                            return;
                        }
                    };
                    let extra_headers =
                        match callbacks.apply_header_transform_async(&extra_headers).await {
                            Ok(headers) => headers,
                            Err(error) => {
                                let _ = send_or_cancel(&producer_cancel, &tx, Err(error)).await;
                                return;
                            }
                        };
                    let body = match callbacks
                        .apply_payload_async(crate::callbacks::ORDINARY_PAYLOAD_VARIANT, body)
                        .await
                        .and_then(|body| {
                            crate::callbacks::validate_openai_payload_shape(
                                &body,
                                crate::callbacks::OpenAiPayloadShape::Chat,
                            )?;
                            Ok(body)
                        }) {
                        Ok(body) => body,
                        Err(error) => {
                            let _ = send_or_cancel(&producer_cancel, &tx, Err(error)).await;
                            return;
                        }
                    };
                    let base_url = crate::auth::builtin::effective_base_url(
                        &provider_id,
                        &model_base_url.unwrap_or(default_base_url),
                        &resolved,
                    );
                    if let Err(e) = Self::stream_http(
                        http_client,
                        resolved,
                        base_url,
                        provider_id,
                        auth_invalid_policy,
                        chat_completions_path,
                        requested_model,
                        model_info,
                        compat,
                        tool_metadata,
                        extra_headers,
                        &body,
                        cancel,
                        timeout,
                        callbacks,
                        producer_snapshot,
                        &tx,
                    )
                    .await
                    {
                        let _ = tx.send(Err(e)).await;
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
                rx,
                stream_cancel,
                snapshot,
            ))
        })
    }
}

impl Provider for OpenAiRouteContext {
    fn validate_simple_options(
        &self,
        model: &ModelInfo,
        _options: &crate::api::contract::SimpleOptions,
    ) -> Result<(), ProviderError> {
        validate_complete_chat_record(model)
    }

    fn stream_model_resource_attempt(
        &self,
        model: ModelInfo,
        options: crate::api::contract::SimpleOptions,
        request: Request,
        auth: crate::auth::ResolvedAuth,
        _attempt: crate::provider::AttemptContext,
        _resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        let mut configured = self.clone();
        configured.models = vec![model];
        configured.catalog_compat = true;

        configured.stream_prepared_with_options(
            request,
            auth,
            Some(OpenAiCompletionsOptions::from_simple(&options)),
        )
    }

    fn stream_prepared(
        &self,
        request: Request,
        resolved: crate::auth::ResolvedAuth,
    ) -> EventStream {
        self.stream_prepared_with_options(request, resolved, None)
    }

    fn id(&self) -> &str {
        &self.provider_id
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    fn replace_model_catalog(&mut self, models: Vec<ModelInfo>) -> Result<(), ProviderError> {
        self.models = models;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Task 20.36 (AIR-049/057, ONB-03): typed native options and the public
// openai-completions direct adapter.
// ---------------------------------------------------------------------------

/// Full OpenAI tool-choice shape. The neutral simple entry maps only
/// `Auto`/`None`; `Required` and named forcing stay native.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenAiToolChoice {
    None,
    Auto,
    Required,
    Named(String),
}

impl OpenAiToolChoice {
    fn wire_value(&self) -> serde_json::Value {
        match self {
            Self::None => serde_json::json!("none"),
            Self::Auto => serde_json::json!("auto"),
            Self::Required => serde_json::json!("required"),
            Self::Named(name) => serde_json::json!({
                "type": "function",
                "function": {"name": name},
            }),
        }
    }
}

/// Typed Chat-only parameters. Common temperature, transport, cache,
/// cancellation and transcript inputs stay on [`Request`]. Construct with
/// [`Self::default`] and set the admitted fields; a native default does not
/// consume neutral `Request::thinking`, the simple entry supplies that
/// mapping separately.
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct OpenAiCompletionsOptions {
    /// Full OpenAI `tool_choice` shape.
    pub tool_choice: Option<OpenAiToolChoice>,
    /// Native reasoning effort (`minimal`..`max`). Unsupported levels fail
    /// typed preflight; the neutral entry clamps instead.
    pub reasoning_effort: Option<crate::model_info::ThinkingLevel>,
    /// Token budgets per thinking level, merged over the source defaults.
    pub thinking_budgets: std::collections::BTreeMap<crate::model_info::ThinkingLevel, u64>,
    simple: bool,
}

impl OpenAiCompletionsOptions {
    /// Neutral mapping for the simple entry: only `Auto`/`None` tool choice
    /// and the request's neutral thinking intent are admitted.
    fn from_simple(options: &crate::api::contract::SimpleOptions) -> Self {
        Self {
            tool_choice: options.tool_choice.map(|choice| match choice {
                crate::api::contract::SimpleToolChoice::Auto => OpenAiToolChoice::Auto,
                crate::api::contract::SimpleToolChoice::None => OpenAiToolChoice::None,
            }),
            reasoning_effort: None,
            thinking_budgets: Default::default(),
            simple: true,
        }
    }

    fn is_simple(&self) -> bool {
        self.simple
    }
}

/// The public openai-completions direct adapter registered through
/// [`crate::Models::register_chat_api`]. Each dispatch clones its immutable
/// provider configuration; native options never mutate a registered provider.
pub struct OpenAiCompletionsChatApi {
    api_id: crate::types::ApiId,
    provider: Arc<OpenAiRouteContext>,
    model: ModelInfo,
}

impl std::fmt::Debug for OpenAiCompletionsChatApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiCompletionsChatApi")
            .field("api_id", &self.api_id)
            .finish_non_exhaustive()
    }
}

impl OpenAiCompletionsChatApi {
    /// Bind a complete Chat record. Wrong-wire or invalid records fail before
    /// authentication or network activity.
    pub fn new(
        provider: Arc<OpenAiRouteContext>,
        model: ModelInfo,
    ) -> Result<Self, crate::provider::ProviderError> {
        if model.wire_api != crate::model_info::WireApi::OpenAiCompletions
            || model.id.trim().is_empty()
            || model.validate().is_err()
        {
            return Err(crate::provider::ProviderError::Config(
                crate::provider::ProviderErrorSummary::attested_static(
                    "openai-completions native options require a valid complete Chat model record",
                ),
            ));
        }
        Ok(Self {
            api_id: crate::types::ApiId::new("openai-completions"),
            provider,
            model,
        })
    }

    fn configured(&self, model: ModelInfo) -> OpenAiRouteContext {
        let mut provider = (*self.provider).clone();
        provider.models = vec![model];
        provider.catalog_compat = true;
        provider
    }
}

impl crate::api::contract::ChatApi<OpenAiCompletionsOptions> for OpenAiCompletionsChatApi {
    fn validate_simple_options(
        &self,
        model: &ModelInfo,
        _options: &crate::api::contract::SimpleOptions,
    ) -> Result<(), crate::provider::ProviderError> {
        validate_complete_chat_record(model)
    }

    fn api_id(&self) -> &crate::types::ApiId {
        &self.api_id
    }

    fn provider_id(&self) -> &str {
        &self.provider.provider_id
    }

    fn complete_record(&self) -> ModelInfo {
        self.model.clone()
    }

    fn simple_options(&self) -> OpenAiCompletionsOptions {
        OpenAiCompletionsOptions::from_simple(&crate::api::contract::SimpleOptions::default())
    }

    fn simple_options_for(
        &self,
        options: &crate::api::contract::SimpleOptions,
    ) -> OpenAiCompletionsOptions {
        OpenAiCompletionsOptions::from_simple(options)
    }

    fn session_resources(&self) -> Option<&dyn crate::codex_resources::SessionResourceOwner> {
        None
    }

    fn validate_native_request(
        &self,
        model: &ModelInfo,
        request: &Request,
        options: &OpenAiCompletionsOptions,
    ) -> Result<(), crate::provider::ProviderError> {
        validate_complete_chat_record(model)?;
        if model.wire_api != crate::model_info::WireApi::OpenAiCompletions {
            return Err(crate::provider::ProviderError::Config(
                crate::provider::ProviderErrorSummary::attested_static(
                    "openai-completions native options require a Chat model record",
                ),
            ));
        }
        // Native/exact effort fails typed preflight on unsupported levels
        // (SP-03); the neutral entry clamps instead.
        if let Some(level) = options.reasoning_effort
            && model.thinking_level_map.resolve(level).is_err()
        {
            return Err(crate::provider::ProviderError::Config(
                crate::provider::ProviderErrorSummary::sanitized(format!(
                    "openai-completions native options reject unsupported thinking level {level:?}"
                )),
            ));
        }
        self.configured(model.clone())
            .validate_tool_projection(model, request)
    }

    fn stream_prepared(
        &self,
        request: Request,
        options: OpenAiCompletionsOptions,
        auth: ResolvedAuth,
    ) -> EventStream {
        self.stream_model_resource_prepared(self.model.clone(), request, options, auth, None)
    }

    fn stream_resource_prepared(
        &self,
        request: Request,
        options: OpenAiCompletionsOptions,
        auth: ResolvedAuth,
        resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        self.stream_model_resource_prepared(self.model.clone(), request, options, auth, resources)
    }

    fn stream_model_resource_prepared(
        &self,
        model: ModelInfo,
        mut request: Request,
        options: OpenAiCompletionsOptions,
        auth: ResolvedAuth,
        resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        if let Err(error) = self.validate_native_request(&model, &request, &options) {
            return Box::pin(futures_util::stream::once(async move { Err(error) }));
        }
        if !options.is_simple() {
            // Native effort is validated and projected independently of the
            // neutral thinking fields, which are unused on this path. The
            // neutral entry clamps at Models::stream_simple before dispatch.
            request.thinking = Default::default();
        }
        let provider = self.configured(model);
        let _ = resources;
        provider.stream_prepared_with_options(request, auth, Some(options))
    }
}

/// One complete, wire-valid Chat model record or a typed failure.
fn validate_complete_chat_record(model: &ModelInfo) -> Result<(), crate::provider::ProviderError> {
    if model.wire_api != crate::model_info::WireApi::OpenAiCompletions
        || model.id.trim().is_empty()
        || model.validate().is_err()
    {
        return Err(crate::provider::ProviderError::Config(
            crate::provider::ProviderErrorSummary::attested_static(
                "openai-completions adapters require a valid complete Chat model record",
            ),
        ));
    }
    if let Some(base) = model.base_url.as_deref() {
        let valid = url::Url::parse(base)
            .is_ok_and(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some());
        if !valid {
            return Err(ProviderError::Config(
                ProviderErrorSummary::attested_static(
                    "The selected model requires an absolute HTTP or HTTPS API base URL",
                ),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod fragmented_sse_tests {
    use super::*;

    #[test]
    fn multiline_frames_survive_every_line_ending_split() {
        for newline in ["\n", "\r\n", "\r"] {
            let wire = format!(
                "data: {{\"id\":\"probe\",{newline}data: \"choices\":[{{\"index\":0,\"delta\":{{\"content\":\"hello\"}},\"finish_reason\":null}}]}}{newline}{newline}"
            );
            let mut expected = vec![
                serde_json::json!({"id":"probe","choices":[{"index":0,"delta":{"content":"hello"},"finish_reason":null}]}),
            ];
            let expected_second =
                serde_json::from_str(&expected[0].to_string().replace("hello", "world")).unwrap();
            expected.push(expected_second);
            let wire = format!("{wire}{}", wire.replace("hello", "world"));
            for split in 0..=wire.len() {
                let mut buffer = wire[..split].to_owned();
                let mut actual = drain(&mut buffer);
                buffer.push_str(&wire[split..]);
                actual.extend(drain(&mut buffer));
                assert_eq!(actual, expected, "newline={newline:?}, split={split}");
            }
            for width in [1, 2, 3, 5, 7] {
                let mut buffer = String::new();
                let mut actual = Vec::new();
                for chunk in wire.as_bytes().chunks(width) {
                    buffer.push_str(std::str::from_utf8(chunk).unwrap());
                    actual.extend(drain(&mut buffer));
                }
                assert_eq!(actual, expected, "newline={newline:?}, width={width}");
            }
        }
    }

    fn drain(buffer: &mut String) -> Vec<serde_json::Value> {
        drain_sse_events(buffer, false)
            .into_iter()
            .map(|(parsed, raw)| {
                assert!(
                    matches!(parsed, ParsedEvent::Valid(_)),
                    "complete legal frame"
                );
                raw.unwrap()
            })
            .collect()
    }
}
