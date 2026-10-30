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
    /// Output-only response identity; the first non-empty value from the
    /// stream is retained (task 20.40, pi `responseId ||= chunk.responseId`).
    #[serde(default, rename = "responseId")]
    response_id: Option<String>,
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
        /// `thought: true` marks thought summaries (task 20.40); the
        /// signature alone never marks a part as thinking.
        #[serde(default)]
        thought: Option<bool>,
        #[serde(default, rename = "thoughtSignature")]
        thought_signature: Option<String>,
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
    /// Gemini 3+ models (and claude/gpt-oss behind Google APIs) may attach
    /// explicit tool call ids; missing or duplicate ids are regenerated
    /// locally (task 20.40, pi unsigned tool call handling).
    #[serde(default)]
    id: Option<String>,
    #[serde(default, rename = "thoughtSignature")]
    thought_signature: Option<String>,
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
    /// Reasoning tokens: contained by `candidatesTokenCount + thoughtsTokenCount`
    /// on the output side (pi usage mapping, task 20.40).
    #[serde(default, rename = "thoughtsTokenCount")]
    thoughts_token_count: Option<u32>,
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
    /// First non-empty response identity (recorded once per stream).
    ResponseId { id: String },
    TextDelta {
        text: String,
        signature: Option<String>,
    },
    ThinkingDelta {
        text: String,
        signature: Option<String>,
    },
    FunctionCall {
        id: Option<String>,
        name: String,
        args: serde_json::Value,
        signature: Option<String>,
    },
    Finish {
        reason: String,
        usage: Option<Usage>,
    },
    Error {
        #[expect(
            dead_code,
            reason = "bounded upstream error text rides the typed stream-failure cause"
        )]
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

        // First non-empty response identity wins (pi `responseId ||=`).
        if let Some(id) = resp.response_id
            && !id.is_empty()
        {
            events.push(ParsedEvent::Valid(GeminiEvent::ResponseId { id }));
        }

        // Usage follows the pi mapping: input = prompt - cached,
        // output = candidates + thoughts, reasoning = thoughts.
        let usage = resp.usage_metadata.map(|u| {
            let cached = u.cached_content_token_count.unwrap_or(0);
            let thoughts = u.thoughts_token_count.unwrap_or(0);
            Usage::reported(
                u.prompt_token_count.unwrap_or(0).saturating_sub(cached),
                u.candidates_token_count.unwrap_or(0) + thoughts,
                cached,
                0,
                None, // cache_write_1h_tokens
                Some(thoughts as u64),
            )
        });

        if let Some(candidates) = &resp.candidates
            && let Some(candidate) = candidates.first()
        {
            let finish_reason = candidate.finish_reason.clone();

            if let Some(content) = &candidate.content
                && let Some(parts) = &content.parts
            {
                // Parts keep wire order: text, thought summaries and
                // function calls interleave as the model produced them.
                for part in parts {
                    match part {
                        Part::Text {
                            text,
                            thought,
                            thought_signature,
                        } => {
                            // Empty strings carry no content; pi skips them
                            // unless a signature rides the part.
                            if text.is_empty() && thought_signature.is_none() {
                                continue;
                            }
                            if *thought == Some(true) {
                                events.push(ParsedEvent::Valid(GeminiEvent::ThinkingDelta {
                                    text: text.clone(),
                                    signature: thought_signature.clone(),
                                }));
                            } else {
                                events.push(ParsedEvent::Valid(GeminiEvent::TextDelta {
                                    text: text.clone(),
                                    signature: thought_signature.clone(),
                                }));
                            }
                        }
                        Part::FunctionCall { function_call } => {
                            events.push(ParsedEvent::Valid(GeminiEvent::FunctionCall {
                                id: function_call.id.clone(),
                                name: function_call.name.clone(),
                                args: function_call.args.clone(),
                                signature: function_call.thought_signature.clone(),
                            }));
                        }
                    }
                }

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

/// The block currently accumulating on the wire (pi alternates text and
/// thinking blocks; signatures are retained per block).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CurrentBlock {
    Text,
    Thinking,
}

pub(crate) struct GeminiMapper {
    snapshot: crate::stream::ProducerSnapshot,
    partial: AssistantMessage,
    saw_done: bool,
    current_block: Option<CurrentBlock>,
    /// Locally generated ids for unsigned or duplicate tool calls; ids are
    /// unique per stream and derive from the tool name (pi
    /// `name_${counter}` shape without wall-clock dependence).
    id_counter: u64,
    used_ids: std::collections::HashSet<String>,
}

impl GeminiMapper {
    pub(crate) fn set_snapshot(&mut self, snapshot: crate::stream::ProducerSnapshot) {
        self.snapshot = snapshot;
        self.snapshot.record(&self.partial);
    }

    pub(crate) fn process(
        &mut self,
        event: GeminiEvent,
    ) -> Vec<Result<AssistantStreamEvent, ProviderError>> {
        let events = self.process_inner(event);
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
            saw_done: false,
            current_block: None,
            id_counter: 0,
            used_ids: std::collections::HashSet::new(),
        }
    }

    /// Start event on first content (exactly once per stream).
    fn start_if_empty(&mut self, events: &mut Vec<Result<AssistantStreamEvent, ProviderError>>) {
        if self.partial.content.is_empty() {
            events.push(Ok(AssistantStreamEvent::Start {
                partial: self.partial.clone(),
            }));
        }
    }

    /// Close the open text/thinking block, if any.
    fn close_block(&mut self, events: &mut Vec<Result<AssistantStreamEvent, ProviderError>>) {
        if let Some(kind) = self.current_block.take() {
            let index = self.partial.content.len().saturating_sub(1);
            match kind {
                CurrentBlock::Text => {
                    if let Some(AssistantContent::Text { text, .. }) = self.partial.content.last() {
                        events.push(Ok(AssistantStreamEvent::TextEnd {
                            content_index: index,
                            content: text.clone(),
                            partial: self.partial.clone(),
                        }));
                    }
                }
                CurrentBlock::Thinking => {
                    if let Some(AssistantContent::Thinking { thinking, .. }) =
                        self.partial.content.last()
                    {
                        events.push(Ok(AssistantStreamEvent::ThinkingEnd {
                            content_index: index,
                            content: thinking.clone(),
                            partial: self.partial.clone(),
                        }));
                    }
                }
            }
        }
    }

    /// A tool call id that is unique on this stream: the provided id when
    /// present and unused, otherwise a name-derived generated id.
    fn resolve_tool_call_id(&mut self, provided: Option<String>, name: &str) -> String {
        if let Some(id) = provided
            && !id.is_empty()
            && !self.used_ids.contains(&id)
        {
            self.used_ids.insert(id.clone());
            return id;
        }
        loop {
            self.id_counter += 1;
            let candidate = format!("{name}_{}", self.id_counter);
            if !self.used_ids.contains(&candidate) {
                self.used_ids.insert(candidate.clone());
                return candidate;
            }
        }
    }

    fn process_inner(
        &mut self,
        event: GeminiEvent,
    ) -> Vec<Result<AssistantStreamEvent, ProviderError>> {
        if self.saw_done {
            return Vec::new();
        }
        match event {
            GeminiEvent::ResponseId { id } => {
                if self.partial.response_id.is_none() {
                    self.partial.response_id = Some(id);
                }
                Vec::new()
            }
            GeminiEvent::TextDelta { text, signature } => {
                let mut events = Vec::new();
                self.start_if_empty(&mut events);
                if self.current_block != Some(CurrentBlock::Text) {
                    self.close_block(&mut events);
                    self.partial.content.push(AssistantContent::Text {
                        text: String::new(),
                        signature: None,
                    });
                    events.push(Ok(AssistantStreamEvent::TextStart {
                        content_index: self.partial.content.len() - 1,
                        partial: self.partial.clone(),
                    }));
                    self.current_block = Some(CurrentBlock::Text);
                }
                let index = self.partial.content.len() - 1;
                if let Some(AssistantContent::Text {
                    text: accumulated,
                    signature: held,
                }) = self.partial.content.last_mut()
                {
                    accumulated.push_str(&text);
                    // Signatures may ride only the first delta of a block;
                    // keep the last non-empty one (pi retainThoughtSignature).
                    if signature.as_deref().is_some_and(|s| !s.is_empty()) {
                        *held = signature.clone();
                    }
                }
                events.push(Ok(AssistantStreamEvent::TextDelta {
                    content_index: index,
                    delta: text,
                    partial: self.partial.clone(),
                }));
                events
            }
            GeminiEvent::ThinkingDelta { text, signature } => {
                let mut events = Vec::new();
                self.start_if_empty(&mut events);
                if self.current_block != Some(CurrentBlock::Thinking) {
                    self.close_block(&mut events);
                    self.partial.content.push(AssistantContent::Thinking {
                        thinking: String::new(),
                        signature: None,
                    });
                    events.push(Ok(AssistantStreamEvent::ThinkingStart {
                        content_index: self.partial.content.len() - 1,
                        partial: self.partial.clone(),
                    }));
                    self.current_block = Some(CurrentBlock::Thinking);
                }
                let index = self.partial.content.len() - 1;
                if let Some(AssistantContent::Thinking {
                    thinking: accumulated,
                    signature: held,
                }) = self.partial.content.last_mut()
                {
                    accumulated.push_str(&text);
                    if signature.as_deref().is_some_and(|s| !s.is_empty()) {
                        *held = signature.clone();
                    }
                }
                events.push(Ok(AssistantStreamEvent::ThinkingDelta {
                    content_index: index,
                    delta: text,
                    partial: self.partial.clone(),
                }));
                events
            }
            GeminiEvent::FunctionCall {
                id,
                name,
                args,
                signature,
            } => {
                let mut events = Vec::new();
                self.close_block(&mut events);
                self.start_if_empty(&mut events);

                let id = self.resolve_tool_call_id(id, &name);
                let args_str = serde_json::to_string(&args).unwrap_or_default();
                let content_index = self.partial.content.len();

                self.partial.content.push(AssistantContent::ToolCall {
                    tool_call: ToolCall {
                        id: id.clone(),
                        name: name.clone(),
                        arguments: args_str.clone(),
                        arguments_error: None,
                        signature: signature.clone(),
                    },
                });

                events.push(Ok(AssistantStreamEvent::ToolCallStart {
                    content_index,
                    partial: self.partial.clone(),
                }));
                events.push(Ok(AssistantStreamEvent::ToolCallEnd {
                    content_index,
                    tool_call: ToolCall {
                        id,
                        name,
                        arguments: args_str,
                        arguments_error: None,
                        signature,
                    },
                    partial: self.partial.clone(),
                }));
                events
            }
            GeminiEvent::Finish { reason, usage } => {
                let mut events = Vec::new();
                self.close_block(&mut events);
                self.start_if_empty(&mut events);

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
                    _ => {
                        // Unmapped finish reasons are provider stops: the
                        // raw reason rides the typed failure text (pi
                        // `Provider stopped with: <raw>`).
                        self.saw_done = true;
                        let message = if reason.is_empty() {
                            "Provider stopped with an unknown finish reason".to_owned()
                        } else {
                            format!("Provider stopped with: {reason}")
                        };
                        return vec![Err(self.failure(ProviderError::StreamError(
                            ProviderErrorSummary::sanitized(message),
                        )))];
                    }
                };
                self.saw_done = true;

                events.push(Ok(AssistantStreamEvent::Done {
                    reason: self.partial.stop_reason,
                    message: self.partial.clone(),
                }));
                events
            }
            GeminiEvent::Error { message: _ } => {
                // Preserve state that did not produce a public event (usage,
                // opaque content or signatures), alongside the typed cause.
                self.saw_done = true;
                vec![Err(self.failure(ProviderError::StreamError(
                    ProviderErrorSummary::attested_static(UPSTREAM_STREAM_ERROR),
                )))]
            }
        }
    }
}

