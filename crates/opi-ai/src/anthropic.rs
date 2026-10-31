//! Anthropic Messages SSE provider (S8.1).

use std::sync::Arc;

use futures_util::{StreamExt, stream};
use secrecy::ExposeSecret;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;

use crate::auth::{AuthInvalidPolicy, AuthScheme, ResolvedAuth};
use crate::http::HttpClient;
use crate::message::{AssistantContent, AssistantMessage, ToolCall};
use crate::model_info::{AnthropicMessagesCompat, WireApi, WireCompat};
use crate::provider::{
    CacheRetention, EventStream, ModelInfo, Provider, ProviderError, ProviderErrorSummary, Request,
    github_copilot_initiator, github_copilot_route_headers,
};
use crate::provider_headers::ProviderHeaders;
use crate::registry::ModelCapabilities;
use crate::stream::{
    AssistantStreamEvent, CancelAwareReceiverStream, StopReason, Usage, send_or_cancel,
};

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
    UsageError(ProviderError),
    Malformed {
        event_type: String,
        data: String,
        error: String,
    },
}

/// Parse SSE text into frames, then deserialize each frame as an AnthropicEvent.
/// Returns [`ParsedEvent`] so callers can decide how to handle malformed data.
pub fn parse_sse_events(input: &str) -> impl Iterator<Item = ParsedEvent> + '_ {
    parse_sse_events_with_raw(input).map(|(parsed, _raw)| parsed)
}

/// [`parse_sse_events`] paired with each processed frame's raw wire JSON (the
/// `data` payload parsed as a JSON value). Task 20.51: the raw-event callback
/// observes exactly these values, before normalization. Malformed frames carry
/// no raw value.
pub fn parse_sse_events_with_raw(
    input: &str,
) -> impl Iterator<Item = (ParsedEvent, Option<serde_json::Value>)> + '_ {
    parse_frames(input).filter_map(|frame| {
        if !ANTHROPIC_EVENTS.contains(&frame.event.as_str()) {
            return None;
        }
        let raw = serde_json::from_str::<serde_json::Value>(&frame.data).ok();
        match serde_json::from_str::<AnthropicRawEvent>(&frame.data) {
            Ok(raw_event) => Some(match AnthropicEvent::from_raw(raw_event) {
                Ok(event) => (ParsedEvent::Valid(event), raw),
                Err(error) => (ParsedEvent::UsageError(error), raw),
            }),
            Err(e) => Some((
                ParsedEvent::Malformed {
                    event_type: frame.event.clone(),
                    data: frame.data.clone(),
                    error: e.to_string(),
                },
                raw,
            )),
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
    /// Subset of `cache_creation_input_tokens` eligible for 1h TTL.
    #[serde(default)]
    cache_creation_input_tokens_1h: Option<u64>,
}

impl RawUsage {
    fn into_usage(self) -> Result<Usage, ProviderError> {
        let cache_write = self.cache_creation_input_tokens.unwrap_or(0);
        let cache_write_1h = self.cache_creation_input_tokens_1h;
        if cache_write_1h.is_some_and(|tokens| tokens > u64::from(cache_write)) {
            return Err(ProviderError::StreamError(ProviderErrorSummary::sanitized(
                format!(
                    "cache_creation_input_tokens_1h ({}) exceeds cache_creation_input_tokens ({cache_write})",
                    cache_write_1h.unwrap_or(0)
                ),
            )));
        }
        if self.input_tokens.is_some()
            || self.output_tokens.is_some()
            || self.cache_read_input_tokens.is_some()
            || self.cache_creation_input_tokens.is_some()
        {
            Ok(Usage::reported(
                self.input_tokens.unwrap_or(0),
                self.output_tokens.unwrap_or(0),
                self.cache_read_input_tokens.unwrap_or(0),
                cache_write,
                cache_write_1h,
                None, // reasoning_tokens
            ))
        } else {
            Ok(Usage::unknown())
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
        thinking: Option<String>,
        signature: Option<String>,
    },
    #[serde(rename = "redacted_thinking")]
    RedactedThinking { data: String },
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
    #[serde(rename = "signature_delta")]
    SignatureDelta { signature: String },
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
    ToolUse {
        id: String,
        name: String,
    },
    Thinking {
        thinking: String,
        signature: Option<String>,
    },
    RedactedThinking {
        data: String,
    },
}

/// Delta data from content_block_delta.
#[derive(Debug, Clone)]
pub enum DeltaData {
    Text { text: String },
    InputJson { partial_json: String },
    Thinking { thinking: String },
    Signature { signature: String },
}

impl AnthropicEvent {
    fn from_raw(raw: AnthropicRawEvent) -> Result<Self, ProviderError> {
        let event = match raw {
            AnthropicRawEvent::MessageStart { message } => {
                let usage = message
                    .usage
                    .map(RawUsage::into_usage)
                    .transpose()?
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
                    RawContentBlock::Thinking {
                        thinking,
                        signature,
                    } => ContentBlockType::Thinking {
                        thinking: thinking.unwrap_or_default(),
                        signature,
                    },
                    RawContentBlock::RedactedThinking { data } => {
                        ContentBlockType::RedactedThinking { data }
                    }
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
                    RawDelta::SignatureDelta { signature } => DeltaData::Signature { signature },
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
                usage: usage.into_usage()?,
            },
            AnthropicRawEvent::MessageStop => AnthropicEvent::MessageStop,
            AnthropicRawEvent::Error { error } => AnthropicEvent::Error {
                message: error.message,
            },
        };
        Ok(event)
    }
}

// ---------------------------------------------------------------------------
// Stateful event mapper: AnthropicEvent ->AssistantStreamEvent
// ---------------------------------------------------------------------------

/// Tracks content block state and accumulates the final message.
/// How one tool actually went out on this dispatch (task 20.11): only a
/// dispatched prefer-strict tool qualifies the source's strict-limit
/// downgrade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StrictToolDisposition {
    NotStrict,
    StrictPrefer,
    StrictRequire,
}

/// OAuth projects only known tool-name fields; arguments, schemas and the
/// canonical transcript keep their original spelling and contents.
fn oauth_tool_name(name: &str) -> &str {
    [
        "Read",
        "Write",
        "Edit",
        "Bash",
        "Grep",
        "Glob",
        "AskUserQuestion",
        "EnterPlanMode",
        "ExitPlanMode",
        "KillShell",
        "NotebookEdit",
        "Skill",
        "Task",
        "TaskOutput",
        "TodoWrite",
        "WebFetch",
        "WebSearch",
    ]
    .into_iter()
    .find(|known| known.eq_ignore_ascii_case(name))
    .unwrap_or(name)
}

fn project_oauth_tool_names(body: &mut serde_json::Value) {
    fn project_name(tool: &mut serde_json::Value) {
        if let Some(name) = tool.get("name").and_then(serde_json::Value::as_str) {
            let name = oauth_tool_name(name).to_owned();
            tool["name"] = name.into();
        }
    }
    if let Some(tools) = body
        .get_mut("tools")
        .and_then(serde_json::Value::as_array_mut)
    {
        for tool in tools {
            project_name(tool);
        }
    }
    if let Some(messages) = body
        .get_mut("messages")
        .and_then(serde_json::Value::as_array_mut)
    {
        for message in messages {
            if let Some(blocks) = message
                .get_mut("content")
                .and_then(serde_json::Value::as_array_mut)
            {
                for block in blocks {
                    match block.get("type").and_then(serde_json::Value::as_str) {
                        Some("tool_use") => project_name(block),
                        Some("tool_removal") => project_name(&mut block["tool"]),
                        Some("tool_addition") => project_name(&mut block["tool"]["definition"]),
                        _ => {}
                    }
                }
            }
        }
    }
}

struct CompiledAnthropicRequest {
    body: serde_json::Value,
    preferred_strict_names: Vec<String>,
    strict_tools: Vec<crate::message::ToolDef>,
}

fn preferred_strict_names(tools: &[crate::message::ToolDef]) -> Vec<String> {
    tools
        .iter()
        .filter(|tool| {
            matches!(
                tool.constrained_sampling,
                Some(
                    crate::constrained_sampling::ConstrainedSampling::JsonSchema {
                        strict: crate::constrained_sampling::StrictPreference::Prefer,
                    }
                )
            )
        })
        .map(|tool| tool.name.clone())
        .collect()
}

pub struct AnthropicMapper {
    snapshot: crate::stream::ProducerSnapshot,
    partial: AssistantMessage,
    /// Trusted declarations of the strict-dispatched tools on this call
    /// (task 20.11): their returned arguments normalize nulls back onto the
    /// original schema at finalization.
    strict_tools: std::collections::HashMap<String, crate::message::ToolDef>,
    oauth_tool_names: Vec<String>,
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
    RedactedThinking,
}

impl Default for AnthropicMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl AnthropicMapper {
    pub(crate) fn set_snapshot(&mut self, snapshot: crate::stream::ProducerSnapshot) {
        self.snapshot = snapshot;
        self.snapshot.record(&self.partial);
    }

    pub fn process(
        &mut self,
        event: AnthropicEvent,
    ) -> Vec<Result<AssistantStreamEvent, ProviderError>> {
        let events = self.process_inner(event);
        self.snapshot.record(&self.partial);
        events
    }

    pub(crate) fn failure(&self, cause: ProviderError) -> ProviderError {
        self.snapshot
            .failure(cause.with_partial(self.partial.clone()))
    }

    pub fn new() -> Self {
        Self::new_for_provider("anthropic")
    }

    fn new_for_provider(provider_id: &str) -> Self {
        Self {
            snapshot: crate::stream::ProducerSnapshot::default(),
            partial: empty_assistant_message(provider_id),
            strict_tools: std::collections::HashMap::new(),
            oauth_tool_names: Vec::new(),
            blocks: Vec::new(),
            saw_done: false,
        }
    }

    /// Record the strict-dispatched tools of this call (task 20.11,
    /// V-34): their returned arguments normalize per the original trusted
    /// schema at block end.
    pub(crate) fn set_strict_tools(&mut self, tools: Vec<crate::message::ToolDef>) {
        self.strict_tools = tools
            .into_iter()
            .map(|tool| (tool.name.clone(), tool))
            .collect();
    }

    /// Seed message-level diagnostics (task 20.11, V-34/V-36): the recorded
    /// strict downgrade rides the message from Start through the terminal.
    pub(crate) fn seed_diagnostics(
        &mut self,
        diagnostics: Vec<crate::stream::AssistantDiagnostic>,
    ) {
        self.partial.diagnostics.extend(diagnostics);
    }

    /// Process one Anthropic event, returning zero or more stream events.
    fn process_inner(
        &mut self,
        event: AnthropicEvent,
    ) -> Vec<Result<AssistantStreamEvent, ProviderError>> {
        if self.saw_done {
            return Vec::new();
        }
        match event {
            AnthropicEvent::MessageStart { id, model, usage } => {
                self.partial.response_id = id;
                if let Some(m) = model {
                    if self.partial.model.is_empty() {
                        self.partial.model = m;
                    } else {
                        self.partial.response_model = Some(m);
                    }
                }
                self.partial.usage = usage;
                let start = self.partial.clone();
                vec![Ok(AssistantStreamEvent::Start { partial: start })]
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
                            signature: None,
                        });
                        vec![Ok(AssistantStreamEvent::TextStart {
                            content_index,
                            partial: self.partial.clone(),
                        })]
                    }
                    ContentBlockType::ToolUse { id, name } => {
                        let name = self
                            .oauth_tool_names
                            .iter()
                            .find(|declared| declared.eq_ignore_ascii_case(&name))
                            .cloned()
                            .unwrap_or(name);
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
                                arguments_error: None,
                                signature: None,
                            },
                        });
                        vec![Ok(AssistantStreamEvent::ToolCallStart {
                            content_index,
                            partial: self.partial.clone(),
                        })]
                    }
                    ContentBlockType::Thinking {
                        thinking,
                        signature,
                    } => {
                        self.blocks.push(BlockState::Thinking {
                            thinking: thinking.clone(),
                        });
                        self.partial.content.push(AssistantContent::Thinking {
                            thinking,
                            signature,
                        });
                        vec![Ok(AssistantStreamEvent::ThinkingStart {
                            content_index,
                            partial: self.partial.clone(),
                        })]
                    }
                    ContentBlockType::RedactedThinking { data } => {
                        self.blocks.push(BlockState::RedactedThinking);
                        self.partial
                            .content
                            .push(AssistantContent::RedactedThinking { data });
                        Vec::new()
                    }
                }
            }
            AnthropicEvent::ContentBlockDelta { index, delta } => {
                // Address the block by its upstream index with safe `get_mut`
                // instead of `blocks.len() - 1`, which underflows when a delta
                // arrives before any block start and mutates the wrong block on
                // out-of-order or interleaved indices. A delta for a block that
                // was never started is ignored rather than panicked on.
                if self.blocks.get(index).is_none() {
                    return Vec::new();
                }
                let content_index = index;
                match delta {
                    DeltaData::Text { text } => {
                        if let Some(BlockState::Text { text: acc }) = self.blocks.get_mut(index) {
                            acc.push_str(&text);
                        }
                        if let Some(AssistantContent::Text { text: acc, .. }) =
                            self.partial.content.get_mut(index)
                        {
                            acc.push_str(&text);
                        }
                        vec![Ok(AssistantStreamEvent::TextDelta {
                            content_index,
                            delta: text,
                            partial: self.partial.clone(),
                        })]
                    }
                    DeltaData::InputJson { partial_json } => {
                        if let Some(BlockState::ToolUse {
                            partial_json: acc, ..
                        }) = self.blocks.get_mut(index)
                        {
                            acc.push_str(&partial_json);
                        }
                        if let Some(AssistantContent::ToolCall { tool_call }) =
                            self.partial.content.get_mut(index)
                        {
                            tool_call.arguments.push_str(&partial_json);
                        }
                        vec![Ok(AssistantStreamEvent::ToolCallDelta {
                            content_index,
                            delta: partial_json,
                            partial: self.partial.clone(),
                        })]
                    }
                    DeltaData::Thinking { thinking } => {
                        if let Some(BlockState::Thinking { thinking: acc }) =
                            self.blocks.get_mut(index)
                        {
                            acc.push_str(&thinking);
                        }
                        if let Some(AssistantContent::Thinking { thinking: acc, .. }) =
                            self.partial.content.get_mut(index)
                        {
                            acc.push_str(&thinking);
                        }
                        vec![Ok(AssistantStreamEvent::ThinkingDelta {
                            content_index,
                            delta: thinking,
                            partial: self.partial.clone(),
                        })]
                    }
                    DeltaData::Signature { signature } => {
                        if let Some(AssistantContent::Thinking { signature: acc, .. }) =
                            self.partial.content.get_mut(index)
                        {
                            acc.get_or_insert_default().push_str(&signature);
                        }
                        Vec::new()
                    }
                }
            }
            AnthropicEvent::ContentBlockStop { index } => {
                let content_index = index;
                match self.blocks.get(index) {
                    Some(BlockState::Text { text }) => {
                        let content = text.clone();
                        vec![Ok(AssistantStreamEvent::TextEnd {
                            content_index,
                            content,
                            partial: self.partial.clone(),
                        })]
                    }
                    Some(BlockState::ToolUse {
                        id,
                        name,
                        partial_json,
                    }) => {
                        // Final textual arguments are parsed once at block
                        // end: empty normalizes to `{}`, repairable escapes
                        // stay literal, anything else keeps the owned raw
                        // text plus `arguments_error` (AIR-048).
                        let (mut arguments, arguments_error) =
                            ToolCall::finalize_text_arguments(partial_json.clone());
                        // Strict-dispatched tools (task 20.11): the strict
                        // wire may answer an explicit null where the original
                        // trusted schema declared an optional non-nullable
                        // position; normalize back onto the original schema.
                        if arguments_error.is_none()
                            && let Some(tool) = self.strict_tools.get(name)
                            && let Ok(parsed) =
                                serde_json::from_str::<serde_json::Value>(&arguments)
                        {
                            let mut normalized = parsed;
                            crate::constrained_sampling::normalize_wire_arguments_nulls(
                                &tool.input_schema,
                                &mut normalized,
                            );
                            arguments = serde_json::to_string(&normalized).unwrap_or(arguments);
                        }
                        let tool_call = ToolCall {
                            id: id.clone(),
                            name: name.clone(),
                            arguments: arguments.clone(),
                            arguments_error,
                            signature: None,
                        };
                        // Update the partial message's tool call with final arguments
                        if let Some(AssistantContent::ToolCall { tool_call: tc }) =
                            self.partial.content.get_mut(index)
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
                    Some(BlockState::Thinking { thinking }) => {
                        let content = thinking.clone();
                        vec![Ok(AssistantStreamEvent::ThinkingEnd {
                            content_index,
                            content,
                            partial: self.partial.clone(),
                        })]
                    }
                    Some(BlockState::RedactedThinking) | None => Vec::new(),
                }
            }
            AnthropicEvent::MessageDelta { stop_reason, usage } => {
                if let Some(reason) = stop_reason.as_ref().filter(|reason| !reason.is_empty()) {
                    self.partial.raw_stop_reason = Some(reason.clone());
                }
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
                if usage.cache_write_1h_tokens.is_some() {
                    self.partial.usage.cache_write_1h_tokens = usage.cache_write_1h_tokens;
                }
                // message_delta doesn't emit a stream event; Done comes from message_stop
                Vec::new()
            }
            AnthropicEvent::MessageStop => {
                self.saw_done = true;
                vec![Ok(AssistantStreamEvent::Done {
                    reason: self.partial.stop_reason,
                    message: self.partial.clone(),
                })]
            }
            AnthropicEvent::Error { message: _ } => {
                // Preserve state that did not produce a public event (usage,
                // opaque content or signatures), alongside the typed cause.
                self.saw_done = true;
                vec![Err(self.failure(ProviderError::StreamError(
                    ProviderErrorSummary::attested_static(ANTHROPIC_STREAM_ERROR),
                )))]
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

fn empty_assistant_message(provider_id: &str) -> AssistantMessage {
    AssistantMessage {
        content: Vec::new(),
        api: crate::ApiKind::Anthropic,
        provider: provider_id.into(),
        model: String::new(),
        response_model: None,
        wire: Some(crate::model_info::WireApi::AnthropicMessages),
        response_id: None,
        usage: Usage::default(),
        stop_reason: StopReason::Stop,
        error_message: None,
        timestamp_ms: crate::time::now_ms(),
        diagnostics: Vec::new(),
        duration_ms: None,
        raw_stop_reason: None,
        provider_thinking_level: None,
        deferred: None,
    }
}

// ---------------------------------------------------------------------------
// AnthropicProvider
// ---------------------------------------------------------------------------

/// Required tags for the pinned Anthropic OAuth protocol. Header-owned bearer
/// auth and proxy credentials do not select this protocol.
const ANTHROPIC_OAUTH_BETA_HEADER: &str = "claude-code-20250219,oauth-2025-04-20";

/// Neutral public text substituted for the raw upstream Anthropic
/// `error.message`. A proxy may echo credential material or request fragments
/// in an error frame, so the raw value is never placed in a persisted session
/// or a public event. Mirrors the dedicated Codex path.
const ANTHROPIC_STREAM_ERROR: &str = "anthropic stream error";

/// Built-in Anthropic model metadata without credentials or HTTP construction.
pub fn model_catalog() -> Vec<ModelInfo> {
    vec![
        ModelInfo::new(
            "claude-sonnet-4-5-20250514",
            "Claude Sonnet 4.5",
            WireApi::AnthropicMessages,
            ModelCapabilities::new(200000, 8192)
                .with_images(true)
                .with_streaming(true)
                .with_thinking(true)
                .with_cache_control(true)
                .with_long_cache_retention(true),
        ),
        ModelInfo::new(
            "claude-opus-4-20250514",
            "Claude Opus 4",
            WireApi::AnthropicMessages,
            ModelCapabilities::new(200000, 8192)
                .with_images(true)
                .with_streaming(true)
                .with_thinking(true)
                .with_cache_control(true)
                .with_long_cache_retention(true),
        ),
        ModelInfo::new(
            "claude-haiku-4-5-20250514",
            "Claude Haiku 4.5",
            WireApi::AnthropicMessages,
            ModelCapabilities::new(200000, 8192)
                .with_images(true)
                .with_streaming(true)
                .with_thinking(true)
                .with_cache_control(true)
                .with_long_cache_retention(true),
        ),
    ]
    .into_iter()
    .map(|model| {
        model
            .with_compat(WireCompat::AnthropicMessages(AnthropicMessagesCompat {
                supports_eager_tool_input_streaming: false,
                force_adaptive_thinking: false,
                supports_temperature: true,
                supports_strict_tools: false,
                supports_mid_convo_system_messages: false,
                supports_mid_convo_tool_changes: false,
            }))
            .expect("built-in Anthropic compatibility uses the matching wire")
    })
    .collect()
}

/// Native thinking display (pi `thinkingDisplay`): the enabled default is
/// `summarized` so newer models behave like Claude 4 (whose API default is
/// also summarized).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnthropicThinkingDisplay {
    Summarized,
    Detailed,
}

impl AnthropicThinkingDisplay {
    fn wire_str(self) -> &'static str {
        match self {
            Self::Summarized => "summarized",
            Self::Detailed => "detailed",
        }
    }
}

