//! Streaming response events (S7.3).

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;
use serde::{Deserialize, Serialize};
use tokio_util::sync::{CancellationToken, WaitForCancellationFutureOwned};

use crate::provider::{ProviderError, ProviderErrorSummary};

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StopReason {
    #[serde(rename = "stop")]
    Stop,
    #[serde(rename = "length")]
    Length,
    #[serde(rename = "tool_use")]
    ToolUse,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "aborted")]
    Aborted,
    /// Deferred submission outcome: the turn is still pending behind a
    /// deferred handle (task 20.50, AIR-043). Terminal `Deferred` results
    /// are `Ok` outcomes exactly like `Stop`/`Length`/`ToolUse`; the
    /// message carries no content and the handle rides the assistant
    /// message's `deferred` field.
    #[serde(rename = "deferred")]
    Deferred,
}

/// A message-level constrained-sampling diagnostic (task 20.11): the
/// recorded strict-prefer downgrade (kind
/// `anthropic_strict_tools_fallback`, reason
/// `strict_tool_limits_exceeded`) or a grammar-target fallback loss
/// (kind `constrained_sampling_fallback`). `detail` carries the
/// redacted-safe provider message when one exists.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantDiagnostic {
    /// Diagnostic kind, e.g. `anthropic_strict_tools_fallback`.
    pub kind: String,
    /// Machine-readable reason, e.g. `strict_tool_limits_exceeded` or
    /// `grammar_target_unsupported`.
    pub reason: String,
    /// Redacted-safe detail line, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl StopReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stop => "stop",
            Self::Length => "length",
            Self::ToolUse => "tool_use",
            Self::Error => "error",
            Self::Aborted => "aborted",
            Self::Deferred => "deferred",
        }
    }
}

/// Provider-selected service tier for one response.
///
/// Known wire values retain their exact names. `Unknown` records a present
/// unrecognized response value; it never permits a guessed price multiplier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceTier {
    Auto,
    Default,
    Flex,
    Scale,
    Priority,
    Fast,
    /// A provider reported a tier that this SDK cannot price reliably.
    Unknown,
}

impl ServiceTier {
    /// Response-sensitive cost multiplier applied to every cost component.
    ///
    /// Mirrors the Responses annex: `flex` halves, `priority`/`fast` double,
    /// with the one model-specific exception (`gpt-5.5` prices priority and
    /// fast at 2.5). Absent/default tiers return 1. Per-adapter tier
    /// resolution stays with the owning adapters.
    pub fn cost_multiplier(self, model_id: &str) -> f64 {
        match self {
            Self::Auto | Self::Default | Self::Scale | Self::Unknown => 1.0,
            Self::Flex => 0.5,
            Self::Priority | Self::Fast if model_id == "gpt-5.5" => 2.5,
            Self::Priority | Self::Fast => 2.0,
        }
    }
}

/// Provider-reported token usage.
///
/// The optional `u64` child subsets preserve absent versus explicit zero:
/// `cache_write_1h_tokens` is contained by `cache_write_tokens`, and
/// `reasoning_tokens` is contained by `output_tokens`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    #[serde(default)]
    pub cache_read_tokens: u32,
    #[serde(default)]
    pub cache_write_tokens: u32,
    /// Subset of `cache_write_tokens` eligible for 1-hour TTL retention.
    #[serde(default)]
    pub cache_write_1h_tokens: Option<u64>,
    /// Subset of `output_tokens` consumed by reasoning/thinking.
    #[serde(default)]
    pub reasoning_tokens: Option<u64>,
    /// Independently reported grand total, when the provider sends one. It is
    /// kept separate from the parent buckets: it may legitimately differ from
    /// [`Usage::bucket_total_tokens`], and an explicit zero stays distinct
    /// from an absent value.
    #[serde(default)]
    pub total_tokens: Option<u64>,
    /// Service tier the provider selected for this response, when reported.
    /// This original response fact is never replaced by a requested fallback.
    #[serde(default)]
    pub service_tier: Option<ServiceTier>,
    /// Wire-qualified billing tier after the adapter resolves request fallback
    /// and response precedence. Absent on wires without service-tier pricing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_service_tier: Option<ServiceTier>,
    #[serde(default)]
    pub reported: bool,
}

impl Default for Usage {
    fn default() -> Self {
        Self::unknown()
    }
}

impl Usage {
    pub fn unknown() -> Self {
        Self {
            input_tokens: 0,
            output_tokens: 0,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            cache_write_1h_tokens: None,
            reasoning_tokens: None,
            total_tokens: None,
            service_tier: None,
            effective_service_tier: None,
            reported: false,
        }
    }

