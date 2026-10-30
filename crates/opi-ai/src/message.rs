//! Provider-facing message types (S7.1).

use serde::{Deserialize, Serialize};

/// Deterministic failure marker prefixed to OpenAI-family tool-result text when
/// a provider wire format has no native tool-error field.
pub(crate) const TOOL_ERROR_MARKER: &str = "[tool_error] ";

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "role")]
pub enum Message {
    #[serde(rename = "user")]
    User(UserMessage),
    #[serde(rename = "system")]
    System(crate::transcript::SystemMessage),
    #[serde(rename = "assistant")]
    Assistant(AssistantMessage),
    #[serde(rename = "tool_result")]
    ToolResult(ToolResultMessage),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserMessage {
    pub content: Vec<InputContent>,
    pub timestamp_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub content: Vec<AssistantContent>,
    pub api: crate::ApiKind,
    pub provider: String,
    pub model: String,
    /// Reported model override: `None` falls back to `model`; an empty string
    /// explicitly records that the provider did not report a model.
    pub response_model: Option<String>,
    /// Exact wire the turn was produced on (continuation provenance,
    /// AIR-010/RP-02). `None` marks legacy or synthesized records with
    /// unknown provenance: replay never treats them as same-identity
    /// (fail-closed; `ApiKind` alone cannot distinguish the
    /// Completions/Responses/Codex wires).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wire: Option<crate::model_info::WireApi>,
    pub response_id: Option<String>,
    pub usage: crate::stream::Usage,
    pub stop_reason: crate::stream::StopReason,
    pub error_message: Option<String>,
    pub timestamp_ms: i64,
    /// Message-level constrained-sampling diagnostics (task 20.11,
    /// V-34/V-36): at most one recorded strict-prefer downgrade with its
    /// reason, plus grammar-target fallback losses. Default empty;
    /// historical messages deserialize without it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<crate::stream::AssistantDiagnostic>,
    /// Monotonic adapter response-stream duration (AIR-061): measured from
    /// response-stream activation/first body bytes to terminal. `None` when
    /// not measured (setup failures before activation, older records);
    /// forwarding and serialization retain known values without recomputation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Wire-native effort this reply ran under (task 20.37, P20-S37):
    /// stamped only when the wire replays the thinking level as mid-
    /// conversation effort items; replay re-emits it before this turn's
    /// items. `None` on wires without that mechanism.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_thinking_level: Option<String>,
    /// Deferred-response handle for a pending submission (task 20.50,
    /// AIR-043): present exactly when `stop_reason` is `Deferred`. The
    /// handle is source-owned identity plus provider advice and never
    /// carries a credential.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deferred: Option<Box<crate::deferred::DeferredHandle>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResultMessage {
    pub tool_call_id: String,
    pub tool_name: String,
    pub content: Vec<OutputContent>,
    pub details: Option<serde_json::Value>,
    pub is_error: bool,
    /// Known non-execution recorded by the caller's trusted execution boundary.
    /// Absence leaves execution effects unknown; neither `is_error` nor result
    /// text establishes this fact. This record grants no execution permission.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_executed: Option<ToolNotExecutedReason>,
    #[serde(default)]
    pub truncated: bool,
    pub timestamp_ms: i64,
}

/// Closed reasons a trusted caller may record before a tool has launched.
/// These facts survive canonical serialization and never originate in model text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolNotExecutedReason {
    InvalidJson,
    NonObjectArguments,
    SchemaValidationFailed,
    OutputLimit,
    InterruptedBeforeExecution,
    NotActive,
}

/// Supported image media types for user-supplied images.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaType {
    #[serde(rename = "image/png")]
    Png,
    #[serde(rename = "image/jpeg")]
    Jpeg,
    #[serde(rename = "image/gif")]
    Gif,
    #[serde(rename = "image/webp")]
    WebP,
}