/// Native `tool_choice` (pi `toolChoice`): the three bare modes plus the
/// named forced tool, which has no neutral equivalent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnthropicToolChoice {
    Auto,
    Any,
    None_,
    Named(String),
}

/// Typed native options for one direct/native dispatch (task 20.39, pi
/// `AnthropicOptions`). The tri-state `thinking_enabled` keeps omitted,
/// disabled and enabled distinct: omitted falls through to the neutral
/// request intent, `Some(false)` projects the typed `disabled` thinking
/// block on reasoning models, `Some(true)` takes the native budget default.
/// `thinking_budget_tokens` follows the source `|| 1024` semantics (a zero
/// or missing budget on enabled thinking sends 1024).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnthropicMessagesOptions {
    pub thinking_enabled: Option<bool>,
    pub thinking_budget_tokens: Option<u64>,
    pub thinking_display: Option<AnthropicThinkingDisplay>,
    /// Native effort (adaptive thinking only): typed preflight rejects
    /// levels the model's thinking map does not resolve.
    pub effort: Option<crate::model_info::ThinkingLevel>,
    /// Interleaved-thinking beta header; defaults to `true`, adaptive
    /// thinking models ignore it entirely.
    pub interleaved_thinking: Option<bool>,
    pub tool_choice: Option<AnthropicToolChoice>,
    simple: bool,
}

