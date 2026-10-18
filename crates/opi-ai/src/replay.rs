//! Canonical replay projection (AIR-010/012/013/014, task 20.7).
//!
//! A replay projection turns a canonical message chronology into the
//! request-local view a provider wire consumes. It never mutates the
//! canonical history, never executes tools, never grants execution
//! permission, and never fabricates successful calls: every departure from
//! the canonical facts is either a recorded projection fact (collapse
//! mode) or a typed error raised before any request is built (strict
//! mode). Pinned TypeScript reference:
//! `.repo/pi-1.3.0/packages/ai/src/api/transform-messages.ts` (Pi AI
//! 1.3.0); the association, missing-result and malformed-proposal
//! semantics follow the reviewed replay contracts
//! (`docs/research/pi-ai-opi-ai-alignment/replay-contracts.md`, RP-04/05/
//! 06/08/09).
//!
//! Contract boundaries:
//! - Continuation identity is the exact wire, the provider and the
//!   requested model (RP-02). A history record without a stored wire is
//!   legacy-unknown and is never treated as same-identity (fail-closed).
//! - Cross-identity thinking is dropped entirely - readable, signed and
//!   redacted blocks alike - and is never converted into visible text
//!   (Pi 1.3 semantics). Same-identity keeps redacted data and signed
//!   blocks (including signed-empty ones) and drops blank unsigned
//!   thinking.
//! - Association pairs tool results with calls by batch position and
//!   source ID. A closed batch may reuse an ID string; inside one batch a
//!   duplicate call ID, a duplicate result, an orphan result or a
//!   result/call name mismatch is a typed association error. System
//!   messages between a call and its results are transparent to the
//!   batch and are emitted after them.
//! - A call with an unambiguous missing result is closed request-locally
//!   with a synthetic error result stating the result is unavailable and
//!   the execution effect is unknown. It never claims the tool was not
//!   executed and never re-executes it; only a trusted Agent path records
//!   known non-execution (AIR-012/RP-09).
//! - Failed assistant turns (`StopReason::Error`/`Aborted`) and their
//!   linked non-execution results are omitted from the wire as a closed
//!   unit; the canonical history keeps them (AIR-013/RP-04). `Length`
//!   turns stay on the wire with their expressible pairs; length never
//!   grants execution permission anywhere in this projection.
//! - A malformed proposal (`ToolCall::arguments_error`) is replayed as its
//!   raw text when the wire accepts string arguments, and replaced by an
//!   explicit named failure context when the wire only accepts objects -
//!   never as `{}`, `null` or a fabricated successful call (RP-08).
//! - Wire IDs are legalized request-locally (charset and length limits)
//!   with a stable bijective mapping applied synchronously to calls and
//!   results; the canonical IDs and any execution/evidence identity are
//!   untouched (RP-07).
//! - Images: a route that does not support images receives explicit text
//!   placeholders (known text-only models are additionally rejected
//!   before dispatch by the capability preflight); supported routes keep
//!   real image blocks with their tool association, order and failure
//!   flag. Private `details` never enter the wire and URL sources are
//!   passed through, never downloaded (AIR-014).

use crate::message::{
    AssistantContent, AssistantMessage, InputContent, Message, OutputContent, ToolCall,
    ToolResultMessage,
};
use crate::model_info::WireApi;
use crate::stream::StopReason;

/// Text emitted in place of user images on routes without image input.
const NON_VISION_USER_IMAGE_PLACEHOLDER: &str = "(image omitted: model does not support images)";
/// Text emitted in place of tool-result images on routes without image input.
const NON_VISION_TOOL_IMAGE_PLACEHOLDER: &str =
    "(tool image omitted: model does not support images)";

/// Route identity and wire capabilities for one replay projection.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplayRoute {
    /// Exact wire the request will be dispatched on.
    pub wire: WireApi,
    /// Provider the request targets.
    pub provider: String,
    /// Requested model ID (aliases resolved the same way the adapter
    /// resolves `Request::model`).
    pub model: String,
    /// Whether the route's model accepts image input.
    pub supports_images: bool,
}

/// Projection policy. `NativeOrCollapse` (the default) performs the
/// smallest lossy projection the route requires and records every
/// departure as a fact; `Strict` rejects the same departures with a typed
/// error before any request is built, mirroring
/// [`crate::transcript::TranscriptProjection`] for the chronology seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReplayProjection {
    #[default]
    NativeOrCollapse,
    Strict,
}

/// One recorded (or strict-rejected) departure from the canonical facts.
#[derive(Debug, Clone, PartialEq)]
pub enum ReplayLoss {
    /// Thinking blocks dropped because the producing turn is not
    /// same-identity; includes readable, signed and redacted blocks.
    DroppedThinkingBlocks { count: usize },
    /// A failed assistant turn (and its linked results) omitted from the
    /// wire.
    OmittedFailedAssistant {
        stop_reason: StopReason,
        tool_call_ids: Vec<String>,
    },
    /// A missing result closed with a synthetic request-local error
    /// result (result unavailable, execution effect unknown).
    SynthesizedMissingResult { tool_call_id: String },
    /// An unrepresentable malformed proposal replaced by an explicit
    /// failure context.
    MalformedFailureContext { tool_call_id: String },
    /// Image content downgraded to an explicit text placeholder.
    ImagePlaceholder { count: usize },
    /// A wire ID legalized for the route's charset/length limits.
    WireIdRemap { from: String, to: String },
}

