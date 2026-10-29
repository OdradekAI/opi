//! Classifier contracts: public context, result and model-record types.
//!
//! The classifier operation is a non-streaming sibling of the chat seam:
//! callers pass a [`ClassifierContext`] (a JSON state object, typed questions
//! and optional images), a complete [`ClassifierModelRecord`] that names the
//! owning classifier API, and receive a [`ClassifierResult`] that never
//! rejects for provider failures — failures ride `stop_reason =
//! Error`/`Aborted` with a typed [`ProviderError`] and a display message,
//! exactly like pi's `ClassifierResult`.
//!
//! Validation stays source-required only: answer parsing checks record
//! shape, wire type agreement, string choices and finite numbers. No
//! universal probability-range, sum, argmax or label-membership rule exists
//! here; per-protocol adapters own their extra rules.

use tokio_util::sync::CancellationToken;

use crate::callbacks::RequestCallbacks;
use crate::model_info::ModelPricing;
use crate::models::ApiId;
use crate::provider::ProviderError;
use crate::stream::{CostBreakdown, Usage};
use crate::telemetry::TelemetryContext;

/// One input modality a classifier model accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassifierInputModality {
    /// Text state (always supported).
    Text,
    /// Image input judged together with the state.
    Image,
}

/// One base64-encoded image judged together with the state.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassifierImage {
    /// MIME type (for example `image/png`).
    pub mime_type: String,
    /// Base64-encoded image bytes, exactly as supplied.
    pub data: String,
}

/// One typed question about the state.
#[derive(Debug, Clone, PartialEq)]
pub enum ClassifierQuestion {
    /// Pick one of the named options.
    Choice {
        /// What the model is asked to classify.
        instructions: String,
        /// Option key to human-readable description, in wire order.
        criteria: std::collections::BTreeMap<String, String>,
    },
    /// Score the state against ordered levels.
    Score {
        /// What the model is asked to score.
        instructions: String,
        /// Ordered level labels, lowest first.
        levels: Vec<String>,
    },
    /// A yes/no question.
    Bool {
        /// What the model is asked to judge.
        instructions: String,
        /// Meaning of a true answer.
        true_criteria: String,
        /// Meaning of a false answer.
        false_criteria: String,
    },
}

impl ClassifierQuestion {
    /// The question's instructions.
    pub fn instructions(&self) -> &str {
        match self {
            ClassifierQuestion::Choice { instructions, .. }
            | ClassifierQuestion::Score { instructions, .. }
            | ClassifierQuestion::Bool { instructions, .. } => instructions,
        }
    }
}

/// The complete classifier input.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ClassifierContext {
    /// The state object being judged (any JSON object).
    pub state: serde_json::Value,
    /// Typed questions keyed by caller id, in wire order.
    pub questions: std::collections::BTreeMap<String, ClassifierQuestion>,
    /// Images judged together with `state`. Adapters that cannot accept
    /// images reject them typed before any request.
    pub images: Option<Vec<ClassifierImage>>,
}

/// One typed answer to a question.
#[derive(Debug, Clone, PartialEq)]
pub enum ClassifierAnswer {
    /// A chosen option with its probability distribution and confidence.
    Choice {
        /// The chosen option key.
        choice: String,
        /// Per-option probabilities, finite numbers exactly as reported.
        probabilities: std::collections::BTreeMap<String, f64>,
        /// Reported confidence.
        confidence: f64,
    },
    /// A score against the question's levels.
    Score {
        /// Reported score.
        score: f64,
        /// Reported confidence.
        confidence: f64,
    },
    /// A yes/no answer with its true-probability.
    Bool {
        /// Probability of a true answer.
        probability: f64,
    },
}

/// Terminal state of one classifier call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassifierStopReason {
    /// The call completed with answers for every question.
    Stop,
    /// The provider call failed; `error`/`error_message` carry the cause.
    Error,
    /// The caller cancelled the call.
    Aborted,
}

