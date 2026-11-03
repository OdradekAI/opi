//! Canonical context/transcript chronology (DEL-03, AIR-008/011/020).
//!
//! The public input shorthand ([`Context`]) carries a system prompt, the
//! conversation messages, and a tool set. [`normalize_context`] folds that
//! shorthand into exactly one leading [`Message::System`] entry and returns
//! the resulting [`TranscriptContext`]; it is the only public constructor of
//! that type, so a raw shorthand can never reach provider adapters by
//! accident. Re-normalizing an already normalized transcript through an empty
//! shorthand is the identity, which is also how stored transcripts re-enter
//! this seam.
//!
//! Later system messages change the running state: `content` appends
//! instructions, named `sections` are replaced or removed in order (a
//! replacement keeps its position, `None` deletes, delete-then-readd appends),
//! and `tools_added`/`tools_removed` change the tool set by value. Replaying
//! every system message in order ([`get_current_system_message`],
//! [`get_current_tools`]) yields the current prompt and tools. Projection
//! ([`resolve_transcript`]) is native-or-collapse by default: transports that
//! accept mid-conversation system messages keep them in place, everything
//! else rebuilds one leading system message from the replayed state. Collapse
//! is a projection: it returns a fresh value and never rewrites the canonical
//! transcript. Explicit [`TranscriptProjection::Strict`] rejects a transcript
//! whose temporal placement the transport cannot honor, before any request
//! is built.
//!
//! The pinned TypeScript reference is
//! `.repo/pi-1.3.0/packages/ai/src/utils/transcript.ts` (upstream-deprecated
//! `hasToolRedefinitions` is deliberately not ported). Two documented Rust
//! boundaries: system `content` is a plain string (sections carry the
//! structure), and [`declarations_equal`] uses structural `serde_json` value
//! equality, which is key-order independent but distinguishes `1` from `1.0`
//! where JavaScript's canonical serialization would not — the divergence only
//! ever records an extra delta, never misses one.

use std::borrow::Cow;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

use crate::message::{Message, ToolDef};
use crate::schema;

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
    /// Serialized as a JSON object in section order; a `null` value removes
    /// the section. A deserialized object with duplicate names keeps the
    /// first position and the last value, matching JavaScript object
    /// assignment semantics.
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

/// Public request input accepted by the public entry points. `system_prompt`
/// and `tools` are shorthand for a leading system message;
/// [`normalize_context`] folds them into one before the request reaches a
/// provider.
#[derive(Debug, Clone, Default)]
pub struct Context {
    pub system_prompt: Option<String>,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolDef>,
}

/// Normalized request context passed to providers and API implementations.
///
/// The prompt and tool declarations live in the transcript's system messages,
/// never in independent runtime fields. Construction through
/// [`normalize_context`] and deserialization of the transparent message list
/// both validate declaration batches before installing them.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(transparent)]
pub struct TranscriptContext {
    messages: Vec<Message>,
}

impl<'de> Deserialize<'de> for TranscriptContext {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let messages = Vec::<Message>::deserialize(deserializer)?;
        validate_messages(&messages).map_err(serde::de::Error::custom)?;
        Ok(Self { messages })
    }
}

fn validate_messages(messages: &[Message]) -> Result<(), TranscriptError> {
    for message in messages {
        if let Message::System(system) = message {
            validate_tool_batch(&system.tools_added)?;
        }
    }
    Ok(())
}

impl TranscriptContext {
    /// The canonical chronology.
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Consumes the context, returning the canonical chronology.
    pub fn into_messages(self) -> Vec<Message> {
        self.messages
    }

    /// Install a system change at the end of the chronology.
    ///
    /// Tool declarations are validated first (non-empty unique names and a
    /// schema that compiles under the private dialect engine); installation
    /// fails with a typed error before the message is appended, so an invalid
    /// declaration never enters the transcript.
    pub fn append_system_change(&mut self, change: SystemMessage) -> Result<(), TranscriptError> {
        validate_tool_batch(&change.tools_added).map_err(TranscriptError::InvalidDeclaration)?;
        self.messages.push(Message::System(change));
        Ok(())
    }
}

/// Errors from transcript projection and declaration installation.
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
}

