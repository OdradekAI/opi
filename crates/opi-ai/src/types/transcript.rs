//! Serializable transcript messages and projection facts.

use super::message::ToolDef;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

/// A named tool reference used by removals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolReference {
    pub name: String,
}

/// One named prompt section. `value` is the replacement text; `None` removes
/// the section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemSection {
    pub name: String,
    pub value: Option<String>,
}

/// System instructions and tool declarations at one point in the transcript.
///
/// The leading system message is the base prompt. Later system messages
/// change it: `content` appends from that point on, `sections` replace or
/// remove named prompt sections rendered verbatim after `content`, and
/// `tools_added`/`tools_removed` change the tool set. Section names should
/// stay self-delimiting (a tag, a heading) so the model can relate an update
/// to the original.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SystemMessage {
    /// Instruction text. On the leading message this is the base prompt;
    /// later messages append.
    pub content: String,
    /// Named, ordered prompt sections rendered verbatim after `content`.
    /// Names must be exactly unique within one programmatic message before
    /// transcript installation; empty, whitespace and case-distinct names
    /// remain valid. Later separate messages may update the same name.
    /// Serialized as a JSON object in section order; a `null` value removes
    /// the section. A deserialized object with duplicate names keeps the
    /// first position and the last value, matching JavaScript object
    /// assignment semantics. Installation cannot recover duplicate raw keys
    /// that the JSON parser already discarded.
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        serialize_with = "serialize_sections",
        deserialize_with = "deserialize_sections"
    )]
    pub sections: Vec<SystemSection>,
    /// Complete definitions of tools that become available at this point.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools_added: Vec<ToolDef>,
    /// Tools that stop being available at this point.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools_removed: Vec<ToolReference>,
    /// Unix timestamp in milliseconds. The initial shorthand entry uses `0`.
    pub timestamp_ms: i64,
}

fn serialize_sections<S: Serializer>(sections: &[SystemSection], s: S) -> Result<S::Ok, S::Error> {
    use serde::ser::SerializeMap;
    let mut map = s.serialize_map(Some(sections.len()))?;
    for section in sections {
        match &section.value {
            Some(value) => map.serialize_entry(&section.name, value)?,
            None => map.serialize_entry(&section.name, &Option::<String>::None)?,
        }
    }
    map.end()
}

fn deserialize_sections<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<SystemSection>, D::Error> {
    use serde_json::Value;
    let raw = serde_json::Map::<String, Value>::deserialize(d)?;
    let mut sections: Vec<SystemSection> = Vec::with_capacity(raw.len());
    for (name, value) in raw {
        let text = match value {
            Value::Null => None,
            Value::String(text) => Some(text),
            other => {
                return Err(<D::Error as serde::de::Error>::invalid_type(
                    serde::de::Unexpected::Other(&format!(
                        "non-string section value for {name:?}: {}",
                        other
                    )),
                    &"a string or null",
                ));
            }
        };
        // Duplicate names keep the first position and the last value.
        if let Some(existing) = sections.iter_mut().find(|s| s.name == name) {
            existing.value = text;
        } else {
            sections.push(SystemSection { name, value: text });
        }
    }
    Ok(sections)
}

/// Errors from transcript projection and system message installation.
#[derive(Debug, Error, PartialEq)]
pub enum TranscriptError {
    /// Strict projection cannot honor the transcript's temporal placement on
    /// a transport without mid-conversation system message support.
    #[error(
        "strict transcript projection is unsupported: {mid_convo_system_messages} mid-conversation system message(s) cannot be placed natively"
    )]
    UnsupportedTemporalFidelity {
        /// Number of system messages after the leading entry.
        mid_convo_system_messages: usize,
    },
    /// A tool declaration failed validation before installation.
    #[error(transparent)]
    InvalidDeclaration(#[from] ToolDeclarationError),
    /// One system message contains the same exact section name more than
    /// once; repeating a name in a separate later message is a valid update.
    #[error("ambiguous system section batch: {name:?} declared twice")]
    DuplicateSectionName { name: String },
}

/// Errors from tool declaration validation.
#[derive(Debug, Error, PartialEq)]
pub enum ToolDeclarationError {
    /// A declaration name is empty.
    #[error("tool declaration name is empty")]
    EmptyName,
    /// The declaration's input schema does not compile under the private
    /// dialect engine. The detail is schema-derived only.
    #[error("tool {name:?} input schema failed to compile: {detail}")]
    SchemaCompilation { name: String, detail: String },
    /// Two declarations in one batch share a name.
    #[error("ambiguous tool declaration batch: {name:?} declared twice")]
    DuplicateName { name: String },
}

/// Original canonical prefix associated with a projection, never a proof of
/// dispatch or permission to reconstruct its contents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcePrefix {
    pub message_count: usize,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectionMode {
    Lossless,
    Native,
    Collapse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemporalProjectionFacts {
    pub mode: ProjectionMode,
    pub affected_ranges: Vec<MessageRange>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionFacts {
    pub source_prefix: SourcePrefix,
    pub temporal: TemporalProjectionFacts,
    pub replay: crate::types::replay::ReplayFacts,
}
