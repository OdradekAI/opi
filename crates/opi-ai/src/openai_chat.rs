//! OpenAI-compatible chat completions SSE provider (S8.1).
//!
//! Implements streaming for OpenAI Chat Completions API, which uses `data: {...}`
//! SSE lines (no `event:` prefix). Exposes [`CompatConfig`] so downstream profiles
//! (OpenRouter, Mistral) can override role mapping, max_tokens field naming, etc.

use std::collections::HashMap;
use std::sync::Arc;

use futures_util::{StreamExt, stream};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use tokio_util::sync::CancellationToken;

use crate::auth::{AuthResolver, AuthScheme, ResolvedAuth, StaticAuthResolver};
use crate::http::HttpClient;
use crate::message::{
    AssistantContent, AssistantMessage, OutputContent, TOOL_ERROR_MARKER, ToolCall,
};
use crate::provider::{EventStream, ModelInfo, Provider, ProviderError, Request};
use crate::registry::ModelCapabilities;
use crate::stream::{AssistantStreamEvent, StopReason, Usage};

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
    Malformed { data: String, error: String },
}

/// Parse SSE text into frames, then deserialize each frame as an OpenAI chunk.
/// Returns [`ParsedEvent`] so callers can decide how to handle malformed data.
pub fn parse_sse_events(input: &str) -> impl Iterator<Item = ParsedEvent> + '_ {
    parse_frames(input).filter_map(|frame| {
        if frame.data == "[DONE]" {
            return None;
        }
        match serde_json::from_str::<OpenAiRawChunk>(&frame.data) {
            Ok(raw) => Some(ParsedEvent::Valid(OpenAiChatEvent::from_raw_vec(raw))),
            Err(e) => Some(ParsedEvent::Malformed {
                data: frame.data.clone(),
                error: e.to_string(),
            }),
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
    reasoning_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct RawError {
    message: Option<String>,
    #[allow(dead_code)]
    r#type: Option<String>,
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
                .and_then(|d| d.reasoning_tokens)
                .unwrap_or(0);
            // Reject malformed subset: reasoning > output is invalid.
            let reasoning = if reasoning > output { 0 } else { reasoning };
            Usage::reported(
                u.prompt_tokens.unwrap_or(0),
                output,
                cached,
                0,
                0, // cache_write_1h_tokens — Chat doesn't write cache
                reasoning,
            )
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
    pub fn new(api: crate::ApiKind, provider: &str) -> Self {
        Self {
            partial: empty_assistant_message(api, provider),
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
            self.partial.model = model;
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

    fn ensure_start(&mut self, events: &mut Vec<AssistantStreamEvent>) {
        if !self.started {
            self.started = true;
            events.push(AssistantStreamEvent::Start {
                partial: self.partial.clone(),
            });
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
    pub fn process(&mut self, event: OpenAiChatEvent) -> Vec<AssistantStreamEvent> {
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
                    events.push(AssistantStreamEvent::TextStart {
                        content_index: 0,
                        partial: self.partial.clone(),
                    });
                }
                if let Some(AssistantContent::Text { text }) = self.partial.content.last_mut() {
                    text.push_str(&content);
                }
                events.push(AssistantStreamEvent::TextDelta {
                    content_index: 0,
                    delta: content,
                    partial: self.partial.clone(),
                });
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
                        events.push(AssistantStreamEvent::TextEnd {
                            content_index: 0,
                            content,
                            partial: self.partial.clone(),
                        });
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
                    },
                });

                events.push(AssistantStreamEvent::ToolCallStart {
                    content_index,
                    partial: self.partial.clone(),
                });
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
                vec![AssistantStreamEvent::ToolCallDelta {
                    content_index: tool_content_index,
                    delta: arguments,
                    partial: self.partial.clone(),
                }]
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
                        events.push(AssistantStreamEvent::TextEnd {
                            content_index: 0,
                            content,
                            partial: self.partial.clone(),
                        });
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
                        if let Some(AssistantContent::ToolCall { tool_call }) =
                            self.partial.content.get_mut(tool_content_index)
                        {
                            tool_call.arguments = tc_state.arguments.clone();
                        }
                        let tool_call = ToolCall {
                            id: tc_state.id.clone(),
                            name: tc_state.name.clone(),
                            arguments: tc_state.arguments.clone(),
                        };
                        events.push(AssistantStreamEvent::ToolCallEnd {
                            content_index: tool_content_index,
                            tool_call,
                            partial: self.partial.clone(),
                        });
                    }
                }

                if is_metadata_only {
                    if let Some(done) = self.flush_pending_done() {
                        events.push(done);
                    } else {
                        events.push(self.emit_done(map_stop_reason(&finish_reason)));
                    }
                    return events;
                }

                let stop_reason = map_stop_reason(&finish_reason);
                if has_usage {
                    events.push(self.emit_done(stop_reason));
                } else {
                    self.pending_stop_reason = Some(stop_reason);
                }
                events
            }
            OpenAiChatEvent::Error { message } => {
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

fn map_stop_reason(raw: &str) -> StopReason {
    match raw {
        "stop" => StopReason::Stop,
        "length" => StopReason::Length,
        "tool_calls" => StopReason::ToolUse,
        "content_filter" => StopReason::Error,
        _ => StopReason::Error,
    }
}

fn empty_assistant_message(api: crate::ApiKind, provider: &str) -> AssistantMessage {
    AssistantMessage {
        content: Vec::new(),
        api,
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
/// - `reasoning_effort`: emit top-level `reasoning_effort` for reasoning models
/// - `cache_key`: emit `prompt_cache_key` for OpenAI prompt-cache affinity
/// - `require_assistant_after_tool_result`: compatibility-metadata flag (see below)
///
/// `require_assistant_after_tool_result` records that a profile targets an
/// endpoint whose legacy wire contract expected a synthetic assistant turn after
/// each tool result. Modern OpenAI Chat Completions does not require this, so the
/// shared adapter records the flag for compatibility metadata without altering
/// message ordering; an endpoint that materially requires the legacy synthesis
/// would need a reviewed first-class adapter (Phase 12 non-goal guard).
#[derive(Debug, Clone)]
pub struct CompatConfig {
    /// Override the role used for system messages (e.g. "developer" for o-series).
    pub system_role_override: Option<String>,
    /// JSON field name for max tokens in the request body.
    pub max_tokens_field: String,
    /// Whether tool result messages should include a "name" field.
    pub tool_result_name_field: bool,
    /// Whether usage data appears in stream chunks (not just the final one).
    pub usage_in_stream: bool,
    /// Emit `"strict": true` on each function tool definition.
    pub strict_tool_schema: bool,
    /// Emit a top-level `reasoning_effort` field (e.g. "low"/"medium"/"high").
    pub reasoning_effort: Option<String>,
    /// Emit a top-level `prompt_cache_key` for OpenAI prompt-cache affinity.
    pub cache_key: Option<String>,
    /// Compatibility-metadata flag for legacy assistant-after-tool-result wires.
    pub require_assistant_after_tool_result: bool,
    /// Endpoint path for chat completions relative to `base_url`.
    pub chat_completions_path: String,
}

impl Default for CompatConfig {
    fn default() -> Self {
        Self {
            system_role_override: None,
            max_tokens_field: "max_tokens".into(),
            tool_result_name_field: false,
            usage_in_stream: false,
            strict_tool_schema: false,
            reasoning_effort: None,
            cache_key: None,
            require_assistant_after_tool_result: false,
            chat_completions_path: "/v1/chat/completions".into(),
        }
    }
}

/// Per-model compat overrides (Phase 12 task 12.3).
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
    auth: Arc<dyn AuthResolver>,
    base_url: String,
    models: Vec<ModelInfo>,
    compat: CompatConfig,
    model_overrides: HashMap<String, ModelCompatOverride>,
    provider_id: String,
    extra_headers: Vec<(String, String)>,
    client: Arc<HttpClient>,
}

impl OpenAiChatProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self::new_with_compat(api_key, base_url, CompatConfig::default())
    }

    pub fn new_with_compat(
        api_key: String,
        base_url: Option<String>,
        compat: CompatConfig,
    ) -> Self {
        let auth = Arc::new(StaticAuthResolver::new(
            AuthScheme::ApiKey,
            SecretString::from(api_key),
        ));
        Self::with_auth(
            auth,
            base_url,
            compat,
            "openai".into(),
            vec![],
            Arc::new(HttpClient::new()),
        )
    }

    /// Create with a shared HTTP client.
    pub fn with_client(
        api_key: String,
        base_url: Option<String>,
        provider_id: String,
        extra_headers: Vec<(String, String)>,
        client: Arc<HttpClient>,
    ) -> Self {
        let auth = Arc::new(StaticAuthResolver::new(
            AuthScheme::ApiKey,
            SecretString::from(api_key),
        ));
        Self::with_auth(
            auth,
            base_url,
            CompatConfig::default(),
            provider_id,
            extra_headers,
            client,
        )
    }

    /// Build with an injected per-request auth resolver (Phase 14.2). The
    /// resolver is consulted inside `Provider::stream` immediately before each
    /// HTTP request; `new`/`new_with_compat`/`with_client` wrap a fixed key in
    /// a [`StaticAuthResolver`]. OAuth/env-backed resolution (Copilot) is
    /// supplied through this entry point by `opi-coding-agent`.
    pub fn with_auth(
        auth: Arc<dyn AuthResolver>,
        base_url: Option<String>,
        compat: CompatConfig,
        provider_id: String,
        extra_headers: Vec<(String, String)>,
        client: Arc<HttpClient>,
    ) -> Self {
        let base_url = base_url.unwrap_or_else(|| "https://api.openai.com".into());
        let models = vec![
            ModelInfo {
                id: "gpt-4o".into(),
                display_name: "GPT-4o".into(),
                capabilities: ModelCapabilities::new(128000, 16384)
                    .with_images(true)
                    .with_streaming(true),
            },
            ModelInfo {
                id: "gpt-4o-mini".into(),
                display_name: "GPT-4o Mini".into(),
                capabilities: ModelCapabilities::new(128000, 16384)
                    .with_images(true)
                    .with_streaming(true),
            },
            ModelInfo {
                id: "o3".into(),
                display_name: "o3".into(),
                capabilities: ModelCapabilities::new(200000, 100000)
                    .with_images(true)
                    .with_streaming(true),
            },
            ModelInfo {
                id: "o4-mini".into(),
                display_name: "o4-mini".into(),
                capabilities: ModelCapabilities::new(200000, 100000)
                    .with_images(true)
                    .with_streaming(true),
            },
        ];
        Self {
            auth,
            base_url,
            models,
            compat,
            model_overrides: HashMap::new(),
            provider_id,
            extra_headers,
            client,
        }
    }

    /// Create a provider for an OpenAI-compatible profile (OpenRouter, Mistral, etc.).
    pub fn new_for_profile(
        api_key: String,
        base_url: String,
        provider_id: String,
        compat: CompatConfig,
        extra_headers: Vec<(String, String)>,
        models: Vec<ModelInfo>,
    ) -> Self {
        let auth = Arc::new(StaticAuthResolver::new(
            AuthScheme::ApiKey,
            SecretString::from(api_key),
        ));
        Self {
            auth,
            base_url,
            models,
            compat,
            model_overrides: HashMap::new(),
            provider_id,
            extra_headers,
            client: Arc::new(HttpClient::new()),
        }
    }

    /// Replace the HTTP client with a shared one (for proxy configuration
    /// and connection pooling).
    pub fn with_shared_client(self, client: Arc<HttpClient>) -> Self {
        Self { client, ..self }
    }

    /// Attach per-model compat overrides (Phase 12 task 12.3). Model-level
    /// overrides win over the provider-level [`CompatConfig`] for the models
    /// they declare; models without an entry inherit the provider default.
    pub fn with_model_overrides(mut self, overrides: HashMap<String, ModelCompatOverride>) -> Self {
        self.model_overrides = overrides;
        self
    }

    /// Resolve the effective [`CompatConfig`] for a request's model, layering
    /// any model-level override on top of the provider-level default.
    fn resolve_compat(&self, model_id: &str) -> CompatConfig {
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
    pub fn build_request_body(&self, request: &Request) -> serde_json::Value {
        let model_id = request
            .model
            .split_once(':')
            .map(|(_, id)| id)
            .unwrap_or(&request.model);

        // Resolve the effective compat config for this model (model-level
        // overrides win over the provider-level default).
        let compat = self.resolve_compat(model_id);

        let mut body = serde_json::json!({
            "model": model_id,
            "stream": true,
            "messages": serialize_messages(&request.messages, &request.system, &compat),
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
        if !request.tools.is_empty() {
            body["tools"] = serde_json::Value::Array(
                request
                    .tools
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
        if let Some(effort) = compat.reasoning_effort.as_ref()
            && !effort.is_empty()
        {
            body["reasoning_effort"] = serde_json::Value::String(effort.clone());
        }
        if let Some(cache_key) = compat.cache_key.as_ref()
            && !cache_key.is_empty()
        {
            body["prompt_cache_key"] = serde_json::Value::String(cache_key.clone());
        }
        body
    }

    /// Stream events from a raw SSE response body.
    pub fn stream_from_sse(&self, sse_body: &str, cancel: CancellationToken) -> EventStream {
        let mut mapper = OpenAiChatMapper::new(crate::ApiKind::OpenAi, &self.provider_id);
        let mut stream_events: Vec<Result<AssistantStreamEvent, ProviderError>> = Vec::new();
        for parsed in parse_sse_events(sse_body) {
            match parsed {
                ParsedEvent::Valid(events) => {
                    for event in events {
                        stream_events.extend(mapper.process(event).into_iter().map(Ok));
                    }
                }
                ParsedEvent::Malformed { data, error } => {
                    stream_events.push(Err(ProviderError::StreamError(format!(
                        "malformed SSE data: {error} (data: {data:.80})"
                    ))));
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
        chat_completions_path: String,
        extra_headers: Vec<(String, String)>,
        body: &serde_json::Value,
        cancel: CancellationToken,
        timeout: Option<std::time::Duration>,
        session_id: Option<String>,
        tx: &tokio::sync::mpsc::Sender<Result<AssistantStreamEvent, ProviderError>>,
    ) -> Result<(), ProviderError> {
        let mut req = http_client
            .post(crate::endpoint::join_endpoint(
                &base_url,
                &chat_completions_path,
            ))
            .header(
                "authorization",
                format!("Bearer {}", resolved.secret.expose_secret()),
            )
            .header("content-type", "application/json");
        // Apply per-request timeout.
        if let Some(d) = timeout {
            req = req.timeout(d);
        }
        for (name, value) in &extra_headers {
            req = req.header(name.as_str(), value.as_str());
        }
        // Map session_id to prompt_cache_key (64-char clamped).
        if let Some(ref sid) = session_id {
            let clamped: String = sid.chars().take(64).collect();
            req = req.header("prompt-cache-key", clamped);
        }
        let response = req
            .body(serde_json::to_string(body).unwrap_or_default())
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
                &provider_id,
            ));
        }

        let mut byte_stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut mapper = OpenAiChatMapper::new(crate::ApiKind::OpenAi, &provider_id);

        loop {
            let chunk = tokio::select! {
                _ = cancel.cancelled() => {
                    return Ok(());
                }
                chunk = byte_stream.next() => {
                    match chunk {
                        Some(c) => c,
                        None => break,
                    }
                }
            };

            let chunk = chunk.map_err(|e| ProviderError::StreamError(e.to_string()))?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            for parsed in drain_sse_events(&mut buffer) {
                match parsed {
                    ParsedEvent::Valid(events) => {
                        for event in events {
                            for stream_event in mapper.process(event) {
                                if tx.send(Ok(stream_event)).await.is_err() {
                                    return Ok(());
                                }
                            }
                        }
                    }
                    ParsedEvent::Malformed { data, error } => {
                        let err = ProviderError::StreamError(format!(
                            "malformed SSE data: {error} (data: {data:.80})"
                        ));
                        if tx.send(Err(err)).await.is_err() {
                            return Ok(());
                        }
                    }
                }
            }
        }

        if let Some(done) = mapper.flush_pending_done() {
            let _ = tx.send(Ok(done)).await;
        } else if !mapper.saw_done {
            let err = ProviderError::StreamError("stream ended without a terminal event".into());
            let _ = tx.send(Err(err)).await;
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Streaming helpers
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

fn drain_sse_events(buffer: &mut String) -> Vec<ParsedEvent> {
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

fn map_http_status(
    status: reqwest::StatusCode,
    body: &str,
    headers: &reqwest::header::HeaderMap,
    scheme: AuthScheme,
    provider_id: &str,
) -> ProviderError {
    match status.as_u16() {
        // A 401 on a Bearer (OAuth) credential — e.g. Copilot — is typed
        // non-retryable CredentialRevoked with the body dropped (the body may
        // echo the submitted token). API-key profiles keep AuthFailed.
        401 if scheme == AuthScheme::Bearer => ProviderError::CredentialRevoked {
            provider_id: provider_id.to_owned(),
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

fn serialize_messages(
    messages: &[crate::message::Message],
    system: &Option<String>,
    compat: &CompatConfig,
) -> serde_json::Value {
    let mut result = Vec::new();

    // System message first
    if let Some(sys) = system {
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
                            let input: serde_json::Value =
                                serde_json::from_str(&tool_call.arguments)
                                    .ok()
                                    .filter(|v: &serde_json::Value| v.is_object())
                                    .unwrap_or(serde_json::json!({}));
                            tool_calls_json.push(serde_json::json!({
                                "id": tool_call.id,
                                "type": "function",
                                "function": {
                                    "name": tool_call.name,
                                    "arguments": serde_json::to_string(&input).unwrap_or_default(),
                                }
                            }));
                        }
                        AssistantContent::Thinking { .. } => {}
                    }
                }

                let mut assistant_msg = serde_json::json!({
                    "role": "assistant",
                });
                if !tool_calls_json.is_empty() {
                    assistant_msg["tool_calls"] = serde_json::Value::Array(tool_calls_json);
                    assistant_msg["content"] = serde_json::Value::Null;
                } else {
                    assistant_msg["content"] = serde_json::Value::String(text_parts.join(""));
                }
                result.push(assistant_msg);
            }
            crate::message::Message::ToolResult(t) => {
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
                // Phase 11.9: the Chat Completions API has no native error field on a
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
    fn stream(&self, request: Request) -> EventStream {
        let auth = self.auth.clone();
        let base_url = self.base_url.clone();
        let provider_id = self.provider_id.clone();
        let chat_completions_path = self.compat.chat_completions_path.clone();
        // Merge static provider extra_headers with per-request extra_headers.
        let mut extra_headers = self.extra_headers.clone();
        extra_headers.extend(request.extra_headers.clone());
        let timeout = request.timeout;
        let session_id = request.session_id.clone();
        let body = self.build_request_body(&request);
        let cancel = request.cancel.clone();
        let http_client = self.client.client().clone();

        let (tx, rx) = tokio::sync::mpsc::channel(64);

        tokio::spawn(async move {
            // Validate merged extra headers before any network call.
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
            if let Err(e) = Self::stream_http(
                http_client,
                resolved,
                base_url,
                provider_id,
                chat_completions_path,
                extra_headers,
                &body,
                cancel,
                timeout,
                session_id,
                &tx,
            )
            .await
            {
                let _ = tx.send(Err(e)).await;
            }
        });

        Box::pin(ReceiverStream { rx })
    }

    fn id(&self) -> &str {
        &self.provider_id
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }
}