impl MediaType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Gif => "image/gif",
            Self::WebP => "image/webp",
        }
    }
}

/// Source of image data: URL reference, base64-encoded string, or raw bytes.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ImageSource {
    #[serde(rename = "url")]
    Url { url: String },
    #[serde(rename = "base64")]
    Base64 { data: String },
    #[serde(rename = "bytes")]
    Bytes { data: Vec<u8> },
}

// ---------------------------------------------------------------------------
// Visible text extraction (task 20.53.1, P20-S53-1)
//
// Pinned source: `.repo/pi-1.3.0/packages/ai/src/utils/text.ts` `contentText`:
// extract and join the text blocks of message content, in exact block order,
// joined by newlines. Everything else — images, thinking in both forms, tool
// calls — is not visible text and never leaks into the rendered string.
// These helpers are read-only: they borrow content and never mutate it.

/// Extract and join the visible text of user-message content blocks, in
/// block order, separated by single newlines. Images contribute nothing.
pub fn content_text(content: &[InputContent]) -> String {
    content
        .iter()
        .filter_map(|block| match block {
            InputContent::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Extract and join the visible text of assistant-message content blocks,
/// in block order, separated by single newlines. Thinking (plain and
/// redacted) and tool calls are never exposed by this rendering.
pub fn assistant_content_text(content: &[AssistantContent]) -> String {
    content
        .iter()
        .filter_map(|block| match block {
            AssistantContent::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Extract and join the visible text of tool-result content blocks, in
/// block order, separated by single newlines. Images contribute nothing.
pub fn output_content_text(content: &[OutputContent]) -> String {
    content
        .iter()
        .filter_map(|block| match block {
            OutputContent::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum InputContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image {
        source: ImageSource,
        media_type: MediaType,
    },
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AssistantContent {
    #[serde(rename = "text")]
    Text {
        text: String,
        /// Opaque provider thought signature riding the text block
        /// (task 20.40, Gemini thought signatures): replay echoes it on the
        /// same wire part for same-provider/same-model history only.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        signature: Option<String>,
    },
    #[serde(rename = "thinking")]
    Thinking {
        thinking: String,
        /// Opaque provider signature; preserve with the original thinking text.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        signature: Option<String>,
    },
    /// Encrypted reasoning retained for continuation, never display text.
    #[serde(rename = "redacted_thinking")]
    RedactedThinking { data: String },
    #[serde(rename = "tool_call")]
    ToolCall { tool_call: ToolCall },
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OutputContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image {
        source: ImageSource,
        media_type: MediaType,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
    /// Final-parse outcome for textual tool arguments (AIR-048).
    ///
    /// `None` means `arguments` finalized as a complete JSON object (the
    /// wire-raw text when it already parsed, `{}` for empty input, or the
    /// escape-repaired text when only invalid escapes were repaired). Some
    /// marks the final text as malformed or non-object: `arguments` keeps the
    /// owned raw model-sent text and the call must not be executed as a
    /// partial-prefix success. Provisional streaming deltas never set this
    /// field. Historical records without the field deserialize as `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments_error: Option<String>,
    /// Opaque provider thought signature for this call (task 20.40, Gemini
    /// thought signatures): replay echoes it on the functionCall part for
    /// same-provider/same-model history only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

impl ToolCall {
    /// Finalize textual tool arguments on the five text-argument wire sites
    /// (Anthropic, Bedrock, Mistral, Chat and shared Responses; AIR-048).
    ///
    /// Returns `(arguments, arguments_error)`: empty input finalizes to `{}`
    /// without an error; a complete JSON object keeps its owned raw text;
    /// complete objects with invalid escapes or raw control characters in
    /// strings are repaired without truncating later fields; any other final
    /// text (incomplete, non-object) keeps the owned raw text and carries a
    /// redacted-safe `arguments_error`, never a partial-prefix success.
    pub(crate) fn finalize_text_arguments(raw: String) -> (String, Option<String>) {
        const NOT_AN_OBJECT: &str = "final tool arguments were not a complete JSON object";
        if raw.trim().is_empty() {
            return ("{}".to_owned(), None);
        }
        if serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&raw).is_ok() {
            return (raw, None);
        }
        let repaired = repair_json(&raw);
        if serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&repaired).is_ok() {
            return (repaired, None);
        }
        (raw, Some(NOT_AN_OBJECT.to_owned()))
    }
}

/// Finalize the complete arguments text of a finished tool call under the
/// public AIR-048 contract (task 20.53.2; delegates to the wire-shared
/// `ToolCall::finalize_text_arguments`).
///
/// Returns `(arguments, arguments_error)`: empty input finalizes to `{}`
/// without an error; a complete JSON object keeps its owned raw text;
/// complete objects with invalid escapes or control characters in strings
/// return repaired text; any other final text keeps the owned raw text and
/// carries a redacted-safe `arguments_error` that never echoes model-sent text.
/// The final result is authoritative: a streaming display parse never
/// overrides it.
pub fn finalize_tool_arguments(raw: String) -> (String, Option<String>) {
    ToolCall::finalize_text_arguments(raw)
}

// ---------------------------------------------------------------------------
// Public JSON repair and partial-parse utilities (task 20.53.2, P20-S53-2)
//
// Pinned source: `.repo/pi-1.3.0/packages/ai/src/utils/json-parse.ts`.

const VALID_JSON_ESCAPES: [char; 8] = ['"', '\\', '/', 'b', 'f', 'n', 'r', 't'];

fn is_json_control_character(ch: char) -> bool {
    (ch as u32) <= 0x1f
}

fn escape_json_control_character(ch: char) -> String {
    match ch {
        '\u{08}' => "\\b".to_owned(),
        '\u{0c}' => "\\f".to_owned(),
        '\n' => "\\n".to_owned(),
        '\r' => "\\r".to_owned(),
        '\t' => "\\t".to_owned(),
        ch => format!("\\u{:04x}", ch as u32),
    }
}

/// Repair malformed JSON string literals (pinned `repairJson`).
///
/// Inside string literals only: raw control characters become valid JSON
/// escapes (`\b`/`\f`/`\n`/`\r`/`\t` or `\u00xx`), a backslash before an
/// invalid escape character is doubled so the model-sent text stays
/// literal, `\u` without four hex digits keeps that treatment, legal
/// escapes (including `\uXXXX`) pass through untouched, and a trailing
/// backslash stays literal. Outside strings the input is copied verbatim.
/// The result is repaired text, not a parse result: callers decide whether
/// it now parses.
pub fn repair_json(raw: &str) -> String {
    let chars: Vec<char> = raw.chars().collect();
    let mut repaired = String::with_capacity(raw.len());
    let mut in_string = false;
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        if !in_string {
            repaired.push(ch);
            if ch == '"' {
                in_string = true;
            }
            index += 1;
            continue;
        }
        if ch == '"' {
            repaired.push(ch);
            in_string = false;
            index += 1;
            continue;
        }
        if ch == '\\' {
            match chars.get(index + 1) {
                None => {
                    repaired.push_str("\\\\");
                    index += 1;
                }
                Some(&'u') => {
                    let hex: String = chars[index + 2..].iter().take(4).collect();
                    if hex.chars().count() == 4 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
                        repaired.push_str("\\u");
                        repaired.push_str(&hex);
                        index += 6;
                    } else {
                        repaired.push_str("\\\\");
                        index += 1;
                    }
                }
                Some(&next) if VALID_JSON_ESCAPES.contains(&next) => {
                    repaired.push('\\');
                    repaired.push(next);
                    index += 2;
                }
                Some(_) => {
                    repaired.push_str("\\\\");
                    index += 1;
                }
            }
            continue;
        }
        if is_json_control_character(ch) {
            repaired.push_str(&escape_json_control_character(ch));
        } else {
            repaired.push(ch);
        }
        index += 1;
    }
    repaired
}

/// Parse potentially incomplete streaming JSON for display (pinned
/// `parseStreamingJson` cascade, exact order).
///
/// 1. Empty or whitespace input returns an empty object.
/// 2. A complete document parses (after escape repair) and is
///    authoritative.
/// 3. The raw text completes as a partial prefix (dangling quotes,
///    brackets, keys and values are filled; truncated numbers and literals
///    complete conservatively; junk after the first complete top-level
///    value is ignored).
/// 4. The repaired text completes as a partial prefix.
/// 5. Otherwise the result is a valid empty object.
///
/// This is a display-only projection: it never fails and never serves as a
/// final-parse authority; `finalize_tool_arguments` owns final results.
pub fn parse_streaming_json(partial: &str) -> serde_json::Value {
    if partial.trim().is_empty() {
        return serde_json::Value::Object(serde_json::Map::new());
    }
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(partial) {
        return value;
    }
    let repaired = repair_json(partial);
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&repaired) {
        return value;
    }
    complete_partial_json(partial)
        .or_else(|| complete_partial_json(&repaired))
        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()))
}

/// Which grammatical slot the innermost open container expects next.
#[derive(Clone, Copy, PartialEq)]
enum JsonSlot {
    /// Object: a key string.
    Key,
    /// Object: the colon after a closed key string.
    Colon,
    /// Object or array: a value or element.
    Value,
    /// A complete member/element was just flushed (comma or close next).
    After,
}

/// Complete a JSON prefix by filling dangling structure, then parse it.
/// Returns `None` when even the completed text is not valid JSON.
fn complete_partial_json(partial: &str) -> Option<serde_json::Value> {
    let mut completed = String::with_capacity(partial.len() + 8);
    let mut stack: Vec<(char, JsonSlot)> = Vec::new();
    let mut in_string = false;
    let mut escaped = false;
    let mut token = String::new();
    let mut top_level_done = false;

    /// A closed string is either a dangling key (colon expected next) or a
    /// completed value/member (comma or close expected next).
    fn close_string_slot(stack: &mut [(char, JsonSlot)]) {
        if let Some((_, slot)) = stack.last_mut() {
            *slot = if *slot == JsonSlot::Key {
                JsonSlot::Colon
            } else {
                JsonSlot::After
            };
        }
    }

    fn flush_token(
        out: &mut String,
        token: &mut String,
        stack: &mut [(char, JsonSlot)],
        completed: bool,
    ) {
        if token.is_empty() {
            return;
        }
        if completed {
            // Only the dangling trailing token gets corrective treatment.
            if let Some(fixed) = fix_dangling_token(token) {
                out.push_str(&fixed);
                if let Some((_, slot)) = stack.last_mut() {
                    *slot = JsonSlot::After;
                }
            }
            token.clear();
            return;
        }
        out.push_str(token);
        token.clear();
        if let Some((_, slot)) = stack.last_mut() {
            *slot = JsonSlot::After;
        }
    }

    for ch in partial.chars() {
        if top_level_done {
            break;
        }
        if in_string {
            completed.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
                close_string_slot(&mut stack);
                if stack.is_empty() {
                    top_level_done = true;
                }
            }
            continue;
        }
        match ch {
            '"' => {
                flush_token(&mut completed, &mut token, &mut stack, false);
                completed.push(ch);
                in_string = true;
            }
            '{' => {
                flush_token(&mut completed, &mut token, &mut stack, false);
                completed.push(ch);
                if let Some((_, slot)) = stack.last_mut() {
                    *slot = JsonSlot::After;
                }
                stack.push(('{', JsonSlot::Key));
            }
            '[' => {
                flush_token(&mut completed, &mut token, &mut stack, false);
                completed.push(ch);
                if let Some((_, slot)) = stack.last_mut() {
                    *slot = JsonSlot::After;
                }
                stack.push(('[', JsonSlot::Value));
            }
            '}' | ']' => {
                flush_token(&mut completed, &mut token, &mut stack, false);
                completed.push(ch);
                if stack.pop().is_some() && !stack.is_empty() {
                    if let Some((_, slot)) = stack.last_mut() {
                        *slot = JsonSlot::After;
                    }
                } else {
                    top_level_done = true;
                }
            }
            ',' => {
                flush_token(&mut completed, &mut token, &mut stack, false);
                completed.push(ch);
                if let Some((container, slot)) = stack.last_mut() {
                    *slot = if *container == '{' {
                        JsonSlot::Key
                    } else {
                        JsonSlot::Value
                    };
                }
            }
            ':' => {
                flush_token(&mut completed, &mut token, &mut stack, false);
                completed.push(ch);
                if let Some((container, slot)) = stack.last_mut()
                    && *container == '{'
                {
                    *slot = JsonSlot::Value;
                }
            }
            ch if ch.is_whitespace() => {
                flush_token(&mut completed, &mut token, &mut stack, false);
                completed.push(ch);
            }
            ch => token.push(ch),
        }
    }

    flush_token(&mut completed, &mut token, &mut stack, true);
    if in_string {
        if escaped {
            // A dangling escape never contributes a character.
            completed.pop();
        }
        completed.push('"');
        close_string_slot(&mut stack);
    }
    while let Some((container, slot)) = stack.pop() {
        match (container, slot) {
            ('{', JsonSlot::Colon) => completed.push_str(":null}"),
            ('{', JsonSlot::Value) => completed.push_str("null}"),
            ('{', _) => {
                if completed.ends_with(',') {
                    completed.pop();
                }
                completed.push('}');
            }
            (_, _) => {
                if completed.ends_with(',') {
                    completed.pop();
                }
                completed.push(']');
            }
        }
    }
    serde_json::from_str(&completed).ok()
}

/// Fix a dangling trailing bare token: a valid JSON number passes through
/// (after dropping a truncated fraction/exponent), a prefix of a JSON
/// literal completes to the literal, anything else is dropped.
fn fix_dangling_token(token: &str) -> Option<String> {
    for literal in ["true", "false", "null"] {
        if !token.is_empty() && token.len() < literal.len() && literal.starts_with(token) {
            return Some(literal.to_owned());
        }
    }
    let mut candidate = token.to_owned();
    loop {
        if serde_json::from_str::<serde_json::Number>(&candidate).is_ok() {
            return Some(candidate);
        }
        match candidate.chars().next_back() {
            Some('e' | 'E' | '+' | '-' | '.') => {
                candidate.pop();
            }
            _ => return None,
        }
    }
}

/// Build a portable string-enum JSON Schema for tool parameters (pinned
/// `StringEnum`; task 20.53.2, P20-S53-2).
///
/// Emits `{"type": "string", "enum": [...]}` plus optional `description`
/// and `default` members (empty strings are omitted, mirroring the pinned
/// truthiness check). The shape is deliberately portable: providers such
/// as Google's API do not support `anyOf`/`const` patterns, and no TypeBox
/// symbols or JavaScript ABI leak into the value - it is a plain
/// `serde_json::Value` ready for `ToolDef::input_schema`.
pub fn string_enum(
    values: &[&str],
    description: Option<&str>,
    default: Option<&str>,
) -> serde_json::Value {
    let mut schema = serde_json::Map::new();
    schema.insert(
        "type".to_owned(),
        serde_json::Value::String("string".to_owned()),
    );
    schema.insert(
        "enum".to_owned(),
        serde_json::Value::Array(
            values
                .iter()
                .map(|value| serde_json::Value::String((*value).to_owned()))
                .collect(),
        ),
    );
    if let Some(text) = description.filter(|text| !text.is_empty()) {
        schema.insert(
            "description".to_owned(),
            serde_json::Value::String(text.to_owned()),
        );
    }
    if let Some(text) = default.filter(|text| !text.is_empty()) {
        schema.insert(
            "default".to_owned(),
            serde_json::Value::String(text.to_owned()),
        );
    }
    serde_json::Value::Object(schema)
}

// ---------------------------------------------------------------------------
// Public context-overflow and recoverable-Length predicates (task 20.53.3,
// P20-S53-3). Pinned source: `.repo/pi-1.3.0/packages/ai/src/utils/overflow.ts`.

/// Pinned context-overflow provider error message patterns (25, case
/// insensitive), merged into one alternation.
fn context_overflow_pattern() -> &'static regex::Regex {
    static PATTERN: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    PATTERN.get_or_init(|| {
        regex::RegexBuilder::new(
            &[
                r"prompt (?:is )?too long",               // Anthropic, z.ai
                r"prompt exceeds max length",             // z.ai CN endpoint
                r"request_too_large",                     // Anthropic HTTP 413
                r"input is too long for requested model", // Amazon Bedrock
                r"exceeds the context window",            // OpenAI
                r"exceeds (?:the )?(?:model'?s )?maximum context length(?: of [\d,]+ tokens?|\s*\([\d,]+\))", // LiteLLM proxies
                r"input token count.*exceeds the maximum", // Google (Gemini)
                r"maximum prompt length is \d+",           // xAI (Grok)
                r"reduce the length of the messages",      // Groq
                r"maximum context length is \d+ tokens",   // OpenRouter
                r"exceeds (?:the )?maximum allowed input length of [\d,]+ tokens?", // OpenRouter/Poolside
                r"input \(\d+ tokens\) is longer than the model'?s context length \(\d+ tokens\)", // Together AI
                r"exceeds the limit of \d+",           // GitHub Copilot
                r"exceeds the available context size", // llama.cpp
                r"greater than the context length",    // LM Studio
                r"context window exceeds limit",       // MiniMax
                r"exceeded model token limit",         // Kimi For Coding
                r"too large for model with \d+ maximum context length", // Mistral
                r"prompt has [\d,]+ tokens?, but the configured context size is [\d,]+ tokens?", // DS4
                r"model_context_window_exceeded", // z.ai finish reason
                r"prompt too long; exceeded (?:max )?context length", // Ollama
                r"range of input length should be", // DashScope/Qwen
                r"context[_ ]length[_ ]exceeded", // generic fallback
                r"too many tokens",               // generic fallback
                r"token limit exceeded",          // generic fallback
            ]
            .join("|"),
        )
        .case_insensitive(true)
        .build()
        .expect("pinned overflow pattern set")
    })
}

/// Pinned non-overflow patterns (throttling/rate limiting) that exclude an
/// error message from overflow detection even when it also matches an
/// overflow pattern.
fn non_context_overflow_pattern() -> &'static regex::Regex {
    static PATTERN: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    PATTERN.get_or_init(|| {
        regex::RegexBuilder::new(
            r"^(Throttling error|Service unavailable):|rate limit|too many requests",
        )
        .case_insensitive(true)
        .build()
        .expect("pinned non-overflow pattern set")
    })
}

/// Pinned Cerebras bodyless 400/413 overflow shape, applied only when the
/// message was produced by the `cerebras` provider.
fn cerebras_bodyless_overflow_pattern() -> &'static regex::Regex {
    static PATTERN: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    PATTERN.get_or_init(|| {
        regex::RegexBuilder::new(r"^4(?:00|13)\s*(?:status code)?\s*\(no body\)")
            .case_insensitive(true)
            .build()
            .expect("pinned cerebras bodyless pattern")
    })
}