impl From<TranscriptError> for crate::provider::ProviderError {
    fn from(error: TranscriptError) -> Self {
        let summary = match error {
            TranscriptError::UnsupportedTemporalFidelity {
                mid_convo_system_messages,
            } => crate::provider::ProviderErrorSummary::unsupported_temporal(
                mid_convo_system_messages,
            ),
            other => crate::provider::ProviderErrorSummary::sanitized(other.to_string()),
        };
        Self::Config(summary)
    }
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

/// Build the leading system message for a prompt and tool set. Returns `None`
/// when both are empty, so an empty transcript stays empty.
pub fn create_initial_system_message(
    system_prompt: Option<&str>,
    tools: &[ToolDef],
) -> Option<SystemMessage> {
    let has_system_prompt = system_prompt.is_some_and(|p| !p.is_empty());
    let has_tools = !tools.is_empty();
    if !has_system_prompt && !has_tools {
        return None;
    }
    Some(SystemMessage {
        content: system_prompt.unwrap_or_default().to_owned(),
        sections: Vec::new(),
        tools_added: if has_tools {
            tools.to_vec()
        } else {
            Vec::new()
        },
        tools_removed: Vec::new(),
        timestamp_ms: 0,
    })
}

/// Fold [`Context::system_prompt`] and [`Context::tools`] into a leading
/// system message. Every provider-facing function expects the resulting
/// [`TranscriptContext`].
///
/// Returns an error before installation if the shorthand tools or any embedded
/// system message contains an invalid or ambiguous declaration batch.
pub fn normalize_context(context: Context) -> Result<TranscriptContext, TranscriptError> {
    validate_tool_batch(&context.tools)?;
    validate_messages(&context.messages)?;
    let initial_message =
        create_initial_system_message(context.system_prompt.as_deref(), &context.tools);
    let mut messages = context.messages;
    if let Some(message) = initial_message {
        messages.insert(0, Message::System(message));
    }
    Ok(TranscriptContext { messages })
}

fn as_system_message(message: &Message) -> Option<&SystemMessage> {
    match message {
        Message::System(system) => Some(system),
        _ => None,
    }
}

/// Return the leading system message, if the transcript starts with one.
pub fn get_initial_system_message(messages: &[Message]) -> Option<&SystemMessage> {
    messages.first().and_then(as_system_message)
}

/// Drop the leading system message for APIs that carry the prompt outside the
/// message list.
pub fn without_initial_system_message(messages: &[Message]) -> Vec<Message> {
    let tail = if get_initial_system_message(messages).is_some() {
        &messages[1..]
    } else {
        messages
    };
    tail.to_vec()
}

/// Resolve the tools available after applying every transcript delta in
/// order. Removals apply before additions within one message; the result
/// keeps first-addition order.
pub fn get_current_tools(messages: &[Message]) -> Vec<ToolDef> {
    let mut tools: Vec<(String, ToolDef)> = Vec::new();
    for message in messages.iter().filter_map(as_system_message) {
        for removed in &message.tools_removed {
            tools.retain(|(name, _)| name != &removed.name);
        }
        for tool in &message.tools_added {
            match tools.iter_mut().find(|(name, _)| *name == tool.name) {
                Some(slot) => slot.1 = tool.clone(),
                None => tools.push((tool.name.clone(), tool.clone())),
            }
        }
    }
    tools.into_iter().map(|(_, tool)| tool).collect()
}

/// Replay every system message into one leading system message holding the
/// current prompt and tools. Later `content` is appended to the base prompt,
/// sections are patched by name (replacement keeps position, `None` deletes,
/// re-add appends), and tools resolve through [`get_current_tools`].
pub fn get_current_system_message(messages: &[Message]) -> Option<SystemMessage> {
    let mut content: Vec<&str> = Vec::new();
    let mut sections: Vec<SystemSection> = Vec::new();
    let mut timestamp: Option<i64> = None;
    for message in messages.iter().filter_map(as_system_message) {
        *timestamp.get_or_insert(message.timestamp_ms) = *timestamp.get_or_insert(0);
        if !message.content.is_empty() {
            content.push(&message.content);
        }
        for section in &message.sections {
            match (
                &section.value,
                sections.iter_mut().find(|s| s.name == section.name),
            ) {
                (Some(value), Some(existing)) => existing.value = Some(value.clone()),
                (Some(value), None) => sections.push(SystemSection {
                    name: section.name.clone(),
                    value: Some(value.clone()),
                }),
                (None, Some(_)) => {
                    sections.retain(|s| s.name != section.name);
                }
                (None, None) => {}
            }
        }
    }
    let tools = get_current_tools(messages);
    if timestamp.is_none() && tools.is_empty() {
        return None;
    }
    Some(SystemMessage {
        content: content.join("\n\n"),
        sections,
        tools_added: tools,
        tools_removed: Vec::new(),
        timestamp_ms: timestamp.unwrap_or_default(),
    })
}

/// Render the current system prompt text after replaying every system
/// message: content parts and live section values, joined by blank lines.
pub fn get_current_system_prompt(messages: &[Message]) -> String {
    get_current_system_message(messages)
        .map(|message| system_message_text(&message))
        .unwrap_or_default()
}

/// Render a system message's prompt text: its content followed by every live
/// section value, joined by blank lines.
pub fn system_message_text(message: &SystemMessage) -> String {
    let mut parts: Vec<&str> = vec![&message.content];
    parts.extend(
        message
            .sections
            .iter()
            .filter_map(|section| section.value.as_deref()),
    );
    parts.retain(|part| !part.is_empty());
    parts.join("\n\n")
}

/// Render a later system message for APIs that accept system messages
/// mid-conversation. Section changes are framed by name so the model can
/// relate them to the leading prompt. This framing is request-time only.
pub fn render_system_message_update(message: &SystemMessage) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !message.content.is_empty() {
        parts.push(message.content.clone());
    }
    for section in &message.sections {
        match &section.value {
            None => parts.push(format!(
                "Removed system prompt section \"{}\".",
                section.name
            )),
            Some(value) => parts.push(format!(
                "Updated system prompt section \"{}\":\n\n{}",
                section.name, value
            )),
        }
    }
    parts.join("\n\n")
}

/// Rebuild the transcript for APIs without mid-conversation system messages:
/// the replayed system message leads, and every system message is dropped.
/// This is a projection into a fresh value; the canonical transcript is never
/// rewritten.
pub fn collapse_system_messages(context: &TranscriptContext) -> TranscriptContext {
    let head = get_current_system_message(context.messages());
    let mut messages: Vec<Message> = Vec::with_capacity(context.messages().len());
    if let Some(head) = head {
        messages.push(Message::System(head));
    }
    messages.extend(
        context
            .messages()
            .iter()
            .filter(|message| !matches!(message, Message::System(_)))
            .cloned(),
    );
    TranscriptContext { messages }
}