    pub fn reported(
        input_tokens: u32,
        output_tokens: u32,
        cache_read_tokens: u32,
        cache_write_tokens: u32,
        cache_write_1h_tokens: Option<u64>,
        reasoning_tokens: Option<u64>,
    ) -> Self {
        Self {
            input_tokens,
            output_tokens,
            cache_read_tokens,
            cache_write_tokens,
            cache_write_1h_tokens,
            reasoning_tokens,
            total_tokens: None,
            service_tier: None,
            effective_service_tier: None,
            reported: true,
        }
    }

    /// Attach an independently reported grand total.
    pub fn with_total_tokens(mut self, total_tokens: Option<u64>) -> Self {
        self.total_tokens = total_tokens;
        self
    }

    /// Attach the provider-selected service tier.
    pub fn with_service_tier(mut self, service_tier: ServiceTier) -> Self {
        self.service_tier = Some(service_tier);
        self
    }

    pub fn is_reported(&self) -> bool {
        self.reported
    }

    /// Total tokens counted across parent buckets only.
    ///
    /// `cache_write_1h_tokens` is a subset of `cache_write_tokens` and
    /// `reasoning_tokens` is a subset of `output_tokens`, so they are
    /// not added here. An independently reported grand total, when present,
    /// lives in the [`Usage::total_tokens`] field and is not folded in.
    pub fn bucket_total_tokens(&self) -> u64 {
        self.input_tokens as u64
            + self.output_tokens as u64
            + self.cache_read_tokens as u64
            + self.cache_write_tokens as u64
    }
}

/// Accumulated usage across multiple turns.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CumulativeUsage {
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
    cache_write_tokens: u64,
    cache_write_1h_tokens: Option<u64>,
    reasoning_tokens: Option<u64>,
    turns: u32,
    unknown_turns: u32,
}

/// Largest cumulative token count representable by the public [`Usage`] buckets.
const MAX_USAGE_BUCKET_TOKENS: u64 = u32::MAX as u64;

/// Convert an internal cumulative total to a public bucket without wrapping.
fn saturating_usage_bucket(tokens: u64) -> u32 {
    u32::try_from(tokens.min(MAX_USAGE_BUCKET_TOKENS))
        .expect("token total was clamped to the u32 range")
}

impl CumulativeUsage {
    /// Construct from pre-computed totals (e.g. when replaying a session).
    #[allow(clippy::too_many_arguments)]
    pub fn from_totals(
        input_tokens: u64,
        output_tokens: u64,
        cache_read_tokens: u64,
        cache_write_tokens: u64,
        cache_write_1h_tokens: Option<u64>,
        reasoning_tokens: Option<u64>,
        turns: u32,
        unknown_turns: u32,
    ) -> Self {
        Self {
            input_tokens,
            output_tokens,
            cache_read_tokens,
            cache_write_tokens,
            cache_write_1h_tokens,
            reasoning_tokens,
            turns,
            unknown_turns,
        }
    }

    pub fn total_input_tokens(&self) -> u64 {
        self.input_tokens
    }

    pub fn total_output_tokens(&self) -> u64 {
        self.output_tokens
    }

    pub fn total_cache_read_tokens(&self) -> u64 {
        self.cache_read_tokens
    }

    pub fn total_cache_write_tokens(&self) -> u64 {
        self.cache_write_tokens
    }

    pub fn total_cache_write_1h_tokens(&self) -> Option<u64> {
        self.cache_write_1h_tokens
    }

    pub fn total_reasoning_tokens(&self) -> Option<u64> {
        self.reasoning_tokens
    }

    pub fn turn_count(&self) -> u32 {
        self.turns
    }

    pub fn has_unknown_usage(&self) -> bool {
        self.unknown_turns > 0
    }

    pub fn accumulate(&mut self, turn: &Usage) {
        self.input_tokens = self
            .input_tokens
            .saturating_add(u64::from(turn.input_tokens));
        self.output_tokens = self
            .output_tokens
            .saturating_add(u64::from(turn.output_tokens));
        self.cache_read_tokens = self
            .cache_read_tokens
            .saturating_add(u64::from(turn.cache_read_tokens));
        self.cache_write_tokens = self
            .cache_write_tokens
            .saturating_add(u64::from(turn.cache_write_tokens));
        if let Some(tokens) = turn.cache_write_1h_tokens {
            self.cache_write_1h_tokens = Some(
                self.cache_write_1h_tokens
                    .unwrap_or(0)
                    .saturating_add(tokens),
            );
        }
        if let Some(tokens) = turn.reasoning_tokens {
            self.reasoning_tokens = Some(self.reasoning_tokens.unwrap_or(0).saturating_add(tokens));
        }
        self.turns = self.turns.saturating_add(1);
        if !turn.reported {
            self.unknown_turns = self.unknown_turns.saturating_add(1);
        }
    }