/// Check whether an assistant message represents a context overflow error
/// (pinned `isContextOverflow`; task 20.53.3, P20-S53-3).
///
/// Three cases, in pinned order:
///
/// 1. Error-based overflow: `stop_reason` is
///    [`crate::stream::StopReason::Error`] with an
///    `error_message` matching a pinned provider overflow pattern. Messages
///    matching a pinned non-overflow pattern first (throttling/rate-limit
///    style) are excluded even when they also match an overflow pattern; the
///    bodyless Cerebras 400/413 shape additionally requires
///    `provider == "cerebras"`.
/// 2. Silent overflow: a `Stop` message whose known usage
///    (`usage.reported`) reports `input_tokens + cache_read_tokens` above the
///    context window.
/// 3. Length-stop overflow: a `Length` message with known zero
///    `output_tokens` whose input plus cache read fills at least 99% of the
///    context window.
///
/// A `context_window` of `None` or `Some(0)` disables the usage-based cases
/// (the source treats a zero window as absent); unknown usage
/// (`usage.reported == false`) is never converted to zero tokens. This is a
/// pure predicate: it never invokes compaction or retry policy.
pub fn is_context_overflow(message: &AssistantMessage, context_window: Option<u64>) -> bool {
    if message.stop_reason == crate::stream::StopReason::Error
        && let Some(error_message) = &message.error_message
        && !non_context_overflow_pattern().is_match(error_message)
    {
        if context_overflow_pattern().is_match(error_message) {
            return true;
        }
        if message.provider == "cerebras"
            && cerebras_bodyless_overflow_pattern().is_match(error_message)
        {
            return true;
        }
    }
    let Some(window) = context_window.filter(|window| *window > 0) else {
        return false;
    };
    if !message.usage.reported {
        return false;
    }
    let input_tokens =
        u64::from(message.usage.input_tokens) + u64::from(message.usage.cache_read_tokens);
    match message.stop_reason {
        crate::stream::StopReason::Stop => input_tokens > window,
        crate::stream::StopReason::Length if message.usage.output_tokens == 0 => {
            input_tokens * 100 >= window * 99
        }
        _ => false,
    }
}