/// Requested projection fidelity for [`resolve_transcript`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TranscriptProjection {
    /// Keep later system messages in place when the transport accepts them;
    /// otherwise collapse to the replayed head. The default.
    #[default]
    NativeOrCollapse,
    /// Reject a transcript whose temporal placement the transport cannot
    /// honor, before any request is built (before auth or network).
    Strict,
}

/// Fidelity selected once for the complete request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProjectionOptions {
    pub temporal: TranscriptProjection,
    pub replay: crate::replay::ReplayProjection,
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
    pub replay: crate::replay::ReplayFacts,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TranscriptProjectionOutput<'a> {
    pub context: Cow<'a, TranscriptContext>,
    pub facts: TemporalProjectionFacts,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NativeTools {
    None,
    AddOnly,
    Full,
}
#[derive(Clone, Copy)]
struct TemporalCapabilities {
    body: bool,
    sections: bool,
    tools: NativeTools,
}

/// Resolve temporal fidelity and record its effect in original canonical
/// message coordinates. The input is never mutated; Strict rejects effective
/// changes that the selected transport cannot represent.
pub fn resolve_transcript(
    context: &TranscriptContext,
    supports_mid_convo_system_messages: bool,
    projection: TranscriptProjection,
) -> Result<TranscriptProjectionOutput<'_>, TranscriptError> {
    resolve_temporal(
        context,
        TemporalCapabilities {
            body: supports_mid_convo_system_messages,
            sections: supports_mid_convo_system_messages,
            tools: if supports_mid_convo_system_messages {
                NativeTools::Full
            } else {
                NativeTools::None
            },
        },
        projection,
    )
}

fn resolve_temporal(
    context: &TranscriptContext,
    capabilities: TemporalCapabilities,
    projection: TranscriptProjection,
) -> Result<TranscriptProjectionOutput<'_>, TranscriptError> {
    // Equal state adds no redundant declaration (AIR-008). Keep the public
    // raw-history helper unchanged; only this request-local compiler removes
    // ineffective deltas before applying an addition-only wire contract.
    let mut changes = Vec::new();
    for (index, message) in context.messages().iter().enumerate().skip(1) {
        if !matches!(message, Message::System(_)) {
            continue;
        }
        let before = get_current_system_message(&context.messages()[..index]).unwrap_or_default();
        let after = get_current_system_message(&context.messages()[..=index]).unwrap_or_default();
        let tools = get_tool_state_changes(&before.tools_added, &after.tools_added);
        changes.push((
            index,
            before.content != after.content,
            before.sections != after.sections,
            tools,
        ));
    }
    let native_tools = capabilities.tools == NativeTools::Full
        || (capabilities.tools == NativeTools::AddOnly
            && changes
                .iter()
                .all(|(_, _, _, tools)| tools.tools_removed.is_empty()));
    let mut changed = false;
    let mut unsupported = 0;
    let mut affected_end = 0;
    for (index, body, sections, tools) in &changes {
        changed |= *body || *sections || !tools.is_empty();
        if (*body && !capabilities.body)
            || (*sections && !capabilities.sections)
            || (!tools.is_empty() && !native_tools)
        {
            unsupported += 1;
            affected_end = index + 1;
        }
    }
    if unsupported > 0 && projection == TranscriptProjection::Strict {
        return Err(TranscriptError::UnsupportedTemporalFidelity {
            mid_convo_system_messages: unsupported,
        });
    }
    let facts = TemporalProjectionFacts {
        mode: if unsupported > 0 {
            ProjectionMode::Collapse
        } else if changed {
            ProjectionMode::Native
        } else {
            ProjectionMode::Lossless
        },
        affected_ranges: if unsupported > 0 {
            vec![MessageRange {
                start: 0,
                end: affected_end,
            }]
        } else {
            Vec::new()
        },
    };
    let needs_projection = context
        .messages()
        .iter()
        .enumerate()
        .skip(1)
        .any(|(index, message)| match message {
            Message::System(system) => {
                (!capabilities.body && !system.content.is_empty())
                    || (!capabilities.sections && !system.sections.is_empty())
                    || (!native_tools
                        && (!system.tools_added.is_empty() || !system.tools_removed.is_empty()))
                    || (capabilities.tools == NativeTools::AddOnly
                        && changes
                            .iter()
                            .find(|(at, _, _, _)| *at == index)
                            .is_some_and(|(_, _, _, tools)| {
                                tools.tools_added != system.tools_added
                                    || tools.tools_removed != system.tools_removed
                            }))
            }
            _ => false,
        });
    if !needs_projection {
        return Ok(TranscriptProjectionOutput {
            context: Cow::Borrowed(context),
            facts,
        });
    }
    let final_head = get_current_system_message(context.messages()).unwrap_or_default();
    let mut head = get_initial_system_message(context.messages())
        .cloned()
        .unwrap_or_default();
    if !capabilities.body {
        head.content = final_head.content;
    }
    if !capabilities.sections {
        head.sections = final_head.sections;
    }
    if !native_tools {
        head.tools_added = final_head.tools_added;
        head.tools_removed.clear();
    }
    let mut messages = vec![Message::System(head)];
    for (index, message) in context.messages().iter().enumerate() {
        if index == 0 && matches!(message, Message::System(_)) {
            continue;
        }
        match message {
            Message::System(system) => {
                let mut system = system.clone();
                if !capabilities.body {
                    system.content.clear();
                }
                if !capabilities.sections {
                    system.sections.clear();
                }
                if !native_tools {
                    system.tools_added.clear();
                    system.tools_removed.clear();
                } else if capabilities.tools == NativeTools::AddOnly
                    && let Some((_, _, _, tools)) =
                        changes.iter().find(|(at, _, _, _)| *at == index)
                {
                    system.tools_added = tools.tools_added.clone();
                    system.tools_removed = tools.tools_removed.clone();
                }
                if !system.content.is_empty()
                    || !system.sections.is_empty()
                    || !system.tools_added.is_empty()
                    || !system.tools_removed.is_empty()
                {
                    messages.push(Message::System(system));
                }
            }
            other => messages.push(other.clone()),
        }
    }
    Ok(TranscriptProjectionOutput {
        context: Cow::Owned(TranscriptContext { messages }),
        facts,
    })
}