impl ReplayLoss {
    fn describe(&self) -> String {
        match self {
            Self::DroppedThinkingBlocks { count } => {
                format!("dropped {count} thinking blocks from cross-identity history")
            }
            Self::OmittedFailedAssistant {
                stop_reason,
                tool_call_ids,
            } => format!(
                "omitted failed assistant turn (stop_reason {:?}) with linked results {tool_call_ids:?}",
                stop_reason.as_str()
            ),
            Self::SynthesizedMissingResult { tool_call_id } => format!(
                "synthesized a request-local result for missing result of tool call {tool_call_id:?}"
            ),
            Self::MalformedFailureContext { tool_call_id } => format!(
                "replaced malformed tool call {tool_call_id:?} with an explicit failure context"
            ),
            Self::ImagePlaceholder { count } => {
                format!("replaced {count} image block(s) with text placeholders")
            }
            Self::WireIdRemap { from, to } => {
                format!("legalized wire id {from:?} to {to:?}")
            }
        }
    }
}

/// Typed replay failures raised before authorization or network use.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ReplayError {
    #[error("duplicate tool call id {tool_call_id:?} inside one assistant batch")]
    DuplicateCallInBatch { tool_call_id: String },
    #[error("duplicate tool result for tool call {tool_call_id:?} in its batch")]
    DuplicateResult { tool_call_id: String },
    #[error(
        "tool result {tool_call_id:?} does not belong to any open tool-call batch; \
         closed batches may not receive further results"
    )]
    OrphanResult { tool_call_id: String },
    #[error(
        "tool result for call {tool_call_id:?} names {result_name:?} but the call names {call_name:?}"
    )]
    ResultNameMismatch {
        tool_call_id: String,
        call_name: String,
        result_name: String,
    },
    #[error(
        "strict replay projection rejected lossy transforms before any request was built: {}",
        .0.iter().map(|l| l.describe()).collect::<Vec<_>>().join("; ")
    )]
    StrictLoss(Vec<ReplayLoss>),
}

/// Recorded projection facts for one call to [`project_replay`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReplayFacts {
    pub losses: Vec<ReplayLoss>,
}

impl ReplayFacts {
    fn record(&mut self, loss: ReplayLoss) {
        self.losses.push(loss);
    }
}

/// Result of a successful replay projection: the request-local message
/// list plus the recorded projection facts. The canonical input is never
/// modified; the same canonical input and route always project to the
/// same output (stable across retries of one prepared call, RP-01).
#[derive(Debug, Clone, PartialEq)]
pub struct ReplayOutput {
    pub messages: Vec<Message>,
    pub facts: ReplayFacts,
}

/// Continuation-identity check shared by the replay projection and wire
/// serializers that need per-message provenance (exact wire + provider +
/// requested model; legacy-unknown is never same, RP-02).
pub fn same_identity(assistant: &AssistantMessage, route: &ReplayRoute) -> bool {
    assistant.wire == Some(route.wire)
        && assistant.provider == route.provider
        && assistant.model == route.model
}

/// Image-input capability for the route from the provider's model
/// catalog. An unknown model defaults to permissive (`true`): the
/// capability preflight (`validate_request_for_model`) is the typed
/// rejection boundary for known text-only models, and an unknown model
/// has no attested text-only fact to reject on.
pub fn supports_images_in_catalog(models: &[crate::model_info::ModelInfo], model_id: &str) -> bool {
    models
        .iter()
        .find(|model| model.id == model_id)
        .is_none_or(|model| model.capabilities.supports_images)
}

/// Map a replay failure onto the provider error surface so adapter
/// request construction can fail typed before authorization or network
/// use (association errors are configuration of the request history, not
/// transport conditions).
impl From<ReplayError> for crate::provider::ProviderError {
    fn from(error: ReplayError) -> Self {
        Self::Config(crate::provider::ProviderErrorSummary::sanitized(
            error.to_string(),
        ))
    }
}

/// Whether the wire's tool-call arguments field accepts raw JSON text
/// (Chat, Responses families) or requires a JSON object value
/// (Anthropic, Bedrock, Google families). Malformed proposals are only
/// wire-expressible on the string side.
fn wire_accepts_string_arguments(wire: WireApi) -> bool {
    matches!(
        wire,
        WireApi::OpenAiCompletions
            | WireApi::AzureOpenAiCompletions
            | WireApi::OpenAiResponses
            | WireApi::OpenAiCodexResponses
    )
}

/// Wire ID constraints applied request-locally (RP-07). `None` means the
/// wire accepts arbitrary non-empty ID strings.
fn wire_id_constraints(wire: WireApi) -> Option<(usize, bool)> {
    match wire {
        // ^[a-zA-Z0-9_-]+$, max 64 chars (Anthropic Messages; Bedrock
        // Converse applies the same shape to toolUseId).
        WireApi::AnthropicMessages | WireApi::BedrockConverseStream => Some((64, true)),
        _ => None,
    }
}