/// Check whether a length stop ended below the caller's or model's intended
/// output limit (pinned `isRecoverableLength`; task 20.53.3, P20-S53-3).
///
/// True when the message stopped for
/// [`crate::stream::StopReason::Length`],
/// `desired_max_output` is the original positive limit before any
/// context-based clamping, and known usage (`usage.reported`) produced fewer
/// output tokens than that limit - the response may have been caused by
/// context pressure or provider-side truncation, so a caller may make one
/// bounded compact-and-retry attempt. Unknown usage is never converted to
/// zero tokens; this is a pure predicate that never invokes retry policy.
pub fn is_recoverable_length(message: &AssistantMessage, desired_max_output: u64) -> bool {
    message.stop_reason == crate::stream::StopReason::Length
        && desired_max_output > 0
        && message.usage.reported
        && u64::from(message.usage.output_tokens) < desired_max_output
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    /// Per-tool constrained-sampling intent (task 20.11, AIR-040):
    /// `json_schema` prefer/require or grammar variants. `None` (the
    /// default and the historical shape) never selects constrained
    /// sampling. Adapters resolve it at body-build time; the trusted
    /// `input_schema` is never mutated by the strict projection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constrained_sampling: Option<crate::constrained_sampling::ConstrainedSampling>,
}

#[cfg(test)]
mod tool_argument_tests {
    use super::ToolCall;

