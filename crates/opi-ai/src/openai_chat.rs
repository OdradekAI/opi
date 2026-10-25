//! OpenAI-compatible chat completions SSE provider (S8.1).
//!
//! Implements streaming for OpenAI Chat Completions API, which uses `data: {...}`
//! SSE lines (no `event:` prefix). Exposes [`CompatConfig`] so downstream profiles
//! (OpenRouter, Mistral) can override role mapping, max_tokens field naming, etc.

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
use crate::model_info::WireApi;
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

/// A raw SSE frame extracted from the byte stream.
struct SseFrame {
    data: String,
}

/// Parsed result for a single SSE frame.
pub enum ParsedEvent {
    Valid(Vec<OpenAiChatEvent>),
    UsageError(ProviderError),
    Malformed { data: String, error: String },
}

/// Parse SSE text into frames, then deserialize each frame as an OpenAI chunk.
/// Returns [`ParsedEvent`] so callers can decide how to handle malformed data.
pub fn parse_sse_events(input: &str) -> impl Iterator<Item = ParsedEvent> + '_ {
    parse_sse_events_with_raw(input).map(|(parsed, _raw)| parsed)
}

/// [`parse_sse_events`] paired with each processed frame's raw wire JSON
/// (task 20.51): the raw-event callback observes exactly these values, before
/// normalization. The `[DONE]` sentinel and malformed frames carry no raw
/// value.
pub fn parse_sse_events_with_raw(
    input: &str,
) -> impl Iterator<Item = (ParsedEvent, Option<serde_json::Value>)> + '_ {
    parse_frames(input).filter_map(|frame| {
        if frame.data == "[DONE]" {
            return None;
        }
        let raw = serde_json::from_str::<serde_json::Value>(&frame.data).ok();
        match serde_json::from_str::<OpenAiRawChunk>(&frame.data) {
            Ok(raw_chunk) => Some(match validate_usage_subset(&raw_chunk) {
                Ok(()) => (
                    ParsedEvent::Valid(OpenAiChatEvent::from_raw_vec(raw_chunk)),
                    raw,
                ),
                Err(error) => (ParsedEvent::UsageError(error), raw),
            }),
            Err(e) => Some((
                ParsedEvent::Malformed {
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
}

#[derive(Debug, Deserialize)]
struct RawDelta {
    role: Option<String>,
    content: Option<String>,
    tool_calls: Option<Vec<RawToolCall>>,
}

#[derive(Debug, Deserialize)]
struct RawToolCall {
    index: usize,
    id: Option<String>,
    #[allow(dead_code)]
    r#type: Option<String>,
    function: Option<RawFunction>,
}

#[derive(Debug, Deserialize)]
struct RawFunction {
    name: Option<String>,
    arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawUsage {
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
    #[allow(dead_code)]
    total_tokens: Option<u32>,
    prompt_tokens_details: Option<RawPromptTokenDetails>,
    #[serde(default)]
    completion_tokens_details: Option<RawCompletionTokenDetails>,
}

#[derive(Debug, Deserialize)]
struct RawPromptTokenDetails {
    cached_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
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
    let Some(usage) = raw.usage.as_ref() else {
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
    fn from_raw_vec(raw: OpenAiRawChunk) -> Vec<Self> {
        // Check for top-level error
        if let Some(err) = raw.error {
            return vec![OpenAiChatEvent::Error {
                message: err.message,
            }];
        }

        let usage = raw.usage.map(|u| {
            let cached = u
                .prompt_tokens_details
                .as_ref()
                .and_then(|d| d.cached_tokens)
                .unwrap_or(0);
            let output = u.completion_tokens.unwrap_or(0);
            let reasoning = u
                .completion_tokens_details
                .as_ref()
                .and_then(|d| d.reasoning_tokens);
            Usage::reported(
                u.prompt_tokens.unwrap_or(0),
                output,
                cached,
                0,
                None, // cache_write_1h_tokens — Chat doesn't write cache
                reasoning,
            )
            .with_total_tokens(u.total_tokens.map(u64::from))
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
            if let Some(reason) = choice.finish_reason {
                return vec![OpenAiChatEvent::Finish {
                    finish_reason: reason,
                    id: response_id,
                    model,
                    usage,
                }];
            }

            let delta = match choice.delta {
                Some(d) => d,
                None => return events,
            };

            // Check for tool calls first (they take priority over content)
            if let Some(tool_calls) = delta.tool_calls {
                for tc in tool_calls {
                    let func = tc.function.unwrap_or(RawFunction {
                        name: None,
                        arguments: None,
                    });

                    if let Some(id) = tc.id {
                        let name = func.name.unwrap_or_default();
                        events.push(OpenAiChatEvent::ToolCallStart {
                            index: tc.index,
                            id,
                            name,
                            response_id: response_id.clone(),
                            model: model.clone(),
                            usage: usage.clone(),
                        });
                        if let Some(arguments) = func.arguments
                            && !arguments.is_empty()
                        {
                            events.push(OpenAiChatEvent::ToolCallDelta {
                                index: tc.index,
                                arguments,
                                id: response_id.clone(),
                                model: model.clone(),
                                usage: usage.clone(),
                            });
                        }
                    } else {
                        let arguments = func.arguments.unwrap_or_default();
                        if !arguments.is_empty() {
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
                if !events.is_empty() {
                    return events;
                }
            }

            // Check for role in the first chunk
            if delta.role.is_some() {
                return vec![OpenAiChatEvent::RoleDelta {
                    role: delta.role,
                    model,
                    id: response_id,
                    usage,
                }];
            }

            // Text content delta
            let content = delta.content.unwrap_or_default();
            if !content.is_empty() {
                return vec![OpenAiChatEvent::ContentDelta {
                    content,
                    id: response_id,
                    model,
                    usage,
                }];
            }
        }

        events
    }
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
    pending_stop_reason: Option<StopReason>,
}

struct ToolCallState {
    id: String,
    name: String,
    arguments: String,
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
            pending_stop_reason: None,
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

    pub fn flush_pending_done(&mut self) -> Option<AssistantStreamEvent> {
        if self.saw_done {
            return None;
        }
        self.pending_stop_reason
            .take()
            .map(|reason| self.emit_done(reason))
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
                if !self.text_started {
                    self.text_started = true;
                    self.partial.content.push(AssistantContent::Text {
                        text: String::new(),
                    });
                    events.push(Ok(AssistantStreamEvent::TextStart {
                        content_index: 0,
                        partial: self.partial.clone(),
                    }));
                }
                if let Some(AssistantContent::Text { text }) = self.partial.content.last_mut() {
                    text.push_str(&content);
                }
                events.push(Ok(AssistantStreamEvent::TextDelta {
                    content_index: 0,
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
                // End any open text block before starting tool calls
                let mut events = Vec::new();
                self.ensure_start(&mut events);
                if self.text_started {
                    self.text_started = false;
                    if let Some(AssistantContent::Text { text }) = self.partial.content.last() {
                        let content = text.clone();
                        events.push(Ok(AssistantStreamEvent::TextEnd {
                            content_index: 0,
                            content,
                            partial: self.partial.clone(),
                        }));
                    }
                }

                // Ensure we have room for this tool call
                while self.tool_calls.len() <= index {
                    self.tool_calls.push(ToolCallState {
                        id: String::new(),
                        name: String::new(),
                        arguments: String::new(),
                    });
                }

                let content_index = self.partial.content.len();
                self.tool_calls[index] = ToolCallState {
                    id: id.clone(),
                    name: name.clone(),
                    arguments: String::new(),
                };
                self.partial.content.push(AssistantContent::ToolCall {
                    tool_call: ToolCall {
                        id,
                        name,
                        arguments: String::new(),
                        arguments_error: None,
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
                self.tool_calls[index].arguments.push_str(&arguments);
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
                let has_usage = usage.is_some();
                let is_metadata_only = finish_reason.is_empty();
                self.update_metadata(id, model, usage);
                let mut events = Vec::new();
                if !is_metadata_only {
                    self.ensure_start(&mut events);
                }

                // Close any open text block
                if !is_metadata_only && self.pending_stop_reason.is_none() && self.text_started {
                    self.text_started = false;
                    if let Some(AssistantContent::Text { text }) = self.partial.content.last() {
                        let content = text.clone();
                        events.push(Ok(AssistantStreamEvent::TextEnd {
                            content_index: 0,
                            content,
                            partial: self.partial.clone(),
                        }));
                    }
                }

                // Close any open tool calls
                if !is_metadata_only && self.pending_stop_reason.is_none() {
                    for (tc_idx, tc_state) in self.tool_calls.iter().enumerate() {
                        // Skip placeholder entries from reserved indices
                        if tc_state.id.is_empty() {
                            continue;
                        }
                        // Map tool index to content index
                        let mut tool_count = 0;
                        let tool_content_index = self
                            .partial
                            .content
                            .iter()
                            .position(|c| {
                                if matches!(c, AssistantContent::ToolCall { .. }) {
                                    if tool_count == tc_idx {
                                        return true;
                                    }
                                    tool_count += 1;
                                }
                                false
                            })
                            .unwrap_or(0);
                        // Final textual arguments are parsed once at finish
                        // (AIR-048); Mistral/Azure/OpenRouter inherit this
                        // site through the shared Chat mapper.
                        let (arguments, arguments_error) =
                            ToolCall::finalize_text_arguments(tc_state.arguments.clone());
                        if let Some(AssistantContent::ToolCall { tool_call }) =
                            self.partial.content.get_mut(tool_content_index)
                        {
                            tool_call.arguments = arguments.clone();
                            tool_call.arguments_error = arguments_error.clone();
                        }
                        let tool_call = ToolCall {
                            id: tc_state.id.clone(),
                            name: tc_state.name.clone(),
                            arguments,
                            arguments_error,
                        };
                        events.push(Ok(AssistantStreamEvent::ToolCallEnd {
                            content_index: tool_content_index,
                            tool_call,
                            partial: self.partial.clone(),
                        }));
                    }
                }

                if is_metadata_only {
                    if let Some(done) = self.flush_pending_done() {
                        events.push(Ok(done));
                    } else {
                        events.push(Ok(self.emit_done(map_stop_reason(&finish_reason))));
                    }
                    return events;
                }

                let stop_reason = map_stop_reason(&finish_reason);
                if has_usage {
                    events.push(Ok(self.emit_done(stop_reason)));
                } else {
                    self.pending_stop_reason = Some(stop_reason);
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
        content: Vec::new(),
        api,
        provider: provider.into(),
        model: model.to_owned(),
        response_model: None,
        wire: Some(wire),
        response_id: None,
        usage: Usage::default(),
        stop_reason: StopReason::Stop,
        error_message: None,
        timestamp_ms: crate::time::now_ms(),
    }
}

/// Neutral public text substituted for the raw upstream OpenAI Chat
/// `error.message`. A proxy may echo credential material or request fragments
/// in an error frame, so the raw value is never placed in a persisted session
/// or a public event. Mirrors the Anthropic path.
const OPENAI_CHAT_STREAM_ERROR: &str = "openai chat stream error";

// ---------------------------------------------------------------------------
// CompatConfig  - configuration points for OpenAI-compatible profiles
// ---------------------------------------------------------------------------

/// Configuration overrides for OpenAI-compatible provider profiles.
///
/// Downstream providers (OpenRouter, Mistral) can customize:
/// - `system_role_override`: use "developer" instead of "system" (o-series models)
/// - `max_tokens_field`: field name for token limit ("max_tokens" vs "max_completion_tokens")
/// - `tool_result_name_field`: whether tool results carry a "name" field
/// - `usage_in_stream`: whether usage appears in every chunk vs only the last
/// - `strict_tool_schema`: emit `"strict": true` on function tool definitions
/// - `reasoning_effort`: legacy profile metadata; request/model thinking maps
///   are authoritative for wire output
/// - `cache_key`: emit `prompt_cache_key` for OpenAI prompt-cache affinity
/// - `send_session_affinity_headers`: emit compatible session-affinity headers
/// - `require_assistant_after_tool_result`: compatibility-metadata flag (see below)
///
/// `require_assistant_after_tool_result` records that a profile targets an
/// endpoint whose legacy wire contract expected a synthetic assistant turn after
/// each tool result. Modern OpenAI Chat Completions does not require this, so the
/// shared adapter records the flag for compatibility metadata without altering
/// message ordering; an endpoint that materially requires the legacy synthesis
/// needs a dedicated first-class adapter.
pub use crate::model_info::OpenAiCompletionsCompat as CompatConfig;

/// Per-model compatibility overrides.
///
/// A model entry may override a subset of the provider-level [`CompatConfig`].
/// At request-build time the provider resolves the effective config by layering
/// a model's override on top of the provider-level default (model wins).
#[derive(Debug, Clone, Default)]
pub struct ModelCompatOverride {
    /// Override the system-role mapping for this model only.
    pub system_role_override: Option<String>,
    /// Override the max-tokens field name for this model only.
    pub max_tokens_field: Option<String>,
}

// ---------------------------------------------------------------------------
// OpenAiChatProvider
// ---------------------------------------------------------------------------

/// Concrete OpenAI Chat Completions API provider.
pub struct OpenAiChatProvider {
    base_url: String,
    models: Vec<ModelInfo>,
    compat: CompatConfig,
    model_overrides: HashMap<String, ModelCompatOverride>,
    provider_id: String,
    headers: ProviderHeaders,
    route_headers: Vec<(String, String)>,
    catalog_compat: bool,
    auth_invalid_policy: AuthInvalidPolicy,
    direct_prompt_cache_key: bool,
    copilot_initiator: bool,
    client: Arc<HttpClient>,
}

/// Built-in OpenAI Chat model metadata without credentials or HTTP construction.
pub fn model_catalog() -> Vec<ModelInfo> {
    vec![
        ModelInfo::new(
            "gpt-4o",
            "GPT-4o",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(128000, 16384)
                .with_images(true)
                .with_streaming(true),
        ),
        ModelInfo::new(
            "gpt-4o-mini",
            "GPT-4o Mini",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(128000, 16384)
                .with_images(true)
                .with_streaming(true),
        ),
        ModelInfo::new(
            "o3",
            "o3",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(200000, 100000)
                .with_images(true)
                .with_streaming(true)
                .with_thinking(true),
        ),
        ModelInfo::new(
            "o4-mini",
            "o4-mini",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(200000, 100000)
                .with_images(true)
                .with_streaming(true)
                .with_thinking(true),
        ),
    ]
}

impl OpenAiChatProvider {
    pub fn new(base_url: Option<String>) -> Self {
        Self::new_with_compat(base_url, CompatConfig::default())
    }

    pub fn new_with_compat(base_url: Option<String>, compat: CompatConfig) -> Self {
        Self::with_auth(
            base_url,
            compat,
            "openai".into(),
            vec![],
            Arc::new(HttpClient::new()),
        )
        .with_auth_invalid_policy(AuthInvalidPolicy::Static)
    }

    /// Create with a shared HTTP client.
    pub fn with_client(
        base_url: Option<String>,
        provider_id: String,
        extra_headers: Vec<(String, String)>,
        client: Arc<HttpClient>,
    ) -> Self {
        Self::with_auth(
            base_url,
            CompatConfig::default(),
            provider_id,
            extra_headers,
            client,
        )
        .with_auth_invalid_policy(AuthInvalidPolicy::Static)
    }

    /// Build with provider route config. The secret arrives per-call via
    /// [`crate::auth::ResolvedAuth`] on `Provider::stream_prepared`.
    pub fn with_auth(
        base_url: Option<String>,
        compat: CompatConfig,
        provider_id: String,
        extra_headers: Vec<(String, String)>,
        client: Arc<HttpClient>,
    ) -> Self {
        let base_url = base_url.unwrap_or_else(|| "https://api.openai.com".into());
        let models = model_catalog();
        let direct_prompt_cache_key = provider_id == "openai";
        Self {
            base_url,
            models,
            compat,
            model_overrides: HashMap::new(),
            provider_id,
            headers: ProviderHeaders::default(),
            route_headers: extra_headers,
            catalog_compat: false,
            auth_invalid_policy: AuthInvalidPolicy::CredentialManaged,
            direct_prompt_cache_key,
            copilot_initiator: false,
            client,
        }
    }

    /// Create a provider for an OpenAI-compatible profile (OpenRouter, Mistral, etc.).
    pub fn new_for_profile(
        base_url: String,
        provider_id: String,
        compat: CompatConfig,
        extra_headers: Vec<(String, String)>,
        models: Vec<ModelInfo>,
    ) -> Self {
        Self {
            base_url,
            models,
            compat,
            model_overrides: HashMap::new(),
            provider_id,
            headers: ProviderHeaders::default(),
            route_headers: extra_headers,
            catalog_compat: false,
            auth_invalid_policy: AuthInvalidPolicy::Static,
            direct_prompt_cache_key: false,
            copilot_initiator: false,
            client: Arc::new(HttpClient::new()),
        }
    }

    /// Build a Chat Completions route for a mapped provider.
    pub fn for_route(
        default_base_url: Option<String>,
        provider_id: String,
        headers: ProviderHeaders,
        models: Vec<ModelInfo>,
        client: Arc<HttpClient>,
    ) -> Self {
        Self {
            base_url: default_base_url.unwrap_or_else(|| "https://api.openai.com".into()),
            models,
            compat: CompatConfig::default(),
            model_overrides: HashMap::new(),
            provider_id,
            headers,
            route_headers: Vec::new(),
            catalog_compat: true,
            auth_invalid_policy: AuthInvalidPolicy::Static,
            direct_prompt_cache_key: false,
            copilot_initiator: false,
            client,
        }
    }

    /// Override how this route classifies provider 401/403 responses.
    pub fn with_auth_invalid_policy(mut self, policy: AuthInvalidPolicy) -> Self {
        self.auth_invalid_policy = policy;
        self
    }

    /// Enable the GitHub Copilot `X-Initiator` request header. The value is
    /// derived inside each stream from the last conversation message: user
    /// prompts map to `user`; assistant/tool continuations map to `agent`.
    pub fn with_copilot_initiator(mut self) -> Self {
        self.copilot_initiator = true;
        self
    }

    /// Replace the HTTP client with a shared one (for proxy configuration
    /// and connection pooling).
    pub fn with_shared_client(self, client: Arc<HttpClient>) -> Self {
        Self { client, ..self }
    }

    /// Attach per-model compatibility overrides. Model-level
    /// overrides win over the provider-level [`CompatConfig`] for the models
    /// they declare; models without an entry inherit the provider default.
    pub fn with_model_overrides(mut self, overrides: HashMap<String, ModelCompatOverride>) -> Self {
        self.model_overrides = overrides;
        self
    }

    /// Resolve the effective [`CompatConfig`] for a request's model, layering
    /// any model-level override on top of the provider-level default.
    fn resolve_compat(&self, model_id: &str) -> CompatConfig {
        if self.catalog_compat
            && let Some(ModelInfo {
                compat: crate::model_info::WireCompat::OpenAiCompletions(compat),
                ..
            }) = self.models.iter().find(|model| model.id == model_id)
        {
            return compat.clone();
        }
        let Some(ov) = self.model_overrides.get(model_id) else {
            return self.compat.clone();
        };
        let mut resolved = self.compat.clone();
        if let Some(role) = &ov.system_role_override {
            resolved.system_role_override = Some(role.clone());
        }
        if let Some(field) = &ov.max_tokens_field {
            resolved.max_tokens_field = field.clone();
        }
        resolved
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
        let model_id = request
            .model
            .split_once(':')
            .map(|(_, id)| id)
            .unwrap_or(&request.model);

        // Resolve the effective compat config for this model (model-level
        // overrides win over the provider-level default).
        let compat = self.resolve_compat(model_id);

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
                wire: crate::model_info::WireApi::OpenAiCompletions,
                provider: self.provider_id.clone(),
                model: model_id.to_owned(),
                supports_images: crate::replay::supports_images_in_catalog(&self.models, model_id),
            },
            crate::replay::ReplayProjection::NativeOrCollapse,
        )?;
        let mut body = serde_json::json!({
            "model": model_id,
            "stream": true,
            "messages": serialize_messages(
                &replay.messages,
                crate::replay::supports_images_in_catalog(&self.models, model_id),
                &system_prompt,
                &compat,
            ),
        });

        if compat.usage_in_stream {
            body["stream_options"] = serde_json::json!({ "include_usage": true });
        }

        if let Some(max_tokens) = request.max_tokens {
            body[&compat.max_tokens_field] = serde_json::Value::Number(max_tokens.into());
        }
        if let Some(temp) = request.temperature
            && let Some(n) = serde_json::Number::from_f64(temp)
        {
            body["temperature"] = serde_json::Value::Number(n);
        }
        if !current_tools.is_empty() {
            body["tools"] = serde_json::Value::Array(
                current_tools
                    .iter()
                    .map(|t| {
                        let mut function = serde_json::json!({
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.input_schema,
                        });
                        if compat.strict_tool_schema {
                            function["strict"] = serde_json::Value::Bool(true);
                        }
                        serde_json::json!({
                            "type": "function",
                            "function": function,
                        })
                    })
                    .collect(),
            );
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
        if request.thinking.enabled
            && let Some(model) = self.models.iter().find(|model| model.id == model_id)
            && let Ok(Some(effort)) = model.thinking_level_map.resolve(request.thinking.level)
        {
            body["reasoning_effort"] = serde_json::Value::String(effort);
        }
        if request.cache_retention != CacheRetention::Disabled {
            if let Some(cache_key) = compat.cache_key.as_ref()
                && !cache_key.is_empty()
            {
                body["prompt_cache_key"] = serde_json::Value::String(cache_key.clone());
            }
            if self.direct_prompt_cache_key
                && let Some(session_id) = request.session_id.as_deref()
                && !session_id.is_empty()
            {
                body["prompt_cache_key"] =
                    serde_json::Value::String(session_id.chars().take(64).collect());
            }
        }
        // Last so model and request sampling parameters override named
        // request fields (model base < effective level < request, per key).
        let effective_level = if request.thinking.enabled {
            request.thinking.level
        } else {
            crate::model_info::ThinkingLevel::None
        };
        if let Some(model) = self.models.iter().find(|model| model.id == model_id)
            && let Some(sampling) = crate::model_info::resolve_sampling_params(
                model,
                effective_level,
                &request.sampling_params,
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
        let mut stream_events: Vec<Result<AssistantStreamEvent, ProviderError>> = Vec::new();
        for (parsed, _raw) in parse_sse_events_with_raw(sse_body) {
            match parsed {
                ParsedEvent::Valid(events) => {
                    for event in events {
                        stream_events.extend(mapper.process(event));
                    }
                }
                ParsedEvent::UsageError(error) => {
                    stream_events.push(Err(mapper.failure(error)));
                    break;
                }
                ParsedEvent::Malformed { .. } => {
                    stream_events.push(Err(mapper.failure(ProviderError::StreamError(
                        ProviderErrorSummary::attested_static("malformed OpenAI Chat SSE frame"),
                    ))));
                    break;
                }
            }
        }
        if let Some(done) = mapper.flush_pending_done() {
            stream_events.push(Ok(done));
        }

        let _cancel = cancel;
        Box::pin(stream::iter(stream_events))
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
            AuthScheme::Bearer => format!("Bearer {secret}"),
            AuthScheme::ApiKey | AuthScheme::AwsSigV4(_) => {
                return Err(ProviderError::Config(
                    ProviderErrorSummary::attested_static(
                        "OpenAI Chat accepts only Bearer authentication",
                    ),
                ));
            }
        };
        let mut req = http_client
            .post(crate::endpoint::join_endpoint(
                &base_url,
                &chat_completions_path,
            ))
            .header("authorization", bearer)
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
        callbacks.notify_response(response.status().as_u16(), &observed_headers)?;

        let status = response.status();
        if !status.is_success() {
            let headers = response.headers().clone();
            return Err(map_http_status(
                status,
                &headers,
                auth_invalid_policy,
                &provider_id,
            ));
        }
        snapshot.received_response();

        let mut byte_stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut decoder = crate::text_decode::StreamTextDecoder::new();
        let mut mapper = OpenAiChatMapper::new(
            crate::ApiKind::OpenAi,
            &provider_id,
            crate::model_info::WireApi::OpenAiCompletions,
            &requested_model,
        );

        mapper.set_snapshot(snapshot);
        let result: Result<(), ProviderError> = async {
            loop {
                let chunk = tokio::select! {
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

                for (parsed, raw) in drain_sse_events(&mut buffer) {
                    // Raw observer first: strictly before normalization, in
                    // wire order (task 20.51); failure ends the call with the
                    // partial preserved.
                    if let Some(raw) = raw.as_ref()
                        && let Err(error) = callbacks.notify_raw(raw)
                    {
                        return Err(mapper.failure(error));
                    }
                    match parsed {
                        ParsedEvent::Valid(events) => {
                            for event in events {
                                for item in mapper.process(event) {
                                    if !send_or_cancel(&cancel, tx, item).await? {
                                        return Ok(());
                                    }
                                }
                            }
                        }
                        ParsedEvent::UsageError(error) => return Err(mapper.failure(error)),
                        ParsedEvent::Malformed { .. } => {
                            return Err(mapper.failure(ProviderError::StreamError(
                                ProviderErrorSummary::attested_static(
                                    "malformed OpenAI Chat SSE frame",
                                ),
                            )));
                        }
                    }
                }
            }

            let eof_tail = decoder.finish();
            buffer.push_str(&eof_tail);
            if let Some(done) = mapper.flush_pending_done() {
                let _ = send_or_cancel(&cancel, tx, Ok(done)).await;
            } else if !mapper.saw_done {
                let err = ProviderError::StreamError(ProviderErrorSummary::attested_static(
                    "stream ended without a terminal event",
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

fn drain_sse_events(buffer: &mut String) -> Vec<(ParsedEvent, Option<serde_json::Value>)> {
    if buffer.contains('\r') {
        *buffer = buffer.replace("\r\n", "\n").replace('\r', "\n");
    }

    let mut events = Vec::new();
    while let Some(idx) = buffer.find("\n\n") {
        let end = idx + 2;
        let chunk: String = buffer.drain(..end).collect();
        events.extend(parse_sse_events_with_raw(&chunk));
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

fn serialize_messages(
    messages: &[crate::message::Message],
    supports_images: bool,
    system_prompt: &str,
    compat: &CompatConfig,
) -> serde_json::Value {
    let mut result = Vec::new();

    // System message first
    if !system_prompt.is_empty() {
        let sys = system_prompt;
        let role = compat.system_role_override.as_deref().unwrap_or("system");
        result.push(serde_json::json!({
            "role": role,
            "content": sys,
        }));
    }

    for msg in messages {
        match msg {
            crate::message::Message::User(u) => {
                let content: Vec<serde_json::Value> = u
                    .content
                    .iter()
                    .map(|c| match c {
                        crate::message::InputContent::Text { text } => {
                            serde_json::json!({"type": "text", "text": text})
                        }
                        crate::message::InputContent::Image { source, media_type } => {
                            let url = match source {
                                crate::message::ImageSource::Url { url } => url.clone(),
                                crate::message::ImageSource::Base64 { data } => {
                                    format!("data:{};base64,{}", media_type.as_str(), data)
                                }
                                crate::message::ImageSource::Bytes { data } => {
                                    format!(
                                        "data:{};base64,{}",
                                        media_type.as_str(),
                                        base64::Engine::encode(
                                            &base64::engine::general_purpose::STANDARD,
                                            data,
                                        )
                                    )
                                }
                            };
                            serde_json::json!({
                                "type": "image_url",
                                "image_url": {"url": url}
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
                    continue;
                }
                result.push(serde_json::json!({
                    "role": "user",
                    "content": content,
                }));
            }
            crate::message::Message::Assistant(a) => {
                let mut tool_calls_json = Vec::new();
                let mut text_parts = Vec::new();

                for c in &a.content {
                    match c {
                        AssistantContent::Text { text } => {
                            text_parts.push(text.clone());
                        }
                        AssistantContent::ToolCall { tool_call } => {
                            // The wire's arguments field is a raw JSON string:
                            // replayed proposals ride their original text
                            // (malformed ones included), never a re-encoded
                            // or fabricated substitute (RP-08).
                            tool_calls_json.push(serde_json::json!({
                                "id": tool_call.id,
                                "type": "function",
                                "function": {
                                    "name": tool_call.name,
                                    "arguments": tool_call.arguments,
                                }
                            }));
                        }
                        AssistantContent::Thinking { .. }
                        | AssistantContent::RedactedThinking { .. } => {}
                    }
                }

                let mut assistant_msg = serde_json::json!({
                    "role": "assistant",
                });
                // Text produced in the same turn stays visible alongside
                // its tool calls (AIR-012/RP-03 representable-order rule);
                // a textless tool turn keeps `content: null`.
                let joined = text_parts.join("");
                if !tool_calls_json.is_empty() {
                    assistant_msg["tool_calls"] = serde_json::Value::Array(tool_calls_json);
                    assistant_msg["content"] = if joined.is_empty() {
                        serde_json::Value::Null
                    } else {
                        serde_json::Value::String(joined)
                    };
                } else {
                    assistant_msg["content"] = serde_json::Value::String(joined);
                }
                result.push(assistant_msg);
            }
            crate::message::Message::System(_) => {
                // Unreachable after collapse + without_initial_system_message.
                unreachable!("system messages are projected out of the body list")
            }
            crate::message::Message::ToolResult(t) => {
                let has_images = t
                    .content
                    .iter()
                    .any(|c| matches!(c, OutputContent::Image { .. }));
                if supports_images && has_images {
                    // Real image parts preserve the tool association, the
                    // in-result ordering and the failure flag (AIR-014);
                    // this wire has no native error field, so the marker
                    // rides the leading text part.
                    let mut parts: Vec<serde_json::Value> = Vec::new();
                    if t.is_error {
                        parts.push(serde_json::json!({
                            "type": "text",
                            "text": TOOL_ERROR_MARKER,
                        }));
                    }
                    for c in &t.content {
                        match c {
                            OutputContent::Text { text } => {
                                parts.push(serde_json::json!({
                                    "type": "text",
                                    "text": text,
                                }));
                            }
                            OutputContent::Image { source, media_type } => {
                                let url = match source {
                                    crate::message::ImageSource::Url { url } => url.clone(),
                                    crate::message::ImageSource::Base64 { data } => {
                                        format!("data:{};base64,{}", media_type.as_str(), data)
                                    }
                                    crate::message::ImageSource::Bytes { data } => {
                                        format!(
                                            "data:{};base64,{}",
                                            media_type.as_str(),
                                            base64::Engine::encode(
                                                &base64::engine::general_purpose::STANDARD,
                                                data,
                                            )
                                        )
                                    }
                                };
                                parts.push(serde_json::json!({
                                    "type": "image_url",
                                    "image_url": {"url": url},
                                }));
                            }
                        }
                    }
                    let mut tool_msg = serde_json::json!({
                        "role": "tool",
                        "tool_call_id": t.tool_call_id,
                        "content": parts,
                    });
                    if compat.tool_result_name_field {
                        tool_msg["name"] = serde_json::Value::String(t.tool_name.clone());
                    }
                    result.push(tool_msg);
                    continue;
                }
                let content_text: String = t
                    .content
                    .iter()
                    .map(|c| match c {
                        OutputContent::Text { text } => text.clone(),
                        OutputContent::Image { media_type, .. } => {
                            format!("[image: {}]", media_type.as_str())
                        }
                    })
                    .collect();
                // The Chat Completions API has no native error field on a
                // role:"tool" message, so prefix the deterministic failure marker when
                // the tool result is an error; leave the success body byte-identical.
                // Azure/OpenRouter/Mistral inherit this via the shared adapter.
                let content_text = if t.is_error {
                    format!("{TOOL_ERROR_MARKER}{content_text}")
                } else {
                    content_text
                };
                let mut tool_msg = serde_json::json!({
                    "role": "tool",
                    "tool_call_id": t.tool_call_id,
                    "content": content_text,
                });
                if compat.tool_result_name_field {
                    tool_msg["name"] = serde_json::Value::String(t.tool_name.clone());
                }
                result.push(tool_msg);
            }
        }
    }

    serde_json::Value::Array(result)
}

impl Provider for OpenAiChatProvider {
    fn stream_prepared(
        &self,
        request: Request,
        resolved: crate::auth::ResolvedAuth,
    ) -> EventStream {
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
        let model_base_url = model.and_then(|model| model.base_url.clone());
        let chat_completions_path = self.resolve_compat(model_id).chat_completions_path;
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
        if self.resolve_compat(model_id).send_session_affinity_headers
            && request.cache_retention != CacheRetention::Disabled
            && let Some(session_id) = request.session_id.as_deref()
            && !session_id.is_empty()
        {
            const AFFINITY_HEADERS: [&str; 3] =
                ["session_id", "x-client-request-id", "x-session-affinity"];
            extra_headers.retain(|(name, _)| {
                !AFFINITY_HEADERS
                    .iter()
                    .any(|managed| name.eq_ignore_ascii_case(managed))
            });
            let clamped: String = session_id.chars().take(64).collect();
            extra_headers.extend(
                AFFINITY_HEADERS
                    .into_iter()
                    .map(|name| (name.into(), clamped.clone())),
            );
        }
        let extra_headers = copilot_headers
            .and_then(|copilot_headers| {
                extra_headers.extend(copilot_headers);
                self.headers
                    .merge_request(&extra_headers, &request.extra_headers)
            })
            // Header transform (task 20.51): applied once to the merged
            // per-call set, revalidated against protected names.
            .and_then(|merged| request.callbacks.apply_header_transform(&merged));
        let timeout = request.timeout;
        let body = match self.build_request_body(&request) {
            Ok(body) => body,
            Err(error) => return Box::pin(stream::once(async move { Err(error) })),
        };
        // Payload callback (task 20.51): after sampling/compat/wire
        // projection, once per variant, frozen across ordinary retries.
        let body = match request
            .callbacks
            .apply_payload(crate::callbacks::ORDINARY_PAYLOAD_VARIANT, body)
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
        let http_client = self.client.client().clone();

        let snapshot = crate::stream::ProducerSnapshot::default();
        let producer_snapshot = snapshot.clone();
        let (tx, rx) = tokio::sync::mpsc::channel(64);
        let tx_closed = tx.clone();
        let producer_cancel = cancel.clone();
        let stream_cancel = cancel.clone();

        tokio::spawn(async move {
            let work = async {
                let extra_headers = match extra_headers {
                    Ok(headers) => headers,
                    Err(error) => {
                        let _ = send_or_cancel(&producer_cancel, &tx, Err(error)).await;
                        return;
                    }
                };
                let base_url = resolved
                    .base_url
                    .clone()
                    .or(model_base_url)
                    .unwrap_or(default_base_url);
                if let Err(e) = Self::stream_http(
                    http_client,
                    resolved,
                    base_url,
                    provider_id,
                    auth_invalid_policy,
                    chat_completions_path,
                    requested_model,
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
                _ = tx_closed.closed() => (),
                _ = work => {},
            }
        });

        Box::pin(CancelAwareReceiverStream::with_snapshot(
            rx,
            stream_cancel,
            snapshot,
        ))
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
