//! Owned synchronous-worker signing.
//!
//! Native AWS-LC signing runs on a dedicated blocking worker. Caller
//! cancellation is observed *between* operations only: a single synchronous
//! native call is never preempted (and this module makes no such claim). The
//! worker closure owns its `Arc<dyn SigningKey>` clone, so owned key
//! resources stay live until the worker finishes and joins, regardless of
//! caller drop or abort. A result that completes after cancellation is
//! suppressed (discarded) instead of published.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};

use rustls::SignatureScheme;
use tokio::sync::oneshot;

use super::CryptoSignError;

/// Cancellation-aware signing future. See the module docs for the ownership
/// and suppression contract.
///
/// The worker is spawned lazily on the first poll; dropping the future
/// before any poll spawns nothing.
pub(crate) struct WorkerSigning {
    key: Option<Arc<dyn rustls::sign::SigningKey>>,
    scheme: SignatureScheme,
    message: Option<Vec<u8>>,
    cancelled: Arc<AtomicBool>,
    inner: Option<oneshot::Receiver<Result<Vec<u8>, CryptoSignError>>>,
}

impl WorkerSigning {
    pub(crate) fn new(
        key: Arc<dyn rustls::sign::SigningKey>,
        scheme: SignatureScheme,
        message: Vec<u8>,
    ) -> Self {
        Self {
            key: Some(key),
            scheme,
            message: Some(message),
            cancelled: Arc::new(AtomicBool::new(false)),
            inner: None,
        }
    }
}

impl Future for WorkerSigning {
    type Output = Result<Vec<u8>, CryptoSignError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let inner = this.inner.get_or_insert_with(|| {
            // First poll: spawn the blocking worker. The key and message move
            // into the worker closure, retaining them until it joins.
            let (tx, rx) = oneshot::channel();
            let key = this.key.take();
            let message = this.message.take();
            if let (Some(key), Some(message)) = (key, message) {
                spawn_sign_worker(key, this.scheme, message, this.cancelled.clone(), tx);
            }
            rx
        });
        match Pin::new(inner).poll(cx) {
            Poll::Ready(Ok(result)) => Poll::Ready(result),
            // The worker only drops its sender after sending or returning;
            // a closed channel means the between-operations cancellation
            // path ran and there is no result to deliver.
            Poll::Ready(Err(_)) => Poll::Ready(Err(CryptoSignError::Suppressed)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for WorkerSigning {
    fn drop(&mut self) {
        // Between-operations cancellation. A worker that already entered its
        // native call finishes it (no hard preemption), then suppresses any
        // late result and releases the owned key on return.
        self.cancelled.store(true, Ordering::SeqCst);
    }
}

/// Spawns the blocking signing worker.
///
/// `pub(crate)` so the admission regressions can drive the between-operations
/// cancellation path deterministically; the production entry is
/// [`WorkerSigning`].
pub(crate) fn spawn_sign_worker(
    key: Arc<dyn rustls::sign::SigningKey>,
    scheme: SignatureScheme,
    message: Vec<u8>,
    cancelled: Arc<AtomicBool>,
    tx: oneshot::Sender<Result<Vec<u8>, CryptoSignError>>,
) -> tokio::task::JoinHandle<()> {
    tokio::task::spawn_blocking(move || {
        // Between-operations cancellation point: skip the native call.
        if cancelled.load(Ordering::SeqCst) {
            return;
        }
        let signature = super::sign_native(&key, scheme, &message);
        // Publish only when the caller still exists. A dropped future (or a
        // dropped receiver) turns this send into a no-op: the late result is
        // suppressed, never published.
        if !cancelled.load(Ordering::SeqCst) {
            let _ = tx.send(signature);
        }
        // The owned key Arc releases when this closure returns.
    })
}