    fn json_object(text: &str) -> serde_json::Map<String, serde_json::Value> {
        serde_json::from_str(text).expect("finalized arguments parse as a JSON object")
    }

    #[test]
    fn empty_input_finalizes_to_an_empty_object_without_error() {
        for raw in ["", " ", "\n\t"] {
            let (arguments, error) = ToolCall::finalize_text_arguments(raw.to_owned());
            assert_eq!(arguments, "{}");
            assert!(error.is_none(), "raw={raw:?}");
        }
    }

    #[test]
    fn complete_object_keeps_owned_raw_text() {
        let raw = r#"{"path":"/tmp/x","n":2}"#;
        let (arguments, error) = ToolCall::finalize_text_arguments(raw.to_owned());
        assert_eq!(arguments, raw);
        assert!(error.is_none());
        let parsed = json_object(&arguments);
        assert_eq!(parsed["path"], "/tmp/x");
    }

    #[test]
    fn invalid_escapes_are_repaired_with_literal_text_and_later_fields() {
        let raw = r#"{"cmd":"\usepackage{x}","n":2}"#;
        let (arguments, error) = ToolCall::finalize_text_arguments(raw.to_owned());
        assert!(error.is_none(), "arguments={arguments}");
        let parsed = json_object(&arguments);
        assert_eq!(parsed["cmd"], r#"\usepackage{x}"#);
        assert_eq!(parsed["n"], 2, "later fields are not dropped");
    }

    #[test]
    fn invalid_unicode_escape_without_hex_digits_is_repaired() {
        // `\u007` has only three hex digits: not a valid JSON escape.
        let raw = r#"{"s":"\u007x","n":1}"#;
        let (arguments, error) = ToolCall::finalize_text_arguments(raw.to_owned());
        assert!(error.is_none());
        let parsed = json_object(&arguments);
        assert_eq!(parsed["s"], r#"\u007x"#);
    }

    #[test]
    fn valid_unicode_escape_survives_repair_path() {
        let raw = "{\"s\":\"\\u0041\",\"n\":1}";
        let (arguments, error) = ToolCall::finalize_text_arguments(raw.to_owned());
        assert_eq!(arguments, raw);
        assert!(error.is_none());
        let parsed = json_object(&arguments);
        assert_eq!(parsed["s"], "A");
    }

    #[test]
    fn incomplete_final_input_keeps_raw_and_marks_error() {
        for raw in [r#"{"path": "trunc"#, r#"{"a": "#, "{\"a\": \"b\",}"] {
            let (arguments, error) = ToolCall::finalize_text_arguments(raw.to_owned());
            assert_eq!(arguments, raw, "owned raw is kept");
            assert_eq!(
                error.as_deref(),
                Some("final tool arguments were not a complete JSON object"),
                "raw={raw:?}"
            );
        }
    }

    #[test]
    fn non_object_final_input_keeps_raw_and_marks_error() {
        for raw in ["[1,2]", "\"text\"", "5", "null", "true"] {
            let (arguments, error) = ToolCall::finalize_text_arguments(raw.to_owned());
            assert_eq!(arguments, raw);
            assert!(error.is_some(), "raw={raw:?}");
        }
    }

    #[test]
    fn arguments_error_round_trips_and_stays_absent_when_none() {
        let call = ToolCall {
            id: "t1".into(),
            name: "read".into(),
            arguments: r#"{"path": "trunc"#.into(),
            arguments_error: Some("final tool arguments were not a complete JSON object".into()),
            signature: None,
        };
        let value = serde_json::to_value(&call).unwrap();
        assert_eq!(
            value["arguments_error"],
            "final tool arguments were not a complete JSON object"
        );
        let back: ToolCall = serde_json::from_value(value).unwrap();
        assert_eq!(back, call);

        let clean = ToolCall {
            id: "t2".into(),
            name: "read".into(),
            arguments: "{}".into(),
            arguments_error: None,
            signature: None,
        };
        let value = serde_json::to_value(&clean).unwrap();
        assert!(value.get("arguments_error").is_none());
        // Historical records without the field keep deserializing.
        let legacy = serde_json::json!({
            "id": "t3",
            "name": "read",
            "arguments": "{\"a\":1}",
        });
        let back: ToolCall = serde_json::from_value(legacy).unwrap();
        assert_eq!(back.arguments_error, None);
    }
}