/// FNV-1a 64-bit: stable, dependency-free digest for collision-safe ID
/// suffixes.
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Legalize one wire ID under the route's constraints. The mapping is a
/// pure function of the input: identical canonical IDs always map to the
/// identical wire ID (retry stability), and the hash suffix keeps the
/// mapping injective for realistic inputs. Empty input maps to a
/// synthesized non-empty ID rather than being passed through.
fn legalize_wire_id(id: &str, constraints: Option<(usize, bool)>) -> String {
    let Some((max_len, restricted_charset)) = constraints else {
        return if id.is_empty() {
            format!("call_{:016x}", fnv1a64(id.as_bytes()))
        } else {
            id.to_owned()
        };
    };
    let sanitized: String = if restricted_charset {
        id.chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                    c
                } else {
                    '-'
                }
            })
            .collect()
    } else {
        id.to_owned()
    };
    if sanitized.len() <= max_len && !sanitized.is_empty() && sanitized == id {
        return sanitized;
    }
    // Reserve room for a 17-char hash suffix ("_" + 16 hex digits) inside
    // the limit; the digest covers the full original ID so distinct
    // originals cannot collide after truncation.
    let budget = max_len.saturating_sub(17).max(1);
    let prefix: String = sanitized.chars().take(budget).collect();
    format!("{prefix}_{:016x}", fnv1a64(id.as_bytes()))
}

/// One registered call in the open batch: the canonical call plus its
/// projection disposition.
struct BatchCall {
    tool_call: ToolCall,
    /// True when the proposal is malformed and the wire only accepts
    /// objects: the call is not emitted and its result is consumed by the
    /// failure context.
    unrepresentable: bool,
}

struct ReplayState<'a> {
    route: &'a ReplayRoute,
    projection: ReplayProjection,
    id_constraints: Option<(usize, bool)>,
    out: Vec<Message>,
    facts: ReplayFacts,
    strict_losses: Vec<ReplayLoss>,
    open_batch: Vec<BatchCall>,
    answered: Vec<String>,
    held_systems: Vec<Message>,
    /// Call IDs of the most recently omitted failed assistant turn: their
    /// linked results are consumed by the omission closure until the next
    /// batch boundary.
    failed_batch_ids: Vec<String>,
    timestamp_ms: i64,
}

impl<'a> ReplayState<'a> {
    fn legalize(&mut self, id: &str) -> String {
        let legalized = legalize_wire_id(id, self.id_constraints);
        if legalized != id {
            self.facts.record(ReplayLoss::WireIdRemap {
                from: id.to_owned(),
                to: legalized.clone(),
            });
        }
        legalized
    }

    fn same_identity(&self, assistant: &AssistantMessage) -> bool {
        same_identity(assistant, self.route)
    }

    fn strict(&mut self, loss: ReplayLoss) {
        if self.projection == ReplayProjection::Strict {
            self.strict_losses.push(loss);
        }
    }

    fn close_batch(&mut self) {
        let batch = std::mem::take(&mut self.open_batch);
        if !batch.is_empty() {
            for call in &batch {
                if call.unrepresentable || self.answered.contains(&call.tool_call.id) {
                    continue;
                }
                self.strict(ReplayLoss::SynthesizedMissingResult {
                    tool_call_id: call.tool_call.id.clone(),
                });
                self.facts.record(ReplayLoss::SynthesizedMissingResult {
                    tool_call_id: call.tool_call.id.clone(),
                });
                let wire_id = self.legalize(&call.tool_call.id);
                self.out.push(Message::ToolResult(ToolResultMessage {
                    tool_call_id: wire_id,
                    tool_name: call.tool_call.name.clone(),
                    content: vec![OutputContent::Text {
                        text: format!(
                            "Result unavailable: no result was recorded for tool call '{}' \
                             ('{}'); this request did not execute the tool and its \
                             execution effect is unknown.",
                            call.tool_call.id, call.tool_call.name
                        ),
                    }],
                    details: None,
                    is_error: true,
                    truncated: false,
                    timestamp_ms: self.timestamp_ms,
                }));
            }
        }
        self.answered.clear();
        self.failed_batch_ids.clear();
        self.out.append(&mut self.held_systems);
    }

    fn push_assistant(&mut self, assistant: &AssistantMessage) -> Result<(), ReplayError> {
        let same = self.same_identity(assistant);
        let string_args = wire_accepts_string_arguments(self.route.wire);
        let mut content: Vec<AssistantContent> = Vec::new();
        let mut calls: Vec<BatchCall> = Vec::new();
        let mut dropped_thinking = 0usize;

        for block in &assistant.content {
            match block {
                AssistantContent::Thinking {
                    thinking,
                    signature,
                } => {
                    if !same {
                        dropped_thinking += 1;
                        continue;
                    }
                    let signed = signature.is_some();
                    let blank = thinking.trim().is_empty();
                    if signed || !blank {
                        content.push(block.clone());
                    }
                    // Blank unsigned thinking carries no replayable
                    // material and is dropped on every wire (including
                    // same-identity ones).
                }
                AssistantContent::RedactedThinking { .. } => {
                    if same {
                        content.push(block.clone());
                    } else {
                        dropped_thinking += 1;
                    }
                }
                AssistantContent::Text { .. } => content.push(block.clone()),
                AssistantContent::ToolCall { tool_call } => {
                    if calls.iter().any(|c| c.tool_call.id == tool_call.id) {
                        // Duplicate call IDs inside one batch are an
                        // association error regardless of policy.
                        return Err(ReplayError::DuplicateCallInBatch {
                            tool_call_id: tool_call.id.clone(),
                        });
                    }
                    // Legacy records without the AIR-048 error mark are
                    // still checked by parse: an unmarked malformed string
                    // must not fabricate an empty object downstream.
                    let unrepresentable = !string_args
                        && (tool_call.arguments_error.is_some()
                            || serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(
                                &tool_call.arguments,
                            )
                            .is_err());
                    if unrepresentable {
                        let wire_id = self.legalize(&tool_call.id);
                        self.strict(ReplayLoss::MalformedFailureContext {
                            tool_call_id: tool_call.id.clone(),
                        });
                        self.facts.record(ReplayLoss::MalformedFailureContext {
                            tool_call_id: tool_call.id.clone(),
                        });
                        let note = format!(
                            "[tool proposal not replayed] tool '{}' (call id '{}') proposed \
                             malformed arguments that this wire cannot represent; the proposal \
                             was not executed and its recorded result is omitted from this \
                             request. Parse failure: {}",
                            tool_call.name,
                            wire_id,
                            tool_call
                                .arguments_error
                                .as_deref()
                                .unwrap_or("malformed arguments")
                        );
                        content.push(AssistantContent::Text { text: note });
                    } else {
                        let mut projected = tool_call.clone();
                        projected.id = self.legalize(&tool_call.id);
                        content.push(AssistantContent::ToolCall {
                            tool_call: projected,
                        });
                    }
                    calls.push(BatchCall {
                        tool_call: tool_call.clone(),
                        unrepresentable,
                    });
                }
            }
        }
        if dropped_thinking > 0 {
            self.strict(ReplayLoss::DroppedThinkingBlocks {
                count: dropped_thinking,
            });
            self.facts.record(ReplayLoss::DroppedThinkingBlocks {
                count: dropped_thinking,
            });
        }

        if content.is_empty() && calls.is_empty() {
            // Every block was projected away (typically a cross-identity
            // thinking-only turn); an empty assistant message would be
            // rejected by the wire, so the turn collapses to nothing. Any
            // results that follow belong to no open batch and are closed
            // by the orphan rule.
            self.open_batch = calls;
            self.answered.clear();
            return Ok(());
        }
        self.open_batch = calls;
        self.answered.clear();
        let mut projected = assistant.clone();
        projected.content = content;
        self.out.push(Message::Assistant(projected));
        Ok(())
    }
}

