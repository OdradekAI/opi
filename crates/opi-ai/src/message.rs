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
    pub response_id: Option<String>,
    pub usage: crate::stream::Usage,
    pub stop_reason: crate::stream::StopReason,
    pub error_message: Option<String>,
    pub timestamp_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResultMessage {
    pub tool_call_id: String,
    pub tool_name: String,
    pub content: Vec<OutputContent>,
    pub details: Option<serde_json::Value>,
    pub is_error: bool,
    #[serde(default)]
    pub truncated: bool,
    pub timestamp_ms: i64,
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
    Text { text: String },
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
}

impl ToolCall {
    /// Finalize textual tool arguments on the five text-argument wire sites
    /// (Anthropic, Bedrock, Mistral, Chat and shared Responses; AIR-048).
    ///
    /// Returns `(arguments, arguments_error)`: empty input finalizes to `{}`
    /// without an error; a complete JSON object keeps its owned raw text;
    /// input that only differs by invalid escape sequences (for example
    /// `\u…`-shaped sequences such as `\usepackage`) is repaired so the
    /// escaped text stays literal and later fields survive; any other final
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
        if let Some(repaired) = repair_invalid_escapes(&raw)
            && serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&repaired).is_ok()
        {
            return (repaired, None);
        }
        (raw, Some(NOT_AN_OBJECT.to_owned()))
    }
}

/// Repair invalid JSON escape sequences by escaping the stray backslash, so
/// the model-sent text stays literal and later fields keep parsing.
///
/// `\x` where `x` is not a legal JSON escape (and `\u` without four hex
/// digits) becomes `\\x` in the repaired text. Returns `None` when the input
/// is not shaped like a JSON object (no repair is attempted).
fn repair_invalid_escapes(raw: &str) -> Option<String> {
    let mut chars = raw.chars().peekable();
    if chars.next() != Some('{') {
        return None;
    }
    let mut repaired = String::with_capacity(raw.len());
    repaired.push('{');
    let mut in_string = false;
    while let Some(ch) = chars.next() {
        if !in_string {
            if ch == '"' {
                in_string = true;
            }
            repaired.push(ch);
            continue;
        }
        match ch {
            '"' => {
                in_string = false;
                repaired.push(ch);
            }
            '\\' => {
                let Some(&next) = chars.peek() else {
                    // Trailing backslash inside a string: keep it literal.
                    repaired.push_str("\\\\");
                    break;
                };
                let legal = matches!(next, '"' | '\\' | '/' | 'b' | 'f' | 'n' | 'r' | 't')
                    || (next == 'u' && {
                        let hex: String = chars.clone().skip(1).take(4).collect();
                        hex.len() == 4 && hex.chars().all(|c| c.is_ascii_hexdigit())
                    });
                if legal {
                    repaired.push('\\');
                } else {
                    repaired.push_str("\\\\");
                }
            }
            _ => repaired.push(ch),
        }
    }
    Some(repaired)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
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