fn empty_assistant_message(
    provider: &str,
    wire: crate::model_info::WireApi,
    model: &str,
) -> AssistantMessage {
    AssistantMessage {
        content: Vec::new(),
        api: crate::ApiKind::Google,
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
        provider_thinking_level: None,
        deferred: None,
    }
}

// ---------------------------------------------------------------------------
// Google model capability helpers (task 20.40, pi google-shared.ts)
// ---------------------------------------------------------------------------

/// Major Gemini version of a model id (`gemini-3-pro`/`gemini-live-4` -> 3/4).
fn google_major_version(model_id: &str) -> Option<u32> {
    let id = model_id.to_ascii_lowercase();
    let rest = id
        .strip_prefix("gemini-live-")
        .or_else(|| id.strip_prefix("gemini-"))?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// Models behind Google APIs that require explicit tool call ids in
/// function calls/responses (pi `requiresToolCallId`).
fn requires_tool_call_id(model_id: &str) -> bool {
    let id = model_id.to_ascii_lowercase();
    id.starts_with("claude-")
        || id.starts_with("gpt-oss-")
        || google_major_version(model_id).is_some_and(|major| major >= 3)
}

/// Normalize a tool call id for the Google wire: `[a-zA-Z0-9_-]` only,
/// at most 64 chars (pi `normalizeToolCallId`).
fn normalize_tool_call_id(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .take(64)
        .collect()
}

/// Whether a model uses the discrete `thinkingLevel` control instead of
/// `thinkingBudget` (pi `usesGoogleThinkingLevel`): Gemini 3 pro/flash with
/// optional minor version, Gemini 4 and later, the two `*-latest` aliases
/// and both Gemma 4 naming forms.
fn uses_google_thinking_level(model_id: &str) -> bool {
    let id = model_id.to_ascii_lowercase();
    if id == "gemini-flash-latest" || id == "gemini-flash-lite-latest" {
        return true;
    }
    if id.starts_with("gemma-4") || id.starts_with("gemma4") {
        return true;
    }
    let Some(rest) = id.strip_prefix("gemini-") else {
        return false;
    };
    // gemini-3, gemini-3.1 (pro/flash), gemini-4 .. gemini-99
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    let Some(major) = digits.parse::<u32>().ok() else {
        return false;
    };
    if major >= 4 {
        return true;
    }
    if major == 3 {
        // Skip an optional minor version segment before the pro/flash tag.
        let after = &rest[digits.len()..];
        let after = after
            .strip_prefix('.')
            .map(|tail| {
                let skip: usize = tail.chars().take_while(char::is_ascii_digit).count();
                &tail[skip..]
            })
            .unwrap_or(after);
        return after.starts_with("-pro") || after.starts_with("-flash");
    }
    false
}

/// Gemini 3+ enforces required function parameters in validated
/// tool-calling modes (pi `supportsGoogleStrictToolSampling`).
fn supports_google_strict_tool_sampling(model_id: &str) -> bool {
    google_major_version(model_id).is_some_and(|major| major >= 3)
}

/// Neutral level budget tables (pi `getGoogleBudget`): custom budgets win,
/// then the model-family table, then the dynamic `-1`. The dedicated
/// flash-lite table is generative-ai only — pi's vertex `getGoogleBudget`
/// checks just 2.5-pro/2.5-flash, and its substring match routes
/// `2.5-flash-lite` ids through the flash table (e.g. minimal 128, not 512).
fn google_budget_for(model_id: &str, pi_level: &str, wire: WireApi) -> i64 {
    let id = model_id.to_ascii_lowercase();
    if id.contains("2.5-pro") {
        return match pi_level {
            "minimal" => 128,
            "low" => 2048,
            "medium" => 8192,
            _ => 32768,
        };
    }
    if wire == WireApi::GoogleGenerativeAi && id.contains("2.5-flash-lite") {
        return match pi_level {
            "minimal" => 512,
            "low" => 2048,
            "medium" => 8192,
            _ => 24576,
        };
    }
    if id.contains("2.5-flash") {
        return match pi_level {
            "minimal" => 128,
            "low" => 2048,
            "medium" => 8192,
            _ => 24576,
        };
    }
    -1
}

/// Resolve a neutral thinking level through the model's map to the four
/// pi levels (pi `resolveGoogleThinkingLevel`; typed failure on an
/// unsupported or non-pi mapping).
fn resolve_google_pi_level(
    model: &ModelInfo,
    level: crate::model_info::ThinkingLevel,
) -> Result<String, crate::provider::ProviderError> {
    let mapped = model
        .thinking_level_map
        .resolve(level)
        .map_err(|error| {
            crate::provider::ProviderError::Config(
                crate::provider::ProviderErrorSummary::sanitized(format!(
                    "unsupported Google thinking level mapping for gemini/{}: {}",
                    model.id, error.level
                )),
            )
        })?
        .unwrap_or_else(|| format!("{level:?}").to_ascii_lowercase());
    if GoogleApiThinkingLevel::from_pi_level(&mapped.to_ascii_lowercase()).is_some() {
        Ok(mapped.to_ascii_lowercase())
    } else {
        Err(crate::provider::ProviderError::Config(
            crate::provider::ProviderErrorSummary::sanitized(format!(
                "unsupported Google thinking level mapping for gemini/{}: {level:?} -> {mapped}",
                model.id
            )),
        ))
    }
}

/// Thought signatures must be base64 (TYPE_BYTES); only same-provider and
/// same-model signatures are usable (pi `resolveThoughtSignature`).
fn valid_thought_signature(signature: Option<&str>) -> bool {
    let Some(signature) = signature else {
        return false;
    };
    if signature.is_empty() || signature.len() % 4 != 0 {
        return false;
    }
    signature
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'=')
}

