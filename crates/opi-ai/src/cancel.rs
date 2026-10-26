//! Cancellable waits over shared cancellation tokens (pinned sources:
//! `.repo/pi-1.3.0/packages/ai/src/utils/sleep.ts` and `utils/abort.ts`;
//! task 20.53.6).
//!
//! These helpers reuse [`tokio_util::sync::CancellationToken`] and the
//! caller's runtime: no executor, timer pool, or global state is created.
//! Dropping the losing side of each wait releases its timer and listeners on
//! cancel, completion, and drop — the Rust-native form of the pinned
//! "keep observing the abandoned promise" rule, since a dropped future is
//! cancelled and cannot reject later.

use std::future::Future;

use tokio_util::sync::CancellationToken;

/// Unit error raised when a cancellable wait loses to cancellation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the operation was cancelled")]
pub struct Cancelled;

/// Create an operation-local token for public APIs whose token is optional
/// (pinned `operationSignal(signal?)`).
///
/// A `Some(parent)` token is a one-way child: parent cancellation reaches
/// the child, but cancelling the child never propagates up to the parent.
#[must_use = "an unused token never cancels anything"]
pub fn operation_token(parent: Option<&CancellationToken>) -> CancellationToken {
    match parent {
        Some(parent) => parent.child_token(),
        None => CancellationToken::new(),
    }
}

/// Cancellable fixed wait (pinned `sleep(ms, signal)`).
///
/// Resolves after `duration_ms`, or fails with [`Cancelled`] as soon as
/// `cancel` fires — including when it was already cancelled, in which case
/// the timer is never armed. The timer is released on cancel, completion,
/// and drop.
pub async fn sleep_cancel(duration_ms: u64, cancel: &CancellationToken) -> Result<(), Cancelled> {
    if cancel.is_cancelled() {
        return Err(Cancelled);
    }
    tokio::select! {
        _ = tokio::time::sleep(std::time::Duration::from_millis(duration_ms)) => Ok(()),
        _ = cancel.cancelled() => Err(Cancelled),
    }
}

/// Stop waiting for `operation` when `cancel` fires (pinned
/// `raceWithAbortSignal(operation, signal)`).
///
/// Resolves with the operation's output, or fails with [`Cancelled`] when
/// the token fires first — immediately, without polling the operation, if
/// the token is already cancelled. On cancellation the operation future is
/// dropped (the Rust-native abandoned-promise handling); on completion the
/// cancellation listener is released.
pub async fn race_with_cancel<T, F: Future<Output = T>>(
    operation: F,
    cancel: &CancellationToken,
) -> Result<T, Cancelled> {
    if cancel.is_cancelled() {
        return Err(Cancelled);
    }
    tokio::select! {
        output = operation => Ok(output),
        _ = cancel.cancelled() => Err(Cancelled),
    }
}