    pub fn as_usage(&self) -> Usage {
        let output_tokens = saturating_usage_bucket(self.output_tokens);
        let cache_write_tokens = saturating_usage_bucket(self.cache_write_tokens);
        Usage {
            input_tokens: saturating_usage_bucket(self.input_tokens),
            output_tokens,
            cache_read_tokens: saturating_usage_bucket(self.cache_read_tokens),
            cache_write_tokens,
            cache_write_1h_tokens: self
                .cache_write_1h_tokens
                .map(|tokens| tokens.min(u64::from(cache_write_tokens))),
            reasoning_tokens: self
                .reasoning_tokens
                .map(|tokens| tokens.min(u64::from(output_tokens))),
            total_tokens: None,
            service_tier: None,
            effective_service_tier: None,
            reported: self.unknown_turns == 0,
        }
    }
}

/// Per-million-token pricing for a model (USD).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Pricing {
    pub input_cost_per_mtok: f64,
    pub output_cost_per_mtok: f64,
    pub cache_read_cost_per_mtok: f64,
    pub cache_write_cost_per_mtok: f64,
}

/// Cost breakdown from a usage + pricing calculation.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CostBreakdown {
    pub input_cost: f64,
    pub output_cost: f64,
    pub cache_read_cost: f64,
    /// Weighted cache-write cost: the short-cache remainder uses
    /// `cache_write_cost_per_mtok`, while the 1-hour subset uses twice the
    /// input-token price.
    pub cache_write_cost: f64,
}

impl CostBreakdown {
    pub fn total_cost(&self) -> f64 {
        self.input_cost + self.output_cost + self.cache_read_cost + self.cache_write_cost
    }
}

/// Calculate cost from usage and pricing.
///
/// The short-cache remainder (`cache_write - cache_write_1h`) is charged at
/// `cache_write_cost_per_mtok`. The 1h subset is charged at twice the input
/// price. `reasoning_tokens` is a subset of `output_tokens` and is already
/// accounted for in `output_cost` — no separate line is produced.
pub fn calculate_cost(usage: &Usage, pricing: &Pricing) -> CostBreakdown {
    calculate_cost_totals(
        u64::from(usage.input_tokens),
        u64::from(usage.output_tokens),
        u64::from(usage.cache_read_tokens),
        u64::from(usage.cache_write_tokens),
        usage.cache_write_1h_tokens,
        pricing,
    )
}

/// Calculate cost from exact cumulative totals without projecting parent
/// buckets through the public `u32` [`Usage`] representation.
pub fn calculate_cumulative_cost(usage: &CumulativeUsage, pricing: &Pricing) -> CostBreakdown {
    calculate_cost_totals(
        usage.input_tokens,
        usage.output_tokens,
        usage.cache_read_tokens,
        usage.cache_write_tokens,
        usage.cache_write_1h_tokens,
        pricing,
    )
}