// ---------------------------------------------------------------------------
// GeminiProvider
// ---------------------------------------------------------------------------

/// Native tool-choice projection (task 20.40, pi `GoogleOptions.toolChoice`):
/// the Google wire has no named-tool mode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GoogleToolChoice {
    Auto,
    None_,
    Any,
}

impl GoogleToolChoice {
    fn wire_mode(&self) -> &'static str {
        match self {
            Self::Auto => "AUTO",
            Self::None_ => "NONE",
            Self::Any => "ANY",
        }
    }
}

/// Gemini 3 discrete thinking levels (pi `GoogleApiThinkingLevel`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GoogleApiThinkingLevel {
    Minimal,
    Low,
    Medium,
    High,
}

impl GoogleApiThinkingLevel {
    fn wire_name(&self) -> &'static str {
        match self {
            Self::Minimal => "MINIMAL",
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
        }
    }

    fn from_pi_level(value: &str) -> Option<Self> {
        match value {
            "minimal" => Some(Self::Minimal),
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            _ => None,
        }
    }
}

/// Native thinking intent (task 20.40, pi `GoogleOptions.thinking`):
/// `budget_tokens` keeps the source semantics untouched — `-1` is the
/// dynamic default and `0` disables; neither is reinterpreted locally.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GoogleThinkingConfig {
    pub enabled: bool,
    pub budget_tokens: Option<i64>,
    pub level: Option<GoogleApiThinkingLevel>,
}

