//! Wire-neutral OpenAI Responses message conversion and SSE event mapping.

use serde::Deserialize;

use crate::message::{
    AssistantContent, AssistantMessage, OutputContent, TOOL_ERROR_MARKER, ToolCall,
};
use crate::provider::Request;
use crate::stream::{AssistantStreamEvent, StopReason, Usage};

/// Neutral public text substituted for the raw upstream Responses
/// `error.message`. A proxy may echo credential material or request fragments
/// in an error frame, so the raw value is never placed in a persisted session
/// or a public event. Mirrors the dedicated Anthropic path.
const OPENAI_RESPONSES_STREAM_ERROR: &str = "openai responses stream error";

/// One parsed Server-Sent Events frame.
pub(crate) struct SseFrame {
    pub(crate) event: String,
    pub(crate) data: String,
}

/// Result of decoding one Responses frame.
pub(crate) enum ParsedEvent {
    Valid(ResponsesEvent),
    UsageError(String),
    Malformed {
        // Raw upstream frame/error detail is captured only to detect the
        // malformed condition; it is deliberately never propagated into a
        // public or persisted error message (C6 redaction).
        #[expect(dead_code)]
        data: String,
        #[expect(dead_code)]
        error: String,
    },
}

/// Parse complete SSE text into frames.
pub(crate) fn parse_sse_frames(input: &str) -> impl Iterator<Item = SseFrame> + '_ {
    let mut lines = input.split('\n').peekable();
    std::iter::from_fn(move || {
        let mut event = String::new();
        let mut data: Vec<String> = Vec::new();
        loop {
            match lines.next() {
                Some(line) if line.starts_with(':') => continue,
                Some(line) if line.trim_end_matches('\r').is_empty() => {
                    if !data.is_empty() {
                        return Some(SseFrame {
                            event: if event.is_empty() {
                                "message".into()
                            } else {
                                event
                            },
                            data: data.join("\n"),
                        });
                    }
                }
                Some(line) => {
                    let line = line.trim_end_matches('\r');
                    let (field, value) = if let Some(index) = line.find(':') {
                        let value = if line.get(index + 1..index + 2) == Some(" ") {
                            &line[index + 2..]
                        } else {
                            &line[index + 1..]
                        };
                        (&line[..index], value)
                    } else {
                        (line, "")
                    };
                    match field {
                        "event" => event = value.into(),
                        "data" => data.push(value.into()),
                        _ => {}
                    }
                }
                None => {
                    if data.is_empty() {
                        return None;
                    }
                    return Some(SseFrame {
                        event: if event.is_empty() {
                            "message".into()
                        } else {
                            event
                        },
                        data: data.join("\n"),
                    });
                }
            }
        }
    })
}