fn calculate_cost_totals(
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
    cache_write_tokens: u64,
    cache_write_1h_tokens: Option<u64>,
    pricing: &Pricing,
) -> CostBreakdown {
    let per_tok = |cost_per_mtok: f64| cost_per_mtok / 1_000_000.0;
    let cache_write_1h = cache_write_1h_tokens.unwrap_or(0).min(cache_write_tokens);
    let short_cache_write = cache_write_tokens.saturating_sub(cache_write_1h);
    CostBreakdown {
        input_cost: input_tokens as f64 * per_tok(pricing.input_cost_per_mtok),
        output_cost: output_tokens as f64 * per_tok(pricing.output_cost_per_mtok),
        cache_read_cost: cache_read_tokens as f64 * per_tok(pricing.cache_read_cost_per_mtok),
        cache_write_cost: short_cache_write as f64 * per_tok(pricing.cache_write_cost_per_mtok)
            + cache_write_1h as f64 * per_tok(pricing.input_cost_per_mtok) * 2.0,
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AssistantStreamEvent {
    #[serde(rename = "start")]
    Start {
        partial: crate::message::AssistantMessage,
    },
    #[serde(rename = "text_start")]
    TextStart {
        content_index: usize,
        partial: crate::message::AssistantMessage,
    },
    #[serde(rename = "text_delta")]
    TextDelta {
        content_index: usize,
        delta: String,
        partial: crate::message::AssistantMessage,
    },
    #[serde(rename = "text_end")]
    TextEnd {
        content_index: usize,
        content: String,
        partial: crate::message::AssistantMessage,
    },
    #[serde(rename = "thinking_start")]
    ThinkingStart {
        content_index: usize,
        partial: crate::message::AssistantMessage,
    },
    #[serde(rename = "thinking_delta")]
    ThinkingDelta {
        content_index: usize,
        delta: String,
        partial: crate::message::AssistantMessage,
    },
    #[serde(rename = "thinking_end")]
    ThinkingEnd {
        content_index: usize,
        content: String,
        partial: crate::message::AssistantMessage,
    },
    #[serde(rename = "tool_call_start")]
    ToolCallStart {
        content_index: usize,
        partial: crate::message::AssistantMessage,
    },
    #[serde(rename = "tool_call_delta")]
    ToolCallDelta {
        content_index: usize,
        delta: String,
        partial: crate::message::AssistantMessage,
    },
    #[serde(rename = "tool_call_end")]
    ToolCallEnd {
        content_index: usize,
        tool_call: crate::message::ToolCall,
        partial: crate::message::AssistantMessage,
    },
    #[serde(rename = "done")]
    Done {
        reason: StopReason,
        message: crate::message::AssistantMessage,
    },
    #[serde(rename = "error")]
    /// Legacy in-band error terminal (v0 stream format).
    ///
    /// Read-only: retained so historical saved events keep deserializing.
    /// Current producers never emit it; the public `ChatStream` seam
    /// converts it to `Err(CallFailure)` retaining the original payload.
    Error {
        reason: StopReason,
        message: crate::message::AssistantMessage,
    },
}

impl AssistantStreamEvent {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Done { .. } | Self::Error { .. })
    }

    /// The latest partial-message snapshot carried by this event, if any.
    ///
    /// Every streaming event clones the producer's partial message at event
    /// time, so consumers observe immutable event-time snapshots regardless of
    /// later stream progress. Terminal events carry their complete payload in
    /// `message`.
    fn latest_partial(&self) -> Option<&crate::message::AssistantMessage> {
        match self {
            Self::Start { partial }
            | Self::TextStart { partial, .. }
            | Self::TextDelta { partial, .. }
            | Self::TextEnd { partial, .. }
            | Self::ThinkingStart { partial, .. }
            | Self::ThinkingDelta { partial, .. }
            | Self::ThinkingEnd { partial, .. }
            | Self::ToolCallStart { partial, .. }
            | Self::ToolCallDelta { partial, .. }
            | Self::ToolCallEnd { partial, .. } => Some(partial),
            Self::Done { message, .. } | Self::Error { message, .. } => Some(message),
        }
    }
}

// ---------------------------------------------------------------------------
// Call failure (task 20.2, AIR-047 / P20-FINAL)
// ---------------------------------------------------------------------------

/// The single final failure of one logical provider call.
///
/// `message` always contains the complete canonical partial or terminal
/// [`AssistantMessage`](crate::message::AssistantMessage): generated content,
/// provenance, stop reason (`Error`, or `Aborted` when the cause is a
/// cancellation), diagnostics in `error_message` and known usage. A setup
/// failure (nothing delivered before the failure) has no generated content and
/// unknown usage. `cause` supplies the typed, redacted classification without
/// a second terminal. Current streams deliver exactly one `Err(CallFailure)`;
/// the in-band [`AssistantStreamEvent::Error`] terminal is a read-only legacy
/// format converted at the [`ChatStream`] seam.
#[derive(Debug, Clone, PartialEq)]
pub struct CallFailure {
    message: Box<crate::message::AssistantMessage>,
    cause: ProviderError,
}

impl CallFailure {
    /// The canonical partial/terminal assistant message for the failed call.
    pub fn message(&self) -> &crate::message::AssistantMessage {
        &self.message
    }

    /// The typed, redacted cause classification.
    pub fn cause(&self) -> &ProviderError {
        &self.cause
    }

    /// Split into the canonical message and the typed cause.
    pub fn into_parts(self) -> (crate::message::AssistantMessage, ProviderError) {
        (*self.message, self.cause)
    }