/// Typed native options for one direct/native dispatch (task 20.40, pi
/// `GoogleOptions`). The neutral/simple entry maps only the tool choice;
/// thinking keeps the neutral request intent (level policy + model tables).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GoogleGenerativeAiOptions {
    pub tool_choice: Option<GoogleToolChoice>,
    pub thinking: Option<GoogleThinkingConfig>,
    simple: bool,
}

impl GoogleGenerativeAiOptions {
    /// The neutral simple entry (task 20.40): only the neutral tool choice
    /// maps (auto/none/any); the neutral request intent drives thinking.
    pub fn from_simple(simple: &crate::models::SimpleOptions) -> Self {
        Self::native(
            None,
            simple.tool_choice.map(|choice| match choice {
                crate::models::SimpleToolChoice::Auto => GoogleToolChoice::Auto,
                crate::models::SimpleToolChoice::None => GoogleToolChoice::None_,
            }),
        )
        .marked_simple()
    }

    pub fn is_simple(&self) -> bool {
        self.simple
    }

    /// Construct a native (non-simple) options value: `thinking: None`
    /// keeps the neutral request intent on the wire.
    pub fn native(
        thinking: Option<GoogleThinkingConfig>,
        tool_choice: Option<GoogleToolChoice>,
    ) -> Self {
        Self {
            thinking,
            tool_choice,
            simple: false,
        }
    }

    pub(crate) fn marked_simple(mut self) -> Self {
        self.simple = true;
        self
    }
}

