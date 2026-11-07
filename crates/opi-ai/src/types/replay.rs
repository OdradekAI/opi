//! Replay policy, route, loss and result values.

use super::message::Message;
use super::model::WireApi;
use super::usage::StopReason;

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
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
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
    /// A missing result closed with a request-local error result or ordinary
    /// failure context (result unavailable, execution effect unknown).
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
                "closed the missing result of tool call {tool_call_id:?} request-locally with unknown execution effect"
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
        "tool result {tool_call_id:?} cannot be omitted without a trusted non-execution error fact"
    )]
    MissingNonExecutionFact { tool_call_id: String },
    #[error(
        "strict replay projection rejected lossy transforms before any request was built: {}",
        .0.iter().map(|l| l.describe()).collect::<Vec<_>>().join("; ")
    )]
    StrictLoss(Vec<ReplayLoss>),
}

/// Recorded projection facts for one call to [`crate::replay::project_replay`].
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayFacts {
    pub losses: Vec<ReplayLoss>,
}

impl ReplayFacts {
    pub(crate) fn record(&mut self, loss: ReplayLoss) {
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