impl AnthropicMessagesOptions {
    /// The neutral simple entry (task 20.39): only the neutral tool choice
    /// maps; thinking keeps the omitted tri-state so the neutral request
    /// intent (and its level-budget policy) drives the wire.
    pub fn from_simple(simple: &crate::models::SimpleOptions) -> Self {
        Self {
            thinking_enabled: None,
            thinking_budget_tokens: None,
            thinking_display: None,
            effort: None,
            interleaved_thinking: None,
            tool_choice: simple.tool_choice.map(|choice| match choice {
                crate::models::SimpleToolChoice::Auto => AnthropicToolChoice::Auto,
                crate::models::SimpleToolChoice::None => AnthropicToolChoice::None_,
            }),
            simple: true,
        }
    }

    pub fn is_simple(&self) -> bool {
        self.simple
    }
}

/// Concrete Anthropic Messages API provider.
#[derive(Clone)]
pub struct AnthropicProvider {
    provider_id: String,
    default_base_url: String,
    models: Vec<ModelInfo>,
    headers: ProviderHeaders,
    auth_invalid_policy: AuthInvalidPolicy,
    direct_oauth_beta: bool,
    copilot_headers: bool,
    client: Arc<HttpClient>,
    federation: crate::auth::anthropic_federation::FederationCache,
    /// Typed native options for one direct/native dispatch (task 20.39);
    /// `None` keeps the plain provider path.
    native_options: Option<AnthropicMessagesOptions>,
}

impl AnthropicProvider {
    pub fn new(base_url: Option<String>) -> Self {
        Self::with_client(base_url, Arc::new(HttpClient::new()))
    }

    /// Create with a shared HTTP client for connection pooling.
    pub fn with_client(base_url: Option<String>, client: Arc<HttpClient>) -> Self {
        Self::for_route(
            "anthropic".into(),
            model_catalog(),
            base_url,
            ProviderHeaders::default(),
            client,
            false,
        )
    }

    /// Build an Anthropic Messages route for a mapped provider.
    #[allow(clippy::too_many_arguments)]
    pub fn for_route(
        provider_id: String,
        models: Vec<ModelInfo>,
        default_base_url: Option<String>,
        headers: ProviderHeaders,
        client: Arc<HttpClient>,
        direct_oauth_beta: bool,
    ) -> Self {
        let default_base_url =
            default_base_url.unwrap_or_else(|| "https://api.anthropic.com".into());
        Self {
            provider_id,
            default_base_url,
            models,
            headers,
            auth_invalid_policy: AuthInvalidPolicy::Static,
            direct_oauth_beta,
            copilot_headers: false,
            client,
            federation: Default::default(),
            native_options: None,
        }
    }

    /// Bind typed native options for one dispatch (task 20.39). The options
    /// live on the cloned dispatch provider; a registered provider is never
    /// mutated.
    pub fn with_native_options(mut self, options: AnthropicMessagesOptions) -> Self {
        self.native_options = Some(options);
        self
    }

    /// Override how this route classifies provider 401/403 responses.
    pub fn with_auth_invalid_policy(mut self, policy: AuthInvalidPolicy) -> Self {
        self.auth_invalid_policy = policy;
        self
    }

    /// Enable GitHub Copilot's request-dependent route headers.
    pub fn with_copilot_headers(mut self) -> Self {
        self.copilot_headers = true;
        self
    }

    /// Access the shared HTTP client (for testing client reuse).
    pub fn http_client(&self) -> &Arc<HttpClient> {
        &self.client
    }

    /// Project one tool declaration onto the Anthropic wire (task 20.11,
    /// V-34/AIR-040), returning the disposition for downgrade
    /// qualification.
    ///
    /// A `json_schema` tool on a strict-capable wire projects `strict: true`
    /// with the converted private wire schema (a clone; the trusted
    /// declaration is never modified). `Prefer` with an unconvertible schema
    /// — or on a wire without strict support — stays non-strict; `Require`
    /// fails typed before any dispatch. Tools without constrained sampling
    /// project their trusted schema unchanged.
    fn project_wire_tool(
        &self,
        tool: &crate::message::ToolDef,
        supports_strict: bool,
        disable_preferred_strict: bool,
    ) -> Result<(serde_json::Value, StrictToolDisposition), crate::provider::ProviderError> {
        use crate::constrained_sampling::{
            AnthropicStrictKeywordAdmission, resolve_json_schema_strict_sampling,
        };
        let strict = if disable_preferred_strict
            && matches!(
                &tool.constrained_sampling,
                Some(
                    crate::constrained_sampling::ConstrainedSampling::JsonSchema {
                        strict: crate::constrained_sampling::StrictPreference::Prefer,
                    }
                ),
            ) {
            // The rebuilt downgrade variant removes only prefer-strict;
            // require stays strict (V-34).
            None
        } else {
            resolve_json_schema_strict_sampling(
                tool,
                supports_strict,
                &AnthropicStrictKeywordAdmission,
            )
            .map_err(|error| {
                crate::provider::ProviderError::Config(
                    crate::provider::ProviderErrorSummary::sanitized(error.detail()),
                )
            })?
        };
        let disposition = match (&tool.constrained_sampling, strict) {
            (
                Some(crate::constrained_sampling::ConstrainedSampling::JsonSchema {
                    strict: crate::constrained_sampling::StrictPreference::Prefer,
                }),
                Some(true),
            ) => StrictToolDisposition::StrictPrefer,
            (
                Some(crate::constrained_sampling::ConstrainedSampling::JsonSchema {
                    strict: crate::constrained_sampling::StrictPreference::Require,
                }),
                Some(true),
            ) => StrictToolDisposition::StrictRequire,
            _ => StrictToolDisposition::NotStrict,
        };
        let mut wire = serde_json::json!({
            "name": tool.name,
            "description": tool.description,
            "input_schema": tool.input_schema,
        });
        if strict == Some(true) {
            let converted = crate::constrained_sampling::make_strict_json_schema(
                &tool.input_schema,
                &AnthropicStrictKeywordAdmission,
            )
            .expect("resolve succeeded so the conversion succeeds");
            wire["input_schema"] = converted;
            wire["strict"] = serde_json::Value::Bool(true);
        }
        Ok((wire, disposition))
    }

    /// Compile the current tool declarations for one dispatch (task 20.11):
    /// the wire tool list, whether any prefer-strict tool actually went out
    /// strict (the 400 downgrade is qualified by this fact), and the
    /// trusted schemas of strict-dispatched tools (returned-argument null
    /// normalization).
    fn compile_wire_tools(
        &self,
        current_tools: &[crate::message::ToolDef],
        supports_strict: bool,
        disable_preferred_strict: bool,
    ) -> Result<
        (Vec<serde_json::Value>, bool, Vec<crate::message::ToolDef>),
        crate::provider::ProviderError,
    > {
        let mut tools = Vec::with_capacity(current_tools.len());
        let mut has_preferred_strict = false;
        let mut strict_tools = Vec::new();
        for tool in current_tools {
            let (wire, disposition) =
                self.project_wire_tool(tool, supports_strict, disable_preferred_strict)?;
            match disposition {
                StrictToolDisposition::StrictPrefer => has_preferred_strict = true,
                StrictToolDisposition::StrictRequire | StrictToolDisposition::NotStrict => {}
            }
            if wire.get("strict") == Some(&serde_json::Value::Bool(true)) {
                strict_tools.push(tool.clone());
            }
            tools.push(wire);
        }
        Ok((tools, has_preferred_strict, strict_tools))
    }

    /// Build the Anthropic Messages API request body.
    ///
    /// Emits `cache_control` markers when the selected model advertises
    /// `supports_cache_control`, the request does not disable caching
    /// (`CacheRetention::Disabled`). `None` preserves the provider default,
    /// which is the ordinary ephemeral marker used by `Short`. Markers
    /// land on: the system prompt block, the last user-message text block,
    /// the last assistant-message text block, and the last tool definition.
    /// TTL `"1h"` is used only when the model also advertises
    /// `supports_long_cache_retention` and the request retention is `Long`.
    pub fn build_request_body(
        &self,
        request: &Request,
    ) -> Result<serde_json::Value, crate::provider::ProviderError> {
        self.build_request_body_variant(request, false, false)
            .map(|compiled| compiled.body)
    }