    /// Build a failure from the latest event-time partial (if any) and a typed
    /// cause. Normalizes the stop reason per the delivery contract: a
    /// cancellation cause maps to [`StopReason::Aborted`], every other cause to
    /// [`StopReason::Error`]. Diagnostics from the cause are preserved on the
    /// message when the producer did not supply one.
    pub(crate) fn from_partial(
        partial: Option<crate::message::AssistantMessage>,
        cause: ProviderError,
    ) -> Self {
        let (producer_partial, cause) = cause.into_stream_parts();
        let partial = producer_partial.or(partial);
        let mut message = partial.unwrap_or_else(|| setup_failure_message(&cause));
        message.stop_reason = if matches!(
            cause,
            ProviderError::Cancelled | ProviderError::LoginCancelled { .. }
        ) {
            StopReason::Aborted
        } else {
            StopReason::Error
        };
        if message.error_message.is_none() {
            message.error_message = Some(cause.to_string());
        }
        Self {
            message: Box::new(message),
            cause,
        }
    }

    /// Convert a legacy in-band error terminal, retaining the original facts
    /// of the delivered payload (content, provenance, diagnostics, usage).
    pub(crate) fn from_legacy_error_event(mut message: crate::message::AssistantMessage) -> Self {
        message.stop_reason = StopReason::Error;
        Self {
            message: Box::new(message),
            cause: ProviderError::StreamError(ProviderErrorSummary::redacted()),
        }
    }

    /// Classify a delivered `Done` terminal whose stop reason is itself a
    /// failure classification (`Error`/`Aborted`) instead of a completion.
    pub(crate) fn from_error_stop_reason(
        reason: StopReason,
        message: crate::message::AssistantMessage,
    ) -> Self {
        match reason {
            StopReason::Aborted => Self::from_partial(Some(message), ProviderError::Cancelled),
            _ => Self::from_partial(
                Some(message),
                ProviderError::StreamError(ProviderErrorSummary::attested_static(
                    "provider terminal carried an error stop reason",
                )),
            ),
        }
    }
}

/// The empty canonical message for a setup failure: no generated content,
/// unknown usage, and no provider classification to report (the failure
/// preceded any delivered event).
fn setup_failure_message(cause: &ProviderError) -> crate::message::AssistantMessage {
    crate::message::AssistantMessage {
        content: Vec::new(),
        api: crate::ApiKind::Anthropic,
        provider: String::new(),
        deferred: None,
        model: String::new(),
        response_model: None,
        wire: None,
        response_id: None,
        usage: crate::stream::Usage::default(),
        stop_reason: StopReason::Error,
        error_message: Some(cause.to_string()),
        timestamp_ms: crate::time::now_ms(),
        diagnostics: Vec::new(),
        duration_ms: None,
        provider_thinking_level: None,
    }
}

/// Retain an attested classification fact without replacing the acquisition
/// cause or admitting provider-controlled text into the canonical message.
pub(crate) fn setup_failure_with_detail(
    cause: ProviderError,
    detail: &'static str,
) -> ProviderError {
    let mut message = setup_failure_message(&cause);
    message.error_message = Some(detail.to_owned());
    cause.with_partial(message)
}

// ---------------------------------------------------------------------------
// ChatStream (task 20.2, AIR-003/AIR-047 / P20-FINAL)
// ---------------------------------------------------------------------------

/// The public owner of one logical provider call's event stream.
///
/// Iteration is `Stream<Item = Result<AssistantStreamEvent, CallFailure>>`:
/// events are delivered with their immutable event-time partials, and the
/// first delivered terminal is absorbed — the inner attempt is released, tail
/// events and repeated terminals are never delivered, and the stream stays at
/// permanent EOF. Failures surface as exactly one `Err(CallFailure)` carrying
/// the canonical partial and a typed, redacted cause; a stream that ends
/// without a terminal yields one typed protocol failure. Cancellation before
/// any delivery is a setup failure with an `Aborted` stop reason.
///
/// [`ChatStream::result`](ChatStream::result) consumes the handle and actively
/// drives the remaining bounded stream to the same
/// `Result<AssistantMessage, CallFailure>` outcome that complete dispatch
/// produces (`Stop`/`Length`/`ToolUse` are `Ok`; `Error`/`Aborted` are `Err`),
/// reusing a delivered terminal cache without resubmission or self-cancel.
/// Dropping the handle without polling performs no inner work; dropping it
/// mid-stream ends the attempt and releases the producer.
pub struct ChatStream {
    inner: Option<crate::provider::EventStream>,
    partial: Option<crate::message::AssistantMessage>,
    terminal: Option<Result<crate::message::AssistantMessage, CallFailure>>,
}

