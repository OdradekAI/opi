//! OpenAI Responses API SSE provider (S8.1).
//!
//! Implements streaming for the OpenAI Responses API (`/v1/responses`), which
//! uses standard SSE with `event:` + `data:` lines. The event types differ
//! significantly from Chat Completions: `response.created`,
//! `response.output_text.delta`, `response.function_call_arguments.delta`,
//! `response.completed`, etc.

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
use crate::provider::{CacheRetention, EventStream, ModelInfo, Provider, ProviderError, Request};
use crate::registry::ModelCapabilities;
use crate::stream::{AssistantStreamEvent, StopReason, Usage};

// ---------------------------------------------------------------------------
// SSE frame parser (Responses API uses standard SSE with event: lines)
// ---------------------------------------------------------------------------

/// A parsed SSE frame with both event type and data.
struct SseFrame {
    event: String,
    data: String,
}

/// Result of parsing a single SSE frame.
enum ParsedEvent {
    Valid(ResponsesEvent),
    Malformed { data: String, error: String },
}

/// Parse SSE text into frames, handling both event: and data: lines.
fn parse_sse_frames(input: &str) -> impl Iterator<Item = SseFrame> + '_ {
    let mut lines = input.split('\n').peekable();
    std::iter::from_fn(move || {
        let mut event_type = String::new();
        let mut data_parts: Vec<String> = Vec::new();

        loop {
            match lines.next() {
                Some(line) if line.starts_with(':') => continue,
                Some(line) if line.trim_end_matches('\r').is_empty() => {
                    if !data_parts.is_empty() {
                        return Some(SseFrame {
                            event: if event_type.is_empty() {
                                "message".into()
                            } else {
                                event_type
                            },
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
                        "event" => event_type = value.into(),
                        "data" => data_parts.push(value.into()),
                        _ => {}
                    }
                }
                None => {
                    if !data_parts.is_empty() {
                        return Some(SseFrame {
                            event: if event_type.is_empty() {
                                "message".into()
                            } else {
                                event_type
                            },
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
// Responses API raw wire types
// ---------------------------------------------------------------------------

/// Deserialized Responses API event data.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RawResponseEvent {
    r#type: String,
    response: Option<RawResponse>,
    output_index: Option<usize>,
    content_index: Option<usize>,
    item: Option<RawOutputItem>,
    part: Option<RawContentPart>,
    delta: Option<String>,
    item_id: Option<String>,
    call_id: Option<String>,
    name: Option<String>,
    arguments: Option<String>,
    text: Option<String>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RawResponse {
    id: Option<String>,
    status: Option<String>,
    model: Option<String>,
    output: Option<Vec<RawOutputItem>>,
    usage: Option<RawUsage>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RawOutputItem {
    r#type: Option<String>,
    id: Option<String>,
    call_id: Option<String>,
    name: Option<String>,
    arguments: Option<String>,
    role: Option<String>,
    content: Option<Vec<RawContentPart>>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RawContentPart {
    r#type: Option<String>,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawUsage {
    input_tokens: Option<u32>,
    output_tokens: Option<u32>,
    input_tokens_details: Option<RawInputTokenDetails>,
    #[serde(default)]
    output_tokens_details: Option<RawOutputTokenDetails>,
}

#[derive(Debug, Deserialize)]
struct RawInputTokenDetails {
    cached_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct RawOutputTokenDetails {
    #[serde(default)]
    reasoning_tokens: Option<u32>,
}

// ---------------------------------------------------------------------------
// Responses API event types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum ResponsesEvent {
    Created {
        model: Option<String>,
        id: Option<String>,
    },
    OutputItemAdded {
        output_index: usize,
        item: RawOutputItemOwned,
    },
    ContentPartAdded {
        output_index: usize,
        content_index: usize,
    },
    TextDelta {
        output_index: usize,
        content_index: usize,
        delta: String,
    },
    TextDone {
        output_index: usize,
        content_index: usize,
        text: String,
    },
    FunctionCallDelta {
        output_index: usize,
        delta: String,
    },
    OutputItemDone {
        output_index: usize,
        item: RawOutputItemOwned,
    },
    Completed {
        usage: Option<Usage>,
        model: Option<String>,
        id: Option<String>,
        output: Vec<RawOutputItemOwned>,
    },
    Error {
        message: String,
    },
}

/// Owned version of output item data for event storage.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct RawOutputItemOwned {
    item_type: String,
    id: Option<String>,
    call_id: Option<String>,
    name: Option<String>,
    arguments: Option<String>,
    role: Option<String>,
}

impl ResponsesEvent {
    fn try_from_frame(frame: &SseFrame) -> ParsedEvent {
        let data: RawResponseEvent = match serde_json::from_str(&frame.data) {
            Ok(d) => d,
            Err(e) => {
                return ParsedEvent::Malformed {
                    data: frame.data.clone(),
                    error: e.to_string(),
                };
            }
        };

        match frame.event.as_str() {
            "response.created" => {
                let model = data.response.as_ref().and_then(|r| r.model.clone());
                let id = data.response.as_ref().and_then(|r| r.id.clone());
                ParsedEvent::Valid(ResponsesEvent::Created { model, id })
            }
            "response.output_item.added" => {
                let output_index = data.output_index.unwrap_or(0);
                let item = match data.item {
                    Some(i) => i,
                    None => {
                        return ParsedEvent::Malformed {
                            data: frame.data.clone(),
                            error: "missing 'item' field in output_item.added".into(),
                        };
                    }
                };
                ParsedEvent::Valid(ResponsesEvent::OutputItemAdded {
                    output_index,
                    item: RawOutputItemOwned {
                        item_type: item.r#type.unwrap_or_default(),
                        id: item.id,
                        call_id: item.call_id,
                        name: item.name,
                        arguments: item.arguments,
                        role: item.role,
                    },
                })
            }
            "response.content_part.added" => ParsedEvent::Valid(ResponsesEvent::ContentPartAdded {
                output_index: data.output_index.unwrap_or(0),
                content_index: data.content_index.unwrap_or(0),
            }),
            "response.output_text.delta" => ParsedEvent::Valid(ResponsesEvent::TextDelta {
                output_index: data.output_index.unwrap_or(0),
                content_index: data.content_index.unwrap_or(0),
                delta: data.delta.unwrap_or_default(),
            }),
            "response.output_text.done" => ParsedEvent::Valid(ResponsesEvent::TextDone {
                output_index: data.output_index.unwrap_or(0),
                content_index: data.content_index.unwrap_or(0),
                text: data.text.unwrap_or_default(),
            }),
            "response.function_call_arguments.delta" => {
                ParsedEvent::Valid(ResponsesEvent::FunctionCallDelta {
                    output_index: data.output_index.unwrap_or(0),
                    delta: data.delta.unwrap_or_default(),
                })
            }
            "response.output_item.done" => {
                let output_index = data.output_index.unwrap_or(0);
                let item = data
                    .item
                    .map(|item| RawOutputItemOwned {
                        item_type: item.r#type.unwrap_or_default(),
                        id: item.id,
                        call_id: item.call_id,
                        name: item.name,
                        arguments: item.arguments,
                        role: item.role,
                    })
                    .unwrap_or_else(|| RawOutputItemOwned {
                        item_type: String::new(),
                        id: None,
                        call_id: None,
                        name: None,
                        arguments: None,
                        role: None,
                    });
                ParsedEvent::Valid(ResponsesEvent::OutputItemDone { output_index, item })
            }
            "response.completed" => {
                let usage = data.response.as_ref().and_then(|r| {
                    r.usage.as_ref().map(|u| {
                        let cached = u
                            .input_tokens_details
                            .as_ref()
                            .and_then(|d| d.cached_tokens)
                            .unwrap_or(0);
                        let output = u.output_tokens.unwrap_or(0);
                        let reasoning = u
                            .output_tokens_details
                            .as_ref()
                            .and_then(|d| d.reasoning_tokens)
                            .unwrap_or(0);
                        // Reject malformed subset: reasoning > output is invalid.
                        let reasoning = if reasoning > output { 0 } else { reasoning };
                        Usage::reported(
                            u.input_tokens.unwrap_or(0),
                            output,
                            cached,
                            0,
                            0, // cache_write_1h_tokens — Responses doesn't write cache
                            reasoning,
                        )
                    })
                });
                let model = data.response.as_ref().and_then(|r| r.model.clone());
                let id = data.response.as_ref().and_then(|r| r.id.clone());
                let output = data
                    .response
                    .as_ref()
                    .and_then(|r| r.output.as_ref())
                    .map(|items| {
                        items
                            .iter()
                            .map(|item| RawOutputItemOwned {
                                item_type: item.r#type.clone().unwrap_or_default(),
                                id: item.id.clone(),
                                call_id: item.call_id.clone(),
                                name: item.name.clone(),
                                arguments: item.arguments.clone(),
                                role: item.role.clone(),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                ParsedEvent::Valid(ResponsesEvent::Completed {
                    usage,
                    model,
                    id,
                    output,
                })
            }
            "error" => ParsedEvent::Valid(ResponsesEvent::Error {
                message: data.message.unwrap_or_else(|| "unknown error".into()),
            }),
            _ => ParsedEvent::Valid(ResponsesEvent::Error {
                message: format!("unknown event type: {}", frame.event),
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Stateful event mapper: ResponsesEvent -> AssistantStreamEvent
// ---------------------------------------------------------------------------

struct ToolCallState {
    output_index: usize,
    content_index: usize,
    id: String,
    name: String,
    arguments: String,
    ended: bool,
}

pub struct ResponsesMapper {
    partial: AssistantMessage,
    saw_done: bool,
    text_started: bool,
    text_content_index: Option<usize>,
    tool_calls: Vec<ToolCallState>,
}

impl ResponsesMapper {
    pub fn new(provider: &str) -> Self {
        Self {
            partial: empty_assistant_message(provider),
            saw_done: false,
            text_started: false,
            text_content_index: None,
            tool_calls: Vec::new(),
        }
    }

    fn process(&mut self, event: ResponsesEvent) -> Vec<AssistantStreamEvent> {
        if self.saw_done {
            return Vec::new();
        }
        match event {
            ResponsesEvent::Created { model, id } => {
                if let Some(m) = model {
                    self.partial.model = m;
                }
                if let Some(rid) = id {
                    self.partial.response_id = Some(rid);
                }
                vec![AssistantStreamEvent::Start {
                    partial: self.partial.clone(),
                }]
            }
            ResponsesEvent::OutputItemAdded { output_index, item } => {
                match item.item_type.as_str() {
                    "message" => Vec::new(),
                    "function_call" => {
                        let id = item.id.unwrap_or_default();
                        let call_id = item.call_id.unwrap_or_default();
                        let name = item.name.unwrap_or_default();
                        let arguments = item.arguments.unwrap_or_default();
                        // Use call_id as the ToolCall.id  - it's what function_call_output needs
                        let effective_id = if call_id.is_empty() {
                            id.clone()
                        } else {
                            call_id.clone()
                        };

                        // End any open text block
                        let mut events = Vec::new();
                        if self.text_started {
                            self.text_started = false;
                            let content_index = self
                                .text_content_index
                                .take()
                                .unwrap_or_else(|| self.partial.content.len().saturating_sub(1));
                            if let Some(AssistantContent::Text { text }) =
                                self.partial.content.get(content_index)
                            {
                                events.push(AssistantStreamEvent::TextEnd {
                                    content_index,
                                    content: text.clone(),
                                    partial: self.partial.clone(),
                                });
                            }
                        }

                        let content_index = self.partial.content.len();
                        self.partial.content.push(AssistantContent::ToolCall {
                            tool_call: ToolCall {
                                id: effective_id.clone(),
                                name: name.clone(),
                                arguments: arguments.clone(),
                            },
                        });

                        self.tool_calls.push(ToolCallState {
                            output_index,
                            content_index,
                            id: effective_id,
                            name: name.clone(),
                            arguments,
                            ended: false,
                        });

                        events.push(AssistantStreamEvent::ToolCallStart {
                            content_index,
                            partial: self.partial.clone(),
                        });
                        events
                    }
                    _ => Vec::new(),
                }
            }
            ResponsesEvent::ContentPartAdded { .. } => {
                // Content part added signals we're about to get text
                Vec::new()
            }
            ResponsesEvent::TextDelta { delta, .. } => {
                if delta.is_empty() {
                    return Vec::new();
                }
                let mut events = Vec::new();
                let content_index = if !self.text_started {
                    self.text_started = true;
                    let content_index = self.partial.content.len();
                    self.partial.content.push(AssistantContent::Text {
                        text: String::new(),
                    });
                    self.text_content_index = Some(content_index);
                    events.push(AssistantStreamEvent::TextStart {
                        content_index,
                        partial: self.partial.clone(),
                    });
                    content_index
                } else {
                    self.text_content_index
                        .unwrap_or_else(|| self.partial.content.len().saturating_sub(1))
                };
                if let Some(AssistantContent::Text { text }) =
                    self.partial.content.get_mut(content_index)
                {
                    text.push_str(&delta);
                }
                events.push(AssistantStreamEvent::TextDelta {
                    content_index,
                    delta,
                    partial: self.partial.clone(),
                });
                events
            }
            ResponsesEvent::TextDone { .. } => Vec::new(),
            ResponsesEvent::FunctionCallDelta {
                output_index,
                delta,
            } => {
                if delta.is_empty() {
                    return Vec::new();
                }

                let Some(tc_idx) = self
                    .tool_calls
                    .iter()
                    .position(|tc| tc.output_index == output_index)
                else {
                    return Vec::new();
                };

                self.tool_calls[tc_idx].arguments.push_str(&delta);
                let tool_content_index = self.tool_calls[tc_idx].content_index;
                if let Some(AssistantContent::ToolCall { tool_call }) =
                    self.partial.content.get_mut(tool_content_index)
                {
                    tool_call.arguments.push_str(&delta);
                }

                vec![AssistantStreamEvent::ToolCallDelta {
                    content_index: tool_content_index,
                    delta,
                    partial: self.partial.clone(),
                }]
            }
            ResponsesEvent::OutputItemDone { output_index, item } => {
                if item.item_type != "function_call" {
                    return Vec::new();
                }
                self.update_tool_call_from_item(output_index, &item);
                self.finish_tool_call(output_index).into_iter().collect()
            }
            ResponsesEvent::Completed {
                usage,
                model,
                id,
                output,
            } => {
                let mut events = Vec::new();

                // Close any open text block
                if self.text_started {
                    self.text_started = false;
                    let content_index = self
                        .text_content_index
                        .take()
                        .unwrap_or_else(|| self.partial.content.len().saturating_sub(1));
                    if let Some(AssistantContent::Text { text }) =
                        self.partial.content.get(content_index)
                    {
                        events.push(AssistantStreamEvent::TextEnd {
                            content_index,
                            content: text.clone(),
                            partial: self.partial.clone(),
                        });
                    }
                }

                for (output_index, item) in output.iter().enumerate() {
                    if item.item_type != "function_call" {
                        continue;
                    }
                    self.update_tool_call_from_item(output_index, item);
                }

                // Close any unclosed tool calls (safety for truncated streams)
                let unfinished_tool_calls: Vec<usize> = self
                    .tool_calls
                    .iter()
                    .filter(|tc| !tc.ended)
                    .map(|tc| tc.output_index)
                    .collect();
                for output_index in unfinished_tool_calls {
                    if let Some(event) = self.finish_tool_call(output_index) {
                        events.push(event);
                    }
                }

                if let Some(m) = model {
                    self.partial.model = m;
                }
                if let Some(rid) = id {
                    self.partial.response_id = Some(rid);
                }
                if let Some(u) = usage {
                    self.partial.usage = u;
                }

                // Determine stop reason from output content
                let has_tool_calls = self
                    .partial
                    .content
                    .iter()
                    .any(|c| matches!(c, AssistantContent::ToolCall { .. }));
                self.partial.stop_reason = if has_tool_calls {
                    StopReason::ToolUse
                } else {
                    StopReason::Stop
                };
                self.saw_done = true;

                events.push(AssistantStreamEvent::Done {
                    reason: self.partial.stop_reason,
                    message: self.partial.clone(),
                });
                events
            }
            ResponsesEvent::Error { message } => {
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

    fn finish_tool_call(&mut self, output_index: usize) -> Option<AssistantStreamEvent> {
        let tc_idx = self
            .tool_calls
            .iter()
            .position(|tc| tc.output_index == output_index)?;
        if self.tool_calls[tc_idx].ended {
            return None;
        }

        let (content_index, id, name, arguments) = {
            let tc = &mut self.tool_calls[tc_idx];
            tc.ended = true;
            (
                tc.content_index,
                tc.id.clone(),
                tc.name.clone(),
                tc.arguments.clone(),
            )
        };

        if let Some(AssistantContent::ToolCall { tool_call }) =
            self.partial.content.get_mut(content_index)
        {
            tool_call.arguments = arguments.clone();
        }

        Some(AssistantStreamEvent::ToolCallEnd {
            content_index,
            tool_call: ToolCall {
                id,
                name,
                arguments,
            },
            partial: self.partial.clone(),
        })
    }

    fn update_tool_call_from_item(&mut self, output_index: usize, item: &RawOutputItemOwned) {
        let Some(tc_idx) = self
            .tool_calls
            .iter()
            .position(|tc| tc.output_index == output_index && !tc.ended)
        else {
            return;
        };

        let (content_index, id, name, arguments) = {
            let tc = &mut self.tool_calls[tc_idx];
            if let Some(call_id) = item.call_id.as_ref()
                && !call_id.is_empty()
            {
                tc.id = call_id.clone();
            } else if let Some(id) = item.id.as_ref()
                && tc.id.is_empty()
            {
                tc.id = id.clone();
            }
            if let Some(name) = item.name.as_ref()
                && !name.is_empty()
            {
                tc.name = name.clone();
            }
            if let Some(arguments) = item.arguments.as_ref() {
                tc.arguments = arguments.clone();
            }
            (
                tc.content_index,
                tc.id.clone(),
                tc.name.clone(),
                tc.arguments.clone(),
            )
        };

        if let Some(AssistantContent::ToolCall { tool_call }) =
            self.partial.content.get_mut(content_index)
        {
            tool_call.id = id;
            tool_call.name = name;
            tool_call.arguments = arguments;
        }
    }
}

fn empty_assistant_message(provider: &str) -> AssistantMessage {
    AssistantMessage {
        content: Vec::new(),
        api: crate::ApiKind::OpenAi,
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
// OpenAiResponsesProvider
// ---------------------------------------------------------------------------

/// Native OpenAI Responses API profile flags (Phase 12 task 12.3).
///
/// These map directly onto Responses-native request fields that have no
/// Chat-Completions analogue. `previous_response_id` is intentionally absent:
/// opi's `Request` model carries no server-side response-chain state (the agent
/// runtime reconstructs context from the local message history), so it is
/// deferred to 12.9 as documented provider-correctness work.
#[derive(Debug, Clone)]
pub struct ResponsesConfig {
    /// Emit the top-level `store` field (server-side response retention).
    pub store: Option<bool>,
    /// Emit `{"reasoning":{"effort": ...}}` for reasoning models.
    pub reasoning_effort: Option<String>,
    /// Emit `"strict": true` on each function tool definition.
    pub strict_tools: bool,
    /// Request path appended to `base_url` (default `/v1/responses`). The Codex
    /// compatibility profile sets this to `/codex/responses`.
    pub responses_path: String,
    /// Derive the `chatgpt-account-id` header from the bearer token (a JWT) on
    /// each request. Set by the Codex profile; the standard OpenAI Responses
    /// profile leaves it false. The derivation is per-request because the token
    /// changes on refresh.
    pub derive_codex_account_id: bool,
    /// Emit the standard Responses `session_id` header together with
    /// `x-client-request-id`. Built-in direct Responses enables this; custom
    /// profiles can disable it, while Codex uses its separate spelling.
    pub send_session_id_header: bool,
}

impl Default for ResponsesConfig {
    fn default() -> Self {
        Self {
            store: None,
            reasoning_effort: None,
            strict_tools: false,
            responses_path: "/v1/responses".into(),
            derive_codex_account_id: false,
            send_session_id_header: true,
        }
    }
}

pub struct OpenAiResponsesProvider {
    auth: Arc<dyn AuthResolver>,
    base_url: String,
    models: Vec<ModelInfo>,
    config: ResponsesConfig,
    /// Provider id returned by `id()`. `"openai-responses"` for standard
    /// construction; the Codex compatibility profile sets `"codex"` so a revoked
    /// credential maps to `CredentialRevoked { provider_id: "codex" }` and
    /// `/login codex` remediation resolves.
    provider_id: String,
    /// Static per-request headers (Codex `OpenAI-Beta`, `originator`, `accept`).
    extra_headers: Vec<(String, String)>,
    client: Arc<HttpClient>,
}

impl OpenAiResponsesProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self::with_client(api_key, base_url, Arc::new(HttpClient::new()))
    }

    /// Create with a shared HTTP client.
    pub fn with_client(api_key: String, base_url: Option<String>, client: Arc<HttpClient>) -> Self {
        let auth = Arc::new(StaticAuthResolver::new(
            AuthScheme::ApiKey,
            SecretString::from(api_key),
        ));
        Self::with_auth(auth, base_url, ResponsesConfig::default(), client)
    }

    /// Create with native Responses profile flags.
    pub fn new_with_config(
        api_key: String,
        base_url: Option<String>,
        config: ResponsesConfig,
    ) -> Self {
        let auth = Arc::new(StaticAuthResolver::new(
            AuthScheme::ApiKey,
            SecretString::from(api_key),
        ));
        Self::with_auth(auth, base_url, config, Arc::new(HttpClient::new()))
    }

    /// Build with an injected per-request auth resolver (Phase 14.2). The
    /// resolver is consulted inside `Provider::stream` immediately before each
    /// HTTP request; `new`/`with_client`/`new_with_config` wrap a fixed key in
    /// a [`StaticAuthResolver`]. OAuth/env-backed resolution (Codex) is supplied
    /// through [`Self::with_auth_extra`] by `opi-coding-agent`. This entry point
    /// keeps the standard `"openai-responses"` id and no extra headers.
    pub fn with_auth(
        auth: Arc<dyn AuthResolver>,
        base_url: Option<String>,
        config: ResponsesConfig,
        client: Arc<HttpClient>,
    ) -> Self {
        Self::with_auth_extra(
            auth,
            base_url,
            config,
            "openai-responses".into(),
            vec![],
            client,
        )
    }

    /// Build with an injected auth resolver AND an explicit provider id + static
    /// headers (Phase 14.2 Codex profile). The Codex compatibility profile
    /// passes `provider_id = "codex"`, `responses_path = "/codex/responses"`,
    /// `derive_codex_account_id = true`, and the required Codex headers; the
    /// standard OpenAI Responses path uses [`Self::with_auth`].
    pub fn with_auth_extra(
        auth: Arc<dyn AuthResolver>,
        base_url: Option<String>,
        config: ResponsesConfig,
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
            config,
            provider_id,
            extra_headers,
            client,
        }
    }

    /// Access the shared HTTP client.
    pub fn http_client(&self) -> &Arc<HttpClient> {
        &self.client
    }

    /// Build the OpenAI Responses API request body.
    pub fn build_request_body(&self, request: &Request) -> serde_json::Value {
        let model_id = request
            .model
            .split_once(':')
            .map(|(_, id)| id)
            .unwrap_or(&request.model);

        let mut input = Vec::new();

        // User/assistant/tool messages
        for msg in &request.messages {
            match msg {
                crate::message::Message::User(u) => {
                    let content: Vec<serde_json::Value> = u
                        .content
                        .iter()
                        .map(|c| match c {
                            crate::message::InputContent::Text { text } => {
                                serde_json::json!({"type": "input_text", "text": text})
                            }
                            crate::message::InputContent::Image { source, media_type } => {
                                let image_url = match source {
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
                                    "type": "input_image",
                                    "image_url": image_url,
                                })
                            }
                        })
                        .collect();
                    if content.len() == 1
                        && let Some(text_val) = content[0].get("text")
                    {
                        input.push(serde_json::json!({
                            "role": "user",
                            "content": text_val,
                        }));
                        continue;
                    }
                    input.push(serde_json::json!({
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
                                tool_calls_json.push(serde_json::json!({
                                    "type": "function_call",
                                    "id": tool_call.id,
                                    "call_id": tool_call.id,
                                    "name": tool_call.name,
                                    "arguments": tool_call.arguments,
                                }));
                            }
                            AssistantContent::Thinking { .. } => {}
                        }
                    }
                    if !text_parts.is_empty() {
                        input.push(serde_json::json!({
                            "role": "assistant",
                            "content": text_parts.join(""),
                        }));
                    }
                    for tc in tool_calls_json {
                        input.push(tc);
                    }
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
                    // Phase 11.9: the Responses API does not accept a client-set
                    // status on input function_call_output items (status is
                    // server-managed on output items only), so prefix the same
                    // deterministic failure marker as Chat Completions; leave the
                    // success body byte-identical.
                    let output = if t.is_error {
                        format!("{TOOL_ERROR_MARKER}{content_text}")
                    } else {
                        content_text
                    };
                    input.push(serde_json::json!({
                        "type": "function_call_output",
                        "call_id": t.tool_call_id,
                        "output": output,
                    }));
                }
            }
        }

        let mut body = serde_json::json!({
            "model": model_id,
            "stream": true,
            "input": input,
        });

        // Responses API uses top-level "instructions" for system prompts
        if let Some(sys) = &request.system {
            body["instructions"] = serde_json::Value::String(sys.clone());
        }

        if let Some(max_tokens) = request.max_tokens {
            body["max_output_tokens"] = serde_json::Value::Number(max_tokens.into());
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
                        let mut tool = serde_json::json!({
                            "type": "function",
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.input_schema,
                        });
                        if self.config.strict_tools {
                            tool["strict"] = serde_json::Value::Bool(true);
                        }
                        tool
                    })
                    .collect(),
            );
        }

        // Native Responses profile flags (Phase 12 task 12.3).
        if let Some(store) = self.config.store {
            body["store"] = serde_json::Value::Bool(store);
        }
        if let Some(effort) = self.config.reasoning_effort.as_ref()
            && !effort.is_empty()
        {
            body["reasoning"] = serde_json::json!({ "effort": effort });
        }

        body
    }

    /// Stream events from a raw SSE response body.
    pub fn stream_from_sse(&self, sse_body: &str, cancel: CancellationToken) -> EventStream {
        let mut mapper = ResponsesMapper::new("openai-responses");
        let mut stream_events: Vec<Result<AssistantStreamEvent, ProviderError>> = Vec::new();

        for frame in parse_sse_frames(sse_body) {
            match ResponsesEvent::try_from_frame(&frame) {
                ParsedEvent::Valid(event) => {
                    stream_events.extend(mapper.process(event).into_iter().map(Ok));
                }
                ParsedEvent::Malformed { data, error } => {
                    stream_events.push(Err(ProviderError::StreamError(format!(
                        "malformed SSE data: {error} (data: {data:.80})"
                    ))));
                }
            }
        }

        let _cancel = cancel;
        Box::pin(stream::iter(stream_events))
    }

    /// Real HTTP streaming: POST to OpenAI Responses API.
    #[allow(clippy::too_many_arguments)]
    async fn stream_http(
        http_client: reqwest::Client,
        resolved: ResolvedAuth,
        base_url: String,
        config: ResponsesConfig,
        extra_headers: Vec<(String, String)>,
        provider_id: String,
        body: &serde_json::Value,
        cancel: CancellationToken,
        timeout: Option<std::time::Duration>,
        session_id: Option<String>,
        tx: &tokio::sync::mpsc::Sender<Result<AssistantStreamEvent, ProviderError>>,
    ) -> Result<(), ProviderError> {
        let url = crate::endpoint::join_endpoint(&base_url, &config.responses_path);
        let mut req = http_client
            .post(url)
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
        // Session-affinity headers use the profile-specific spelling. Standard
        // Responses sends `session_id`; Codex sends `session-id`.
        if let Some(ref sid) = session_id {
            if config.derive_codex_account_id {
                req = req.header("session-id", sid);
                req = req.header("x-client-request-id", sid);
            } else if config.send_session_id_header {
                req = req.header("session_id", sid);
                req = req.header("x-client-request-id", sid);
            }
        }
        if config.derive_codex_account_id
            && let Some(account_id) = derive_codex_account_id(&resolved.secret)
        {
            req = req.header("chatgpt-account-id", account_id);
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
        let mut mapper = ResponsesMapper::new("openai-responses");

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

            for frame in drain_sse_frames(&mut buffer) {
                match ResponsesEvent::try_from_frame(&frame) {
                    ParsedEvent::Valid(event) => {
                        for stream_event in mapper.process(event) {
                            if tx.send(Ok(stream_event)).await.is_err() {
                                return Ok(());
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

        if !mapper.saw_done {
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

/// Drain complete SSE frames from the buffer, leaving incomplete data for the
/// next chunk.
fn drain_sse_frames(buffer: &mut String) -> Vec<SseFrame> {
    if buffer.contains('\r') {
        *buffer = buffer.replace("\r\n", "\n").replace('\r', "\n");
    }

    let mut frames = Vec::new();
    while let Some(idx) = buffer.find("\n\n") {
        let end = idx + 2;
        let chunk: String = buffer.drain(..end).collect();
        frames.extend(parse_sse_frames(&chunk));
    }
    frames
}

/// Derive the `chatgpt-account-id` from a Codex access token (a JWT) for the
/// per-request `chatgpt-account-id` header. The Codex token's JWT payload
/// carries the account id under `https://api.openai.com/auth.chatgpt_account_id`.
///
/// Returns `None` on any decode failure (opaque token, non-JWT, malformed
/// base64, non-JSON payload, missing claim). The token and every JWT segment
/// are NEVER formatted into a `String` flowing to an error or header value, so
/// an undecodable token simply omits the header rather than leaking material.
fn derive_codex_account_id(secret: &secrecy::SecretString) -> Option<String> {
    use base64::Engine;
    let raw: &str = secret.expose_secret();
    // A JWT has three dot-separated base64url segments; the payload is middle.
    let mut parts = raw.split('.');
    let _header = parts.next()?;
    let payload = parts.next()?;
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(payload))
        .ok()?;
    let value: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    value
        .get("https://api.openai.com/auth")
        .and_then(|auth| auth.get("chatgpt_account_id"))
        .and_then(|v| v.as_str())
        .map(str::to_owned)
}

fn map_http_status(
    status: reqwest::StatusCode,
    body: &str,
    headers: &reqwest::header::HeaderMap,
    scheme: AuthScheme,
    provider_id: &str,
) -> ProviderError {
    match status.as_u16() {
        // A 401 on a Bearer (OAuth) credential — e.g. Codex — is typed
        // non-retryable CredentialRevoked with the body dropped (the body may
        // echo the submitted token). API-key Responses stays AuthFailed.
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

impl Provider for OpenAiResponsesProvider {
    fn stream(&self, request: Request) -> EventStream {
        let auth = self.auth.clone();
        let base_url = self.base_url.clone();
        let config = self.config.clone();
        let mut extra_headers = self.extra_headers.clone();
        extra_headers.extend(request.extra_headers.clone());
        let provider_id = self.provider_id.clone();
        let timeout = request.timeout;
        let session_id = if request.cache_retention != CacheRetention::Disabled {
            request.session_id.clone().filter(|id| !id.is_empty())
        } else {
            None
        };
        let mut body = self.build_request_body(&request);
        if !config.derive_codex_account_id
            && config.send_session_id_header
            && let Some(session_id) = session_id.as_deref()
        {
            body["prompt_cache_key"] =
                serde_json::Value::String(session_id.chars().take(64).collect());
        }
        let cancel = request.cancel.clone();
        let http_client = self.client.client().clone();

        let (tx, rx) = tokio::sync::mpsc::channel(64);

        tokio::spawn(async move {
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
                config,
                extra_headers,
                provider_id,
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