    /// The variant-aware body compiler (task 20.11): the wire body plus the
    /// downgrade qualification fact (whether any prefer-strict tool actually
    /// went out strict) and the trusted declarations of strict-dispatched
    /// tools (returned-argument null normalization).
    fn build_request_body_variant(
        &self,
        request: &Request,
        disable_preferred_strict: bool,
        oauth: bool,
    ) -> Result<CompiledAnthropicRequest, crate::provider::ProviderError> {
        let model_id = request
            .model
            .split_once(':')
            .map(|(_, id)| id)
            .unwrap_or(&request.model);

        // Resolve cache-control policy for this model+request combination.
        let cache_enabled = request.cache_retention != CacheRetention::Disabled;
        let model_caps = self.models.iter().find(|m| m.id == model_id);
        let compat = model_caps
            .and_then(|model| match &model.compat {
                WireCompat::AnthropicMessages(compat) => Some(compat),
                _ => None,
            })
            .cloned()
            .unwrap_or(AnthropicMessagesCompat {
                supports_eager_tool_input_streaming: false,
                force_adaptive_thinking: false,
                supports_temperature: true,
                supports_strict_tools: false,
                supports_mid_convo_system_messages: false,
                supports_mid_convo_tool_changes: false,
            });
        let supports_cache = model_caps
            .map(|m| m.capabilities.supports_cache_control)
            .unwrap_or(false);
        let emit_cache = cache_enabled && supports_cache;
        let use_long_ttl = emit_cache
            && request.cache_retention == CacheRetention::Long
            && model_caps
                .map(|m| m.capabilities.supports_long_cache_retention)
                .unwrap_or(false);

        let cache_marker = if use_long_ttl {
            serde_json::json!({"type": "ephemeral", "ttl": "1h"})
        } else {
            serde_json::json!({"type": "ephemeral"})
        };

        // Canonical transcript consumption (AIR-020): the prompt and tool
        // declarations live in the transcript's system messages. Models
        // with the mid-convo compat keep later system messages native
        // (task 20.39); everything else projects by collapse.
        let transcript = crate::transcript::resolve_transcript(
            &request.context,
            compat.supports_mid_convo_system_messages,
            crate::transcript::TranscriptProjection::NativeOrCollapse,
        )
        .expect("native-or-collapse projection never fails");
        let transcript = transcript.as_ref();
        let system_prompt = crate::transcript::get_initial_system_message(transcript.messages())
            .map(crate::transcript::system_message_text)
            .unwrap_or_default();
        let current_tools = crate::transcript::get_current_tools(transcript.messages());
        // Native tool changes (task 20.39, pi `nativeToolChanges`): the
        // request-level tool list stays the initial active set plus the
        // deferred placeholder; later tools arrive by value in
        // `tool_addition` blocks (same-name redefinitions included) and
        // withdrawals as `tool_removal` references.
        let initial_tools = crate::transcript::get_initial_system_message(transcript.messages())
            .map(|head| head.tools_added.clone())
            .unwrap_or_default();
        let native_tool_changes = compat.supports_mid_convo_system_messages
            && compat.supports_mid_convo_tool_changes
            && !initial_tools.is_empty();

        // Canonical replay projection (task 20.7): association is
        // validated and lossy transforms applied before the body is
        // built; association failures fail typed with zero
        // authorization/network use.
        let replay = crate::replay::project_replay(
            &crate::transcript::without_initial_system_message(transcript.messages()),
            &crate::replay::ReplayRoute {
                wire: crate::model_info::WireApi::AnthropicMessages,
                provider: self.provider_id.clone(),
                model: model_id.to_owned(),
                supports_images: crate::replay::supports_images_in_catalog(&self.models, model_id),
            },
            crate::replay::ReplayProjection::NativeOrCollapse,
        )?;
        let request_level_tools = if native_tool_changes {
            &initial_tools
        } else {
            &current_tools
        };
        let (mut tools, _, strict_tools) = self.compile_wire_tools(
            request_level_tools,
            compat.supports_strict_tools,
            disable_preferred_strict,
        )?;
        let mut tool_projector = NativeToolChanges {
            provider: self,
            supports_strict: compat.supports_strict_tools,
            disable_preferred_strict,
            preferred_strict_names: preferred_strict_names(&strict_tools),
            strict_tools,
        };
        let messages = serialize_messages(
            &replay.messages,
            emit_cache,
            &cache_marker,
            crate::replay::supports_images_in_catalog(&self.models, model_id),
            native_tool_changes.then_some(&mut tool_projector),
        )?;
        let mut body = serde_json::json!({
            "model": model_id,
            "stream": true,
            "messages": messages,
        });

        // System prompt: when caching is active, wrap the bare string as a
        // singled-content-block array so the marker has a place to live.
        if oauth {
            let mut blocks = vec![serde_json::json!({
                "type": "text",
                "text": "You are Claude Code, Anthropic's official CLI for Claude.",
            })];
            if !system_prompt.is_empty() {
                blocks.push(serde_json::json!({"type": "text", "text": system_prompt}));
            }
            if emit_cache {
                for block in &mut blocks {
                    block["cache_control"] = cache_marker.clone();
                }
            }
            body["system"] = serde_json::Value::Array(blocks);
        } else if !system_prompt.is_empty() {
            if emit_cache {
                body["system"] = serde_json::json!([{
                    "type": "text",
                    "text": system_prompt,
                    "cache_control": cache_marker,
                }]);
            } else {
                body["system"] = serde_json::Value::String(system_prompt.clone());
            }
        }
        let max_tokens = request
            .max_tokens
            .or_else(|| model_caps.map(|model| model.capabilities.max_output_tokens))
            .ok_or_else(|| {
                ProviderError::Config(ProviderErrorSummary::attested_static(
                    "Anthropic requires max_tokens or a complete model output limit",
                ))
            })?;
        body["max_tokens"] = serde_json::Value::Number(max_tokens.into());
        let native = self.native_options.as_ref();
        let native_enabled = native.and_then(|options| options.thinking_enabled);
        if compat.supports_temperature
            && !native_enabled.unwrap_or(request.thinking.enabled)
            && let Some(temp) = request.temperature
        {
            body["temperature"] = serde_json::Number::from_f64(temp)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null);
        }
        if !request_level_tools.is_empty() {
            if emit_cache && let Some(last) = tools.last_mut() {
                last["cache_control"] = cache_marker.clone();
            }
            if native_tool_changes {
                // Deferred placeholder (pi DEFERRED_TOOL_PLACEHOLDER): the
                // active list never changes after this point, so the cached
                // prefix survives every tool change. Never activated.
                tools.push(serde_json::json!({
                    "name": "__pi_deferred_placeholder__",
                    "description": "Reserved placeholder. Never available. Never call this.",
                    "input_schema": {"type": "object", "properties": {}, "required": []},
                    "defer_loading": true,
                }));
            }
            body["tools"] = serde_json::Value::Array(tools);
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
        // Thinking assembly (task 20.39): native tri-state first, then the
        // neutral request intent. Enabled budgets follow the source
        // `|| 1024` semantics; the enabled display defaults to summarized
        // on both adaptive and numeric thinking.
        let neutral_enabled = request.thinking.enabled || native_enabled == Some(true);
        let model_thinking = model_caps
            .map(|model| model.capabilities.supports_thinking)
            .unwrap_or(false);
        let thinking_disabled = native_enabled == Some(false)
            && model_thinking
            && model_caps.is_some_and(|model| {
                model
                    .thinking_level_map
                    .resolve(crate::ThinkingLevel::None)
                    .is_ok()
            });
        if thinking_disabled {
            // Anthropic rejects `block_binding` and effort markers under
            // disabled thinking; the typed block replaces omission.
            body["thinking"] = serde_json::json!({"type": "disabled"});
        } else if neutral_enabled {
            let display = native
                .and_then(|options| options.thinking_display)
                .unwrap_or(AnthropicThinkingDisplay::Summarized);
            if compat.force_adaptive_thinking {
                body["thinking"] = serde_json::json!({
                    "type": "adaptive",
                    "display": display.wire_str(),
                });
                if let Some(level) = native.and_then(|options| options.effort) {
                    let resolved = model_caps
                        .map(|model| model.thinking_level_map.resolve(level))
                        .transpose()
                        .map_err(|_| {
                            crate::provider::ProviderError::Config(
                                crate::provider::ProviderErrorSummary::sanitized(format!(
                                    "anthropic-messages native options reject unsupported thinking level {level:?}"
                                )),
                            )
                        })?;
                    if let Some(effort) = resolved.flatten() {
                        body["output_config"] = serde_json::json!({"effort": effort});
                    }
                }
            } else {
                // `|| 1024`: a missing or zero budget on enabled thinking
                // sends the source native default (pi `budget_tokens:
                // thinkingBudgetTokens || 1024`).
                let budget = native
                    .and_then(|options| options.thinking_budget_tokens)
                    .or(request.thinking.budget_tokens)
                    .filter(|budget| *budget > 0)
                    .unwrap_or(1024);
                body["thinking"] = serde_json::json!({
                    "type": "enabled",
                    "budget_tokens": budget,
                    "display": display.wire_str(),
                });
            }
        }

        // Betas (task 20.39): interleaved thinking only for enabled
        // non-adaptive thinking that did not opt out; inline-tools only
        // when native tool changes anchor the request tool list.
        let mut betas: Vec<&str> = Vec::new();
        if oauth {
            betas.extend(["claude-code-20250219", "oauth-2025-04-20"]);
        }
        if neutral_enabled
            && !compat.force_adaptive_thinking
            && native.and_then(|options| options.interleaved_thinking) != Some(false)
            && !thinking_disabled
        {
            betas.push("interleaved-thinking-2025-05-14");
        }
        if native_tool_changes {
            betas.push("inline-tools-2026-09-15");
        }
        if !betas.is_empty() {
            body["betas"] = serde_json::Value::Array(
                betas
                    .into_iter()
                    .map(|beta| serde_json::Value::String(beta.into()))
                    .collect(),
            );
        }

        if let Some(choice) = native.and_then(|options| options.tool_choice.as_ref()) {
            body["tool_choice"] = match choice {
                AnthropicToolChoice::Auto => serde_json::json!({"type": "auto"}),
                AnthropicToolChoice::Any => serde_json::json!({"type": "any"}),
                AnthropicToolChoice::None_ => serde_json::json!({"type": "none"}),
                AnthropicToolChoice::Named(name) => {
                    serde_json::json!({"type": "tool", "name": name})
                }
            };
        }
        if oauth {
            project_oauth_tool_names(&mut body);
        }
        let preferred_strict_names = tool_projector
            .preferred_strict_names
            .into_iter()
            .map(|name| {
                if oauth {
                    oauth_tool_name(&name).to_owned()
                } else {
                    name
                }
            })
            .collect();
        Ok(CompiledAnthropicRequest {
            body,
            preferred_strict_names,
            strict_tools: tool_projector.strict_tools,
        })
    }

    /// Stream events from a raw SSE response body.
    pub fn stream_from_sse(&self, sse_body: &str, cancel: CancellationToken) -> EventStream {
        let mut mapper = AnthropicMapper::new();
        let mut stream_events: Vec<Result<AssistantStreamEvent, ProviderError>> = Vec::new();
        for (parsed, _raw) in parse_sse_events_with_raw(sse_body) {
            match parsed {
                ParsedEvent::Valid(event) => {
                    stream_events.extend(mapper.process(event));
                }
                ParsedEvent::UsageError(error) => {
                    stream_events.push(Err(mapper.failure(error)));
                    break;
                }
                ParsedEvent::Malformed { event_type, .. } => {
                    stream_events.push(Err(mapper.failure(ProviderError::StreamError(
                        ProviderErrorSummary::sanitized(format!(
                            "malformed Anthropic SSE event '{event_type}'"
                        )),
                    ))));
                }
            }
        }

        let _cancel = cancel; // used by the real HTTP path
        Box::pin(stream::iter(stream_events))
    }