/// The complete classifier-model record one classify call binds to.
///
/// The record carries the exact classifier API identity (pi string, e.g.
/// `typesafe-system-one`), the owning provider, the endpoint base URL, the
/// accepted input modalities, the context window and the catalog pricing
/// (tiers included: the classifier path prices usage like chat, unlike the
/// image path's base-rate exception). It is the operation-qualified
/// counterpart of the chat seam's complete record: chat models never satisfy
/// classifier selection, and classifier records never enter the chat
/// catalog.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassifierModelRecord {
    /// Owning provider id (for example `typesafe`).
    pub provider_id: String,
    /// Bare model id.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Exact classifier API identity (pi string).
    pub api: ApiId,
    /// Endpoint base URL (trailing slashes are stripped when joining).
    pub base_url: String,
    /// Accepted input modalities.
    pub input: Vec<ClassifierInputModality>,
    /// Context window in tokens.
    pub context_window: u64,
    /// Catalog pricing with tiers; cost applies chat-style to reported usage.
    pub cost: ModelPricing,
}

/// Per-call classifier values: timeout, cancellation, headers, callbacks and
/// telemetry.
///
/// Mirrors the per-request fields of the chat seam's `Request` that a
/// classifier call consumes. `timeout` covers the whole response body read
/// and parse and is **fresh for every retry attempt**; there is no default
/// timeout (the pinned source leaves `timeoutMs` undefined by default).
#[derive(Clone, Default)]
pub struct ClassifierCall {
    /// Per-attempt body-inclusive timeout. `None` applies no timeout.
    pub timeout: Option<std::time::Duration>,
    /// Cancellation token; cancelling maps the result to `Aborted`.
    pub cancel: CancellationToken,
    /// Extra HTTP headers appended to the outbound request.
    pub extra_headers: Vec<(String, String)>,
    /// Functional callbacks (payload/response observers). Adapter-unsupported
    /// families are rejected typed before dispatch.
    pub callbacks: RequestCallbacks,
    /// Explicit frozen telemetry carrier (task 20.54). Passes through to the
    /// adapter's request path; never auto-captures.
    pub telemetry_context: Option<TelemetryContext>,
}

impl std::fmt::Debug for ClassifierCall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClassifierCall")
            .field("timeout", &self.timeout)
            .field("extra_headers", &self.extra_headers.len())
            .field("callbacks", &self.callbacks)
            .finish_non_exhaustive()
    }
}

/// The classifier result: pi `ClassifierResult` semantics.
///
/// Provider failures never surface as `Err`: they keep the record identity,
/// empty answers, `stop_reason = Error`/`Aborted`, a sanitized display
/// message and the typed [`ProviderError`] cause. A request that was billed
/// keeps its reported usage: parsing failures after a successful response
/// retain the usage the transport already recorded.
#[derive(Debug, Clone)]
pub struct ClassifierResult {
    /// Classifier API identity the record named.
    pub api: String,
    /// Owning provider id.
    pub provider: String,
    /// Model id.
    pub model: String,
    /// Answers keyed by question id. Empty on failure.
    pub answers: std::collections::BTreeMap<String, ClassifierAnswer>,
    /// Reported token usage, when the service reported token counts (also
    /// retained when later answer parsing failed: the request was billed).
    pub usage: Option<Usage>,
    /// Chat-style tiered cost computed from the record's catalog pricing,
    /// when usage exists.
    pub cost: Option<CostBreakdown>,
    /// Terminal state.
    pub stop_reason: ClassifierStopReason,
    /// Sanitized display message for `Error`/`Aborted`.
    pub error_message: Option<String>,
    /// Typed cause for `Error`/`Aborted`.
    pub error: Option<ProviderError>,
}

impl ClassifierResult {
    /// A pre-dispatch or provider failure result for the given record
    /// identity: empty answers, typed cause and display message.
    pub fn failure(
        record: &ClassifierModelRecord,
        stop_reason: ClassifierStopReason,
        error: ProviderError,
        message: String,
    ) -> Self {
        Self {
            api: record.api.as_str().to_owned(),
            provider: record.provider_id.clone(),
            model: record.id.clone(),
            answers: std::collections::BTreeMap::new(),
            usage: None,
            cost: None,
            stop_reason,
            error_message: Some(message),
            error: Some(error),
        }
    }

    /// An in-progress success result for the given record identity: answers
    /// and usage/cost are filled by the adapter as it parses.
    pub fn in_progress(record: &ClassifierModelRecord) -> Self {
        Self {
            api: record.api.as_str().to_owned(),
            provider: record.provider_id.clone(),
            model: record.id.clone(),
            answers: std::collections::BTreeMap::new(),
            usage: None,
            cost: None,
            stop_reason: ClassifierStopReason::Stop,
            error_message: None,
            error: None,
        }
    }
}