impl ChatStream {
    /// Wrap one provider event stream (for example
    /// [`PreparedProviderCall::start_attempt`](crate::models::PreparedProviderCall::start_attempt)).
    ///
    /// Construction is lazy: the inner stream is not polled until the first
    /// `poll_next`/`next().await` or [`ChatStream::result`].
    pub fn new(stream: crate::provider::EventStream) -> Self {
        Self {
            inner: Some(stream),
            partial: None,
            terminal: None,
        }
    }

    /// Consume the handle and actively drive the remaining bounded stream to
    /// the single final outcome.
    ///
    /// Equivalent to full iteration for the same event sequence: a terminal
    /// already delivered to an iterator is reused from cache, partial
    /// iteration does not change the outcome, and a stream that ends without
    /// a terminal produces the one typed protocol failure (AIR-003).
    pub async fn result(self) -> Result<crate::message::AssistantMessage, CallFailure> {
        let mut stream = self;
        while stream.terminal.is_none() {
            use futures_util::StreamExt as _;
            if stream.next().await.is_none() {
                // poll_next synthesizes the premature-EOF protocol failure, so
                // exhaustion always leaves a terminal set.
                break;
            }
        }
        stream
            .terminal
            .expect("chat stream exhaustion always records a terminal")
    }
}

impl Stream for ChatStream {
    type Item = Result<AssistantStreamEvent, CallFailure>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if this.terminal.is_some() {
            // The first terminal was delivered; permanent EOF.
            return Poll::Ready(None);
        }
        let Some(inner) = this.inner.as_mut() else {
            // Absorbed without a recorded terminal: the protocol failure was
            // delivered and EOF is permanent.
            return Poll::Ready(None);
        };
        let item = inner.as_mut().poll_next(cx);
        match item {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => {
                // Premature EOF: one typed protocol failure, then permanent
                // EOF. The failure preserves the latest event-time partial.
                this.inner = None;
                let partial = this.partial.clone();
                let failure = CallFailure::from_partial(
                    partial,
                    ProviderError::StreamError(ProviderErrorSummary::attested_static(
                        "stream ended without a terminal event",
                    )),
                );
                this.terminal = Some(Err(failure.clone()));
                Poll::Ready(Some(Err(failure)))
            }
            Poll::Ready(Some(Ok(event))) => {
                if let Some(partial) = event.latest_partial() {
                    this.partial = Some(partial.clone());
                }
                match &event {
                    AssistantStreamEvent::Done { reason, message } => {
                        let outcome = match reason {
                            StopReason::Stop
                            | StopReason::Length
                            | StopReason::ToolUse
                            | StopReason::Deferred => Ok(message.clone()),
                            StopReason::Error | StopReason::Aborted => Err(
                                CallFailure::from_error_stop_reason(*reason, message.clone()),
                            ),
                        };
                        // Absorb: release the inner attempt; the terminal event
                        // itself is delivered exactly once.
                        this.inner = None;
                        this.terminal = Some(outcome.clone());
                        Poll::Ready(Some(outcome.map(|_| event)))
                    }
                    AssistantStreamEvent::Error { message, .. } => {
                        // Legacy in-band error terminal (historical/mock
                        // producers): convert, retain original facts, absorb.
                        let failure = CallFailure::from_legacy_error_event(message.clone());
                        this.inner = None;
                        this.terminal = Some(Err(failure.clone()));
                        Poll::Ready(Some(Err(failure)))
                    }
                    _ => Poll::Ready(Some(Ok(event))),
                }
            }
            Poll::Ready(Some(Err(cause))) => {
                let partial = this.partial.clone();
                let failure = CallFailure::from_partial(partial, cause);
                this.inner = None;
                this.terminal = Some(Err(failure.clone()));
                Poll::Ready(Some(Err(failure)))
            }
        }
    }
}

/// Call-local canonical state, including facts that emitted no public event.
/// Cancellation reads this snapshot before discarding buffered delivery.
#[derive(Clone, Default)]
pub(crate) struct ProducerSnapshot(std::sync::Arc<std::sync::Mutex<ProducerState>>);

#[derive(Default)]
struct ProducerState {
    message: Option<crate::message::AssistantMessage>,
    response_received: bool,
    terminal_delivered: bool,
}

impl ProducerSnapshot {
    pub(crate) fn terminal_delivered(&self) -> bool {
        self.0
            .lock()
            .expect("producer snapshot lock")
            .terminal_delivered
    }
    pub(crate) fn record(&self, message: &crate::message::AssistantMessage) {
        self.0.lock().expect("producer snapshot lock").message = Some(message.clone());
    }

