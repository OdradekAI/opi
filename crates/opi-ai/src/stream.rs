//! Streaming response events (S7.3).

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;
use serde::{Deserialize, Serialize};
use tokio_util::sync::{CancellationToken, WaitForCancellationFutureOwned};

use crate::provider::ProviderError;

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
}

impl StopReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stop => "stop",
            Self::Length => "length",
            Self::ToolUse => "tool_use",
            Self::Error => "error",
            Self::Aborted => "aborted",
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
            reported: true,
        }
    }

    pub fn is_reported(&self) -> bool {
        self.reported
    }

    /// Total tokens counted across parent buckets only.
    ///
    /// `cache_write_1h_tokens` is a subset of `cache_write_tokens` and
    /// `reasoning_tokens` is a subset of `output_tokens`, so they are
    /// not added here.
    pub fn total_tokens(&self) -> u64 {
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
    Error {
        reason: StopReason,
        message: crate::message::AssistantMessage,
    },
}

impl AssistantStreamEvent {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Done { .. } | Self::Error { .. })
    }
}

/// Crate-private provider stream wrapper that gives request cancellation
/// priority over already-buffered transport events.
pub(crate) struct CancelAwareReceiverStream {
    rx: Option<tokio::sync::mpsc::Receiver<Result<AssistantStreamEvent, ProviderError>>>,
    cancelled: Pin<Box<WaitForCancellationFutureOwned>>,
    yielded_cancelled: bool,
}

impl CancelAwareReceiverStream {
    pub(crate) fn new(
        rx: tokio::sync::mpsc::Receiver<Result<AssistantStreamEvent, ProviderError>>,
        cancel: CancellationToken,
    ) -> Self {
        Self {
            rx: Some(rx),
            cancelled: Box::pin(cancel.cancelled_owned()),
            yielded_cancelled: false,
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
            return Poll::Ready(Some(Err(ProviderError::Cancelled)));
        }

        match this.rx.as_mut() {
            Some(rx) => rx.poll_recv(cx),
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
