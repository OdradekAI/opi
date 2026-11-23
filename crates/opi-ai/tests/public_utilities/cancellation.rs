//! Cancellable waits, future release and parent-child tokens.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use opi_ai::cancel::{Cancelled, operation_token, race_with_cancel, sleep_cancel};
use tokio_util::sync::CancellationToken;

#[tokio::test(start_paused = true)]
async fn sleep_cancel_resolves_after_the_requested_duration() {
    let token = CancellationToken::new();
    let start = tokio::time::Instant::now();
    sleep_cancel(120, &token).await.unwrap();
    assert!(start.elapsed() >= Duration::from_millis(120));
}

#[tokio::test(start_paused = true)]
async fn sleep_cancel_fails_immediately_when_pre_cancelled() {
    let token = CancellationToken::new();
    token.cancel();
    let start = tokio::time::Instant::now();
    assert_eq!(sleep_cancel(60_000, &token).await, Err(Cancelled));
    assert!(start.elapsed() < Duration::from_millis(1));
}

#[tokio::test(start_paused = true)]
async fn sleep_cancel_fails_when_cancelled_mid_wait() {
    let token = CancellationToken::new();
    let cancel_at = token.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(40)).await;
        cancel_at.cancel();
    });
    let start = tokio::time::Instant::now();
    assert_eq!(sleep_cancel(60_000, &token).await, Err(Cancelled));
    assert!(start.elapsed() < Duration::from_millis(60_000));
}

#[tokio::test(start_paused = true)]
async fn race_with_cancel_returns_output_when_ready_before_cancel() {
    let token = CancellationToken::new();
    let cancel_at = token.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        cancel_at.cancel();
    });
    assert_eq!(race_with_cancel(async { 42u32 }, &token).await, Ok(42));
}

#[tokio::test(start_paused = true)]
async fn race_with_cancel_releases_the_operation_future_on_cancel() {
    struct Sentinel(Arc<AtomicBool>);
    impl Drop for Sentinel {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    let token = CancellationToken::new();
    let dropped = Arc::new(AtomicBool::new(false));
    let sentinel = Sentinel(dropped.clone());
    let operation = async move {
        let _keep = sentinel;
        tokio::time::sleep(Duration::from_secs(1_000)).await;
        0u32
    };
    let cancel_at = token.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        cancel_at.cancel();
    });
    assert_eq!(race_with_cancel(operation, &token).await, Err(Cancelled));
    assert!(
        dropped.load(Ordering::SeqCst),
        "the abandoned operation future must be dropped on cancel"
    );
}

#[tokio::test]
async fn operation_token_links_parent_to_child_one_way() {
    let parent = CancellationToken::new();
    let child = operation_token(Some(&parent));
    child.cancel();
    assert!(child.is_cancelled());
    assert!(
        !parent.is_cancelled(),
        "child cancellation must not propagate to the parent"
    );

    let parent_two = CancellationToken::new();
    let child_two = operation_token(Some(&parent_two));
    parent_two.cancel();
    assert!(
        child_two.is_cancelled(),
        "parent cancellation must reach the child"
    );

    let standalone = operation_token(None);
    standalone.cancel();
    // parent_two was cancelled on purpose above; only parent must stay clean.
    assert!(!parent.is_cancelled());
}