#[derive(Clone)]
pub struct GeminiProvider {
    base_url: String,
    models: Vec<ModelInfo>,
    client: Arc<HttpClient>,
    /// Typed native options for one direct/native dispatch (task 20.40);
    /// `None` keeps the plain provider path.
    native_options: Option<GoogleGenerativeAiOptions>,
    /// Owning Google wire (task 20.41): the vertex adapter shares this body
    /// engine but pins per-wire differences (the neutral budget table).
    wire: WireApi,
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
                .with_streaming(true)
                .with_thinking(true),
        ),
        ModelInfo::new(
            "gemini-2.5-pro",
            "Gemini 2.5 Pro",
            WireApi::GoogleGenerativeAi,
            ModelCapabilities::new(1_000_000, 65536)
                .with_images(true)
                .with_streaming(true)
                .with_thinking(true),
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
            native_options: None,
            wire: WireApi::GoogleGenerativeAi,
        }
    }

    /// Pin the owning Google wire (task 20.41): the vertex transport reuses
    /// this body engine under `WireApi::GoogleVertex` for the per-wire
    /// neutral-policy branches (budget table).
    pub(crate) fn with_wire(mut self, wire: WireApi) -> Self {
        self.wire = wire;
        self
    }

    /// Override the catalog records (task 20.40): direct adapters and tests
    /// bind complete custom records; the builtin catalog stays untouched.
    pub fn with_models(mut self, models: Vec<ModelInfo>) -> Self {
        self.models = models;
        self
    }

    /// Override the API base URL (task 20.40).
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Bind typed native options for the dispatches of this provider clone
    /// (task 20.40); the plain constructor keeps `None`.
    pub fn with_native_options(mut self, options: GoogleGenerativeAiOptions) -> Self {
        self.native_options = Some(options);
        self
    }

    /// Access the shared HTTP client.
    pub fn http_client(&self) -> &Arc<HttpClient> {
        &self.client
    }

    /// Build the Gemini `generateContent` request body.
    /// The model ID goes in the URL path, not the body.
    pub fn build_request_body(
        &self,
        request: &Request,
    ) -> Result<serde_json::Value, crate::provider::ProviderError> {
        let model_id = request
            .model
            .split_once(':')
            .map(|(_, id)| id)
            .unwrap_or(&request.model);

        let mut contents = Vec::new();

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
        // Task 20.11 (AIR-040, fail-closed): a require intent on a wire
        // without strict projection fails typed before dispatch.
        crate::constrained_sampling::assert_supported_constraints(&current_tools, false).map_err(
            |error| {
                crate::provider::ProviderError::Config(
                    crate::provider::ProviderErrorSummary::sanitized(error.detail()),
                )
            },
        )?;

        // Canonical replay projection (task 20.7) before body building.
        let replay = crate::replay::project_replay(
            &crate::transcript::without_initial_system_message(transcript.messages()),
            &crate::replay::ReplayRoute {
                wire: crate::model_info::WireApi::GoogleGenerativeAi,
                provider: "gemini".to_owned(),
                model: model_id.to_owned(),
                supports_images: crate::replay::supports_images_in_catalog(&self.models, model_id),
            },
            crate::replay::ReplayProjection::NativeOrCollapse,
        )?;
        let body_messages = replay.messages;

        for msg in &body_messages {
            match msg {
                crate::message::Message::System(_) => {
                    // Unreachable after collapse + without_initial_system_message.
                    unreachable!("system messages are projected out of contents")
                }
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
                    // Signatures replay only for same-provider/same-model
                    // history and only in valid base64 (task 20.40).
                    let same_model = a.provider == "gemini" && a.model == model_id;
                    let usable = |signature: &Option<String>| {
                        (same_model && valid_thought_signature(signature.as_deref())).then(|| {
                            serde_json::Value::String(signature.clone().unwrap_or_default())
                        })
                    };
                    let include_id = requires_tool_call_id(model_id);
                    let parts: Vec<serde_json::Value> = a
                        .content
                        .iter()
                        .filter_map(|c| match c {
                            AssistantContent::Text { text, signature } => {
                                // Empty text survives only when it carries a
                                // usable signature (pi keeps signature-only
                                // parts; dropping them breaks the chain).
                                if text.trim().is_empty() && usable(signature).is_none() {
                                    return None;
                                }
                                let mut part = serde_json::json!({"text": text});
                                if let Some(value) = usable(signature) {
                                    part["thoughtSignature"] = value;
                                }
                                Some(part)
                            }
                            AssistantContent::Thinking {
                                thinking,
                                signature,
                            } => {
                                if same_model {
                                    if thinking.trim().is_empty() && usable(signature).is_none() {
                                        return None;
                                    }
                                    let mut part =
                                        serde_json::json!({"thought": true, "text": thinking});
                                    if let Some(value) = usable(signature) {
                                        part["thoughtSignature"] = value;
                                    }
                                    Some(part)
                                } else {
                                    // Cross-model thinking replays as plain
                                    // text without tags; unusable signatures
                                    // drop entirely.
                                    if thinking.trim().is_empty() {
                                        return None;
                                    }
                                    Some(serde_json::json!({"text": thinking}))
                                }
                            }
                            AssistantContent::ToolCall { tool_call } => {
                                // Convert to functionCall response part.
                                // Replay guarantees remaining calls
                                // finalized as complete JSON objects
                                // (malformed and legacy unmarked ones
                                // were projected away); parsing cannot
                                // fabricate a null substitute.
                                let args: serde_json::Value =
                                    serde_json::from_str(&tool_call.arguments)
                                        .expect("replay projected only complete object arguments");
                                let mut function_call = serde_json::json!({
                                    "name": tool_call.name,
                                    "args": args,
                                });
                                if include_id {
                                    function_call["id"] =
                                        serde_json::json!(normalize_tool_call_id(&tool_call.id));
                                }
                                let mut part = serde_json::json!({"functionCall": function_call});
                                if let Some(value) = usable(&tool_call.signature) {
                                    part["thoughtSignature"] = value;
                                }
                                Some(part)
                            }
                            AssistantContent::RedactedThinking { .. } => None,
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
                    // Pi convertMessages: success responses use the
                    // `output` key with the text value, failures the
                    // `error` key; images nest inside functionResponse
                    // parts on Gemini 3+, and ride a separate labeled user
                    // turn below Gemini 3 (task 20.40).
                    let texts: Vec<&str> = t
                        .content
                        .iter()
                        .filter_map(|c| match c {
                            OutputContent::Text { text } => Some(text.as_str()),
                            OutputContent::Image { .. } => None,
                        })
                        .collect();
                    let images: Vec<(&crate::message::ImageSource, &crate::message::MediaType)> = t
                        .content
                        .iter()
                        .filter_map(|c| match c {
                            OutputContent::Image { source, media_type } => {
                                Some((source, media_type))
                            }
                            OutputContent::Text { .. } => None,
                        })
                        .collect();
                    let has_images = !images.is_empty();
                    let value = if !texts.is_empty() {
                        texts.join("\n")
                    } else if has_images {
                        "(see attached image)".to_owned()
                    } else {
                        String::new()
                    };
                    let mut function_response = serde_json::json!({
                        "name": t.tool_name,
                        "response": if t.is_error {
                            serde_json::json!({"error": value})
                        } else {
                            serde_json::json!({"output": value})
                        },
                    });
                    if requires_tool_call_id(model_id) {
                        function_response["id"] =
                            serde_json::json!(normalize_tool_call_id(&t.tool_call_id));
                    }
                    // Google REST image parts are camelCase on these new
                    // task-20.40 paths (pi SDK serialization).
                    let inline_images: Vec<serde_json::Value> = images
                        .iter()
                        .map(|(source, media_type)| match source {
                            crate::message::ImageSource::Url { url } => serde_json::json!({
                                "fileData": {"fileUri": url, "mimeType": media_type.as_str()}
                            }),
                            crate::message::ImageSource::Base64 { data } => serde_json::json!({
                                "inlineData": {"mimeType": media_type.as_str(), "data": data}
                            }),
                            crate::message::ImageSource::Bytes { data } => serde_json::json!({
                                "inlineData": {
                                    "mimeType": media_type.as_str(),
                                    "data": base64::Engine::encode(
                                        &base64::engine::general_purpose::STANDARD,
                                        data,
                                    ),
                                }
                            }),
                        })
                        .collect();
                    if has_images && supports_google_strict_tool_sampling(model_id) {
                        // Multimodal function responses: Gemini 3+ nests the
                        // image parts inside the functionResponse itself.
                        function_response["parts"] =
                            serde_json::Value::Array(inline_images.clone());
                    }
                    let part = serde_json::json!({"functionResponse": function_response});

                    // Consecutive function responses share one user turn
                    // (pi merges them into the last response turn).
                    let merged = contents.last_mut().is_some_and(|last| {
                        last.get("role") == Some(&serde_json::json!("user"))
                            && last["parts"].as_array().is_some_and(|parts| {
                                parts.iter().any(|p| p.get("functionResponse").is_some())
                            })
                    }) && {
                        if let Some(parts) = contents
                            .last_mut()
                            .and_then(|last| last["parts"].as_array_mut())
                        {
                            parts.push(part.clone());
                            true
                        } else {
                            false
                        }
                    };
                    if !merged {
                        contents.push(serde_json::json!({
                            "role": "user",
                            "parts": [part],
                        }));
                    }

                    // Below Gemini 3 the images cannot ride the response:
                    // they follow as a separate labeled user turn.
                    if has_images && !supports_google_strict_tool_sampling(model_id) {
                        contents.push(serde_json::json!({
                            "role": "user",
                            "parts": [
                                {"text": "Tool result image:"},
                            ],
                        }));
                        if let Some(parts) = contents
                            .last_mut()
                            .and_then(|last| last["parts"].as_array_mut())
                        {
                            parts.extend(inline_images);
                        }
                    }
                }
            }
        }

        let mut body = serde_json::json!({
            "contents": contents,
        });

        // System instruction is a separate object
        if !system_prompt.is_empty() {
            body["systemInstruction"] = serde_json::json!({
                "parts": [{"text": system_prompt}]
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

        // Tools (task 20.40): declarations ride the original JSON Schema via
        // `parametersJsonSchema`; strict-capable models (Gemini 3+) project
        // prefer/require strict and select VALIDATED function calling.
        if !current_tools.is_empty() {
            use crate::constrained_sampling::{
                NoRejectedKeywords, make_strict_json_schema, resolve_json_schema_strict_sampling,
            };
            let supports_strict = supports_google_strict_tool_sampling(model_id);
            let mut any_strict = false;
            let declarations: Vec<serde_json::Value> = current_tools
                .iter()
                .map(|tool| {
                    let strict = resolve_json_schema_strict_sampling(
                        tool,
                        supports_strict,
                        &NoRejectedKeywords,
                    )
                    .map_err(|error| {
                        crate::provider::ProviderError::Config(
                            crate::provider::ProviderErrorSummary::sanitized(error.detail()),
                        )
                    })?;
                    let schema = if strict == Some(true) {
                        any_strict = true;
                        make_strict_json_schema(&tool.input_schema, &NoRejectedKeywords)
                            .expect("resolve succeeded so the conversion succeeds")
                    } else {
                        tool.input_schema.clone()
                    };
                    Ok(serde_json::json!({
                        "name": tool.name,
                        "description": tool.description,
                        "parametersJsonSchema": schema,
                    }))
                })
                .collect::<Result<Vec<_>, crate::provider::ProviderError>>()?;
            body["tools"] = serde_json::Value::Array(vec![serde_json::json!({
                "functionDeclarations": declarations
            })]);

            // Pi `resolveGoogleFunctionCallingMode`: explicit none/any win,
            // then VALIDATED when any tool went strict, then an explicit
            // auto; absent intent emits no toolConfig.
            let mode = self
                .native_options
                .as_ref()
                .and_then(|options| options.tool_choice.as_ref())
                .map(|choice| choice.wire_mode());
            let mode = match mode {
                Some(mode @ ("NONE" | "ANY")) => Some(mode),
                _ if any_strict => Some("VALIDATED"),
                other => other,
            };
            if let Some(mode) = mode {
                body["toolConfig"] = serde_json::json!({
                    "functionCallingConfig": {"mode": mode}
                });
            }
        }

        // Thinking config (task 20.40): native options own the wire when
        // bound; the neutral request intent drives the plain path.
        let model_info = self.models.iter().find(|m| m.id == model_id);
        let reasoning = model_info
            .map(|m| m.capabilities.supports_thinking)
            .unwrap_or(false);
        // Native binding owns the thinking intent entirely: a bound native
        // options value without `thinking` omits the config (pi native
        // `buildParams` only reacts when `options.thinking` exists).
        let native_bound = self
            .native_options
            .as_ref()
            .is_some_and(|options| !options.is_simple());
        let native_thinking = if native_bound {
            self.native_options
                .as_ref()
                .and_then(|options| options.thinking.as_ref())
        } else {
            None
        };
        let thinking_config: Option<serde_json::Value> = if !reasoning {
            None
        } else if let Some(thinking) = native_thinking {
            if thinking.enabled {
                let mut config = serde_json::json!({"includeThoughts": true});
                if let Some(level) = thinking.level.as_ref() {
                    config["thinkingLevel"] = serde_json::json!(level.wire_name());
                } else if let Some(budget) = thinking.budget_tokens {
                    config["thinkingBudget"] = serde_json::json!(budget);
                }
                Some(config)
            } else if uses_google_thinking_level(model_id) {
                // Level-mode models disable via the mapped off level when
                // the map provides one, else budget zero (pi
                // getDisabledGoogleThinkingConfig).
                let off_level = model_info.and_then(|model| {
                    model
                        .thinking_level_map
                        .resolve(crate::model_info::ThinkingLevel::None)
                        .ok()
                        .flatten()
                });
                match off_level
                    .as_deref()
                    .and_then(GoogleApiThinkingLevel::from_pi_level)
                {
                    Some(level) => Some(serde_json::json!({"thinkingLevel": level.wire_name()})),
                    None => Some(serde_json::json!({"thinkingBudget": 0})),
                }
            } else {
                Some(serde_json::json!({"thinkingBudget": 0}))
            }
        } else if native_bound {
            // Native binding without a thinking intent: no config.
            None
        } else if request.thinking.enabled {
            let model = model_info.expect("reasoning models resolve a record");
            let pi_level = resolve_google_pi_level(model, request.thinking.level)?;
            let mut config = serde_json::json!({"includeThoughts": true});
            if uses_google_thinking_level(model_id) {
                let level = GoogleApiThinkingLevel::from_pi_level(&pi_level)
                    .expect("resolve validated one of the four pi levels");
                config["thinkingLevel"] = serde_json::json!(level.wire_name());
            } else {
                let budget = request
                    .thinking
                    .budget_tokens
                    .map(|budget| budget as i64)
                    .unwrap_or_else(|| google_budget_for(model_id, &pi_level, self.wire));
                config["thinkingBudget"] = serde_json::json!(budget);
            }
            Some(config)
        } else {
            // Neutral explicit off (pi streamSimple always sends the
            // disabled config on reasoning models).
            if uses_google_thinking_level(model_id) {
                let off_level = model_info.and_then(|model| {
                    model
                        .thinking_level_map
                        .resolve(crate::model_info::ThinkingLevel::None)
                        .ok()
                        .flatten()
                });
                match off_level
                    .as_deref()
                    .and_then(GoogleApiThinkingLevel::from_pi_level)
                {
                    Some(level) => Some(serde_json::json!({"thinkingLevel": level.wire_name()})),
                    None => Some(serde_json::json!({"thinkingBudget": 0})),
                }
            } else {
                Some(serde_json::json!({"thinkingBudget": 0}))
            }
        };
        if let Some(config) = thinking_config {
            body["thinkingConfig"] = config;
        }

        Ok(body)
    }

    /// Stream events from a raw SSE response body.
    pub fn stream_from_sse(&self, sse_body: &str, cancel: CancellationToken) -> EventStream {
        let mut mapper =
            GeminiMapper::new("gemini", crate::model_info::WireApi::GoogleGenerativeAi, "");
        let mut stream_events: Vec<Result<AssistantStreamEvent, ProviderError>> = Vec::new();

        for data in parse_sse_data(sse_body) {
            for parsed in ParsedEvent::from_data(&data) {
                match parsed {
                    ParsedEvent::Valid(event) => {
                        stream_events.extend(mapper.process(event));
                    }
                    ParsedEvent::Malformed => {
                        stream_events.push(Err(mapper.failure(ProviderError::StreamError(
                            ProviderErrorSummary::attested_static(MALFORMED_STREAM_FRAME),
                        ))));
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
        callbacks: crate::callbacks::RequestCallbacks,
        snapshot: crate::stream::ProducerSnapshot,
        tx: &tokio::sync::mpsc::Sender<Result<AssistantStreamEvent, ProviderError>>,
    ) -> Result<(), ProviderError> {
        let url = format!("{base_url}/v1beta/models/{model_id}:streamGenerateContent?alt=sse");
        let route_headers = vec![
            ("x-goog-api-key".to_owned(), api_key),
            ("content-type".to_owned(), "application/json".to_owned()),
        ];
        let merged = ProviderHeaders::default().merge_request(&route_headers, &extra_headers)?;
        // Header transform (task 20.51) on the transformable per-call set; the
        // protected identity/wire headers are stamped back after the
        // transform and never enter it.
        let transformable: Vec<(String, String)> = merged
            .iter()
            .filter(|(name, _)| {
                !route_headers
                    .iter()
                    .any(|(managed, _)| name.eq_ignore_ascii_case(managed))
            })
            .cloned()
            .collect();
        let transformed = callbacks
            .apply_header_transform_async(&transformable)
            .await?;
        let mut headers = route_headers.clone();
        headers.extend(transformed);
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

        snapshot.received_response();
        let mut byte_stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut decoder = crate::text_decode::StreamTextDecoder::new();
        let mut mapper = GeminiMapper::new(
            "gemini",
            crate::model_info::WireApi::GoogleGenerativeAi,
            &model_id,
        );

        mapper.set_snapshot(snapshot);
        let result: Result<(), ProviderError> = async {
            loop {
                let chunk = tokio::select! {
                    biased;
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

                let chunk = chunk.map_err(|error| {
                    if error.is_timeout() {
                        ProviderError::Timeout
                    } else {
                        ProviderError::StreamError(ProviderErrorSummary::sanitized(
                            error.to_string(),
                        ))
                    }
                })?;
                buffer.push_str(&decoder.push(&chunk));

                for data in drain_sse_payloads(&mut buffer) {
                    if let Ok(raw) = serde_json::from_str::<serde_json::Value>(&data) {
                        callbacks.notify_raw_async(&raw).await?;
                    }
                    for parsed in ParsedEvent::from_data(&data) {
                        match parsed {
                            ParsedEvent::Valid(event) => {
                                for item in mapper.process(event) {
                                    if !send_or_cancel(&cancel, tx, item).await? {
                                        return Ok(());
                                    }
                                }
                            }
                            ParsedEvent::Malformed => {
                                let err = ProviderError::StreamError(
                                    ProviderErrorSummary::attested_static(MALFORMED_STREAM_FRAME),
                                );
                                if !send_or_cancel(&cancel, tx, Err(mapper.failure(err))).await? {
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            }

            let eof_tail = decoder.finish();
            buffer.push_str(&eof_tail);
            if !mapper.saw_done {
                let err = ProviderError::StreamError(ProviderErrorSummary::attested_static(
                    "stream ended without a terminal event",
                ));
                let _ = send_or_cancel(&cancel, tx, Err(mapper.failure(err))).await?;
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

/// Drain complete SSE events from the buffer (delimited by `\n\n`).
pub(crate) fn drain_sse_payloads(buffer: &mut String) -> Vec<String> {
    if buffer.contains('\r') {
        *buffer = buffer.replace("\r\n", "\n").replace('\r', "\n");
    }

    let mut events = Vec::new();
    while let Some(idx) = buffer.find("\n\n") {
        let end = idx + 2;
        let chunk: String = buffer.drain(..end).collect();
        for data in parse_sse_data(&chunk) {
            events.push(data);
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
    fn stream_prepared(
        &self,
        mut request: Request,
        auth: crate::auth::ResolvedAuth,
    ) -> EventStream {
        request.callbacks = request.callbacks.ensure_call_scope();
        // Google exposes parsed raw chunks but has no HTTP response hook.
        if request.callbacks.has_response_observer() {
            return Box::pin(stream::iter(vec![Err(
                ProviderError::UnsupportedCapability(ProviderErrorSummary::attested_static(
                    "Google wires do not support response callbacks",
                )),
            )]));
        }
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
        let body = match self.build_request_body(&request) {
            Ok(body) => body,
            Err(error) => return Box::pin(stream::once(async move { Err(error) })),
        };
        let callbacks = request.callbacks.clone();
        let cancel = request.cancel.clone();
        let timeout = request.timeout;
        let extra_headers = request.extra_headers;
        let http_client = self.client.client().clone();

        let snapshot = crate::stream::ProducerSnapshot::default();
        let producer_snapshot = snapshot.clone();
        let (tx, rx) = tokio::sync::mpsc::channel(64);
        let producer_cancel = cancel.clone();
        // Explicit frozen telemetry context (task 20.54/AIR-055): the producer
        // task runs under the captured parent/Dispatch when provided;
        // omission observes nothing.
        let telemetry = request.telemetry_context.clone();

        let tx_closed = tx.clone();
        let producer = async move {
            let run = async {
                let result = async {
                    let body = callbacks
                        .apply_payload_async(crate::callbacks::ORDINARY_PAYLOAD_VARIANT, body)
                        .await?;
                    Self::stream_http(
                        http_client,
                        api_key,
                        base_url,
                        model_id,
                        &body,
                        producer_cancel.clone(),
                        timeout,
                        extra_headers,
                        callbacks,
                        producer_snapshot,
                        &tx,
                    )
                    .await
                }
                .await;
                match result {
                    Ok(()) | Err(ProviderError::Cancelled) => {}
                    Err(error) => {
                        let _ = send_or_cancel(&producer_cancel, &tx, Err(error)).await;
                    }
                }
            };
            tokio::select! {
                biased;
                _ = producer_cancel.cancelled() => (),
                _ = tx_closed.closed() => (),
                _ = run => (),
            }
        };
        if let Some(context) = telemetry {
            tokio::spawn(context.instrument(producer));
        } else {
            tokio::spawn(producer);
        }

        Box::pin(CancelAwareReceiverStream::with_snapshot(
            rx, cancel, snapshot,
        ))
    }

    fn id(&self) -> &str {
        "gemini"
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }
}

// ---------------------------------------------------------------------------
// Public direct adapter (task 20.40)
// ---------------------------------------------------------------------------

/// The `google-generative-ai` direct Chat adapter (task 20.40, DEL-11).
///
/// Binds one complete Chat model record and serves the typed native
/// [`GoogleGenerativeAiOptions`] inventory through the four `Models`
/// entries (`register_chat_api` / native complete-stream / neutral simple
/// complete-stream).
pub struct GoogleGenerativeAiChatApi {
    api_id: crate::models::ApiId,
    provider: Arc<GeminiProvider>,
    model: ModelInfo,
}

impl std::fmt::Debug for GoogleGenerativeAiChatApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GoogleGenerativeAiChatApi")
            .field("api_id", &self.api_id)
            .finish_non_exhaustive()
    }
}

impl GoogleGenerativeAiChatApi {
    /// Bind a complete Chat record. Wrong-wire or invalid records fail
    /// before authentication or network activity.
    pub fn new(
        provider: Arc<GeminiProvider>,
        model: ModelInfo,
    ) -> Result<Self, crate::provider::ProviderError> {
        validate_complete_google_record(&model)?;
        Ok(Self {
            api_id: crate::models::ApiId::new("google-generative-ai"),
            provider,
            model,
        })
    }

    fn configured(&self, model: ModelInfo, options: GoogleGenerativeAiOptions) -> GeminiProvider {
        let mut provider = (*self.provider).clone();
        provider.models = vec![model];
        provider.native_options = Some(options);
        provider
    }
}

/// One complete, wire-valid Chat model record or a typed failure.
fn validate_complete_google_record(
    model: &ModelInfo,
) -> Result<(), crate::provider::ProviderError> {
    if model.wire_api != crate::model_info::WireApi::GoogleGenerativeAi
        || model.id.trim().is_empty()
        || model.validate().is_err()
    {
        return Err(crate::provider::ProviderError::Config(
            crate::provider::ProviderErrorSummary::attested_static(
                "google-generative-ai adapters require a valid complete Chat model record",
            ),
        ));
    }
    Ok(())
}

impl crate::models::ChatApi<GoogleGenerativeAiOptions> for GoogleGenerativeAiChatApi {
    fn validate_simple_options(
        &self,
        model: &ModelInfo,
        _options: &crate::models::SimpleOptions,
    ) -> Result<(), crate::provider::ProviderError> {
        validate_complete_google_record(model)
    }

    fn api_id(&self) -> &crate::models::ApiId {
        &self.api_id
    }

    fn provider_id(&self) -> &str {
        "gemini"
    }

    fn complete_record(&self) -> ModelInfo {
        self.model.clone()
    }

    fn simple_options(&self) -> GoogleGenerativeAiOptions {
        GoogleGenerativeAiOptions::from_simple(&crate::models::SimpleOptions::default())
    }

    fn simple_options_for(
        &self,
        options: &crate::models::SimpleOptions,
    ) -> GoogleGenerativeAiOptions {
        GoogleGenerativeAiOptions::from_simple(options)
    }

    fn session_resources(&self) -> Option<&dyn crate::codex_resources::SessionResourceOwner> {
        None
    }

    fn validate_native_request(
        &self,
        model: &ModelInfo,
        request: &Request,
        _options: &GoogleGenerativeAiOptions,
    ) -> Result<(), crate::provider::ProviderError> {
        validate_complete_google_record(model)?;
        crate::provider::validate_request_for_model(self.provider_id(), Some(model), request)
    }

    fn stream_prepared(
        &self,
        request: Request,
        options: GoogleGenerativeAiOptions,
        auth: crate::auth::ResolvedAuth,
    ) -> EventStream {
        self.stream_model_resource_prepared(self.model.clone(), request, options, auth, None)
    }

    fn stream_resource_prepared(
        &self,
        request: Request,
        options: GoogleGenerativeAiOptions,
        auth: crate::auth::ResolvedAuth,
        resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        self.stream_model_resource_prepared(self.model.clone(), request, options, auth, resources)
    }

    fn stream_model_resource_prepared(
        &self,
        model: ModelInfo,
        mut request: Request,
        options: GoogleGenerativeAiOptions,
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
        // The Google wire holds no pooled session resources today; a
        // resource lease still dispatches through the ordinary producer.
        let _ = resources;
        provider.stream_prepared(request, auth)
    }
}