/// Replace image runs in user content with explicit text placeholders
/// (Pi 1.3 `downgradeUnsupportedImages` semantics: one placeholder per
/// run of images, not one per block).
fn downgrade_user_images(content: &[InputContent], placeholder: &str) -> Vec<InputContent> {
    let mut result = Vec::new();
    let mut previous_was_placeholder = false;
    for block in content {
        match block {
            InputContent::Image { .. } => {
                if !previous_was_placeholder {
                    result.push(InputContent::Text {
                        text: placeholder.to_owned(),
                    });
                }
                previous_was_placeholder = true;
            }
            InputContent::Text { text } => {
                previous_was_placeholder = text == placeholder;
                result.push(block.clone());
            }
        }
    }
    result
}

/// Replace image runs in tool-result content with explicit placeholders.
fn downgrade_result_images(content: &[OutputContent], placeholder: &str) -> Vec<OutputContent> {
    let mut result = Vec::new();
    let mut previous_was_placeholder = false;
    for block in content {
        match block {
            OutputContent::Image { .. } => {
                if !previous_was_placeholder {
                    result.push(OutputContent::Text {
                        text: placeholder.to_owned(),
                    });
                }
                previous_was_placeholder = true;
            }
            OutputContent::Text { text } => {
                previous_was_placeholder = text == placeholder;
                result.push(block.clone());
            }
        }
    }
    result
}