pub(crate) struct RequestProjection<'a> {
    pub transcript: Cow<'a, TranscriptContext>,
    pub replay: crate::replay::ReplayOutput,
    pub facts: ProjectionFacts,
    pub tools: TranscriptTools,
}

/// The shared pure preauth/payload compiler. Auth-derived headers and endpoints
/// never participate in selecting canonical temporal or replay meaning.
pub(crate) fn compile_request_projection<'a>(
    request: &'a crate::provider::Request,
    provider: &str,
    model: Option<&crate::ModelInfo>,
    wire: crate::WireApi,
) -> Result<RequestProjection<'a>, crate::provider::ProviderError> {
    use crate::model_info::WireCompat;
    let caps = match model.map(|model| &model.compat) {
        Some(WireCompat::AnthropicMessages(compat)) => TemporalCapabilities {
            body: compat.supports_mid_convo_system_messages,
            sections: compat.supports_mid_convo_system_messages,
            tools: if compat.supports_mid_convo_system_messages
                && compat.supports_mid_convo_tool_changes
                && get_initial_system_message(request.context.messages())
                    .is_some_and(|head| !head.tools_added.is_empty())
            {
                NativeTools::Full
            } else {
                NativeTools::None
            },
        },
        Some(WireCompat::OpenAiCompletions(compat)) => {
            let native = compat.supports_mid_convo_system_messages.unwrap_or(false);
            TemporalCapabilities {
                body: native,
                sections: native,
                tools: if native && compat.supports_mid_convo_tool_additions.unwrap_or(false) {
                    NativeTools::AddOnly
                } else {
                    NativeTools::None
                },
            }
        }
        Some(WireCompat::OpenAiResponses(compat) | WireCompat::AzureOpenAiResponses(compat)) => {
            responses_temporal(&compat.source)
        }
        Some(WireCompat::OpenAiCodexResponses(compat)) => responses_temporal(&compat.source),
        _ if wire == crate::WireApi::PiMessages => TemporalCapabilities {
            body: true,
            sections: true,
            tools: NativeTools::Full,
        },
        _ => TemporalCapabilities {
            body: false,
            sections: false,
            tools: NativeTools::None,
        },
    };
    let temporal = resolve_temporal(&request.context, caps, request.projection.temporal)?;
    let tools =
        resolve_transcript_tools(temporal.context.messages(), caps.tools != NativeTools::None);
    let replay = if wire == crate::WireApi::PiMessages {
        crate::replay::ReplayOutput {
            messages: temporal.context.messages().to_vec(),
            facts: Default::default(),
        }
    } else {
        let messages = if wire == crate::WireApi::MistralConversations {
            temporal.context.messages().to_vec()
        } else {
            without_initial_system_message(temporal.context.messages())
        };
        crate::replay::project_replay(
            &messages,
            &crate::replay::ReplayRoute {
                wire,
                provider: provider.to_owned(),
                model: model.map_or_else(
                    || {
                        request
                            .model
                            .split_once(':')
                            .map_or(request.model.as_str(), |(_, id)| id)
                            .to_owned()
                    },
                    |model| model.id.clone(),
                ),
                supports_images: model.is_none_or(|model| model.capabilities.supports_images),
            },
            request.projection.replay,
        )?
    };
    use sha2::Digest as _;
    let bytes = serde_json::to_vec(request.context.messages()).map_err(|_| {
        crate::provider::ProviderError::Config(
            crate::provider::ProviderErrorSummary::attested_static(
                "canonical transcript could not be serialized",
            ),
        )
    })?;
    let facts = ProjectionFacts {
        source_prefix: SourcePrefix {
            message_count: request.context.messages().len(),
            sha256: format!("{:x}", sha2::Sha256::digest(bytes)),
        },
        temporal: temporal.facts,
        replay: replay.facts.clone(),
    };
    Ok(RequestProjection {
        transcript: temporal.context,
        replay,
        facts,
        tools,
    })
}

fn responses_temporal(
    source: &crate::model_info::OpenAiResponsesSourceCompat,
) -> TemporalCapabilities {
    let native = source.supports_mid_convo_system_messages.unwrap_or(false);
    TemporalCapabilities {
        body: native,
        sections: native,
        tools: if source.supports_additional_tools.unwrap_or(false)
            || source.supports_tool_search.unwrap_or(false)
        {
            NativeTools::AddOnly
        } else {
            NativeTools::None
        },
    }
}