#[derive(Debug, Deserialize)]
struct RawResponseEvent {
    // Canonical Responses/Codex SSE is data-only: the frame type lives in the
    // JSON `type` field rather than the SSE `event:` line.
    r#type: Option<String>,
    response: Option<RawResponse>,
    output_index: Option<usize>,
    content_index: Option<usize>,
    item: Option<RawOutputItem>,
    delta: Option<String>,
    text: Option<String>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawResponse {
    id: Option<String>,
    model: Option<String>,
    output: Option<Vec<RawOutputItem>>,
    usage: Option<RawUsage>,
}

#[derive(Debug, Deserialize)]
struct RawOutputItem {
    r#type: Option<String>,
    id: Option<String>,
    call_id: Option<String>,
    name: Option<String>,
    arguments: Option<String>,
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
    reasoning_tokens: Option<u64>,
}

#[derive(Debug, Clone)]
pub(crate) enum ResponsesEvent {
    Created {
        model: Option<String>,
        id: Option<String>,
    },
    OutputItemAdded {
        output_index: usize,
        item: RawOutputItemOwned,
    },
    ContentPartAdded,
    TextDelta {
        delta: String,
    },
    TextDone,
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
    Incomplete {
        usage: Option<Usage>,
    },
    /// A benign or unrecognized frame that must not advance or terminate the
    /// stream: lifecycle events, reasoning events, function-call finalization,
    /// the `[DONE]` sentinel, and future protocol extensions.
    Ignore,
    Error {
        // Raw upstream error text is captured only to classify the event; it
        // is deliberately never propagated (C6 redaction).
        #[expect(dead_code)]
        message: String,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct RawOutputItemOwned {
    item_type: String,
    id: Option<String>,
    call_id: Option<String>,
    name: Option<String>,
    arguments: Option<String>,
}

fn owned_item(item: RawOutputItem) -> RawOutputItemOwned {
    RawOutputItemOwned {
        item_type: item.r#type.unwrap_or_default(),
        id: item.id,
        call_id: item.call_id,
        name: item.name,
        arguments: item.arguments,
    }
}

impl ResponsesEvent {
    pub(crate) fn try_from_frame(frame: &SseFrame) -> ParsedEvent {
        // Real Responses/Codex streams terminate with the `data: [DONE]`
        // sentinel, which carries no JSON payload and is redundant with the
        // `response.completed`/`response.incomplete` terminal frame. Treat it
        // as a no-op rather than a malformed frame.
        if frame.data.trim() == "[DONE]" {
            return ParsedEvent::Valid(Self::Ignore);
        }
        let data: RawResponseEvent = match serde_json::from_str(&frame.data) {
            Ok(data) => data,
            Err(error) => {
                return ParsedEvent::Malformed {
                    data: frame.data.clone(),
                    error: error.to_string(),
                };
            }
        };
        // Canonical Responses/Codex SSE is data-only (the event type lives in
        // the JSON `type` field). Prefer it and fall back to the SSE `event:`
        // name only when the server emits one. The parser defaults an absent
        // name to "message", which is never a real Responses event, so a
        // typeless data-only frame falls through to the ignore arm below.
        let event_name: &str = data
            .r#type
            .as_deref()
            .filter(|name| !name.is_empty())
            .unwrap_or(frame.event.as_str());
        match event_name {
            "response.created" => ParsedEvent::Valid(Self::Created {
                model: data
                    .response
                    .as_ref()
                    .and_then(|response| response.model.clone()),
                id: data
                    .response
                    .as_ref()
                    .and_then(|response| response.id.clone()),
            }),
            "response.output_item.added" => match data.item {
                Some(item) => ParsedEvent::Valid(Self::OutputItemAdded {
                    output_index: data.output_index.unwrap_or(0),
                    item: owned_item(item),
                }),
                None => ParsedEvent::Malformed {
                    data: frame.data.clone(),
                    error: "missing 'item' field in output_item.added".into(),
                },
            },
            "response.content_part.added" => {
                let _ = (data.output_index, data.content_index);
                ParsedEvent::Valid(Self::ContentPartAdded)
            }
            "response.output_text.delta" => {
                let _ = (data.output_index, data.content_index);
                ParsedEvent::Valid(Self::TextDelta {
                    delta: data.delta.unwrap_or_default(),
                })
            }
            "response.output_text.done" => {
                let _ = (data.output_index, data.content_index, data.text);
                ParsedEvent::Valid(Self::TextDone)
            }
            "response.function_call_arguments.delta" => {
                ParsedEvent::Valid(Self::FunctionCallDelta {
                    output_index: data.output_index.unwrap_or(0),
                    delta: data.delta.unwrap_or_default(),
                })
            }
            "response.output_item.done" => ParsedEvent::Valid(Self::OutputItemDone {
                output_index: data.output_index.unwrap_or(0),
                item: data.item.map(owned_item).unwrap_or(RawOutputItemOwned {
                    item_type: String::new(),
                    id: None,
                    call_id: None,
                    name: None,
                    arguments: None,
                }),
            }),
            "response.completed" => {
                let usage = match Self::parse_response_usage(&data) {
                    Ok(usage) => usage,
                    Err(error) => {
                        return ParsedEvent::UsageError(error);
                    }
                };
                let model = data
                    .response
                    .as_ref()
                    .and_then(|response| response.model.clone());
                let id = data
                    .response
                    .as_ref()
                    .and_then(|response| response.id.clone());
                let output = data
                    .response
                    .and_then(|response| response.output)
                    .unwrap_or_default()
                    .into_iter()
                    .map(owned_item)
                    .collect();
                ParsedEvent::Valid(Self::Completed {
                    usage,
                    model,
                    id,
                    output,
                })
            }
            "response.incomplete" => {
                let usage = match Self::parse_response_usage(&data) {
                    Ok(usage) => usage,
                    Err(error) => {
                        return ParsedEvent::UsageError(error);
                    }
                };
                ParsedEvent::Valid(Self::Incomplete { usage })
            }
            "response.failed" => ParsedEvent::Valid(Self::Error {
                message: "response.failed".into(),
            }),
            "error" => ParsedEvent::Valid(Self::Error {
                message: data.message.unwrap_or_else(|| "unknown error".into()),
            }),
            // Benign lifecycle, reasoning, function-call-finalization, and any
            // future protocol-extension events must NOT terminate the stream.
            // Genuine errors arrive only via the `error` and `response.failed`
            // types above; an SSE `event:` name is never trusted as an error
            // signal on its own.
            _ => ParsedEvent::Valid(Self::Ignore),
        }
    }

    /// Extract and subset-validate `response.usage`. Returns `Err` with a
    /// redaction-safe literal message when the reasoning-token subset
    /// invariant trips, so the caller can surface the safe validation detail
    /// without treating it as malformed upstream data.
    fn parse_response_usage(data: &RawResponseEvent) -> Result<Option<Usage>, String> {
        let Some(usage) = data
            .response
            .as_ref()
            .and_then(|response| response.usage.as_ref())
        else {
            return Ok(None);
        };
        let output = u64::from(usage.output_tokens.unwrap_or(0));
        if let Some(reasoning) = usage
            .output_tokens_details
            .as_ref()
            .and_then(|details| details.reasoning_tokens)
            && reasoning > output
        {
            return Err(format!(
                "reasoning_tokens ({reasoning}) exceeds output_tokens ({output})"
            ));
        }
        let cached = usage
            .input_tokens_details
            .as_ref()
            .and_then(|details| details.cached_tokens)
            .unwrap_or(0);
        Ok(Some(Usage::reported(
            usage.input_tokens.unwrap_or(0),
            usage.output_tokens.unwrap_or(0),
            cached,
            0,
            None,
            usage
                .output_tokens_details
                .as_ref()
                .and_then(|details| details.reasoning_tokens),
        )))
    }
}

struct ToolCallState {
    output_index: usize,
    content_index: usize,
    id: String,
    name: String,
    arguments: String,
    ended: bool,
}

/// Stateful conversion from Responses events to opi stream events.
pub(crate) struct ResponsesMapper {
    partial: AssistantMessage,
    pub(crate) saw_done: bool,
    text_started: bool,
    text_content_index: Option<usize>,
    tool_calls: Vec<ToolCallState>,
}

impl ResponsesMapper {
    pub(crate) fn new(provider: &str) -> Self {
        Self {
            partial: empty_assistant_message(provider),
            saw_done: false,
            text_started: false,
            text_content_index: None,
            tool_calls: Vec::new(),
        }
    }

    pub(crate) fn process(&mut self, event: ResponsesEvent) -> Vec<AssistantStreamEvent> {
        if self.saw_done {
            return Vec::new();
        }
        match event {
            ResponsesEvent::Created { model, id } => {
                if let Some(model) = model {
                    self.partial.model = model;
                }
                if let Some(id) = id {
                    self.partial.response_id = Some(id);
                }
                vec![AssistantStreamEvent::Start {
                    partial: self.partial.clone(),
                }]
            }
            ResponsesEvent::OutputItemAdded { output_index, item } => {
                if item.item_type != "function_call" {
                    return Vec::new();
                }
                let mut events = self.end_text();
                let id = match (item.call_id, item.id) {
                    (Some(call_id), _) if !call_id.is_empty() => call_id,
                    (_, Some(id)) => id,
                    _ => String::new(),
                };
                let name = item.name.unwrap_or_default();
                let arguments = item.arguments.unwrap_or_default();
                let content_index = self.partial.content.len();
                self.partial.content.push(AssistantContent::ToolCall {
                    tool_call: ToolCall {
                        id: id.clone(),
                        name: name.clone(),
                        arguments: arguments.clone(),
                    },
                });
                self.tool_calls.push(ToolCallState {
                    output_index,
                    content_index,
                    id,
                    name,
                    arguments,
                    ended: false,
                });
                events.push(AssistantStreamEvent::ToolCallStart {
                    content_index,
                    partial: self.partial.clone(),
                });
                events
            }
            ResponsesEvent::ContentPartAdded
            | ResponsesEvent::TextDone
            | ResponsesEvent::Ignore => Vec::new(),
            ResponsesEvent::TextDelta { delta } => {
                if delta.is_empty() {
                    return Vec::new();
                }
                let mut events = Vec::new();
                let content_index = if self.text_started {
                    self.text_content_index
                        .unwrap_or_else(|| self.partial.content.len().saturating_sub(1))
                } else {
                    self.text_started = true;
                    let index = self.partial.content.len();
                    self.partial.content.push(AssistantContent::Text {
                        text: String::new(),
                    });
                    self.text_content_index = Some(index);
                    events.push(AssistantStreamEvent::TextStart {
                        content_index: index,
                        partial: self.partial.clone(),
                    });
                    index
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
            ResponsesEvent::FunctionCallDelta {
                output_index,
                delta,
            } => {
                if delta.is_empty() {
                    return Vec::new();
                }
                let Some(state) = self
                    .tool_calls
                    .iter_mut()
                    .find(|state| state.output_index == output_index)
                else {
                    return Vec::new();
                };
                state.arguments.push_str(&delta);
                let content_index = state.content_index;
                if let Some(AssistantContent::ToolCall { tool_call }) =
                    self.partial.content.get_mut(content_index)
                {
                    tool_call.arguments.push_str(&delta);
                }
                vec![AssistantStreamEvent::ToolCallDelta {
                    content_index,
                    delta,
                    partial: self.partial.clone(),
                }]
            }
            ResponsesEvent::OutputItemDone { output_index, item } => {
                if item.item_type != "function_call" {
                    return Vec::new();
                }
                self.update_tool_call(output_index, &item);
                self.finish_tool_call(output_index).into_iter().collect()
            }
            ResponsesEvent::Incomplete { usage } => {
                let mut events = self.end_text();
                if let Some(usage) = usage {
                    self.partial.usage = usage;
                }
                self.partial.stop_reason = StopReason::Length;
                self.saw_done = true;
                events.push(AssistantStreamEvent::Done {
                    reason: StopReason::Length,
                    message: self.partial.clone(),
                });
                events
            }
            ResponsesEvent::Completed {
                usage,
                model,
                id,
                output,
            } => {
                let mut events = self.end_text();
                for (output_index, item) in output.iter().enumerate() {
                    if item.item_type == "function_call" {
                        self.update_tool_call(output_index, item);
                    }
                }
                let unfinished: Vec<usize> = self
                    .tool_calls
                    .iter()
                    .filter(|state| !state.ended)
                    .map(|state| state.output_index)
                    .collect();
                for output_index in unfinished {
                    if let Some(event) = self.finish_tool_call(output_index) {
                        events.push(event);
                    }
                }
                if let Some(model) = model {
                    self.partial.model = model;
                }
                if let Some(id) = id {
                    self.partial.response_id = Some(id);
                }
                if let Some(usage) = usage {
                    self.partial.usage = usage;
                }
                self.partial.stop_reason = if self
                    .partial
                    .content
                    .iter()
                    .any(|content| matches!(content, AssistantContent::ToolCall { .. }))
                {
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
            ResponsesEvent::Error { message: _ } => {
                self.saw_done = true;
                let mut error_message = self.partial.clone();
                // Substitute a bounded, provider-neutral message: the raw
                // upstream `error.message` may echo credential material and is
                // never persisted or exposed publicly.
                error_message.error_message = Some(OPENAI_RESPONSES_STREAM_ERROR.to_owned());
                vec![AssistantStreamEvent::Error {
                    reason: StopReason::Error,
                    message: error_message,
                }]
            }
        }
    }

    fn end_text(&mut self) -> Vec<AssistantStreamEvent> {
        if !self.text_started {
            return Vec::new();
        }
        self.text_started = false;
        let content_index = self
            .text_content_index
            .take()
            .unwrap_or_else(|| self.partial.content.len().saturating_sub(1));
        match self.partial.content.get(content_index) {
            Some(AssistantContent::Text { text }) => vec![AssistantStreamEvent::TextEnd {
                content_index,
                content: text.clone(),
                partial: self.partial.clone(),
            }],
            _ => Vec::new(),
        }
    }

    fn finish_tool_call(&mut self, output_index: usize) -> Option<AssistantStreamEvent> {
        let state = self
            .tool_calls
            .iter_mut()
            .find(|state| state.output_index == output_index)?;
        if state.ended {
            return None;
        }
        state.ended = true;
        let content_index = state.content_index;
        let tool_call = ToolCall {
            id: state.id.clone(),
            name: state.name.clone(),
            arguments: state.arguments.clone(),
        };
        if let Some(AssistantContent::ToolCall { tool_call: partial }) =
            self.partial.content.get_mut(content_index)
        {
            partial.arguments = tool_call.arguments.clone();
        }
        Some(AssistantStreamEvent::ToolCallEnd {
            content_index,
            tool_call,
            partial: self.partial.clone(),
        })
    }

    fn update_tool_call(&mut self, output_index: usize, item: &RawOutputItemOwned) {
        let Some(state) = self
            .tool_calls
            .iter_mut()
            .find(|state| state.output_index == output_index && !state.ended)
        else {
            return;
        };
        if let Some(call_id) = item.call_id.as_ref()
            && !call_id.is_empty()
        {
            state.id = call_id.clone();
        } else if let Some(id) = item.id.as_ref()
            && state.id.is_empty()
        {
            state.id = id.clone();
        }
        if let Some(name) = item.name.as_ref()
            && !name.is_empty()
        {
            state.name = name.clone();
        }
        if let Some(arguments) = item.arguments.as_ref() {
            state.arguments = arguments.clone();
        }
        if let Some(AssistantContent::ToolCall { tool_call }) =
            self.partial.content.get_mut(state.content_index)
        {
            tool_call.id = state.id.clone();
            tool_call.name = state.name.clone();
            tool_call.arguments = state.arguments.clone();
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

/// Convert opi messages to standard Responses input items.
pub(crate) fn convert_messages(request: &Request) -> Vec<serde_json::Value> {
    let mut input = Vec::new();
    for message in &request.messages {
        match message {
            crate::message::Message::User(user) => {
                let content: Vec<serde_json::Value> = user
                    .content
                    .iter()
                    .map(|content| match content {
                        crate::message::InputContent::Text { text } => {
                            serde_json::json!({"type":"input_text","text":text})
                        }
                        crate::message::InputContent::Image { source, media_type } => {
                            let image_url = match source {
                                crate::message::ImageSource::Url { url } => url.clone(),
                                crate::message::ImageSource::Base64 { data } => {
                                    format!("data:{};base64,{data}", media_type.as_str())
                                }
                                crate::message::ImageSource::Bytes { data } => {
                                    use base64::Engine;
                                    format!(
                                        "data:{};base64,{}",
                                        media_type.as_str(),
                                        base64::engine::general_purpose::STANDARD.encode(data)
                                    )
                                }
                            };
                            serde_json::json!({"type":"input_image","image_url":image_url})
                        }
                    })
                    .collect();
                if content.len() == 1
                    && let Some(text) = content[0].get("text")
                {
                    input.push(serde_json::json!({"role":"user","content":text}));
                } else {
                    input.push(serde_json::json!({"role":"user","content":content}));
                }
            }
            crate::message::Message::Assistant(assistant) => {
                let mut text = String::new();
                let mut calls = Vec::new();
                for content in &assistant.content {
                    match content {
                        AssistantContent::Text { text: part } => text.push_str(part),
                        AssistantContent::ToolCall { tool_call } => {
                            calls.push(serde_json::json!({
                                "type":"function_call",
                                "id":tool_call.id,
                                "call_id":tool_call.id,
                                "name":tool_call.name,
                                "arguments":tool_call.arguments,
                            }));
                        }
                        AssistantContent::Thinking { .. } => {}
                    }
                }
                if !text.is_empty() {
                    input.push(serde_json::json!({"role":"assistant","content":text}));
                }
                input.extend(calls);
            }
            crate::message::Message::ToolResult(result) => {
                let content: String = result
                    .content
                    .iter()
                    .map(|content| match content {
                        OutputContent::Text { text } => text.clone(),
                        OutputContent::Image { media_type, .. } => {
                            format!("[image: {}]", media_type.as_str())
                        }
                    })
                    .collect();
                let output = if result.is_error {
                    format!("{TOOL_ERROR_MARKER}{content}")
                } else {
                    content
                };
                input.push(serde_json::json!({
                    "type":"function_call_output",
                    "call_id":result.tool_call_id,
                    "output":output,
                }));
            }
        }
    }
    input
}

/// Drain complete SSE frames while retaining an incomplete trailing frame.
pub(crate) fn drain_sse_frames(buffer: &mut String) -> Vec<SseFrame> {
    if buffer.contains('\r') {
        *buffer = buffer.replace("\r\n", "\n").replace('\r', "\n");
    }
    let mut frames = Vec::new();
    while let Some(index) = buffer.find("\n\n") {
        let chunk: String = buffer.drain(..index + 2).collect();
        frames.extend(parse_sse_frames(&chunk));
    }
    frames
}

#[cfg(test)]
mod tests {
    use super::drain_sse_frames;

    #[test]
    fn split_sse_frame_is_retained_until_the_blank_line_arrives() {
        let mut buffer =
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"Hel".to_owned();
        assert!(drain_sse_frames(&mut buffer).is_empty());
        assert!(!buffer.is_empty());

        buffer.push_str("lo\"}\n\n");
        let frames = drain_sse_frames(&mut buffer);

        assert_eq!(frames.len(), 1);
        assert_eq!(
            frames[0].data,
            r#"{"type":"response.output_text.delta","delta":"Hello"}"#
        );
        assert!(buffer.is_empty());
    }
}