/// Project a canonical chronology into the request-local wire view.
///
/// The input is the body message list after the transcript projection
/// (leading system entry removed by
/// [`crate::transcript::without_initial_system_message`]); mid-conversation
/// system entries are handled here and stay transparent to tool-call
/// batches. Association errors and strict-policy losses are raised as
/// [`ReplayError`] before any request is built; collapse-mode departures
/// are recorded in [`ReplayFacts`].
pub fn project_replay(
    messages: &[Message],
    route: &ReplayRoute,
    projection: ReplayProjection,
) -> Result<ReplayOutput, ReplayError> {
    let mut state = ReplayState {
        route,
        projection,
        id_constraints: wire_id_constraints(route.wire),
        out: Vec::new(),
        facts: ReplayFacts::default(),
        strict_losses: Vec::new(),
        open_batch: Vec::new(),
        answered: Vec::new(),
        held_systems: Vec::new(),
        failed_batch_ids: Vec::new(),
        timestamp_ms: 0,
    };

    for message in messages {
        match message {
            Message::Assistant(assistant) => {
                state.close_batch();
                match assistant.stop_reason {
                    StopReason::Error | StopReason::Aborted => {
                        let tool_call_ids: Vec<String> = assistant
                            .content
                            .iter()
                            .filter_map(|block| match block {
                                AssistantContent::ToolCall { tool_call } => {
                                    Some(tool_call.id.clone())
                                }
                                _ => None,
                            })
                            .collect();
                        state.strict(ReplayLoss::OmittedFailedAssistant {
                            stop_reason: assistant.stop_reason,
                            tool_call_ids: tool_call_ids.clone(),
                        });
                        state.facts.record(ReplayLoss::OmittedFailedAssistant {
                            stop_reason: assistant.stop_reason,
                            tool_call_ids: tool_call_ids.clone(),
                        });
                        state.failed_batch_ids = tool_call_ids;
                    }
                    _ => state.push_assistant(assistant)?,
                }
            }
            Message::ToolResult(result) => {
                if state.failed_batch_ids.contains(&result.tool_call_id) {
                    // Linked non-execution result of an omitted failed
                    // turn: consumed by the omission closure (AIR-013).
                    continue;
                }
                if state
                    .open_batch
                    .iter()
                    .any(|c| c.unrepresentable && c.tool_call.id == result.tool_call_id)
                {
                    // Consumed by the explicit failure context that
                    // replaced the malformed proposal.
                    continue;
                }
                let call = state
                    .open_batch
                    .iter()
                    .find(|c| c.tool_call.id == result.tool_call_id)
                    .ok_or_else(|| ReplayError::OrphanResult {
                        tool_call_id: result.tool_call_id.clone(),
                    })?;
                if state.answered.contains(&result.tool_call_id) {
                    return Err(ReplayError::DuplicateResult {
                        tool_call_id: result.tool_call_id.clone(),
                    });
                }
                if result.tool_name != call.tool_call.name {
                    return Err(ReplayError::ResultNameMismatch {
                        tool_call_id: result.tool_call_id.clone(),
                        call_name: call.tool_call.name.clone(),
                        result_name: result.tool_name.clone(),
                    });
                }
                state.answered.push(result.tool_call_id.clone());
                let mut projected = result.clone();
                projected.tool_call_id = state.legalize(&result.tool_call_id);
                if !state.route.supports_images {
                    let before = projected
                        .content
                        .iter()
                        .filter(|c| matches!(c, OutputContent::Image { .. }))
                        .count();
                    if before > 0 {
                        projected.content = downgrade_result_images(
                            &projected.content,
                            NON_VISION_TOOL_IMAGE_PLACEHOLDER,
                        );
                        state.strict(ReplayLoss::ImagePlaceholder { count: before });
                        state
                            .facts
                            .record(ReplayLoss::ImagePlaceholder { count: before });
                    }
                }
                state.out.push(Message::ToolResult(projected));
            }
            Message::System(_) => {
                if !state.open_batch.is_empty() {
                    // Transparent to the batch: held back until the
                    // batch's results (synthetic ones included) are
                    // emitted, so it never splits a call from its result.
                    state.held_systems.push(message.clone());
                } else {
                    state.out.push(message.clone());
                }
            }
            Message::User(user) => {
                state.close_batch();
                let mut projected = user.clone();
                if !state.route.supports_images {
                    let before = projected
                        .content
                        .iter()
                        .filter(|c| matches!(c, InputContent::Image { .. }))
                        .count();
                    if before > 0 {
                        projected.content = downgrade_user_images(
                            &projected.content,
                            NON_VISION_USER_IMAGE_PLACEHOLDER,
                        );
                        state.strict(ReplayLoss::ImagePlaceholder { count: before });
                        state
                            .facts
                            .record(ReplayLoss::ImagePlaceholder { count: before });
                    }
                }
                state.out.push(Message::User(projected));
            }
        }
    }
    state.close_batch();

    if !state.strict_losses.is_empty() {
        return Err(ReplayError::StrictLoss(std::mem::take(
            &mut state.strict_losses,
        )));
    }
    Ok(ReplayOutput {
        messages: state.out,
        facts: state.facts,
    })
}