/// Strip executable and display-only fields from a tool before transcript
/// comparison or persistence. `ToolDef` is already a pure declaration, so
/// this is currently an identity clone; the seam remains so later
/// declaration-adjacent fields (for example constrained sampling) have one
/// canonical strip point.
pub fn to_tool_declaration(tool: &ToolDef) -> ToolDef {
    tool.clone()
}

/// Whether two tools declare the same interface to the model. Structural
/// `serde_json` equality is key-order independent; see the module docs for
/// the documented numeric boundary.
pub fn declarations_equal(left: &ToolDef, right: &ToolDef) -> bool {
    left == right
}

/// The delta between two complete tool states. A changed definition is a
/// removal followed by an addition.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ToolStateChanges {
    pub tools_added: Vec<ToolDef>,
    pub tools_removed: Vec<ToolReference>,
}

impl ToolStateChanges {
    /// Whether the state is unchanged; equal states add no redundant change.
    pub fn is_empty(&self) -> bool {
        self.tools_added.is_empty() && self.tools_removed.is_empty()
    }
}

/// Compare two complete tool states. Additions keep the current order,
/// removals the previous order; only real value changes are recorded.
pub fn get_tool_state_changes(previous: &[ToolDef], current: &[ToolDef]) -> ToolStateChanges {
    ToolStateChanges {
        tools_added: current
            .iter()
            .filter(|tool| {
                previous
                    .iter()
                    .find(|old| old.name == tool.name)
                    .is_none_or(|old| !declarations_equal(old, tool))
            })
            .map(to_tool_declaration)
            .collect(),
        tools_removed: previous
            .iter()
            .filter(|tool| {
                current
                    .iter()
                    .find(|new| new.name == tool.name)
                    .is_none_or(|new| !declarations_equal(tool, new))
            })
            .map(|tool| ToolReference {
                name: tool.name.clone(),
            })
            .collect(),
    }
}

/// Build the system message that moves the transcript from `previous` to
/// `current`, or `None` when the state is unchanged (equal states add no
/// redundant change).
pub fn tool_state_change_message(
    previous: &[ToolDef],
    current: &[ToolDef],
    timestamp_ms: i64,
) -> Option<SystemMessage> {
    let changes = get_tool_state_changes(previous, current);
    if changes.is_empty() {
        return None;
    }
    Some(SystemMessage {
        content: String::new(),
        sections: Vec::new(),
        tools_added: changes.tools_added,
        tools_removed: changes.tools_removed,
        timestamp_ms,
    })
}

/// Every definition referenced by transcript tool state, in first-declaration
/// order.
pub fn get_declared_tools(messages: &[Message]) -> Vec<ToolDef> {
    let mut definitions: Vec<(String, ToolDef)> = Vec::new();
    for message in messages.iter().filter_map(as_system_message) {
        for tool in &message.tools_added {
            match definitions.iter_mut().find(|(name, _)| *name == tool.name) {
                Some(slot) => slot.1 = tool.clone(),
                None => definitions.push((tool.name.clone(), tool.clone())),
            }
        }
    }
    definitions.into_iter().map(|(_, tool)| tool).collect()
}

/// Whether tool history contains a removal or same-name redeclaration that an
/// addition-only transport cannot replay.
pub fn has_non_additive_tool_changes(messages: &[Message]) -> bool {
    let mut declared: Vec<String> = Vec::new();
    for message in messages.iter().filter_map(as_system_message) {
        if !message.tools_removed.is_empty() {
            return true;
        }
        for tool in &message.tools_added {
            if declared.contains(&tool.name) {
                return true;
            }
            declared.push(tool.name.clone());
        }
    }
    false
}

/// Split tool declarations between the top-level request field and in-place
/// additions. Transports that can anchor additions at a system message keep
/// the initial tools at the top and load later ones where they appear; that
/// only works when no tool was removed or redeclared, so everything else
/// sends the current tool list.
#[derive(Debug, Clone, PartialEq)]
pub struct TranscriptTools {
    /// Tools sent in the top-level request field.
    pub request_tools: Vec<ToolDef>,
    /// Whether later system messages carry their own `tools_added` as
    /// in-place additions. When `false`, `request_tools` already holds the
    /// complete current tool set.
    pub anchors_additions: bool,
}

/// See [`TranscriptTools`].
pub fn resolve_transcript_tools(
    messages: &[Message],
    supports_tool_additions: bool,
) -> TranscriptTools {
    let anchors_additions = supports_tool_additions && !has_non_additive_tool_changes(messages);
    TranscriptTools {
        request_tools: if anchors_additions {
            get_initial_system_message(messages)
                .map(|head| head.tools_added.clone())
                .unwrap_or_default()
        } else {
            get_current_tools(messages)
        },
        anchors_additions,
    }
}

/// Validate one tool declaration: the name must be non-empty and the input
/// schema must compile under the private dialect engine. The failure detail
/// is schema-derived only.
pub fn validate_tool_declaration(tool: &ToolDef) -> Result<(), ToolDeclarationError> {
    if tool.name.is_empty() {
        return Err(ToolDeclarationError::EmptyName);
    }
    if let Err(error) = schema::compile(tool.input_schema.clone()) {
        return Err(ToolDeclarationError::SchemaCompilation {
            name: tool.name.clone(),
            detail: error.to_string(),
        });
    }
    Ok(())
}

