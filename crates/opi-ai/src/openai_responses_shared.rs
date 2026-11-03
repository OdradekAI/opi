//! Wire-neutral OpenAI Responses message conversion and SSE event mapping.

use serde::Deserialize;

use crate::message::{
    AssistantContent, AssistantMessage, OutputContent, TOOL_ERROR_MARKER, ToolCall,
};
use crate::provider::{ProviderError, ProviderErrorSummary};
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
    /// The final raw input of a grammar custom-tool call
    /// (`response.custom_tool_call_input.done`).
    #[serde(default)]
    input: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawResponse {
    id: Option<String>,
    model: Option<String>,
    output: Option<Vec<RawOutputItem>>,
    usage: Option<RawUsage>,
    service_tier: Option<String>,
    status: Option<String>,
    incomplete_details: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct RawOutputItem {
    r#type: Option<String>,
    id: Option<String>,
    call_id: Option<String>,
    name: Option<String>,
    arguments: Option<String>,
    /// The raw input string of a `custom_tool_call` item (grammar tools).
    #[serde(default)]
    input: Option<String>,
    /// Reasoning-item summary parts (task 20.37).
    #[serde(default)]
    summary: Option<Vec<RawSummaryPart>>,
    /// Reasoning-item raw content parts (task 20.37).
    #[serde(default)]
    content: Option<Vec<RawContentPart>>,
    /// Encrypted reasoning carried for stateless replay (task 20.37).
    #[serde(default)]
    encrypted_content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawSummaryPart {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawContentPart {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawUsage {
    input_tokens: Option<u32>,
    output_tokens: Option<u32>,
    total_tokens: Option<u64>,
    input_tokens_details: Option<RawInputTokenDetails>,
    #[serde(default)]
    output_tokens_details: Option<RawOutputTokenDetails>,
}

#[derive(Debug, Deserialize)]
struct RawInputTokenDetails {
    cached_tokens: Option<u32>,
    cache_write_tokens: Option<u32>,
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
    /// A grammar custom-tool input delta (task 20.11): the raw wire input
    /// grows by `delta`.
    CustomToolCallInputDelta {
        output_index: usize,
        delta: String,
    },
    /// Reasoning summary text grew by `delta` (task 20.37).
    ReasoningSummaryTextDelta {
        output_index: usize,
        delta: String,
    },
    /// Raw reasoning content text grew by `delta` (task 20.37).
    ReasoningTextDelta {
        output_index: usize,
        delta: String,
    },
    /// One reasoning summary part finished; parts join with a blank line
    /// (task 20.37).
    ReasoningSummaryPartDone {
        output_index: usize,
    },
    /// The final raw input of a grammar custom-tool call.
    CustomToolCallInputDone {
        output_index: usize,
        input: String,
    },
    OutputItemDone {
        output_index: usize,
        item: RawOutputItemOwned,
    },
    Completed {
        usage: Option<Usage>,
        raw_stop_reason: Option<String>,
        model: Option<String>,
        id: Option<String>,
        output: Vec<RawOutputItemOwned>,
    },
    Incomplete {
        usage: Option<Usage>,
        raw_stop_reason: Option<String>,
    },
    /// A benign or unrecognized frame that must not advance or terminate the
    /// stream: lifecycle events, reasoning events, function-call finalization,
    /// the `[DONE]` sentinel, and future protocol extensions.
    Ignore,
    Error {
        raw_stop_reason: Option<String>,
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
    input: Option<String>,
    /// Serialized reasoning item for signature replay: the id plus every
    /// captured reasoning field that was present on the wire (task 20.37).
    reasoning_signature: Option<String>,
    /// Raw encrypted reasoning payload (task 20.37): terminal-output
    /// backfill matches it onto reasoning blocks missing the field.
    encrypted_content: Option<String>,
}

/// Serialize the persisted reasoning item for signature replay (task
/// 20.37). The Rust signature is the deliberate subset a stateless replay
/// needs (id, summary, content, encrypted payload); unknown upstream
/// fields are not fabricated.
fn reasoning_signature(item: &RawOutputItem) -> Option<String> {
    if item.r#type.as_deref() != Some("reasoning") {
        return None;
    }
    let mut value = serde_json::json!({"type": "reasoning"});
    if let Some(id) = &item.id {
        value["id"] = serde_json::json!(id);
    }
    if let Some(encrypted) = &item.encrypted_content {
        value["encrypted_content"] = serde_json::json!(encrypted);
    }
    if let Some(summary) = &item.summary {
        value["summary"] = serde_json::Value::Array(
            summary
                .iter()
                .map(|part| serde_json::json!({"type": "summary_text", "text": part.text}))
                .collect(),
        );
    }
    if let Some(content) = &item.content {
        value["content"] = serde_json::Value::Array(
            content
                .iter()
                .map(|part| serde_json::json!({"type": "reasoning_text", "text": part.text}))
                .collect(),
        );
    }
    Some(value.to_string())
}

fn owned_item(item: RawOutputItem) -> RawOutputItemOwned {
    let reasoning_signature = reasoning_signature(&item);
    RawOutputItemOwned {
        item_type: item.r#type.unwrap_or_default(),
        id: item.id,
        call_id: item.call_id,
        name: item.name,
        arguments: item.arguments,
        input: item.input,
        reasoning_signature,
        encrypted_content: item.encrypted_content,
    }
}

impl ResponsesEvent {
    /// Shared event-decoding entry (task 20.16): the WebSocket transport
    /// decodes each text frame's payload through exactly the SSE frame
    /// classifier, so both transports publish identical public events.
    pub(crate) fn try_from_json_str(data: &str) -> ParsedEvent {
        Self::try_from_frame(&SseFrame {
            event: String::new(),
            data: data.to_owned(),
        })
    }

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
            "response.reasoning_summary_text.delta" => {
                ParsedEvent::Valid(Self::ReasoningSummaryTextDelta {
                    output_index: data.output_index.unwrap_or(0),
                    delta: data.delta.unwrap_or_default(),
                })
            }
            "response.reasoning_text.delta" => ParsedEvent::Valid(Self::ReasoningTextDelta {
                output_index: data.output_index.unwrap_or(0),
                delta: data.delta.unwrap_or_default(),
            }),
            "response.reasoning_summary_part.done" => {
                ParsedEvent::Valid(Self::ReasoningSummaryPartDone {
                    output_index: data.output_index.unwrap_or(0),
                })
            }
            "response.custom_tool_call_input.delta" => {
                ParsedEvent::Valid(Self::CustomToolCallInputDelta {
                    output_index: data.output_index.unwrap_or(0),
                    delta: data.delta.unwrap_or_default(),
                })
            }
            "response.custom_tool_call_input.done" => {
                ParsedEvent::Valid(Self::CustomToolCallInputDone {
                    output_index: data.output_index.unwrap_or(0),
                    input: data.input.clone().unwrap_or_default(),
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
                    input: None,
                    reasoning_signature: None,
                    encrypted_content: None,
                }),
            }),
            "response.completed" => {
                let raw_stop_reason = Self::raw_stop_reason(&data);
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
                    raw_stop_reason,
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
                ParsedEvent::Valid(Self::Incomplete {
                    usage,
                    raw_stop_reason: Self::raw_stop_reason(&data),
                })
            }
            "response.failed" => ParsedEvent::Valid(Self::Error {
                message: "response.failed".into(),
                raw_stop_reason: data.response.and_then(|response| response.status),
            }),
            "error" => ParsedEvent::Valid(Self::Error {
                message: data.message.unwrap_or_else(|| "unknown error".into()),
                raw_stop_reason: None,
            }),
            // Benign lifecycle, reasoning, function-call-finalization, and any
            // future protocol-extension events must NOT terminate the stream.
            // Genuine errors arrive only via the `error` and `response.failed`
            // types above; an SSE `event:` name is never trusted as an error
            // signal on its own.
            _ => ParsedEvent::Valid(Self::Ignore),
        }
    }

    fn raw_stop_reason(data: &RawResponseEvent) -> Option<String> {
        let response = data.response.as_ref()?;
        let reason = response
            .incomplete_details
            .as_ref()
            .and_then(|details| details.get("reason"))
            .and_then(serde_json::Value::as_str)
            .filter(|reason| !reason.is_empty());
        match reason {
            Some(reason) => Some(format!(
                "{}.{reason}",
                response.status.as_deref().unwrap_or("undefined")
            )),
            None => response.status.clone(),
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
        let cache_write = usage
            .input_tokens_details
            .as_ref()
            .and_then(|details| details.cache_write_tokens)
            .unwrap_or(0);
        let total_tokens = usage.total_tokens;
        let reported = usage.input_tokens.is_some() && usage.output_tokens.is_some();
        let mut usage = Usage::reported(
            usage
                .input_tokens
                .unwrap_or(0)
                .saturating_sub(cached)
                .saturating_sub(cache_write),
            usage.output_tokens.unwrap_or(0),
            cached,
            cache_write,
            None,
            usage
                .output_tokens_details
                .as_ref()
                .and_then(|details| details.reasoning_tokens),
        );
        usage.reported = reported;
        usage.total_tokens = total_tokens;
        if let Some(tier) = parse_service_tier(data) {
            usage = usage.with_service_tier(tier);
        }
        Ok(Some(usage))
    }
}

/// Parse the response-level service tier into its multiplier-bearing enum.
/// A present unknown value stays distinguishable from absence so it cannot
/// silently fall back to a requested discounted tier.
fn parse_service_tier(data: &RawResponseEvent) -> Option<crate::stream::ServiceTier> {
    use crate::stream::ServiceTier;
    match data
        .response
        .as_ref()
        .and_then(|response| response.service_tier.as_deref())
    {
        Some("auto") => Some(ServiceTier::Auto),
        Some("default") => Some(ServiceTier::Default),
        Some("flex") => Some(ServiceTier::Flex),
        Some("scale") => Some(ServiceTier::Scale),
        Some("priority") => Some(ServiceTier::Priority),
        Some("fast") => Some(ServiceTier::Fast),
        Some(_) => Some(ServiceTier::Unknown),
        None => None,
    }
}

struct ToolCallState {
    output_index: usize,
    content_index: usize,
    id: String,
    item_id: String,
    name: String,
    arguments: String,
    ended: bool,
    /// Grammar custom-call state (task 20.11): the wrapping buffer for the
    /// raw wire input under its JSON property.
    custom_input: Option<CustomInputState>,
}

/// One streaming reasoning item (task 20.37): a thinking block whose
/// signature finalizes at output_item.done.
struct ReasoningSlot {
    output_index: usize,
    content_index: usize,
}

struct CustomInputState {
    property: String,
    buffer: crate::constrained_sampling::GrammarInputBuffer,
}

/// Options for the shared Responses-family tool conversion (task 20.11,
/// V-34/AIR-040).
#[derive(Debug, Clone, Copy)]
pub(crate) struct ResponsesToolOptions {
    /// The wire-wide default strictness: `None` emits the provider's null
    /// strict (Codex), `Some(bool)` the configured default.
    pub default_strict: Option<bool>,
    /// Whether the wire supports per-tool strict mode at all.
    pub supports_strict_mode: bool,
    /// Whether the wire supports grammar custom tools.
    pub supports_grammar_tools: bool,
}

/// The compiled tool list plus the per-call grammar/strict facts the mapper
/// and diagnostics consume.
#[derive(Clone)]
pub(crate) struct ResponsesWireTools {
    pub tools: Vec<serde_json::Value>,
    /// Grammar tool name -> input property.
    pub grammar_input_properties: std::collections::HashMap<String, String>,
    /// Trusted declarations of strict-dispatched tools.
    pub strict_tools: Vec<crate::message::ToolDef>,
    /// Recorded constrained-sampling losses (grammar-target fallback).
    pub diagnostics: Vec<crate::stream::AssistantDiagnostic>,
}

/// Compile request-local tool metadata independently of wire placement.
/// Native additions remain inline; replay and response mapping still need
/// their grammar properties, strict wire-null normalization metadata and fallback diagnostics.
pub(crate) fn compile_projected_response_tools(
    messages: &[crate::message::Message],
    request_tools: &[crate::message::ToolDef],
    options: ResponsesToolOptions,
) -> Result<Option<ResponsesWireTools>, ProviderError> {
    let current_tools = crate::transcript::get_current_tools(messages);
    if current_tools.is_empty() {
        return Ok(None);
    }
    let mut wire = convert_responses_tools(&current_tools, options)?;
    if current_tools.as_slice() != request_tools {
        wire.tools = convert_responses_tools(request_tools, options)?.tools;
    }
    Ok(Some(wire))
}

/// Convert the current tool declarations onto the Responses wire (pinned
/// pi 1.3.0 `convertResponsesTools` semantics).
///
/// Grammar tools on a supporting wire project as `custom` tools with the
/// lark-preferred grammar format; on an unsupported target they fall back to
/// a plain JSON function tool with a recorded loss diagnostic. A supported
/// target without a usable variant or schema shape fails typed before
/// dispatch. `json_schema` tools resolve per-tool strict over the wire
/// default; require fails typed when strict mode is unavailable, and strict
/// parameters carry the converted private wire schema (the trusted
/// declaration is never modified).
pub(crate) fn convert_responses_tools(
    tools: &[crate::message::ToolDef],
    options: ResponsesToolOptions,
) -> Result<ResponsesWireTools, ProviderError> {
    use crate::constrained_sampling::{
        ConstrainedSampling, NoRejectedKeywords, make_strict_json_schema,
        resolve_grammar_constrained_sampling, resolve_json_schema_strict_sampling,
    };
    let fail = |detail: String| ProviderError::Config(ProviderErrorSummary::sanitized(detail));
    let mut wire_tools = Vec::with_capacity(tools.len());
    let mut grammar_input_properties = std::collections::HashMap::new();
    let mut strict_tools = Vec::new();
    let mut diagnostics = Vec::new();
    for tool in tools {
        let grammar = resolve_grammar_constrained_sampling(tool, options.supports_grammar_tools)
            .map_err(|error| fail(error.detail().to_owned()))?;
        if let Some(grammar) = grammar {
            grammar_input_properties.insert(tool.name.clone(), grammar.input_property.clone());
            wire_tools.push(serde_json::json!({
                "type": "custom",
                "name": tool.name,
                "description": tool.description,
                "format": {
                    "type": "grammar",
                    "syntax": grammar.format,
                    "definition": grammar.definition,
                },
            }));
            continue;
        }
        if matches!(
            &tool.constrained_sampling,
            Some(ConstrainedSampling::Grammar { .. })
        ) && !options.supports_grammar_tools
        {
            // Unsupported grammar target (V-34): plain JSON projection with
            // the loss recorded on the message.
            diagnostics.push(crate::stream::AssistantDiagnostic {
                kind: "constrained_sampling_fallback".into(),
                reason: "grammar_target_unsupported".into(),
                detail: Some(tool.name.clone()),
            });
        }
        let per_tool_strict = resolve_json_schema_strict_sampling(
            tool,
            options.supports_strict_mode,
            &NoRejectedKeywords,
        )
        .map_err(|error| fail(error.detail().to_owned()))?;
        let strict = per_tool_strict.or(options.default_strict);
        let parameters = if strict == Some(true) {
            let converted = make_strict_json_schema(&tool.input_schema, &NoRejectedKeywords)
                .map_err(|error| fail(error.detail().to_owned()))?;
            strict_tools.push(tool.clone());
            converted
        } else {
            tool.input_schema.clone()
        };
        let mut wire = serde_json::json!({
            "type": "function",
            "name": tool.name,
            "description": tool.description,
            "parameters": parameters,
        });
        if options.supports_strict_mode {
            wire["strict"] = match strict {
                Some(value) => serde_json::Value::Bool(value),
                None => serde_json::Value::Null,
            };
        }
        wire_tools.push(wire);
    }
    Ok(ResponsesWireTools {
        tools: wire_tools,
        grammar_input_properties,
        strict_tools,
        diagnostics,
    })
}

/// Dual-ID packing (task 20.11): `call_id|item_id` with the wire-type
/// prefix (`fc_` function calls, `ctc_` grammar custom calls); a
/// foreign-shaped item id is legalized through the prefix rather than
/// guessed.
fn dual_packed_id(call_id: &str, item_id: &str, prefix: &str) -> String {
    if item_id.is_empty() {
        return call_id.to_owned();
    }
    let mut item_id = item_id.to_owned();
    if !item_id.starts_with(prefix) {
        item_id = format!("{prefix}{item_id}");
    }
    format!("{call_id}|{item_id}")
}

/// Stateful conversion from Responses events to opi stream events.
pub(crate) struct ResponsesMapper {
    snapshot: crate::stream::ProducerSnapshot,
    partial: AssistantMessage,
    requested_service_tier: Option<crate::stream::ServiceTier>,
    pub(crate) saw_done: bool,
    text_started: bool,
    text_content_index: Option<usize>,
    tool_calls: Vec<ToolCallState>,
    /// Live reasoning items (task 20.37), keyed by their wire id for
    /// terminal-output backfill.
    reasoning_slots: Vec<ReasoningSlot>,
    /// Grammar tools of this call (task 20.11): tool name -> input
    /// property.
    grammar_input_properties: std::collections::HashMap<String, String>,
    /// Strict-dispatched tools of this call: their returned arguments
    /// normalize nulls back onto the original trusted schema.
    strict_tools: std::collections::HashMap<String, crate::message::ToolDef>,
}

impl ResponsesMapper {
    pub(crate) fn with_requested_service_tier(
        mut self,
        tier: Option<crate::stream::ServiceTier>,
    ) -> Self {
        self.requested_service_tier = tier;
        self
    }

    fn record_usage(&mut self, mut usage: Usage) {
        use crate::model_info::WireApi;
        use crate::stream::ServiceTier;
        usage.effective_service_tier = match self.partial.wire {
            Some(WireApi::OpenAiCodexResponses)
                if usage.service_tier == Some(ServiceTier::Default)
                    && matches!(
                        self.requested_service_tier,
                        Some(ServiceTier::Flex | ServiceTier::Priority)
                    ) =>
            {
                self.requested_service_tier
            }
            Some(WireApi::OpenAiResponses | WireApi::OpenAiCodexResponses) => {
                usage.service_tier.or(self.requested_service_tier)
            }
            _ => None,
        };
        self.partial.usage = usage;
    }

    /// The mapper's current partial assistant message (task 20.16): the
    /// WebSocket continuation publishes from the same accumulated state the
    /// public stream observed.
    pub(crate) fn partial(&self) -> &crate::message::AssistantMessage {
        &self.partial
    }

    pub(crate) fn set_snapshot(&mut self, snapshot: crate::stream::ProducerSnapshot) {
        self.snapshot = snapshot;
        self.snapshot.record(&self.partial);
    }

    /// Record the grammar custom-tool map of this call (task 20.11).
    pub(crate) fn set_grammar_tools(
        &mut self,
        properties: std::collections::HashMap<String, String>,
    ) {
        self.grammar_input_properties = properties;
    }

    /// Record the strict-dispatched tool declarations of this call.
    pub(crate) fn set_strict_tools(&mut self, tools: Vec<crate::message::ToolDef>) {
        self.strict_tools = tools
            .into_iter()
            .map(|tool| (tool.name.clone(), tool))
            .collect();
    }

    /// Record the wire-native effort this reply runs under (task 20.37):
    /// stamped only when the wire replays thinking levels as
    /// configuration_update items.
    pub(crate) fn set_provider_thinking_level(&mut self, level: Option<String>) {
        self.partial.provider_thinking_level = level;
        self.snapshot.record(&self.partial);
    }

    /// Seed message-level constrained-sampling diagnostics (task 20.11):
    /// recorded grammar-target fallback losses ride the message.
    pub(crate) fn seed_diagnostics(
        &mut self,
        diagnostics: Vec<crate::stream::AssistantDiagnostic>,
    ) {
        self.partial.diagnostics.extend(diagnostics);
    }

    pub(crate) fn process(
        &mut self,
        event: ResponsesEvent,
    ) -> Vec<Result<AssistantStreamEvent, ProviderError>> {
        let events = self.process_inner(event);
        if events.iter().any(Result::is_err) {
            self.saw_done = true;
        }
        self.snapshot.record(&self.partial);
        events
    }

    pub(crate) fn failure(&self, cause: ProviderError) -> ProviderError {
        self.snapshot
            .failure(cause.with_partial(self.partial.clone()))
    }

    pub(crate) fn new(provider: &str, wire: crate::model_info::WireApi, model: &str) -> Self {
        Self {
            snapshot: crate::stream::ProducerSnapshot::default(),
            partial: empty_assistant_message(provider, wire, model),
            requested_service_tier: None,
            saw_done: false,
            text_started: false,
            text_content_index: None,
            tool_calls: Vec::new(),
            reasoning_slots: Vec::new(),
            grammar_input_properties: std::collections::HashMap::new(),
            strict_tools: std::collections::HashMap::new(),
        }
    }

    fn process_inner(
        &mut self,
        event: ResponsesEvent,
    ) -> Vec<Result<AssistantStreamEvent, ProviderError>> {
        if self.saw_done {
            return Vec::new();
        }
        match event {
            ResponsesEvent::Created { model, id } => {
                // Reported model is a separate fact from the requested
                // identity set at construction (AIR-010/RP-02).
                if let Some(model) = model {
                    self.partial.response_model = Some(model);
                }
                if let Some(id) = id {
                    self.partial.response_id = Some(id);
                }
                vec![Ok(AssistantStreamEvent::Start {
                    partial: self.partial.clone(),
                })]
            }
            ResponsesEvent::OutputItemAdded { output_index, item } => {
                if item.item_type == "reasoning" {
                    // Reasoning item (task 20.37): a thinking block whose
                    // signature (the serialized wire item) is finalized at
                    // output_item.done.
                    let mut events: Vec<Result<AssistantStreamEvent, ProviderError>> =
                        self.end_text().into_iter().map(Ok).collect();
                    let content_index = self.partial.content.len();
                    self.partial.content.push(AssistantContent::Thinking {
                        thinking: String::new(),
                        signature: None,
                    });
                    self.reasoning_slots.push(ReasoningSlot {
                        output_index,
                        content_index,
                    });
                    events.push(Ok(AssistantStreamEvent::ThinkingStart {
                        content_index,
                        partial: self.partial.clone(),
                    }));
                    return events;
                }
                if item.item_type == "custom_tool_call" {
                    // Grammar custom call (task 20.11): a tool-call slot whose
                    // raw wire input wraps into incremental JSON under the
                    // tool's input property.
                    let mut events: Vec<Result<AssistantStreamEvent, ProviderError>> =
                        self.end_text().into_iter().map(Ok).collect();
                    let id = item.call_id.clone().unwrap_or_default();
                    let name = item.name.clone().unwrap_or_default();
                    let property = self
                        .grammar_input_properties
                        .get(&name)
                        .cloned()
                        .unwrap_or_else(|| "input".to_owned());
                    let content_index = self.partial.content.len();
                    self.partial.content.push(AssistantContent::ToolCall {
                        tool_call: ToolCall {
                            id: id.clone(),
                            name: name.clone(),
                            arguments: String::new(),
                            arguments_error: None,
                            signature: None,
                        },
                    });
                    self.tool_calls.push(ToolCallState {
                        output_index,
                        content_index,
                        id,
                        item_id: item.id.clone().unwrap_or_default(),
                        name,
                        arguments: String::new(),
                        ended: false,
                        custom_input: Some(CustomInputState {
                            buffer: crate::constrained_sampling::GrammarInputBuffer::new(
                                property.clone(),
                                item.input.clone().unwrap_or_default(),
                            ),
                            property,
                        }),
                    });
                    events.push(Ok(AssistantStreamEvent::ToolCallStart {
                        content_index,
                        partial: self.partial.clone(),
                    }));
                    return events;
                }
                if item.item_type != "function_call" {
                    return Vec::new();
                }
                let mut events: Vec<Result<AssistantStreamEvent, ProviderError>> =
                    self.end_text().into_iter().map(Ok).collect();
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
                        arguments_error: None,
                        signature: None,
                    },
                });
                self.tool_calls.push(ToolCallState {
                    output_index,
                    item_id: String::new(),
                    content_index,
                    id,
                    name,
                    arguments,
                    ended: false,
                    custom_input: None,
                });
                events.push(Ok(AssistantStreamEvent::ToolCallStart {
                    content_index,
                    partial: self.partial.clone(),
                }));
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
                        signature: None,
                    });
                    self.text_content_index = Some(index);
                    events.push(Ok(AssistantStreamEvent::TextStart {
                        content_index: index,
                        partial: self.partial.clone(),
                    }));
                    index
                };
                if let Some(AssistantContent::Text { text, .. }) =
                    self.partial.content.get_mut(content_index)
                {
                    text.push_str(&delta);
                }
                events.push(Ok(AssistantStreamEvent::TextDelta {
                    content_index,
                    delta,
                    partial: self.partial.clone(),
                }));
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
                vec![Ok(AssistantStreamEvent::ToolCallDelta {
                    content_index,
                    delta,
                    partial: self.partial.clone(),
                })]
            }
            ResponsesEvent::ReasoningSummaryTextDelta {
                output_index,
                delta,
            }
            | ResponsesEvent::ReasoningTextDelta {
                output_index,
                delta,
            } => {
                if delta.is_empty() {
                    return Vec::new();
                }
                self.push_thinking_delta(output_index, &delta)
            }
            ResponsesEvent::ReasoningSummaryPartDone { output_index } => {
                self.push_thinking_delta(output_index, "\n\n")
            }
            ResponsesEvent::CustomToolCallInputDelta {
                output_index,
                delta,
            } => match self.append_custom_input_increment(output_index, &delta) {
                Ok(Some(wrapped_delta)) => {
                    let Some(content_index) = self
                        .tool_calls
                        .iter()
                        .find(|state| state.output_index == output_index)
                        .map(|state| state.content_index)
                    else {
                        return Vec::new();
                    };
                    vec![Ok(AssistantStreamEvent::ToolCallDelta {
                        content_index,
                        delta: wrapped_delta,
                        partial: self.partial.clone(),
                    })]
                }
                Ok(None) => Vec::new(),
                Err(error) => vec![Err(self.failure(error))],
            },
            ResponsesEvent::CustomToolCallInputDone {
                output_index,
                input,
            } => match self.append_custom_input(output_index, &input, true) {
                Ok(Some(wrapped_delta)) => {
                    let Some(content_index) = self
                        .tool_calls
                        .iter()
                        .find(|state| state.output_index == output_index)
                        .map(|state| state.content_index)
                    else {
                        return Vec::new();
                    };
                    vec![Ok(AssistantStreamEvent::ToolCallDelta {
                        content_index,
                        delta: wrapped_delta,
                        partial: self.partial.clone(),
                    })]
                }
                Ok(None) => Vec::new(),
                Err(error) => vec![Err(self.failure(error))],
            },
            ResponsesEvent::OutputItemDone { output_index, item } => {
                if item.item_type == "reasoning" {
                    return self
                        .finish_reasoning(output_index, &item)
                        .into_iter()
                        .map(Ok)
                        .collect();
                }
                if item.item_type == "custom_tool_call" {
                    if self
                        .tool_calls
                        .iter()
                        .any(|state| state.output_index == output_index && state.ended)
                    {
                        return Vec::new();
                    }
                    let mut events = if self
                        .tool_calls
                        .iter()
                        .all(|state| state.output_index != output_index)
                    {
                        self.process_inner(ResponsesEvent::OutputItemAdded {
                            output_index,
                            item: item.clone(),
                        })
                    } else {
                        Vec::new()
                    };
                    self.update_tool_call(output_index, &item);
                    let input = item.input.clone().unwrap_or_else(|| {
                        self.tool_calls
                            .iter()
                            .find(|state| state.output_index == output_index)
                            .and_then(|state| state.custom_input.as_ref())
                            .map(|custom| custom.buffer.input().to_owned())
                            .unwrap_or_default()
                    });
                    match self.append_custom_input(output_index, &input, true) {
                        Ok(Some(delta)) => {
                            if let Some(state) = self
                                .tool_calls
                                .iter()
                                .find(|state| state.output_index == output_index)
                            {
                                events.push(Ok(AssistantStreamEvent::ToolCallDelta {
                                    content_index: state.content_index,
                                    delta,
                                    partial: self.partial.clone(),
                                }));
                            }
                        }
                        Ok(None) => {}
                        Err(error) => {
                            events.push(Err(self.failure(error)));
                            return events;
                        }
                    }
                    events.extend(self.finish_tool_call(output_index).map(Ok));
                    return events;
                }
                if item.item_type != "function_call" {
                    return Vec::new();
                }
                self.update_tool_call(output_index, &item);
                self.finish_tool_call(output_index)
                    .into_iter()
                    .map(Ok)
                    .collect()
            }
            ResponsesEvent::Incomplete {
                usage,
                raw_stop_reason,
            } => {
                self.partial.raw_stop_reason = raw_stop_reason;
                let mut events: Vec<Result<AssistantStreamEvent, ProviderError>> = self
                    .end_text()
                    .into_iter()
                    .chain(self.end_thinking_all())
                    .map(Ok)
                    .collect();
                if let Some(usage) = usage {
                    self.record_usage(usage);
                }
                self.partial.stop_reason = StopReason::Length;
                self.saw_done = true;
                events.push(Ok(AssistantStreamEvent::Done {
                    reason: StopReason::Length,
                    message: self.partial.clone(),
                }));
                events
            }
            ResponsesEvent::Completed {
                usage,
                raw_stop_reason,
                model,
                id,
                output,
            } => {
                self.partial.raw_stop_reason = raw_stop_reason;
                let mut events: Vec<Result<AssistantStreamEvent, ProviderError>> =
                    self.end_text().into_iter().map(Ok).collect();
                // Terminal-output backfill (task 20.37) before Done: patch
                // reasoning signatures the per-item frames omitted.
                self.backfill_reasoning_signatures(&output);
                for (output_index, item) in output.iter().enumerate() {
                    if item.item_type == "custom_tool_call" {
                        events.extend(self.process_inner(ResponsesEvent::OutputItemDone {
                            output_index,
                            item: item.clone(),
                        }));
                    } else if item.item_type == "function_call" {
                        self.update_tool_call(output_index, item);
                    }
                }
                if events.iter().any(Result::is_err) {
                    return events;
                }
                let unfinished: Vec<usize> = self
                    .tool_calls
                    .iter()
                    .filter(|state| !state.ended)
                    .map(|state| state.output_index)
                    .collect();
                for output_index in unfinished {
                    if let Some(event) = self.finish_tool_call(output_index) {
                        events.push(Ok(event));
                    }
                }
                if let Some(model) = model {
                    self.partial.response_model = Some(model);
                }
                if let Some(id) = id {
                    self.partial.response_id = Some(id);
                }
                if let Some(usage) = usage {
                    self.record_usage(usage);
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
                events.push(Ok(AssistantStreamEvent::Done {
                    reason: self.partial.stop_reason,
                    message: self.partial.clone(),
                }));
                events
            }
            ResponsesEvent::Error {
                message: _,
                raw_stop_reason,
            } => {
                self.partial.raw_stop_reason = raw_stop_reason;
                // Preserve state that did not produce a public event (usage,
                // opaque content or signatures), alongside the typed cause.
                self.saw_done = true;
                vec![Err(self.failure(ProviderError::StreamError(
                    ProviderErrorSummary::attested_static(OPENAI_RESPONSES_STREAM_ERROR),
                )))]
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
            Some(AssistantContent::Text { text, .. }) => vec![AssistantStreamEvent::TextEnd {
                content_index,
                content: text.clone(),
                partial: self.partial.clone(),
            }],
            _ => Vec::new(),
        }
    }

    /// Append one reasoning increment to the matching live thinking block
    /// (task 20.37) and publish the delta.
    fn push_thinking_delta(
        &mut self,
        output_index: usize,
        delta: &str,
    ) -> Vec<Result<AssistantStreamEvent, ProviderError>> {
        let Some(slot) = self
            .reasoning_slots
            .iter()
            .find(|slot| slot.output_index == output_index)
        else {
            return Vec::new();
        };
        let content_index = slot.content_index;
        if let Some(AssistantContent::Thinking { thinking, .. }) =
            self.partial.content.get_mut(content_index)
        {
            thinking.push_str(delta);
        }
        vec![Ok(AssistantStreamEvent::ThinkingDelta {
            content_index,
            delta: delta.to_owned(),
            partial: self.partial.clone(),
        })]
    }

    /// Finalize one reasoning item (task 20.37): the block's text prefers
    /// the item's summary/content over the streamed deltas, and the block
    /// persists the serialized wire item as its replay signature.
    fn finish_reasoning(
        &mut self,
        output_index: usize,
        item: &RawOutputItemOwned,
    ) -> Vec<AssistantStreamEvent> {
        let Some(position) = self
            .reasoning_slots
            .iter()
            .position(|slot| slot.output_index == output_index)
        else {
            return Vec::new();
        };
        let slot = self.reasoning_slots.remove(position);
        let signature = item.reasoning_signature.clone();
        let finalized_text = reasoning_item_text(signature.as_deref());
        let Some(AssistantContent::Thinking {
            thinking,
            signature: target,
        }) = self.partial.content.get_mut(slot.content_index)
        else {
            return Vec::new();
        };
        // Source order: the item's summary, then its raw content, then the
        // streamed deltas already accumulated.
        if !finalized_text.is_empty() {
            *thinking = finalized_text;
        }
        *target = signature;
        let content = thinking.clone();
        vec![AssistantStreamEvent::ThinkingEnd {
            content_index: slot.content_index,
            content,
            partial: self.partial.clone(),
        }]
    }

    /// Close the live thinking slot without a terminal item (EOF safety):
    /// publishes the accumulated text without a signature.
    fn end_thinking_all(&mut self) -> Vec<AssistantStreamEvent> {
        let mut events = Vec::new();
        while !self.reasoning_slots.is_empty() {
            let slot = self.reasoning_slots.remove(0);
            let content = match self.partial.content.get(slot.content_index) {
                Some(AssistantContent::Thinking { thinking, .. }) => thinking.clone(),
                _ => continue,
            };
            events.push(AssistantStreamEvent::ThinkingEnd {
                content_index: slot.content_index,
                content,
                partial: self.partial.clone(),
            });
        }
        events
    }

    /// Backfill reasoning signatures from the terminal response output
    /// (task 20.37): some servers (Azure) omit `encrypted_content` from
    /// output_item.done and provide it only on response.completed; patch
    /// matching finalized blocks (signatures embed the item id) so
    /// store:false replay stays stateless.
    fn backfill_reasoning_signatures(&mut self, output: &[RawOutputItemOwned]) {
        for item in output {
            if item.item_type != "reasoning" {
                continue;
            }
            let (Some(item_id), Some(encrypted)) =
                (item.id.as_deref(), item.encrypted_content.as_deref())
            else {
                continue;
            };
            for content in &mut self.partial.content {
                let AssistantContent::Thinking { signature, .. } = content else {
                    continue;
                };
                let Some(sig) = signature.clone() else {
                    continue;
                };
                let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&sig) else {
                    continue;
                };
                if value.get("id").and_then(serde_json::Value::as_str) == Some(item_id)
                    && value.get("encrypted_content").is_none()
                {
                    value["encrypted_content"] = serde_json::json!(encrypted);
                    *signature = Some(value.to_string());
                }
            }
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
        // Grammar custom calls (task 20.11): the authoritative arguments are
        // the raw input under its property (already maintained per delta);
        // textual finalization and its error fact do not apply.
        if state.custom_input.is_some() {
            let arguments = state.arguments.clone();
            let item_id = state.item_id.clone();
            let id = state.id.clone();
            let name = state.name.clone();
            let tool_call = ToolCall {
                id: dual_packed_id(&id, &item_id, "ctc_"),
                name,
                arguments: arguments.clone(),
                arguments_error: None,
                signature: None,
            };
            if let Some(AssistantContent::ToolCall { tool_call: partial }) =
                self.partial.content.get_mut(content_index)
            {
                partial.arguments = arguments;
                partial.id = tool_call.id.clone();
            }
            return Some(AssistantStreamEvent::ToolCallEnd {
                content_index,
                tool_call,
                partial: self.partial.clone(),
            });
        }
        // Final textual arguments are parsed once at item end (AIR-048).
        let (mut arguments, arguments_error) =
            ToolCall::finalize_text_arguments(state.arguments.clone());
        // Strict-dispatched tools (task 20.11): normalize explicit nulls
        // back onto the original trusted schema.
        if arguments_error.is_none()
            && let Some(tool) = self.strict_tools.get(&state.name)
            && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&arguments)
        {
            let mut normalized = parsed;
            crate::constrained_sampling::normalize_wire_arguments_nulls(
                &tool.input_schema,
                &mut normalized,
            );
            arguments = serde_json::to_string(&normalized).unwrap_or(arguments);
        }
        // Dual-ID packing (pinned pi 1.3.0 Responses mechanism): the
        // canonical id carries `call_id|item_id` so same-identity replay
        // can restore both fields. Item ids must start with `fc_`; a
        // foreign-shaped one is legalized through a stable digest instead
        // of being guessed (RP-02/RP-07).
        let item_id = state.item_id.clone();
        let id = state.id.clone();
        let packed_id = dual_packed_id(&id, &item_id, "fc_");
        let tool_call = ToolCall {
            id: packed_id,
            name: state.name.clone(),
            arguments: arguments.clone(),
            arguments_error,
            signature: None,
        };
        if let Some(AssistantContent::ToolCall { tool_call: partial }) =
            self.partial.content.get_mut(content_index)
        {
            partial.arguments = arguments;
            partial.arguments_error = tool_call.arguments_error.clone();
            // The finalized message carries the packed dual id so the
            // canonical history round-trips both parts (V-07).
            partial.id = tool_call.id.clone();
        }
        Some(AssistantStreamEvent::ToolCallEnd {
            content_index,
            tool_call,
            partial: self.partial.clone(),
        })
    }

    /// Accumulate the wire's incremental input before updating the wrapping
    /// buffer, whose input contract is the complete value observed so far.
    fn append_custom_input_increment(
        &mut self,
        output_index: usize,
        delta: &str,
    ) -> Result<Option<String>, ProviderError> {
        let Some(custom) = self
            .tool_calls
            .iter()
            .find(|state| state.output_index == output_index)
            .and_then(|state| state.custom_input.as_ref())
        else {
            return Ok(None);
        };
        let next_input = format!("{}{delta}", custom.buffer.input());
        self.append_custom_input(output_index, &next_input, false)
    }

    /// Append one raw grammar-input increment (task 20.11): the buffer
    /// enforces monotonic growth and single close; violations are typed
    /// stream failures that preserve the delivered partial.
    fn append_custom_input(
        &mut self,
        output_index: usize,
        next_input: &str,
        close: bool,
    ) -> Result<Option<String>, ProviderError> {
        let Some(state) = self
            .tool_calls
            .iter_mut()
            .find(|state| state.output_index == output_index)
        else {
            return Ok(None);
        };
        let Some(custom) = state.custom_input.as_mut() else {
            return Ok(None);
        };
        let wrapped = crate::constrained_sampling::append_grammar_tool_input_delta(
            &mut custom.buffer,
            next_input,
            close,
        )
        .map_err(|error| {
            ProviderError::StreamError(ProviderErrorSummary::sanitized(error.detail()))
        })?;
        // The authoritative arguments stay the raw input under its
        // property; the wrapped text is the streaming view.
        if let Ok(arguments) = serde_json::to_string(&serde_json::json!({
            &custom.property: custom.buffer.input(),
        })) {
            state.arguments = arguments;
            if let Some(AssistantContent::ToolCall { tool_call }) =
                self.partial.content.get_mut(state.content_index)
            {
                tool_call.arguments = state.arguments.clone();
            }
        }
        Ok(wrapped)
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
        if let Some(item_id) = item.id.as_ref()
            && !item_id.is_empty()
        {
            state.item_id = item_id.clone();
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

fn empty_assistant_message(
    provider: &str,
    wire: crate::model_info::WireApi,
    model: &str,
) -> AssistantMessage {
    AssistantMessage {
        projection: None,
        content: Vec::new(),
        api: crate::ApiKind::OpenAi,
        provider: provider.into(),
        model: model.to_owned(),
        response_model: None,
        wire: Some(wire),
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

/// Prefer the reasoning item's summary text, then its raw content text,
/// from a persisted reasoning signature; empty when neither is present.
fn reasoning_item_text(signature: Option<&str>) -> String {
    let Some(signature) = signature else {
        return String::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(signature) else {
        return String::new();
    };
    let join_parts = |key: &str| -> String {
        value
            .get(key)
            .and_then(serde_json::Value::as_array)
            .map(|parts| {
                parts
                    .iter()
                    .filter_map(|part| part.get("text").and_then(serde_json::Value::as_str))
                    .collect::<Vec<_>>()
                    .join("\n\n")
            })
            .unwrap_or_default()
    };
    let summary = join_parts("summary");
    if !summary.is_empty() {
        return summary;
    }
    join_parts("content")
}

/// Split a packed dual Responses tool-call id (`call_id|fc_item_id`) into
/// its call and item parts; ids without a separator carry only a call id.
fn split_packed_call_id(id: &str) -> (&str, Option<&str>) {
    match id.split_once('|') {
        Some((call_id, item_id)) => (call_id, Some(item_id)),
        None => (id, None),
    }
}

/// Convert the replay-projected messages to standard Responses input items.
///
/// `configuration_update_effort` (task 20.37): when set, one
/// `configuration_update` item carrying the reply's recorded effort is
/// inserted before each earlier same-identity assistant turn that recorded
/// a level, and a final item with this effort closes the list; earlier
/// input bytes never change between turns.
// The continuation owner converts only its freshly completed assistant turn;
// it has no temporal tool declarations and retains the existing internal seam.
pub(crate) fn convert_messages(
    messages: &[crate::message::Message],
    route: &crate::replay::ReplayRoute,
    grammar_input_properties: &std::collections::HashMap<String, String>,
    effort: Option<&str>,
) -> Vec<serde_json::Value> {
    convert_projected_messages(
        messages,
        route,
        grammar_input_properties,
        effort,
        &Default::default(),
        "system",
    )
    .expect("assistant continuation has no fallible tool declarations")
}

pub(crate) fn convert_projected_messages(
    messages: &[crate::message::Message],
    route: &crate::replay::ReplayRoute,
    grammar_input_properties: &std::collections::HashMap<String, String>,
    configuration_update_effort: Option<&str>,
    source: &crate::model_info::OpenAiResponsesSourceCompat,
    instruction_role: &str,
) -> Result<Vec<serde_json::Value>, ProviderError> {
    let mut input = Vec::new();
    let configuration_item = |effort: &str| {
        serde_json::json!({
            "type": "configuration_update",
            "reasoning": {"effort": effort},
        })
    };
    for (message_index, message) in messages.iter().enumerate() {
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
                // Items follow the canonical block order: text and calls
                // interleave exactly as the turn produced them (AIR-012/
                // RP-03). Thinking blocks ride this wire only through the
                // 20.34 encrypted-reasoning parity, not as fabricated
                // visible items.
                let same = crate::replay::same_identity(assistant, route);
                // Mid-conversation effort stamp (task 20.37): re-emit the
                // level this earlier turn ran under, before its items.
                if configuration_update_effort.is_some()
                    && same
                    && let Some(recorded) = assistant.provider_thinking_level.as_deref()
                {
                    input.push(configuration_item(recorded));
                }
                for content in &assistant.content {
                    match content {
                        AssistantContent::Text { text: part, .. } => {
                            input.push(serde_json::json!({
                                "type":"message",
                                "role":"assistant",
                                "content":[{"type":"output_text","text":part,"annotations":[]}],
                                "status":"completed",
                            }));
                        }
                        AssistantContent::ToolCall { tool_call } => {
                            let (call_id, item_id) = split_packed_call_id(&tool_call.id);
                            if let Some(property) = grammar_input_properties.get(&tool_call.name) {
                                // Grammar custom call (task 20.11): replay
                                // restores the raw input string under its
                                // property, with the ctc_ item id.
                                let raw_input =
                                    serde_json::from_str::<serde_json::Value>(&tool_call.arguments)
                                        .ok()
                                        .and_then(|arguments| {
                                            arguments
                                                .get(property)
                                                .and_then(serde_json::Value::as_str)
                                                .map(String::from)
                                        })
                                        .unwrap_or_default();
                                let mut custom = serde_json::json!({
                                    "type":"custom_tool_call",
                                    "call_id":call_id,
                                    "name":tool_call.name,
                                    "input":raw_input,
                                });
                                if same && let Some(item_id) = item_id {
                                    custom["id"] = serde_json::json!(item_id);
                                }
                                input.push(custom);
                                continue;
                            }
                            // Same-identity replay restores both ids; a
                            // different-identity call omits the item id so
                            // the service's reasoning-pairing validation
                            // never fires on foreign history (pinned
                            // behavior).
                            let mut call = serde_json::json!({
                                "type":"function_call",
                                "call_id":call_id,
                                "name":tool_call.name,
                                "arguments":tool_call.arguments,
                            });
                            if same && let Some(item_id) = item_id {
                                call["id"] = serde_json::json!(item_id);
                            }
                            input.push(call);
                        }
                        AssistantContent::Thinking { signature, .. } => {
                            // Encrypted reasoning replay (task 20.37): the
                            // persisted signature is the wire reasoning
                            // item; push it back verbatim so store:false
                            // multi-turn stays stateless. Unsigned thinking
                            // never fabricates an item.
                            if let Some(signature) = signature
                                && let Ok(item) =
                                    serde_json::from_str::<serde_json::Value>(signature)
                                && item.get("type").and_then(|t| t.as_str()) == Some("reasoning")
                            {
                                input.push(item);
                            }
                        }
                        AssistantContent::RedactedThinking { .. } => {}
                    }
                }
            }
            crate::message::Message::System(system) => {
                if !system.tools_added.is_empty()
                    && (source.supports_additional_tools.unwrap_or(false)
                        || source.supports_tool_search.unwrap_or(false))
                {
                    let strict = source.supports_strict_mode.unwrap_or(matches!(
                        route.wire,
                        crate::WireApi::OpenAiCodexResponses | crate::WireApi::AzureOpenAiResponses
                    ));
                    let mut tools = convert_responses_tools(
                        &system.tools_added,
                        ResponsesToolOptions {
                            default_strict: if route.wire == crate::WireApi::OpenAiCodexResponses {
                                None
                            } else {
                                Some(false)
                            },
                            supports_strict_mode: strict,
                            supports_grammar_tools: source.supports_grammar_tools.unwrap_or(false),
                        },
                    )?
                    .tools;
                    if source.supports_additional_tools.unwrap_or(false) {
                        input.push(serde_json::json!({"type":"additional_tools","role":"developer","tools":tools}));
                    } else {
                        let names: Vec<_> = system
                            .tools_added
                            .iter()
                            .map(|tool| tool.name.as_str())
                            .collect();
                        let call_id = format!(
                            "pi_tool_load_{}",
                            crate::identity::short_hash(&format!(
                                "system:{message_index}:{}",
                                names.join(",")
                            ))
                        );
                        for tool in &mut tools {
                            tool["defer_loading"] = serde_json::json!(true);
                        }
                        input.push(serde_json::json!({"type":"tool_search_call","call_id":call_id,"execution":"client","status":"completed","arguments":{"query":names.join(" "),"limit":names.len()}}));
                        input.push(serde_json::json!({"type":"tool_search_output","call_id":call_id,"execution":"client","status":"completed","tools":tools}));
                    }
                }
                let text = crate::transcript::render_system_message_update(system);
                if !text.is_empty() {
                    input.push(serde_json::json!({"role":instruction_role,"content":text}));
                }
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
                let (call_id, _item_id) = split_packed_call_id(&result.tool_call_id);
                let item_type = if grammar_input_properties.contains_key(&result.tool_name) {
                    "custom_tool_call_output"
                } else {
                    "function_call_output"
                };
                input.push(serde_json::json!({
                    "type":item_type,
                    "call_id":call_id,
                    "output":output,
                }));
            }
        }
    }
    if let Some(effort) = configuration_update_effort {
        input.push(configuration_item(effort));
    }
    Ok(input)
}

/// Drain complete SSE frames while retaining an incomplete trailing frame.
pub(crate) fn drain_sse_frames(buffer: &mut String) -> Vec<SseFrame> {
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

#[cfg(test)]
mod fragmented_sse_tests {
    use super::*;

    #[test]
    fn multiline_frames_survive_every_line_ending_split() {
        for newline in ["\n", "\r\n", "\r"] {
            let wire = format!(
                "event: response.output_text.delta{newline}data: {{\"type\":\"response.output_text.delta\",{newline}data: \"delta\":\"hello\"}}{newline}{newline}"
            );
            let mut expected =
                vec![serde_json::json!({"type":"response.output_text.delta","delta":"hello"})];
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
        drain_sse_frames(buffer)
            .into_iter()
            .map(|frame| {
                assert_eq!(frame.event, "response.output_text.delta");
                serde_json::from_str(&frame.data).unwrap()
            })
            .collect()
    }
}