    /// Freeze successful transport acquisition before reading response data.
    pub(crate) fn received_response(&self) {
        self.0
            .lock()
            .expect("producer snapshot lock")
            .response_received = true;
    }

    pub(crate) fn failure(&self, cause: ProviderError) -> ProviderError {
        let state = self.0.lock().expect("producer snapshot lock");
        let cause = match &state.message {
            Some(message) => cause.with_partial(message.clone()),
            None => cause,
        };
        if state.response_received {
            cause.after_response()
        } else {
            cause
        }
    }
}

/// Gives cancellation priority over buffered events while retaining the
/// producer's canonical partial, then closes delivery permanently.
pub(crate) struct CancelAwareReceiverStream {
    rx: Option<tokio::sync::mpsc::Receiver<Result<AssistantStreamEvent, ProviderError>>>,
    cancelled: Pin<Box<WaitForCancellationFutureOwned>>,
    yielded_cancelled: bool,
    snapshot: ProducerSnapshot,
}

impl CancelAwareReceiverStream {
    #[cfg(test)]
    pub(crate) fn new(
        rx: tokio::sync::mpsc::Receiver<Result<AssistantStreamEvent, ProviderError>>,
        cancel: CancellationToken,
    ) -> Self {
        Self::with_snapshot(rx, cancel, ProducerSnapshot::default())
    }

    pub(crate) fn with_snapshot(
        rx: tokio::sync::mpsc::Receiver<Result<AssistantStreamEvent, ProviderError>>,
        cancel: CancellationToken,
        snapshot: ProducerSnapshot,
    ) -> Self {
        Self {
            rx: Some(rx),
            cancelled: Box::pin(cancel.cancelled_owned()),
            yielded_cancelled: false,
            snapshot,
        }
    }
}

impl Stream for CancelAwareReceiverStream {
    type Item = Result<AssistantStreamEvent, ProviderError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if !this.yielded_cancelled && this.cancelled.as_mut().poll(cx).is_ready() {
            this.rx.take();
            this.yielded_cancelled = true;
            return Poll::Ready(Some(Err(this.snapshot.failure(ProviderError::Cancelled))));
        }

        match this.rx.as_mut() {
            Some(rx) => {
                let item = rx.poll_recv(cx);
                if matches!(&item, Poll::Ready(Some(Ok(event))) if event.is_terminal()) {
                    this.snapshot
                        .0
                        .lock()
                        .expect("producer snapshot lock")
                        .terminal_delivered = true;
                }
                item
            }
            None => Poll::Ready(None),
        }
    }
}

pub(crate) async fn send_or_cancel(
    cancel: &CancellationToken,
    tx: &tokio::sync::mpsc::Sender<Result<AssistantStreamEvent, ProviderError>>,
    item: Result<AssistantStreamEvent, ProviderError>,
) -> Result<bool, ProviderError> {
    tokio::select! {
        biased;
        _ = cancel.cancelled() => Err(ProviderError::Cancelled),
        sent = tx.send(item) => Ok(sent.is_ok()),
    }
}

#[cfg(test)]
mod cancellation_tests {
    use std::future::Future as _;

    use futures_util::StreamExt;
    use tokio_util::sync::CancellationToken;

    use super::{CancelAwareReceiverStream, send_or_cancel};
    use crate::provider::{ProviderError, ProviderErrorSummary};

    #[tokio::test]
    async fn cancellation_retains_silent_producer_state_at_the_barrier() {
        let cancel = CancellationToken::new();
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let snapshot = super::ProducerSnapshot::default();
        let mut mapper = crate::anthropic::AnthropicMapper::new();
        mapper.set_snapshot(snapshot.clone());
        // Silent facts belong to the producer even when no public snapshot
        // was queued. A task barrier ensures they exist before cancellation.
        let (published_tx, published_rx) = tokio::sync::oneshot::channel();
        let producer_cancel = cancel.clone();
        let producer = tokio::spawn(async move {
            let _keep_sender = tx;
            assert!(
                mapper
                    .process(crate::anthropic::AnthropicEvent::MessageDelta {
                        stop_reason: None,
                        usage: super::Usage::reported(10, 7, 0, 0, None, None),
                    })
                    .is_empty()
            );
            assert!(
                mapper
                    .process(crate::anthropic::AnthropicEvent::ContentBlockStart {
                        index: 0,
                        block_type: crate::anthropic::ContentBlockType::RedactedThinking {
                            data: "opaque-artificial".into(),
                        },
                    })
                    .is_empty()
            );
            published_tx.send(()).unwrap();
            producer_cancel.cancelled().await;
        });
        let stream = super::ChatStream::new(Box::pin(CancelAwareReceiverStream::with_snapshot(
            rx,
            cancel.clone(),
            snapshot,
        )));
        published_rx.await.unwrap();
        cancel.cancel();
        let failure = stream.result().await.expect_err("one cancellation");
        assert_eq!(failure.message().usage.output_tokens, 7);
        assert_eq!(failure.message().content.len(), 1);
        assert_eq!(failure.message().stop_reason, super::StopReason::Aborted);
        assert_eq!(failure.cause(), &ProviderError::Cancelled);
        producer.await.unwrap();
    }