/// Validate a batch of tool declarations: each one individually, plus unique
/// names within the batch (an ambiguous name fails before installation).
pub fn validate_tool_batch(tools: &[ToolDef]) -> Result<(), ToolDeclarationError> {
    for tool in tools {
        validate_tool_declaration(tool)?;
    }
    for (index, tool) in tools.iter().enumerate() {
        if tools[..index].iter().any(|other| other.name == tool.name) {
            return Err(ToolDeclarationError::DuplicateName {
                name: tool.name.clone(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool(name: &str, description: &str) -> ToolDef {
        ToolDef {
            name: name.to_owned(),
            description: description.to_owned(),
            input_schema: serde_json::json!({"type": "object"}),
            constrained_sampling: None,
        }
    }

    #[test]
    fn sections_round_trip_through_an_ordered_json_object() {
        let message = SystemMessage {
            content: "base".to_owned(),
            sections: vec![
                SystemSection {
                    name: "a".to_owned(),
                    value: Some("one".to_owned()),
                },
                SystemSection {
                    name: "b".to_owned(),
                    value: None,
                },
            ],
            tools_added: vec![],
            tools_removed: vec![],
            timestamp_ms: 5,
        };
        let json = serde_json::to_value(&message).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "content": "base",
                "sections": {"a": "one", "b": null},
                "timestamp_ms": 5,
            })
        );
        let back: SystemMessage = serde_json::from_value(json).unwrap();
        assert_eq!(back, message);
    }

    #[test]
    fn duplicate_section_names_keep_first_position_and_last_value() {
        let json = serde_json::json!({"a": "first", "b": "keep", "a": "last"});
        // serde_json keeps the last duplicate key; assert via manual map.
        let raw = r#"{"sections": {"a": "first", "b": "keep", "a": "last"}, "content": "", "timestamp_ms": 0}"#;
        let _ = json;
        let message: SystemMessage = serde_json::from_str(raw).unwrap();
        assert_eq!(
            message.sections,
            vec![
                SystemSection {
                    name: "a".to_owned(),
                    value: Some("last".to_owned())
                },
                SystemSection {
                    name: "b".to_owned(),
                    value: Some("keep".to_owned())
                },
            ]
        );
    }

    #[test]
    fn non_string_section_value_is_rejected() {
        let raw = r#"{"sections": {"a": 3}, "content": "", "timestamp_ms": 0}"#;
        assert!(serde_json::from_str::<SystemMessage>(raw).is_err());
    }

    #[test]
    fn normalize_folds_shorthand_into_one_leading_entry() {
        let context = Context {
            system_prompt: Some("prompt".to_owned()),
            messages: vec![Message::User(crate::message::UserMessage {
                content: vec![],
                timestamp_ms: 1,
            })],
            tools: vec![tool("t", "d")],
        };
        let normalized = normalize_context(context.clone()).expect("valid context declarations");
        let head = get_initial_system_message(normalized.messages()).unwrap();
        assert_eq!(head.content, "prompt");
        assert_eq!(head.tools_added.len(), 1);
        assert_eq!(head.timestamp_ms, 0);
        // Re-normalizing through an empty shorthand is the identity.
        let again = normalize_context(Context {
            messages: normalized.messages().to_vec(),
            ..Context::default()
        })
        .expect("valid context declarations");
        assert_eq!(again, normalized);
    }

    #[test]
    fn empty_shorthand_stays_empty() {
        let normalized = normalize_context(Context::default()).expect("valid context declarations");
        assert!(normalized.messages().is_empty());
        assert!(get_current_system_message(normalized.messages()).is_none());
    }

    #[test]
    fn tools_only_shorthand_has_empty_content() {
        let normalized = normalize_context(Context {
            tools: vec![tool("t", "d")],
            ..Context::default()
        })
        .expect("valid context declarations");
        let head = get_initial_system_message(normalized.messages()).unwrap();
        assert_eq!(head.content, "");
        assert_eq!(get_current_tools(normalized.messages()).len(), 1);
    }

    #[test]
    fn replay_appends_content_patches_sections_and_resolves_tools() {
        let mut transcript = normalize_context(Context {
            system_prompt: Some("base".to_owned()),
            tools: vec![tool("a", "first"), tool("b", "keep")],
            ..Context::default()
        })
        .expect("valid context declarations");
        transcript
            .append_system_change(SystemMessage {
                content: "more".to_owned(),
                sections: vec![
                    SystemSection {
                        name: "s1".to_owned(),
                        value: Some("v1".to_owned()),
                    },
                    SystemSection {
                        name: "s2".to_owned(),
                        value: Some("v2".to_owned()),
                    },
                ],
                tools_added: vec![tool("a", "changed")],
                tools_removed: vec![],
                timestamp_ms: 10,
            })
            .unwrap();
        transcript
            .append_system_change(SystemMessage {
                content: String::new(),
                sections: vec![
                    SystemSection {
                        name: "s1".to_owned(),
                        value: None,
                    },
                    SystemSection {
                        name: "s2".to_owned(),
                        value: Some("v2-new".to_owned()),
                    },
                ],
                tools_added: vec![],
                tools_removed: vec![ToolReference {
                    name: "b".to_owned(),
                }],
                timestamp_ms: 20,
            })
            .unwrap();

        let head = get_current_system_message(transcript.messages()).unwrap();
        assert_eq!(head.content, "base\n\nmore");
        // s2 replacement keeps its position after s1 was deleted.
        assert_eq!(
            head.sections,
            vec![SystemSection {
                name: "s2".to_owned(),
                value: Some("v2-new".to_owned())
            }]
        );
        let tools = get_current_tools(transcript.messages());
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].description, "changed");
        assert_eq!(head.timestamp_ms, 0);
        // delete + readd appends to the end
        let mut transcript2 = transcript.clone();
        transcript2
            .append_system_change(SystemMessage {
                sections: vec![SystemSection {
                    name: "s3".to_owned(),
                    value: Some("x".to_owned()),
                }],
                timestamp_ms: 30,
                ..SystemMessage::default()
            })
            .unwrap();
        let head2 = get_current_system_message(transcript2.messages()).unwrap();
        assert_eq!(head2.sections.len(), 2);
        assert_eq!(head2.sections[1].name, "s3");
    }

    #[test]
    fn body_and_sections_stay_distinguishable_in_the_rendered_prompt() {
        let message = SystemMessage {
            content: "body".to_owned(),
            sections: vec![SystemSection {
                name: "sec".to_owned(),
                value: Some("value".to_owned()),
            }],
            ..SystemMessage::default()
        };
        assert_eq!(system_message_text(&message), "body\n\nvalue");
    }

    #[test]
    fn collapse_returns_a_fresh_head_and_never_rewrites_canonical() {
        let user = Message::User(crate::message::UserMessage {
            content: vec![],
            timestamp_ms: 1,
        });
        let mut transcript = normalize_context(Context {
            system_prompt: Some("base".to_owned()),
            messages: vec![user],
            ..Context::default()
        })
        .expect("valid context declarations");
        transcript
            .append_system_change(SystemMessage {
                content: "later".to_owned(),
                timestamp_ms: 9,
                ..SystemMessage::default()
            })
            .unwrap();
        let canonical_len = transcript.messages().len();
        let collapsed = collapse_system_messages(&transcript);
        assert_eq!(collapsed.messages().len(), 2); // head + user messages
        assert_eq!(
            get_current_system_prompt(collapsed.messages()),
            "base\n\nlater"
        );
        assert_eq!(transcript.messages().len(), canonical_len); // untouched
    }

    #[test]
    fn strict_projection_rejects_unplaceable_history_before_requests() {
        let mut transcript = normalize_context(Context {
            system_prompt: Some("base".to_owned()),
            ..Context::default()
        })
        .expect("valid context declarations");
        transcript
            .append_system_change(SystemMessage {
                content: "later".to_owned(),
                timestamp_ms: 9,
                ..SystemMessage::default()
            })
            .unwrap();
        let error =
            resolve_transcript(&transcript, false, TranscriptProjection::Strict).unwrap_err();
        assert_eq!(
            error,
            TranscriptError::UnsupportedTemporalFidelity {
                mid_convo_system_messages: 1
            }
        );
        // Native capability keeps the canonical value by reference.
        let native = resolve_transcript(&transcript, true, TranscriptProjection::Strict).unwrap();
        assert!(matches!(native.context, Cow::Borrowed(_)));
        // Default projection collapses instead of failing.
        let collapsed =
            resolve_transcript(&transcript, false, TranscriptProjection::NativeOrCollapse).unwrap();
        assert!(matches!(collapsed.context, Cow::Owned(_)));
    }

    #[test]
    fn one_unchanged_initial_state_is_lossless_under_collapse() {
        let transcript = normalize_context(Context {
            system_prompt: Some("base".to_owned()),
            tools: vec![tool("t", "d")],
            ..Context::default()
        })
        .expect("valid context declarations");
        let collapsed = collapse_system_messages(&transcript);
        assert_eq!(collapsed, transcript);
    }

    #[test]
    fn tool_state_changes_redefine_as_remove_plus_add_and_dedup_equal_state() {
        let previous = vec![tool("a", "1"), tool("b", "2")];
        let current = vec![tool("a", "1-changed"), tool("c", "3")];
        let changes = get_tool_state_changes(&previous, &current);
        assert_eq!(changes.tools_added.len(), 2); // a-changed + c
        // A changed definition is a removal followed by an addition: "a"
        // changed and "b" gone are both removals.
        assert_eq!(
            changes.tools_removed,
            vec![
                ToolReference {
                    name: "a".to_owned()
                },
                ToolReference {
                    name: "b".to_owned()
                },
            ]
        );
        // equal state -> no redundant change
        let same = get_tool_state_changes(&previous, &previous);
        assert!(same.is_empty());
        assert!(tool_state_change_message(&previous, &previous, 1).is_none());
        let message = tool_state_change_message(&previous, &current, 7).unwrap();
        assert_eq!(message.timestamp_ms, 7);
    }

    #[test]
    fn declared_tools_keep_first_declaration_order() {
        let mut transcript = normalize_context(Context {
            tools: vec![tool("a", "1"), tool("b", "2")],
            ..Context::default()
        })
        .expect("valid context declarations");
        transcript
            .append_system_change(SystemMessage {
                tools_added: vec![tool("c", "3"), tool("a", "1-again")],
                timestamp_ms: 5,
                ..SystemMessage::default()
            })
            .unwrap();
        let declared = get_declared_tools(transcript.messages());
        let names: Vec<&str> = declared.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["a", "b", "c"]);
        assert!(has_non_additive_tool_changes(transcript.messages()));
    }

    #[test]
    fn non_additive_detection_flags_removals_and_redeclarations() {
        let mut transcript = normalize_context(Context {
            tools: vec![tool("a", "1")],
            ..Context::default()
        })
        .expect("valid context declarations");
        assert!(!has_non_additive_tool_changes(transcript.messages()));
        transcript
            .append_system_change(SystemMessage {
                tools_removed: vec![ToolReference {
                    name: "a".to_owned(),
                }],
                timestamp_ms: 5,
                ..SystemMessage::default()
            })
            .unwrap();
        assert!(has_non_additive_tool_changes(transcript.messages()));
    }

    #[test]
    fn anchored_additions_split_keeps_initial_tools_only_when_additive() {
        let mut transcript = normalize_context(Context {
            tools: vec![tool("a", "1")],
            ..Context::default()
        })
        .expect("valid context declarations");
        transcript
            .append_system_change(SystemMessage {
                tools_added: vec![tool("b", "2")],
                timestamp_ms: 5,
                ..SystemMessage::default()
            })
            .unwrap();
        let split = resolve_transcript_tools(transcript.messages(), true);
        assert!(split.anchors_additions);
        assert_eq!(split.request_tools.len(), 1);
        assert_eq!(split.request_tools[0].name, "a");
        let split_off = resolve_transcript_tools(transcript.messages(), false);
        assert!(!split_off.anchors_additions);
        assert_eq!(split_off.request_tools.len(), 2);
    }

    #[test]
    fn invalid_declarations_fail_before_installation() {
        let bad_name = ToolDef {
            name: String::new(),
            description: "d".to_owned(),
            input_schema: serde_json::json!({"type": "object"}),
            constrained_sampling: None,
        };
        assert_eq!(
            validate_tool_declaration(&bad_name),
            Err(ToolDeclarationError::EmptyName)
        );
        // The dialect engine is fail-closed at admission for regex leaves;
        // an inert unknown keyword is NOT a metaschema admission error
        // (AIR-008), so the failing shape is an uncompilable pattern.
        let bad_schema = ToolDef {
            name: "t".to_owned(),
            description: "d".to_owned(),
            input_schema: serde_json::json!({"properties": {"x": {"type": "string", "pattern": "["}}}),
            constrained_sampling: None,
        };
        assert!(matches!(
            validate_tool_declaration(&bad_schema),
            Err(ToolDeclarationError::SchemaCompilation { .. })
        ));
        let dup = vec![tool("t", "1"), tool("t", "2")];
        assert!(matches!(
            validate_tool_batch(&dup),
            Err(ToolDeclarationError::DuplicateName { .. })
        ));
        let mut transcript =
            normalize_context(Context::default()).expect("valid context declarations");
        let before = transcript.messages().len();
        assert!(
            transcript
                .append_system_change(SystemMessage {
                    tools_added: vec![bad_schema],
                    timestamp_ms: 1,
                    ..SystemMessage::default()
                })
                .is_err()
        );
        assert_eq!(transcript.messages().len(), before); // not installed
    }

    #[test]
    fn update_rendering_frames_sections_by_name() {
        let message = SystemMessage {
            content: "note".to_owned(),
            sections: vec![
                SystemSection {
                    name: "gone".to_owned(),
                    value: None,
                },
                SystemSection {
                    name: "kept".to_owned(),
                    value: Some("text".to_owned()),
                },
            ],
            ..SystemMessage::default()
        };
        assert_eq!(
            render_system_message_update(&message),
            "note\n\nRemoved system prompt section \"gone\".\n\nUpdated system prompt section \"kept\":\n\ntext"
        );
    }

    #[test]
    fn without_initial_drops_only_the_leading_system_message() {
        let mut transcript = normalize_context(Context {
            system_prompt: Some("base".to_owned()),
            ..Context::default()
        })
        .expect("valid context declarations");
        transcript
            .append_system_change(SystemMessage {
                content: "later".to_owned(),
                timestamp_ms: 9,
                ..SystemMessage::default()
            })
            .unwrap();
        let tail = without_initial_system_message(transcript.messages());
        assert_eq!(tail.len(), 1);
        assert!(matches!(tail[0], Message::System(_)));
    }

    #[test]
    fn transcript_context_is_serde_transparent() {
        let transcript = normalize_context(Context {
            system_prompt: Some("p".to_owned()),
            ..Context::default()
        })
        .expect("valid context declarations");
        let json = serde_json::to_string(&transcript).unwrap();
        let back: TranscriptContext = serde_json::from_str(&json).unwrap();
        assert_eq!(back, transcript);
    }

    #[test]
    fn declaration_equality_is_key_order_independent() {
        let left = tool("t", "d");
        let mut right = tool("t", "d");
        right.input_schema = serde_json::json!({"properties": {"a": {"type": "string"}, "b": {"type": "integer"}}, "type": "object"});
        let mut reordered = tool("t", "d");
        reordered.input_schema = serde_json::json!({"type": "object", "properties": {"b": {"type": "integer"}, "a": {"type": "string"}}});
        assert!(declarations_equal(&left, &right) == declarations_equal(&left, &reordered));
        assert!(declarations_equal(&right, &reordered));
    }
}