    /// Dispatch one attempt with its payload variant (task 20.11): the
    /// ordinary frozen payload, or the single rebuilt strict downgrade
    /// (prefer-strict removed, its own payload-callback freeze slot, one
    /// recorded downgrade diagnostic on the message).
    fn stream_dispatch(
        &self,
        mut request: Request,
        mut resolved: crate::auth::ResolvedAuth,
        variant: crate::provider::PayloadVariant,
    ) -> EventStream {
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
            .unwrap_or(&self.default_base_url);
        let effective_base =
            crate::auth::builtin::effective_base_url(&self.provider_id, selected_base, &resolved);
        if effective_base != selected_base {
            let mut configured = self.clone();
            if let Some(model) = configured
                .models
                .iter_mut()
                .find(|model| model.id == model_id)
            {
                model.base_url = Some(effective_base);
            } else {
                configured.default_base_url = effective_base;
            }
            return configured.stream_dispatch(request, resolved, variant);
        }
        request.callbacks = request.callbacks.ensure_call_scope();
        // Same as `stream` but skips per-call auth resolution: the collection
        // resolved and froze authentication once for the logical call.
        if let Err(error) = crate::provider::validate_request_capabilities(self, &request) {
            return Box::pin(stream::once(async move { Err(error) }));
        }
        let default_base_url = self.default_base_url.clone();
        let provider_id = self.provider_id.clone();
        let auth_invalid_policy = self.auth_invalid_policy;
        let header_owned = matches!(&resolved.provenance.source,
            crate::auth::AuthProvenanceSource::Environment { name } if name == "ANTHROPIC_AUTH_TOKEN");
        let direct_oauth_beta = !header_owned
            && !self.copilot_headers
            && self.provider_id != "cloudflare-ai-gateway"
            && ((matches!(resolved.scheme, AuthScheme::ApiKey)
                && resolved.secret.expose_secret().contains("sk-ant-oat"))
                || (self.direct_oauth_beta && matches!(resolved.scheme, AuthScheme::Bearer)));
        if direct_oauth_beta {
            resolved.scheme = AuthScheme::Bearer;
        }
        let oauth_tool_names = if direct_oauth_beta {
            crate::transcript::get_current_tools(request.context.messages())
                .into_iter()
                .map(|tool| tool.name)
                .collect()
        } else {
            Vec::new()
        };
        let model_id = request
            .model
            .split_once(':')
            .map(|(_, model_id)| model_id)
            .unwrap_or(&request.model);
        let model_base_url = self
            .models
            .iter()
            .find(|model| model.id == model_id)
            .and_then(|model| model.base_url.clone());
        let disable_preferred_strict =
            variant == crate::provider::PayloadVariant::AnthropicStrictDowngrade;
        let CompiledAnthropicRequest {
            body,
            preferred_strict_names,
            strict_tools,
        } = match self.build_request_body_variant(
            &request,
            disable_preferred_strict,
            direct_oauth_beta,
        ) {
            Ok(compiled) => compiled,
            Err(error) => {
                return Box::pin(stream::once(async move { Err(error) }));
            }
        };
        let cancel = request.cancel.clone();
        let timeout = request.timeout;
        let copilot_headers = if self.copilot_headers {
            match github_copilot_route_headers(&request) {
                Ok(mut headers) => {
                    let initiator = github_copilot_initiator(&request);
                    headers.push(("X-Initiator".into(), initiator.into()));
                    Ok(headers)
                }
                Err(error) => Err(error),
            }
        } else {
            Ok(Vec::new())
        };
        let extra_headers = copilot_headers.and_then(|mut route_headers| {
            if direct_oauth_beta {
                route_headers.extend([
                    ("accept".into(), "application/json".into()),
                    (
                        "anthropic-dangerous-direct-browser-access".into(),
                        "true".into(),
                    ),
                    ("user-agent".into(), "claude-cli/2.1.280".into()),
                    ("x-app".into(), "cli".into()),
                ]);
            }
            self.headers.merge_model_request(
                self.models
                    .iter()
                    .find(|model| model.id == model_id)
                    .map(|model| model.headers.as_slice())
                    .unwrap_or_default(),
                &route_headers,
                &request.extra_headers,
            )
        });
        let callbacks = request.callbacks.clone();
        let http_client = self.client.client().clone();
        let federation = self.federation.clone();
        let federation_client = self.client.clone();
        // The downgrade diagnostic rides the rebuilt variant's message from
        // Start through the terminal (V-34: one recorded downgrade).
        let diagnostics_seed =
            if variant == crate::provider::PayloadVariant::AnthropicStrictDowngrade {
                vec![crate::stream::AssistantDiagnostic {
                    kind: "anthropic_strict_tools_fallback".into(),
                    reason: "strict_tool_limits_exceeded".into(),
                    detail: None,
                }]
            } else {
                Vec::new()
            };

        let snapshot = crate::stream::ProducerSnapshot::default();
        let producer_snapshot = snapshot.clone();
        let (tx, rx) = tokio::sync::mpsc::channel(64);
        let tx_closed = tx.clone();
        let producer_cancel = cancel.clone();
        let stream_cancel = cancel.clone();
        // Explicit frozen telemetry context (task 20.54/AIR-055): the producer
        // task runs under the captured parent/Dispatch when provided;
        // omission observes nothing.
        let telemetry = request.telemetry_context.clone();

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
                let body = match callbacks.apply_payload_async(variant.label(), body).await {
                    Ok(body) => body,
                    Err(error) => {
                        let _ = send_or_cancel(&producer_cancel, &tx, Err(error)).await;
                        return;
                    }
                };
                let request_tools = body
                    .get("tools")
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten();
                let inline_tools = body
                    .get("messages")
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten()
                    .flat_map(|message| {
                        message
                            .get("content")
                            .and_then(serde_json::Value::as_array)
                            .into_iter()
                            .flatten()
                    })
                    .filter(|block| block["type"] == "tool_addition")
                    .filter_map(|block| block.get("tool").and_then(|tool| tool.get("definition")));
                let has_preferred_strict = !preferred_strict_names.is_empty()
                    && request_tools.chain(inline_tools).any(|tool| {
                        tool.get("strict").and_then(serde_json::Value::as_bool) == Some(true)
                            && tool
                                .get("name")
                                .and_then(serde_json::Value::as_str)
                                .is_some_and(|name| {
                                    preferred_strict_names
                                        .iter()
                                        .any(|preferred| preferred == name)
                                })
                    });
                let base_url = crate::auth::builtin::effective_base_url(
                    &provider_id,
                    &model_base_url.unwrap_or(default_base_url),
                    &resolved,
                );
                let federation = if matches!(resolved.scheme, AuthScheme::AnthropicFederation) {
                    let result = if provider_id == "anthropic" {
                        federation.bind(&base_url, &resolved, &federation_client)
                    } else {
                        Err(ProviderError::Config(
                            ProviderErrorSummary::attested_static(
                                "Federation is only supported by the anthropic provider",
                            ),
                        ))
                    };
                    match result {
                        Ok(entry) => match entry.token().await {
                            Ok(token) => {
                                resolved.secret = token;
                                Some(entry)
                            }
                            Err(error) => {
                                let _ = tx.send(Err(error)).await;
                                return;
                            }
                        },
                        Err(error) => {
                            let _ = tx.send(Err(error)).await;
                            return;
                        }
                    }
                } else {
                    None
                };
                if let Err(e) = Self::stream_http(
                    http_client,
                    resolved,
                    base_url,
                    provider_id,
                    auth_invalid_policy,
                    direct_oauth_beta,
                    has_preferred_strict,
                    strict_tools,
                    oauth_tool_names,
                    diagnostics_seed,
                    &body,
                    cancel,
                    timeout,
                    extra_headers,
                    callbacks,
                    producer_snapshot,
                    federation,
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
    }

    /// Real HTTP streaming: POST to Anthropic Messages API and parse SSE from the byte stream.
    #[allow(clippy::too_many_arguments)]
    async fn stream_http(
        client: reqwest::Client,
        resolved: ResolvedAuth,
        base_url: String,
        provider_id: String,
        auth_invalid_policy: AuthInvalidPolicy,
        direct_oauth_beta: bool,
        has_preferred_strict: bool,
        strict_tools: Vec<crate::message::ToolDef>,
        oauth_tool_names: Vec<String>,
        diagnostics_seed: Vec<crate::stream::AssistantDiagnostic>,
        body: &serde_json::Value,
        cancel: CancellationToken,
        timeout: Option<std::time::Duration>,
        extra_headers: Vec<(String, String)>,
        callbacks: crate::callbacks::RequestCallbacks,
        snapshot: crate::stream::ProducerSnapshot,
        federation: Option<Arc<crate::auth::anthropic_federation::FederationEntry>>,
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
            AuthScheme::ApiKey if provider_id == "cloudflare-ai-gateway" => {
                request.header("cf-aig-authorization", format!("Bearer {secret}"))
            }
            AuthScheme::ApiKey => request.header("x-api-key", secret),
            // OAuth Bearer credential: the required beta header rides the same
            // arm. API-key construction never enters this arm.
            AuthScheme::Bearer if direct_oauth_beta => request
                .header("authorization", format!("Bearer {secret}"))
                .header("anthropic-beta", ANTHROPIC_OAUTH_BETA_HEADER),
            AuthScheme::Bearer => request.header("authorization", format!("Bearer {secret}")),
            AuthScheme::AnthropicFederation => request
                .header("authorization", format!("Bearer {secret}"))
                .header("anthropic-beta", "oauth-2025-04-20"),
            AuthScheme::AwsSigV4(_) => {
                return Err(ProviderError::Config(
                    ProviderErrorSummary::attested_static(
                        "Anthropic cannot dispatch the selected authentication scheme",
                    ),
                ));
            }
        };
        let response = request.send().await.map_err(|e| {
            if e.is_timeout() {
                ProviderError::Timeout
            } else {
                ProviderError::Network(ProviderErrorSummary::sanitized(e.to_string()))
            }
        })?;

        // Response observer (task 20.51): fires once the status and headers
        // exist, before any body processing or error mapping. Failures that
        // never produce a response are not notified.
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
        if status.as_u16() == 401
            && let Some(federation) = &federation
        {
            federation.invalidate();
        }
        if !status.is_success() {
            // The source-qualified strict-limit downgrade (task 20.11,
            // V-34): a 400 invalid_request_error whose message carries one
            // of the documented strict-tool limit markers, on a request that
            // actually dispatched prefer-strict tools, is typed so the
            // acquisition owner may admit exactly one rebuilt variant. Any
            // other 400 stays the ordinary provider-side failure.
            if status.as_u16() == 400 && has_preferred_strict {
                let body_text = response.text().await.unwrap_or_default();
                if let Some(error) = body_text
                    .parse::<serde_json::Value>()
                    .ok()
                    .and_then(|value| value.get("error").cloned())
                    && error.get("type").and_then(serde_json::Value::as_str)
                        == Some("invalid_request_error")
                    && let Some(message) = error.get("message").and_then(serde_json::Value::as_str)
                    && [
                        "The compiled grammar is too large",
                        "Too many strict tools (",
                        "Schemas contains too many parameters with union types (",
                    ]
                    .iter()
                    .any(|marker| message.contains(marker))
                {
                    return Err(ProviderError::StrictToolLimits {
                        detail: ProviderErrorSummary::sanitized(message),
                    });
                }
                // A 400 carries no retry-after hint; the ordinary
                // provider-side mapping for this status needs no headers.
                return Err(ProviderError::ProviderSide(
                    ProviderErrorSummary::from_http_response(400),
                ));
            }
            let headers = response.headers().clone();
            return Err(map_http_status(
                status,
                &headers,
                auth_invalid_policy,
                &provider_id,
            ));
        }
        let mut mapper = AnthropicMapper::new_for_provider(&provider_id);
        mapper.set_strict_tools(strict_tools);
        mapper.oauth_tool_names = oauth_tool_names;
        mapper.seed_diagnostics(diagnostics_seed);
        // Keep the requested route for continuation even when the service
        // reports a resolved model name for an alias.
        mapper.partial.model = body["model"].as_str().unwrap_or_default().to_owned();
        // An explicitly unknown report must not fall back to the requested
        // alias when the Agent derives actual-route evidence.
        mapper.partial.response_model = Some(String::new());

        // Successful headers hand off response ownership before callbacks
        // or body processing. A later error cannot authorize another send.
        snapshot.received_response();
        mapper.set_snapshot(snapshot);
        // Only the successfully acquired response is observed; strict-limit
        // rejections never masquerade as a success callback.
        callbacks
            .notify_response_async(status.as_u16(), &observed_headers)
            .await
            .map_err(|error| mapper.failure(error))?;
        let mut byte_stream = response.bytes_stream();
        let mut buffer = Vec::new();
        let result: Result<(), ProviderError> = async {
            loop {
                let chunk = tokio::select! {
                    _ = cancel.cancelled() => {
                        return Err(mapper.failure(ProviderError::Cancelled));
                    }
                    chunk = byte_stream.next() => {
                        chunk
                    }
                };

                let eof = chunk.is_none();
                if let Some(chunk) = chunk {
                    let chunk = chunk.map_err(|e| {
                        if e.is_timeout() {
                            ProviderError::Timeout
                        } else if e.is_connect() {
                            ProviderError::Network(ProviderErrorSummary::sanitized(e.to_string()))
                        } else {
                            ProviderError::StreamError(ProviderErrorSummary::sanitized(
                                e.to_string(),
                            ))
                        }
                    })?;
                    buffer.extend_from_slice(&chunk);
                }

                for (parsed, raw) in drain_sse_events(&mut buffer, eof)? {
                    if mapper.saw_done {
                        return Ok(());
                    }
                    // Raw observer first: strictly before normalization, in
                    // wire order; its failure ends the call with the partial
                    // preserved (task 20.51).
                    if let Some(raw) = raw.as_ref()
                        && let Err(error) = callbacks.notify_raw_async(raw).await
                    {
                        return Err(mapper.failure(error));
                    }
                    match parsed {
                        ParsedEvent::Valid(event) => {
                            for item in mapper.process(event) {
                                if !send_or_cancel(&cancel, tx, item).await? {
                                    return Ok(()); // receiver dropped
                                }
                            }
                        }
                        ParsedEvent::UsageError(error) => return Err(mapper.failure(error)),
                        ParsedEvent::Malformed { event_type, .. } => {
                            let err = ProviderError::StreamError(ProviderErrorSummary::sanitized(
                                format!("malformed Anthropic SSE event '{event_type}'"),
                            ));
                            if !send_or_cancel(&cancel, tx, Err(mapper.failure(err))).await? {
                                return Ok(());
                            }
                        }
                    }
                }
                if eof {
                    break;
                }
            }

            // Stream ended without a terminal event  - surface as provider protocol error
            if !mapper.saw_done {
                let err = ProviderError::StreamError(ProviderErrorSummary::attested_static(
                    "stream ended without a terminal event (message_stop or error)",
                ));
                let _ = send_or_cancel(&cancel, tx, Err(mapper.failure(err))).await;
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

/// Wrapper to adapt `tokio::sync::mpsc::Receiver` to `futures_core::Stream`.
/// Drain complete SSE events from a growing buffer.
/// Returns parsed [`ParsedEvent`]s and leaves incomplete data in the buffer.
/// Decode only complete frames so UTF-8 and CRLF split across HTTP chunks
/// remain unchanged, including the text covered by a thinking signature.
fn drain_sse_events(
    buffer: &mut Vec<u8>,
    eof: bool,
) -> Result<Vec<(ParsedEvent, Option<serde_json::Value>)>, ProviderError> {
    let mut events = Vec::new();
    loop {
        let mut line_start = 0;
        let mut offset = 0;
        let end = loop {
            let Some(byte) = buffer.get(offset) else {
                break None;
            };
            let width = match byte {
                b'\n' => 1,
                b'\r' => {
                    // A trailing CR may be the first half of CRLF. At EOF it
                    // is a complete lone-CR line ending.
                    if offset + 1 == buffer.len() && !eof {
                        break None;
                    }
                    if buffer.get(offset + 1) == Some(&b'\n') {
                        2
                    } else {
                        1
                    }
                }
                _ => {
                    offset += 1;
                    continue;
                }
            };
            let empty_line = offset == line_start;
            offset += width;
            if empty_line {
                break Some(offset);
            }
            line_start = offset;
        };
        let Some(end) = end else {
            break;
        };
        // Frames end on ASCII line terminators, so decoding complete frames
        // with the streaming replacement decoder preserves multi-byte
        // characters and replaces each maximal invalid subsequence with one
        // U+FFFD instead of failing the call (AIR-006).
        let frame_bytes: Vec<u8> = buffer.drain(..end).collect();
        let mut frame_decoder = crate::text_decode::StreamTextDecoder::new();
        let mut chunk = frame_decoder.push(&frame_bytes);
        chunk.push_str(&frame_decoder.finish());
        let normalized = chunk.replace("\r\n", "\n").replace('\r', "\n");
        events.extend(parse_sse_events_with_raw(&normalized));
    }
    Ok(events)
}

/// Map an HTTP status code and headers to a `ProviderError`.
///
/// `policy` is route construction policy rather than HTTP auth syntax. Both
/// auth-invalid outcomes drop the response body because a proxy may echo the
/// submitted credential.
fn map_http_status(
    status: reqwest::StatusCode,
    headers: &reqwest::header::HeaderMap,
    policy: AuthInvalidPolicy,
    provider_id: &str,
) -> ProviderError {
    match status.as_u16() {
        401 | 403 => policy.error(provider_id),
        429 => ProviderError::RateLimited {
            retry_after_ms: crate::retry::parse_retry_after(headers),
        },
        408 | 504 => ProviderError::Timeout,
        code => ProviderError::ProviderSide(ProviderErrorSummary::from_http_response(code)),
    }
}

fn serialize_messages(
    messages: &[crate::message::Message],
    emit_cache: bool,
    cache_marker: &serde_json::Value,
    supports_images: bool,
    mut tool_changes: Option<&mut NativeToolChanges<'_>>,
) -> Result<serde_json::Value, crate::provider::ProviderError> {
    use crate::message::{AssistantContent, ImageSource, OutputContent};

    fn image_block(
        source: &ImageSource,
        media_type: &crate::message::MediaType,
    ) -> serde_json::Value {
        match source {
            ImageSource::Url { url } => serde_json::json!({
                "type": "image",
                "source": {"type": "url", "url": url},
            }),
            ImageSource::Base64 { data } => serde_json::json!({
                "type": "image",
                "source": {
                    "type": "base64",
                    "media_type": media_type.as_str(),
                    "data": data,
                },
            }),
            ImageSource::Bytes { data } => serde_json::json!({
                "type": "image",
                "source": {
                    "type": "base64",
                    "media_type": media_type.as_str(),
                    "data": base64::Engine::encode(
                        &base64::engine::general_purpose::STANDARD,
                        data,
                    ),
                },
            }),
        }
    }

    let mut array: Vec<serde_json::Value> = Vec::new();
    // Native tool changes (task 20.39, pi `nativeToolChanges`): later
    // system messages stay native and carry by-value `tool_addition`
    // blocks (same-name redefinitions included) plus `tool_removal`
    // references. They are held back until the next assistant message (or
    // the end) so a `tool_result` still immediately follows its `tool_use`.
    let mut pending_system: Vec<serde_json::Value> = Vec::new();
    let flush_pending_system =
        |array: &mut Vec<serde_json::Value>, pending: &mut Vec<serde_json::Value>| {
            array.append(pending);
        };
    // Error-result images split out of their tool_result blocks, flushed
    // after the turn's tool results (pinned splitErrorToolResultImages,
    // #10794): each entry is a caption plus the deferred image blocks, and
    // the source text is preserved with a placeholder in place of each
    // image run.
    let mut turn_tool_results: Vec<serde_json::Value> = Vec::new();
    let mut turn_error_images: Vec<(String, Vec<serde_json::Value>)> = Vec::new();

    fn flush_tool_turn(
        array: &mut Vec<serde_json::Value>,
        turn_tool_results: &mut Vec<serde_json::Value>,
        turn_error_images: &mut Vec<(String, Vec<serde_json::Value>)>,
    ) {
        if turn_tool_results.is_empty() && turn_error_images.is_empty() {
            return;
        }
        let mut content = std::mem::take(turn_tool_results);
        for (caption, images) in turn_error_images.drain(..) {
            content.push(serde_json::json!({"type": "text", "text": caption}));
            content.extend(images);
        }
        array.push(serde_json::json!({"role": "user", "content": content}));
    }

    for msg in messages {
        match msg {
            crate::message::Message::User(u) => {
                flush_tool_turn(&mut array, &mut turn_tool_results, &mut turn_error_images);
                let content: Vec<serde_json::Value> = u
                    .content
                    .iter()
                    .map(|c| match c {
                        crate::message::InputContent::Text { text } => {
                            serde_json::json!({"type": "text", "text": text})
                        }
                        crate::message::InputContent::Image { source, media_type } => {
                            image_block(source, media_type)
                        }
                    })
                    .collect();
                array.push(serde_json::json!({"role": "user", "content": content}));
            }
            crate::message::Message::Assistant(a) => {
                flush_tool_turn(&mut array, &mut turn_tool_results, &mut turn_error_images);
                flush_pending_system(&mut array, &mut pending_system);
                // The replay projection already applied the continuation
                // identity rules (AIR-010/RP-02): every thinking block in
                // this list is same-identity, blank unsigned thinking is
                // gone, and malformed proposals on this object wire were
                // replaced by explicit failure contexts.
                let content: Vec<serde_json::Value> = a
                    .content
                    .iter()
                    .map(|c| match c {
                        AssistantContent::Text { text, .. } => {
                            serde_json::json!({"type": "text", "text": text})
                        }
                        AssistantContent::ToolCall { tool_call } => {
                            serde_json::json!({
                                "type": "tool_use",
                                "id": tool_call.id,
                                "name": tool_call.name,
                                "input": serde_json::from_str::<serde_json::Value>(
                                    &tool_call.arguments
                                ).unwrap_or_else(|_| serde_json::json!({})),
                            })
                        }
                        AssistantContent::Thinking {
                            thinking,
                            signature,
                        } => {
                            let mut block =
                                serde_json::json!({"type": "thinking", "thinking": thinking});
                            if let Some(signature) = signature {
                                block["signature"] = serde_json::json!(signature);
                            }
                            block
                        }
                        AssistantContent::RedactedThinking { data } => {
                            serde_json::json!({"type": "redacted_thinking", "data": data})
                        }
                    })
                    .collect();
                array.push(serde_json::json!({"role": "assistant", "content": content}));
            }
            crate::message::Message::System(system) => {
                // Task 20.39: native tool changes keep later system messages
                // on the wire; the collapse projection removed them on every
                // other route, so without the projector this stays
                // unreachable.
                let Some(projector) = tool_changes.as_mut() else {
                    unreachable!("system messages are projected out of the body list")
                };
                pending_system.push(serde_json::json!({
                    "role": "system",
                    "content": projector.system_blocks(system)?,
                }));
            }
            crate::message::Message::ToolResult(t) => {
                // Success results keep their tool association, ordering and
                // failure flag: real image blocks ride the content array on
                // image-capable routes (AIR-014/V-08). Error results cannot
                // carry images on this wire: they are split out and sent
                // after the turn's tool results in the same user message
                // (pinned splitErrorToolResultImages, #10794).
                let has_images = t
                    .content
                    .iter()
                    .any(|c| matches!(c, OutputContent::Image { .. }));
                let is_error_with_images = t.is_error && has_images && supports_images;
                if !supports_images || !has_images || is_error_with_images {
                    // Replay already replaced images with text placeholders
                    // on non-image routes; an error result about to be split
                    // keeps its text with a placeholder in place of each
                    // image run.
                    let text = t
                        .content
                        .iter()
                        .filter_map(|c| match c {
                            OutputContent::Text { text } => Some(text.clone()),
                            OutputContent::Image { .. } => None,
                        })
                        .collect::<Vec<_>>()
                        .join("");
                    let mut block = serde_json::json!({
                        "type": "tool_result",
                        "tool_use_id": t.tool_call_id,
                        "content": text,
                    });
                    if is_error_with_images {
                        // Placeholder text notes the deferred images; the
                        // split payload follows all of this turn's results
                        // in the same user message.
                        let mut content = vec![serde_json::json!({
                            "type": "text",
                            "text": text,
                        })];
                        if !text.is_empty() {
                            content.push(serde_json::json!({
                                "type": "text",
                                "text": "(image attached after the tool results)",
                            }));
                        } else {
                            content = vec![serde_json::json!({
                                "type": "text",
                                "text": "(image attached after the tool results)",
                            })];
                        }
                        block["content"] = serde_json::Value::Array(content);
                        let images: Vec<serde_json::Value> = t
                            .content
                            .iter()
                            .filter_map(|c| match c {
                                OutputContent::Image { source, media_type } => {
                                    Some(image_block(source, media_type))
                                }
                                _ => None,
                            })
                            .collect();
                        turn_error_images.push((
                            format!(
                                "Images from the error result of tool call {} ({}):",
                                t.tool_call_id, t.tool_name
                            ),
                            images,
                        ));
                        block["is_error"] = serde_json::Value::Bool(true);
                    } else if t.is_error {
                        // The Anthropic Messages API documents `is_error` on
                        // the tool_result content block as the failure
                        // signal. Emit it only on failure so successful
                        // results keep their established wire shape.
                        block["is_error"] = serde_json::Value::Bool(true);
                    }
                    turn_tool_results.push(block);
                    continue;
                }
                let blocks: Vec<serde_json::Value> = t
                    .content
                    .iter()
                    .map(|c| match c {
                        OutputContent::Text { text } => {
                            serde_json::json!({"type": "text", "text": text})
                        }
                        OutputContent::Image { source, media_type } => {
                            image_block(source, media_type)
                        }
                    })
                    .collect();
                turn_tool_results.push(serde_json::json!({
                    "type": "tool_result",
                    "tool_use_id": t.tool_call_id,
                    "content": blocks,
                }));
            }
        }
    }
    flush_tool_turn(&mut array, &mut turn_tool_results, &mut turn_error_images);
    flush_pending_system(&mut array, &mut pending_system);

    // Post-process: when caching is active, mark the last text block in the
    // last user message and the last text block in the last assistant message.
    if emit_cache && !array.is_empty() {
        mark_last_role_text(&mut array, "user", cache_marker);
        mark_last_role_text(&mut array, "assistant", cache_marker);
    }

    array.retain(|message| {
        message["role"] != "assistant"
            || message["content"]
                .as_array()
                .is_none_or(|content| !content.is_empty())
    });

    Ok(serde_json::Value::Array(array))
}

/// Native tool-change projection for one mid-conversation system message
/// (task 20.39): by-value `tool_addition` definitions (same-name
/// redefinitions are additions, never removals) and `tool_removal`
/// references, after the message's text block.
struct NativeToolChanges<'a> {
    provider: &'a AnthropicProvider,
    supports_strict: bool,
    disable_preferred_strict: bool,
    preferred_strict_names: Vec<String>,
    strict_tools: Vec<crate::message::ToolDef>,
}

impl NativeToolChanges<'_> {
    fn system_blocks(
        &mut self,
        system: &crate::transcript::SystemMessage,
    ) -> Result<Vec<serde_json::Value>, crate::provider::ProviderError> {
        let mut blocks = Vec::new();
        if !system.content.is_empty() {
            blocks.push(serde_json::json!({"type": "text", "text": system.content}));
        }
        let redefined: std::collections::HashSet<&str> = system
            .tools_added
            .iter()
            .map(|tool| tool.name.as_str())
            .collect();
        for removed in &system.tools_removed {
            self.strict_tools.retain(|tool| tool.name != removed.name);
            // A new definition under the same name replaces the old one, so
            // no removal is needed (pi `tool_removal` redefinition rule).
            if redefined.contains(removed.name.as_str()) {
                continue;
            }
            blocks.push(serde_json::json!({
                "type": "tool_removal",
                "tool": {"type": "tool_reference", "name": removed.name},
            }));
        }
        if !system.tools_added.is_empty() {
            let (tools, _, strict_tools) = self.provider.compile_wire_tools(
                &system.tools_added,
                self.supports_strict,
                self.disable_preferred_strict,
            )?;
            self.preferred_strict_names
                .extend(preferred_strict_names(&strict_tools));
            self.strict_tools
                .retain(|tool| !redefined.contains(tool.name.as_str()));
            self.strict_tools.extend(strict_tools);
            for definition in tools {
                blocks.push(serde_json::json!({
                    "type": "tool_addition",
                    "tool": {"type": "tool_definition", "definition": definition},
                }));
            }
        }
        Ok(blocks)
    }
}

fn mark_last_role_text(
    messages: &mut [serde_json::Value],
    role: &str,
    cache_marker: &serde_json::Value,
) {
    for message in messages.iter_mut().rev() {
        if message["role"] != role {
            continue;
        }
        let Some(blocks) = message["content"].as_array_mut() else {
            continue;
        };
        if let Some(block) = blocks
            .iter_mut()
            .rev()
            .find(|block| block["type"] == "text")
        {
            block["cache_control"] = cache_marker.clone();
            return;
        }
    }
}

impl Provider for AnthropicProvider {
    fn validate_simple_options(
        &self,
        model: &ModelInfo,
        _options: &crate::models::SimpleOptions,
    ) -> Result<(), ProviderError> {
        validate_complete_anthropic_record(model)
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
        let mut configured = self.clone();
        configured.models = vec![model];
        configured.native_options = Some(AnthropicMessagesOptions::from_simple(&options));

        configured.stream_resource_attempt(request, auth, attempt, resources)
    }

    fn stream_prepared(
        &self,
        request: Request,
        resolved: crate::auth::ResolvedAuth,
    ) -> EventStream {
        crate::models::drive_resolved_call(Arc::new(self.clone()), request, resolved)
    }

    /// The acquisition owner's per-attempt entry (task 20.11): the rebuilt
    /// strict-downgrade variant arrives here with its own payload-variant
    /// label; ordinary attempts keep the frozen ordinary payload.
    fn stream_attempt(
        &self,
        request: Request,
        resolved: crate::auth::ResolvedAuth,
        attempt: crate::provider::AttemptContext,
    ) -> EventStream {
        self.stream_dispatch(request, resolved, attempt.payload_variant)
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

/// The public anthropic-messages direct adapter registered through
/// [`crate::Models::register_chat_api`] (task 20.39). Each dispatch clones
/// its immutable provider configuration; native options never mutate a
/// registered provider.
pub struct AnthropicMessagesChatApi {
    api_id: crate::models::ApiId,
    provider: Arc<AnthropicProvider>,
    model: ModelInfo,
}

impl std::fmt::Debug for AnthropicMessagesChatApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnthropicMessagesChatApi")
            .field("api_id", &self.api_id)
            .finish_non_exhaustive()
    }
}

impl AnthropicMessagesChatApi {
    /// Bind a complete Chat record. Wrong-wire or invalid records fail
    /// before authentication or network activity.
    pub fn new(
        provider: Arc<AnthropicProvider>,
        model: ModelInfo,
    ) -> Result<Self, crate::provider::ProviderError> {
        validate_complete_anthropic_record(&model)?;
        Ok(Self {
            api_id: crate::models::ApiId::new("anthropic-messages"),
            provider,
            model,
        })
    }

    fn configured(&self, model: ModelInfo, options: AnthropicMessagesOptions) -> AnthropicProvider {
        let mut provider = (*self.provider).clone();
        provider.models = vec![model];
        provider.native_options = Some(options);
        provider
    }
}

/// One complete, wire-valid Chat model record or a typed failure.
fn validate_complete_anthropic_record(
    model: &ModelInfo,
) -> Result<(), crate::provider::ProviderError> {
    if model.wire_api != crate::model_info::WireApi::AnthropicMessages
        || model.id.trim().is_empty()
        || model.validate().is_err()
    {
        return Err(crate::provider::ProviderError::Config(
            crate::provider::ProviderErrorSummary::attested_static(
                "anthropic-messages adapters require a valid complete Chat model record",
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

impl crate::models::ChatApi<AnthropicMessagesOptions> for AnthropicMessagesChatApi {
    fn validate_simple_options(
        &self,
        model: &ModelInfo,
        _options: &crate::models::SimpleOptions,
    ) -> Result<(), crate::provider::ProviderError> {
        validate_complete_anthropic_record(model)
    }

    fn api_id(&self) -> &crate::models::ApiId {
        &self.api_id
    }

    fn provider_id(&self) -> &str {
        &self.provider.provider_id
    }

    fn complete_record(&self) -> ModelInfo {
        self.model.clone()
    }

    fn simple_options(&self) -> AnthropicMessagesOptions {
        AnthropicMessagesOptions::from_simple(&crate::models::SimpleOptions::default())
    }

    fn simple_options_for(
        &self,
        options: &crate::models::SimpleOptions,
    ) -> AnthropicMessagesOptions {
        AnthropicMessagesOptions::from_simple(options)
    }

    fn session_resources(&self) -> Option<&dyn crate::codex_resources::SessionResourceOwner> {
        None
    }

    fn validate_native_request(
        &self,
        model: &ModelInfo,
        _request: &Request,
        options: &AnthropicMessagesOptions,
    ) -> Result<(), crate::provider::ProviderError> {
        validate_complete_anthropic_record(model)?;
        // Native effort fails typed preflight on unsupported levels; the
        // neutral entry clamps instead.
        if let Some(level) = options.effort
            && model.thinking_level_map.resolve(level).is_err()
        {
            return Err(crate::provider::ProviderError::Config(
                crate::provider::ProviderErrorSummary::sanitized(format!(
                    "anthropic-messages native options reject unsupported thinking level {level:?}"
                )),
            ));
        }
        Ok(())
    }

    fn stream_prepared(
        &self,
        request: Request,
        options: AnthropicMessagesOptions,
        auth: crate::auth::ResolvedAuth,
    ) -> EventStream {
        self.stream_model_resource_prepared(self.model.clone(), request, options, auth, None)
    }

    fn stream_resource_prepared(
        &self,
        request: Request,
        options: AnthropicMessagesOptions,
        auth: crate::auth::ResolvedAuth,
        resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        self.stream_model_resource_prepared(self.model.clone(), request, options, auth, resources)
    }

    fn stream_model_resource_prepared(
        &self,
        model: ModelInfo,
        mut request: Request,
        options: AnthropicMessagesOptions,
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
        if resources.is_none() {
            return provider.stream_prepared(request, auth);
        }
        let attempt = request.callbacks.inherited_attempt().unwrap_or_else(|| {
            crate::provider::AttemptContext {
                invocation_id: uuid::Uuid::new_v4().to_string(),
                attempt: 1,
                max_attempts: 1,
                payload_variant: crate::provider::PayloadVariant::Ordinary,
            }
        });
        provider.stream_resource_attempt(request, auth, attempt, resources)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{
        AssistantContent, AssistantMessage, ImageSource, InputContent, MediaType, Message,
        OutputContent, ToolCall, ToolResultMessage, UserMessage,
    };
    use crate::stream::{StopReason, Usage};

    fn test_assistant_msg(content: Vec<AssistantContent>) -> Message {
        Message::Assistant(AssistantMessage {
            content,
            api: crate::ApiKind::Anthropic,
            provider: "anthropic".into(),
            model: "claude-sonnet-4-5-20250514".into(),
            response_model: None,
            wire: Some(crate::model_info::WireApi::AnthropicMessages),
            response_id: None,
            usage: Usage::default(),
            stop_reason: StopReason::Stop,
            error_message: None,
            timestamp_ms: 0,
            diagnostics: Vec::new(),
            duration_ms: None,
            raw_stop_reason: None,
            provider_thinking_level: None,
            deferred: None,
        })
    }

    #[test]
    fn serialize_tool_call_input_is_json_object() {
        let msg = test_assistant_msg(vec![AssistantContent::ToolCall {
            tool_call: ToolCall {
                id: "tc_1".into(),
                name: "read".into(),
                arguments: r#"{"path":"/tmp/foo.txt"}"#.into(),
                arguments_error: None,
                signature: None,
            },
        }]);

        let serialized =
            serialize_messages(&[msg], false, &serde_json::Value::Null, true, None).unwrap();
        let input = &serialized[0]["content"][0]["input"];
        assert!(input.is_object(), "input must be JSON object, got: {input}");
        assert_eq!(input["path"], "/tmp/foo.txt");
    }

    #[test]
    fn malformed_args_become_failure_context_not_empty_object() {
        // A proposal without a result has unknown effects. An object-only
        // wire receives that fact as context rather than a fabricated call.
        let msg = test_assistant_msg(vec![AssistantContent::ToolCall {
            tool_call: ToolCall {
                id: "tc_2".into(),
                name: "bash".into(),
                arguments: "not valid json".into(),
                arguments_error: None,
                signature: None,
            },
        }]);

        let replay = crate::replay::project_replay(
            &[msg],
            &crate::replay::ReplayRoute {
                wire: crate::model_info::WireApi::AnthropicMessages,
                provider: "anthropic".into(),
                model: "claude-sonnet-4-5-20250514".into(),
                supports_images: true,
            },
            crate::replay::ReplayProjection::NativeOrCollapse,
        )
        .unwrap();
        let serialized = serialize_messages(
            &replay.messages,
            false,
            &serde_json::Value::Null,
            true,
            None,
        )
        .unwrap();
        let body = serde_json::to_string(&serialized).unwrap();
        assert!(body.contains("Result unavailable"));
        assert!(body.contains("execution effect is unknown"));
        assert!(!body.contains("not-executed"));
        assert!(!body.contains("\"tool_use\""));
    }

    #[test]
    fn non_object_json_args_become_failure_contexts() {
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
                    arguments_error: None,
                    signature: None,
                },
            }]);

            for known_non_execution in [false, true] {
                let mut messages = vec![msg.clone()];
                if known_non_execution {
                    messages.push(Message::ToolResult(crate::message::ToolResultMessage {
                        tool_call_id: "tc".into(),
                        tool_name: "test".into(),
                        content: vec![OutputContent::Text {
                            text: "private diagnostic".into(),
                        }],
                        details: None,
                        is_error: true,
                        not_executed: Some(
                            crate::message::ToolNotExecutedReason::NonObjectArguments,
                        ),
                        truncated: false,
                        timestamp_ms: 0,
                    }));
                }
                let replay = crate::replay::project_replay(
                    &messages,
                    &crate::replay::ReplayRoute {
                        wire: crate::model_info::WireApi::AnthropicMessages,
                        provider: "anthropic".into(),
                        model: "claude-sonnet-4-5-20250514".into(),
                        supports_images: true,
                    },
                    crate::replay::ReplayProjection::NativeOrCollapse,
                )
                .unwrap();
                let serialized = serialize_messages(
                    &replay.messages,
                    false,
                    &serde_json::Value::Null,
                    true,
                    None,
                )
                .unwrap();
                let body = serde_json::to_string(&serialized).unwrap();
                if known_non_execution {
                    assert!(
                        body.contains(crate::replay::MALFORMED_PROPOSAL_NOTE_PREFIX),
                        "{label}"
                    );
                    assert!(body.contains("non_object_arguments"));
                } else {
                    assert!(body.contains("Result unavailable"), "{label}");
                    assert!(body.contains("execution effect is unknown"));
                    assert!(!body.contains("not-executed"));
                }
                assert!(!body.contains("private diagnostic"));
                assert!(
                    !body.contains("\"tool_use\""),
                    "{label}: no fabricated call"
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // Cache-control marker tests (task 14.4)
    // -----------------------------------------------------------------------

    /// Build a test AnthropicProvider whose models have cache capabilities.
    fn cache_capable_provider() -> AnthropicProvider {
        AnthropicProvider::with_client(
            Some("https://api.anthropic.com".into()),
            Arc::new(crate::http::HttpClient::new()),
        )
    }

    fn make_request(cache_retention: CacheRetention) -> Request {
        Request {
            transport: Default::default(),
            model: "anthropic:claude-sonnet-4-5-20250514".into(),
            context: crate::transcript::normalize_context(crate::transcript::Context {
                system_prompt: Some("You are a test assistant.".into()),
                messages: vec![
                    Message::User(UserMessage {
                        content: vec![crate::message::InputContent::Text {
                            text: "Hello".into(),
                        }],
                        timestamp_ms: 0,
                    }),
                    Message::Assistant(AssistantMessage {
                        content: vec![AssistantContent::Text {
                            text: "Hi there!".into(),
                            signature: None,
                        }],
                        api: crate::ApiKind::Anthropic,
                        provider: "anthropic".into(),
                        model: "claude-sonnet-4-5-20250514".into(),
                        response_model: None,
                        wire: None,
                        response_id: None,
                        usage: crate::stream::Usage::unknown(),
                        stop_reason: crate::stream::StopReason::Stop,
                        error_message: None,
                        timestamp_ms: 0,
                        diagnostics: Vec::new(),
                        duration_ms: None,
                        raw_stop_reason: None,
                        provider_thinking_level: None,
                        deferred: None,
                    }),
                ],
                tools: vec![crate::message::ToolDef {
                    name: "greet".into(),
                    description: "Say hello".into(),
                    input_schema: serde_json::json!({"type": "object"}),
                    constrained_sampling: None,
                }],
            })
            .expect("valid context declarations"),
            max_tokens: Some(100),
            temperature: None,
            sampling_params: Default::default(),
            thinking: crate::provider::ThinkingConfig::default(),
            stop_sequences: vec![],
            metadata: None,
            cancel: tokio_util::sync::CancellationToken::new(),
            timeout: None,
            extra_headers: vec![],
            cache_retention,
            session_id: None,
            telemetry_context: None,
            callbacks: Default::default(),
        }
    }

    #[test]
    fn cache_control_long_ttl_emits_all_markers() {
        let provider = cache_capable_provider();
        let request = make_request(CacheRetention::Long);
        let body = provider.build_request_body(&request).unwrap();

        // System prompt wrapped as array with cache_control
        let system = &body["system"];
        assert!(system.is_array(), "system must be array when caching");
        let sys_block = &system[0];
        assert_eq!(sys_block["type"], "text");
        assert_eq!(sys_block["cache_control"]["type"], "ephemeral");
        assert_eq!(sys_block["cache_control"]["ttl"], "1h");

        // Last user text block has cache_control
        let msgs = body["messages"].as_array().unwrap();
        let user_msg = msgs.iter().rev().find(|m| m["role"] == "user").unwrap();
        let user_blocks = user_msg["content"].as_array().unwrap();
        let last_text = user_blocks
            .iter()
            .rev()
            .find(|b| b["type"] == "text")
            .unwrap();
        assert_eq!(last_text["cache_control"]["type"], "ephemeral");
        assert_eq!(last_text["cache_control"]["ttl"], "1h");

        // Last assistant text block has cache_control
        let assistant_msg = msgs
            .iter()
            .rev()
            .find(|m| m["role"] == "assistant")
            .unwrap();
        let asst_blocks = assistant_msg["content"].as_array().unwrap();
        let last_asst_text = asst_blocks
            .iter()
            .rev()
            .find(|b| b["type"] == "text")
            .unwrap();
        assert_eq!(last_asst_text["cache_control"]["type"], "ephemeral");
        assert_eq!(last_asst_text["cache_control"]["ttl"], "1h");

        // Last tool has cache_control
        let tools = body["tools"].as_array().unwrap();
        let last_tool = tools.last().unwrap();
        assert_eq!(last_tool["cache_control"]["type"], "ephemeral");
        assert_eq!(last_tool["cache_control"]["ttl"], "1h");
    }

    #[test]
    fn cache_control_short_uses_ephemeral_no_ttl() {
        let provider = cache_capable_provider();
        let request = make_request(CacheRetention::Short);
        let body = provider.build_request_body(&request).unwrap();

        let system = &body["system"];
        assert!(system.is_array());
        assert!(
            system[0]["cache_control"]["ttl"].is_null(),
            "Short retention should not have ttl field"
        );
        assert_eq!(system[0]["cache_control"]["type"], "ephemeral");
    }

    #[test]
    fn cache_control_disabled_suppresses_all_markers() {
        let provider = cache_capable_provider();
        let request = make_request(CacheRetention::Disabled);
        let body = provider.build_request_body(&request).unwrap();

        // System stays as string (no array wrapping)
        assert!(
            body["system"].is_string(),
            "system must be string when caching disabled"
        );

        // No cache_control on any content block
        let msgs = body["messages"].as_array().unwrap();
        for msg in msgs {
            for block in msg["content"].as_array().unwrap() {
                assert!(
                    block["cache_control"].is_null(),
                    "cache_control must be absent when Disabled"
                );
            }
        }

        // No cache_control on tools
        if let Some(tools) = body["tools"].as_array() {
            for tool in tools {
                assert!(tool["cache_control"].is_null());
            }
        }
    }

    #[test]
    fn cache_control_none_preserves_provider_default_markers() {
        let provider = cache_capable_provider();
        let request = make_request(CacheRetention::None);
        let body = provider.build_request_body(&request).unwrap();

        assert_eq!(body["system"][0]["cache_control"]["type"], "ephemeral");
        assert!(
            body["system"][0]["cache_control"].get("ttl").is_none(),
            "provider-default markers use the ordinary ephemeral retention"
        );
    }

    #[test]
    fn cache_control_reverse_scan_skips_textless_matching_roles() {
        let provider = cache_capable_provider();
        let mut request = make_request(CacheRetention::Short);
        let messages = vec![
            Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "cache this user text".into(),
                }],
                timestamp_ms: 0,
            }),
            Message::User(UserMessage {
                content: vec![InputContent::Image {
                    source: ImageSource::Url {
                        url: "https://example.test/image.png".into(),
                    },
                    media_type: MediaType::Png,
                }],
                timestamp_ms: 0,
            }),
            test_assistant_msg(vec![AssistantContent::ToolCall {
                tool_call: ToolCall {
                    id: "tool-1".into(),
                    name: "inspect".into(),
                    arguments: "{}".into(),
                    arguments_error: None,
                    signature: None,
                },
            }]),
            Message::ToolResult(ToolResultMessage {
                not_executed: None,
                tool_call_id: "tool-1".into(),
                tool_name: "inspect".into(),
                content: vec![OutputContent::Image {
                    source: ImageSource::Bytes {
                        data: vec![1, 2, 3],
                    },
                    media_type: MediaType::Png,
                }],
                details: None,
                is_error: false,
                truncated: false,
                timestamp_ms: 0,
            }),
            test_assistant_msg(vec![AssistantContent::Text {
                text: "cache this assistant text".into(),
                signature: None,
            }]),
            test_assistant_msg(vec![AssistantContent::ToolCall {
                tool_call: ToolCall {
                    id: "tool-2".into(),
                    name: "read".into(),
                    arguments: "{}".into(),
                    arguments_error: None,
                    signature: None,
                },
            }]),
            test_assistant_msg(vec![AssistantContent::Thinking {
                thinking: "textless for cache marker purposes".into(),
                signature: None,
            }]),
        ];
        request.context = crate::transcript::normalize_context(crate::transcript::Context {
            messages,
            ..crate::transcript::Context::default()
        })
        .expect("valid context declarations");

        let body = provider.build_request_body(&request).unwrap();
        assert_eq!(
            body["messages"][0]["content"][0]["cache_control"]["type"],
            "ephemeral"
        );
        // Last assistant message carrying a text block is the plain-text
        // turn; textless tool-call/thinking turns stay unmarked.
        assert_eq!(
            body["messages"][4]["content"][0]["cache_control"]["type"],
            "ephemeral"
        );
        for message_index in [1, 2, 3, 5, 6] {
            for block in body["messages"][message_index]["content"]
                .as_array()
                .unwrap()
            {
                assert!(
                    block.get("cache_control").is_none(),
                    "textless trailing block at message {message_index} must not be marked"
                );
            }
        }
    }

    #[test]
    fn cache_control_unknown_model_emits_no_markers() {
        let provider = cache_capable_provider();
        let mut request = make_request(CacheRetention::Long);
        request.model = "anthropic:unknown-model".into();
        let body = provider.build_request_body(&request).unwrap();

        // Unknown model has no capabilities, so no cache markers
        assert!(body["system"].is_string());
    }
}

#[cfg(test)]
mod framing_tests {
    use super::drain_sse_events;

    #[test]
    fn invalid_utf8_byte_in_frame_is_replaced_and_frame_parses() {
        let mut frame: Vec<u8> = br#"event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"a"#
            .to_vec();
        frame.push(0x80);
        frame.extend_from_slice(
            br#"b"}}

"#,
        );
        let events = drain_sse_events(&mut frame, false).expect("frames parse with replacement");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], (super::ParsedEvent::Valid(_), Some(_))));
        assert!(frame.is_empty(), "complete frame drained");
    }
}