/// Explicit failure-context marker prefix for unrepresentable malformed
/// proposals. Adapters and tests can recognize projected failure notes.
pub const MALFORMED_PROPOSAL_NOTE_PREFIX: &str = "[tool proposal not replayed]";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{InputContent, OutputContent, ToolCall, ToolResultMessage, UserMessage};
    use crate::stream::StopReason;
    use serde_json::json;

    fn route(wire: WireApi) -> ReplayRoute {
        ReplayRoute {
            wire,
            provider: "p".into(),
            model: "m".into(),
            supports_images: true,
        }
    }

    fn user(text: &str) -> Message {
        Message::User(UserMessage {
            content: vec![InputContent::Text { text: text.into() }],
            timestamp_ms: 0,
        })
    }

    fn assistant(content: Vec<AssistantContent>, stop_reason: StopReason) -> AssistantMessage {
        AssistantMessage {
            content,
            api: crate::ApiKind::Anthropic,
            provider: "p".into(),
            model: "m".into(),
            response_model: None,
            response_id: None,
            usage: Default::default(),
            stop_reason,
            error_message: None,
            timestamp_ms: 0,
            wire: Some(WireApi::AnthropicMessages),
        }
    }

    fn call(id: &str, name: &str) -> AssistantContent {
        AssistantContent::ToolCall {
            tool_call: ToolCall {
                id: id.into(),
                name: name.into(),
                arguments: "{}".into(),
                arguments_error: None,
            },
        }
    }

    fn result(id: &str, name: &str) -> Message {
        Message::ToolResult(ToolResultMessage {
            tool_call_id: id.into(),
            tool_name: name.into(),
            content: vec![OutputContent::Text { text: "ok".into() }],
            details: None,
            is_error: false,
            truncated: false,
            timestamp_ms: 0,
        })
    }

    #[test]
    fn closed_batches_may_reuse_ids() {
        let messages = vec![
            user("q"),
            Message::Assistant(assistant(vec![call("fc_0", "a")], StopReason::ToolUse)),
            result("fc_0", "a"),
            Message::Assistant(assistant(vec![call("fc_0", "a")], StopReason::ToolUse)),
            result("fc_0", "a"),
        ];
        let out = project_replay(
            &messages,
            &route(WireApi::AnthropicMessages),
            ReplayProjection::NativeOrCollapse,
        )
        .unwrap();
        assert_eq!(out.messages.len(), 5);
        assert!(out.facts.losses.is_empty());
    }

    #[test]
    fn duplicate_call_in_batch_fails() {
        let messages = vec![
            user("q"),
            Message::Assistant(assistant(
                vec![call("x", "a"), call("x", "b")],
                StopReason::ToolUse,
            )),
        ];
        let error = project_replay(
            &messages,
            &route(WireApi::AnthropicMessages),
            ReplayProjection::NativeOrCollapse,
        )
        .unwrap_err();
        assert_eq!(
            error,
            ReplayError::DuplicateCallInBatch {
                tool_call_id: "x".into()
            }
        );
    }

    #[test]
    fn orphan_and_duplicate_results_fail() {
        let base = vec![
            user("q"),
            Message::Assistant(assistant(vec![call("x", "a")], StopReason::ToolUse)),
        ];
        let orphan = project_replay(
            &[base.clone(), vec![result("nope", "a")]].concat(),
            &route(WireApi::AnthropicMessages),
            ReplayProjection::NativeOrCollapse,
        )
        .unwrap_err();
        assert_eq!(
            orphan,
            ReplayError::OrphanResult {
                tool_call_id: "nope".into()
            }
        );

        let duplicate = project_replay(
            &[base, vec![result("x", "a"), result("x", "a")]].concat(),
            &route(WireApi::AnthropicMessages),
            ReplayProjection::NativeOrCollapse,
        )
        .unwrap_err();
        assert_eq!(
            duplicate,
            ReplayError::DuplicateResult {
                tool_call_id: "x".into()
            }
        );

        // A result after the batch closed (a user turn ends it) is an
        // orphan even for an ID the closed batch used.
        let late = project_replay(
            &[
                vec![
                    user("q"),
                    Message::Assistant(assistant(vec![call("x", "a")], StopReason::ToolUse)),
                ],
                vec![result("x", "a")],
                vec![user("next turn")],
                vec![result("x", "a")],
            ]
            .concat(),
            &route(WireApi::AnthropicMessages),
            ReplayProjection::NativeOrCollapse,
        )
        .unwrap_err();
        assert_eq!(
            late,
            ReplayError::OrphanResult {
                tool_call_id: "x".into()
            }
        );
    }

    #[test]
    fn result_name_mismatch_fails() {
        let messages = vec![
            user("q"),
            Message::Assistant(assistant(vec![call("x", "a")], StopReason::ToolUse)),
            result("x", "other"),
        ];
        let error = project_replay(
            &messages,
            &route(WireApi::AnthropicMessages),
            ReplayProjection::NativeOrCollapse,
        )
        .unwrap_err();
        assert_eq!(
            error,
            ReplayError::ResultNameMismatch {
                tool_call_id: "x".into(),
                call_name: "a".into(),
                result_name: "other".into(),
            }
        );
    }

    #[test]
    fn system_between_call_and_results_is_batch_transparent() {
        let system = Message::System(crate::transcript::SystemMessage {
            content: "mid".into(),
            ..Default::default()
        });
        let messages = vec![
            user("q"),
            Message::Assistant(assistant(vec![call("x", "a")], StopReason::ToolUse)),
            system.clone(),
            result("x", "a"),
        ];
        let out = project_replay(
            &messages,
            &route(WireApi::AnthropicMessages),
            ReplayProjection::NativeOrCollapse,
        )
        .unwrap();
        // The system entry moves after the pair instead of splitting it,
        // and no synthetic result is synthesized for the still-answered
        // call.
        let positions: Vec<&str> = out
            .messages
            .iter()
            .filter_map(|m| match m {
                Message::System(_) => Some("system"),
                Message::ToolResult(_) => Some("result"),
                _ => None,
            })
            .collect();
        assert_eq!(positions, ["result", "system"]);
        assert!(out.facts.losses.is_empty());
    }

    #[test]
    fn missing_result_closes_request_locally_with_unknown_effect() {
        let messages = vec![
            user("q"),
            Message::Assistant(assistant(vec![call("x", "a")], StopReason::ToolUse)),
            user("next turn"),
        ];
        let out = project_replay(
            &messages,
            &route(WireApi::AnthropicMessages),
            ReplayProjection::NativeOrCollapse,
        )
        .unwrap();
        let synthesized = out
            .messages
            .iter()
            .find_map(|m| match m {
                Message::ToolResult(t) => Some(t.clone()),
                _ => None,
            })
            .expect("synthetic result");
        assert!(synthesized.is_error);
        let text = match &synthesized.content[0] {
            OutputContent::Text { text } => text.clone(),
            _ => String::new(),
        };
        assert!(text.contains("Result unavailable"));
        assert!(text.contains("execution effect is unknown"));
        assert!(!text.to_lowercase().contains("not executed"));
        // Canonical input untouched: no mutation of the source slice is
        // possible (shared reference), and the projection records the loss.
        assert!(out
            .facts
            .losses
            .iter()
            .any(|l| matches!(l, ReplayLoss::SynthesizedMissingResult { tool_call_id } if tool_call_id == "x")));

        // Strict rejects the same history before any request is built.
        let error = project_replay(
            &messages,
            &route(WireApi::AnthropicMessages),
            ReplayProjection::Strict,
        )
        .unwrap_err();
        assert!(matches!(error, ReplayError::StrictLoss(_)));
    }

    #[test]
    fn cross_identity_drops_all_thinking_without_text_conversion() {
        let mut foreign = assistant(
            vec![
                AssistantContent::Thinking {
                    thinking: "readable reasoning".into(),
                    signature: Some("sig".into()),
                },
                AssistantContent::RedactedThinking {
                    data: "opaque-data".into(),
                },
                AssistantContent::Text {
                    text: "answer".into(),
                },
            ],
            StopReason::Stop,
        );
        foreign.provider = "other".into();
        let messages = vec![user("q"), Message::Assistant(foreign)];
        let out = project_replay(
            &messages,
            &route(WireApi::AnthropicMessages),
            ReplayProjection::NativeOrCollapse,
        )
        .unwrap();
        let body = serde_json::to_string(&out.messages).unwrap();
        assert!(!body.contains("readable reasoning"));
        assert!(!body.contains("sig"));
        assert!(!body.contains("opaque-data"));
        assert!(body.contains("answer"));
        assert!(
            out.facts
                .losses
                .iter()
                .any(|l| matches!(l, ReplayLoss::DroppedThinkingBlocks { count: 2 }))
        );

        // Same identity keeps signed and redacted blocks, including a
        // signed-empty one, and drops blank unsigned thinking.
        let same = assistant(
            vec![
                AssistantContent::Thinking {
                    thinking: String::new(),
                    signature: Some("sig".into()),
                },
                AssistantContent::RedactedThinking {
                    data: "opaque-data".into(),
                },
                AssistantContent::Thinking {
                    thinking: "   ".into(),
                    signature: None,
                },
                AssistantContent::Thinking {
                    thinking: "real reasoning".into(),
                    signature: None,
                },
            ],
            StopReason::Stop,
        );
        let out = project_replay(
            &[user("q"), Message::Assistant(same)],
            &route(WireApi::AnthropicMessages),
            ReplayProjection::NativeOrCollapse,
        )
        .unwrap();
        let body = serde_json::to_string(&out.messages).unwrap();
        assert!(body.contains("sig"));
        assert!(body.contains("opaque-data"));
        assert!(!body.contains("\"thinking\":\"   \""));
        assert!(body.contains("real reasoning"));
    }

    #[test]
    fn legacy_wireless_history_is_foreign() {
        let mut legacy = assistant(vec![], StopReason::Stop);
        legacy.wire = None;
        legacy.content = vec![AssistantContent::Thinking {
            thinking: "t".into(),
            signature: None,
        }];
        let out = project_replay(
            &[user("q"), Message::Assistant(legacy)],
            &route(WireApi::AnthropicMessages),
            ReplayProjection::NativeOrCollapse,
        )
        .unwrap();
        assert!(
            out.facts
                .losses
                .iter()
                .any(|l| matches!(l, ReplayLoss::DroppedThinkingBlocks { .. }))
        );
    }

    #[test]
    fn error_and_aborted_turns_omit_with_linked_results() {
        for stop_reason in [StopReason::Error, StopReason::Aborted] {
            let messages = vec![
                user("q"),
                Message::Assistant(assistant(vec![call("x", "a")], stop_reason)),
                result("x", "a"),
                Message::Assistant(assistant(
                    vec![AssistantContent::Text {
                        text: "recovered".into(),
                    }],
                    StopReason::Stop,
                )),
            ];
            let out = project_replay(
                &messages,
                &route(WireApi::AnthropicMessages),
                ReplayProjection::NativeOrCollapse,
            )
            .unwrap();
            let body = serde_json::to_string(&out.messages).unwrap();
            assert!(
                !body.contains("tool_use"),
                "failed turn body omitted for {stop_reason:?}"
            );
            assert!(body.contains("recovered"));
            assert!(
                out.messages
                    .iter()
                    .all(|m| !matches!(m, Message::ToolResult(_)))
            );
            assert!(out
                .facts
                .losses
                .iter()
                .any(|l| matches!(l, ReplayLoss::OmittedFailedAssistant { stop_reason: sr, .. } if *sr == stop_reason)));
        }
    }

    #[test]
    fn length_turns_keep_expressible_pairs() {
        let messages = vec![
            user("q"),
            Message::Assistant(assistant(vec![call("x", "a")], StopReason::Length)),
            result("x", "a"),
        ];
        let out = project_replay(
            &messages,
            &route(WireApi::AnthropicMessages),
            ReplayProjection::NativeOrCollapse,
        )
        .unwrap();
        assert!(
            out.messages.iter().any(
                |m| matches!(m, Message::ToolResult(t) if t.tool_call_id == "x" && !t.is_error)
            )
        );
        assert!(out.facts.losses.is_empty());
    }

    #[test]
    fn malformed_proposals_split_by_wire_argument_shape() {
        let malformed = || AssistantContent::ToolCall {
            tool_call: ToolCall {
                id: "x".into(),
                name: "a".into(),
                arguments: "{\"broken".into(),
                arguments_error: Some("not a complete JSON object".into()),
            },
        };
        // String-argument wires replay the raw text faithfully.
        let out = project_replay(
            &[
                user("q"),
                Message::Assistant(assistant(vec![malformed()], StopReason::ToolUse)),
                result("x", "a"),
            ],
            &route(WireApi::OpenAiCompletions),
            ReplayProjection::NativeOrCollapse,
        )
        .unwrap();
        let body = serde_json::to_string(&out.messages).unwrap();
        assert!(
            body.contains("broken"),
            "raw malformed text reaches the wire"
        );
        assert!(
            !body.contains("arguments\":\"{}\""),
            "no fabricated empty-object arguments"
        );

        // Object-argument wires replace the pair with an explicit failure
        // context; neither `{}` nor the raw proposal is fabricated onto
        // the wire, and the recorded result is consumed.
        let out = project_replay(
            &[
                user("q"),
                Message::Assistant(assistant(vec![malformed()], StopReason::ToolUse)),
                result("x", "a"),
            ],
            &route(WireApi::AnthropicMessages),
            ReplayProjection::NativeOrCollapse,
        )
        .unwrap();
        let body = serde_json::to_string(&out.messages).unwrap();
        assert!(body.contains(MALFORMED_PROPOSAL_NOTE_PREFIX));
        assert!(!body.contains("{\"broken"));
        assert!(
            !out.messages
                .iter()
                .any(|m| matches!(m, Message::ToolResult(_)))
        );
        assert!(out
            .facts
            .losses
            .iter()
            .any(|l| matches!(l, ReplayLoss::MalformedFailureContext { tool_call_id } if tool_call_id == "x")));

        // A missing result for an unrepresentable call synthesizes
        // nothing: there is no wire call left to answer.
        let out = project_replay(
            &[
                user("q"),
                Message::Assistant(assistant(vec![malformed()], StopReason::ToolUse)),
                user("next"),
            ],
            &route(WireApi::AnthropicMessages),
            ReplayProjection::NativeOrCollapse,
        )
        .unwrap();
        assert!(
            !out.messages
                .iter()
                .any(|m| matches!(m, Message::ToolResult(_)))
        );
    }

    #[test]
    fn wire_ids_are_legalized_bijectively_and_synchronously() {
        let messages = vec![
            user("q"),
            Message::Assistant(assistant(
                vec![call("call|with|pipes and spaces!", "a")],
                StopReason::ToolUse,
            )),
            result("call|with|pipes and spaces!", "a"),
        ];
        let out = project_replay(
            &messages,
            &route(WireApi::AnthropicMessages),
            ReplayProjection::NativeOrCollapse,
        )
        .unwrap();
        let mut ids = out.messages.iter().filter_map(|m| match m {
            Message::Assistant(a) => a.content.iter().find_map(|c| match c {
                AssistantContent::ToolCall { tool_call } => Some(tool_call.id.clone()),
                _ => None,
            }),
            Message::ToolResult(t) => Some(t.tool_call_id.clone()),
            _ => None,
        });
        let (call_id, result_id) = (
            ids.next().expect("legalized call id"),
            ids.next().expect("legalized result id"),
        );
        // Both sides of the pair share the legalized mapping and satisfy
        // the wire charset.
        assert_eq!(call_id, result_id);
        assert!(
            call_id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        );
        assert!(call_id.len() <= 64);
        // Distinct originals map to distinct wire IDs (injective).
        let second = legalize_wire_id("call|with|pipes and spaces!x", Some((64, true)));
        assert_ne!(call_id, second);
        // Unconstrained wires pass IDs through untouched.
        assert_eq!(legalize_wire_id("call|x", None), "call|x");
        assert!(
            out.facts
                .losses
                .iter()
                .any(|l| matches!(l, ReplayLoss::WireIdRemap { .. }))
        );
    }

    #[test]
    fn image_downgrades_use_explicit_placeholders() {
        let image_user = Message::User(UserMessage {
            content: vec![
                InputContent::Text { text: "see".into() },
                InputContent::Image {
                    source: crate::message::ImageSource::Url {
                        url: "https://example.test/i.png".into(),
                    },
                    media_type: crate::message::MediaType::Png,
                },
                InputContent::Image {
                    source: crate::message::ImageSource::Url {
                        url: "https://example.test/j.png".into(),
                    },
                    media_type: crate::message::MediaType::Png,
                },
            ],
            timestamp_ms: 0,
        });
        let mut no_images = route(WireApi::OpenAiResponses);
        no_images.supports_images = false;
        let out = project_replay(
            std::slice::from_ref(&image_user),
            &no_images,
            ReplayProjection::NativeOrCollapse,
        )
        .unwrap();
        let body = serde_json::to_string(&out.messages).unwrap();
        assert!(body.contains(NON_VISION_USER_IMAGE_PLACEHOLDER));
        // One placeholder per run of images, not one per block.
        assert_eq!(body.matches(NON_VISION_USER_IMAGE_PLACEHOLDER).count(), 1);
        assert!(!body.contains("example.test"));

        // Image-capable routes keep the real image (URLs are passed
        // through, never downloaded).
        let out = project_replay(
            &[image_user],
            &route(WireApi::OpenAiResponses),
            ReplayProjection::NativeOrCollapse,
        )
        .unwrap();
        let body = serde_json::to_string(&out.messages).unwrap();
        assert!(body.contains("example.test"));
    }

    #[test]
    fn projection_is_deterministic_for_retries() {
        let messages = vec![
            user("q"),
            Message::Assistant(assistant(vec![call("x", "a")], StopReason::ToolUse)),
            user("next"),
        ];
        let route = route(WireApi::AnthropicMessages);
        let first = project_replay(&messages, &route, ReplayProjection::NativeOrCollapse).unwrap();
        let second = project_replay(&messages, &route, ReplayProjection::NativeOrCollapse).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn assistant_message_wire_field_round_trips_and_defaults() {
        let mut a = assistant(vec![], StopReason::Stop);
        let json = serde_json::to_value(&a).unwrap();
        assert_eq!(json["wire"], json!("anthropic-messages"));
        a.wire = None;
        let legacy = json!({
            "content": [],
            "api": "Anthropic",
            "provider": "p",
            "model": "m",
            "response_model": null,
            "response_id": null,
            "usage": {
                "input_tokens": 0,
                "output_tokens": 0,
                "cache_read_tokens": 0,
                "cache_write_tokens": 0
            },
            "stop_reason": "stop",
            "error_message": null,
            "timestamp_ms": 0,
        });
        let parsed: AssistantMessage = serde_json::from_value(legacy).unwrap();
        assert_eq!(parsed.wire, None);
        let _ = a;
    }
}