    #[tokio::test]
    async fn backpressured_producer_cancels_and_buffered_events_are_discarded() {
        let cancel = CancellationToken::new();
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        tx.send(Err(ProviderError::StreamError(
            ProviderErrorSummary::redacted(),
        )))
        .await
        .expect("prefill event channel");
        let mut stream = CancelAwareReceiverStream::new(rx, cancel.clone());
        let producer_cancel = cancel.clone();
        let producer_tx = tx.clone();
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let producer = tokio::spawn(async move {
            let mut started_tx = Some(started_tx);
            let mut blocked_send = Box::pin(send_or_cancel(
                &producer_cancel,
                &producer_tx,
                Err(ProviderError::StreamError(ProviderErrorSummary::redacted())),
            ));
            std::future::poll_fn(move |cx| {
                let result = blocked_send.as_mut().poll(cx);
                if result.is_pending()
                    && let Some(started_tx) = started_tx.take()
                {
                    started_tx.send(()).expect("signal producer backpressure");
                }
                result
            })
            .await
        });

        started_rx.await.expect("producer reached backpressure");
        cancel.cancel();

        let producer_result = tokio::time::timeout(std::time::Duration::from_secs(1), producer)
            .await
            .expect("backpressured producer must terminate promptly")
            .expect("producer task did not panic");
        assert!(matches!(producer_result, Err(ProviderError::Cancelled)));
        assert!(matches!(
            stream.next().await,
            Some(Err(ProviderError::Cancelled))
        ));
        assert!(
            stream.next().await.is_none(),
            "buffered pre-cancel events must be discarded"
        );
    }

    #[tokio::test]
    async fn backpressured_producer_exits_at_receiver_drop_barrier() {
        let cancel = CancellationToken::new();
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        tx.send(Err(ProviderError::Cancelled)).await.unwrap();
        let (blocked_tx, blocked_rx) = tokio::sync::oneshot::channel();
        let producer = tokio::spawn(async move {
            let mut blocked_tx = Some(blocked_tx);
            let mut send = Box::pin(send_or_cancel(&cancel, &tx, Err(ProviderError::Cancelled)));
            std::future::poll_fn(move |cx| {
                let outcome = send.as_mut().poll(cx);
                if outcome.is_pending()
                    && let Some(signal) = blocked_tx.take()
                {
                    signal.send(()).unwrap();
                }
                outcome
            })
            .await
        });
        blocked_rx.await.unwrap();
        drop(rx);
        assert_eq!(
            tokio::time::timeout(std::time::Duration::from_secs(1), producer)
                .await
                .unwrap()
                .unwrap(),
            Ok(false)
        );
    }

    #[tokio::test]
    async fn idle_live_sender_does_not_hide_cancellation_from_pending_consumer() {
        let cancel = CancellationToken::new();
        let (_idle_tx, rx) = tokio::sync::mpsc::channel(1);
        let mut stream = CancelAwareReceiverStream::new(rx, cancel.clone());
        let (pending_tx, pending_rx) = tokio::sync::oneshot::channel();
        let consumer = tokio::spawn(async move {
            let mut pending_tx = Some(pending_tx);
            let first = {
                let mut next = Box::pin(stream.next());
                std::future::poll_fn(move |cx| {
                    let result = next.as_mut().poll(cx);
                    if result.is_pending()
                        && let Some(pending_tx) = pending_tx.take()
                    {
                        pending_tx.send(()).expect("signal pending consumer");
                    }
                    result
                })
                .await
            };
            (first, stream.next().await)
        });

        pending_rx.await.expect("consumer reached Pending");
        cancel.cancel();

        let (first, second) = tokio::time::timeout(std::time::Duration::from_secs(1), consumer)
            .await
            .expect("pending consumer must wake promptly on cancellation")
            .expect("consumer task did not panic");
        assert!(matches!(first, Some(Err(ProviderError::Cancelled))));
        assert!(second.is_none());
    }
}
