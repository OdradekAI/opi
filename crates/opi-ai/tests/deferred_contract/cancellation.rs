//! Local abort, explicit remote cancellation and cancellation during auth.

use opi_ai::api::contract::DeferredFetchCall;
use opi_ai::auth::ResolvedAuth;
use opi_ai::retry::{WholeCallRetryClassification, classify_whole_call_retry};
use opi_ai::stream::StopReason;
use opi_ai::test_support::{self, ScriptedDeferredBackend};
use opi_ai::{AuthProvenanceSource, Models};
use tokio_util::sync::CancellationToken;

use super::fixtures::registered_collection;

#[tokio::test]
async fn local_abort_never_cancels_the_remote_job() {
    let backend = std::sync::Arc::new(ScriptedDeferredBackend::new("test", "mock-model"));
    let handle = backend.submit(test_support::text_response("late"), 3, None);
    let (collection, _record) = registered_collection(&backend);

    // Cancel the local fetch token before polling the attempt. The attempt
    // aborts locally without calling the backend's remote cancel entry.
    let cancel = CancellationToken::new();
    cancel.cancel();
    let call = DeferredFetchCall {
        cancel,
        ..Default::default()
    };
    let failure = collection.fetch_deferred(&handle, call).await.unwrap_err();
    assert!(matches!(
        failure.cause(),
        opi_ai::provider::ProviderError::Cancelled
    ));
    assert!(backend.cancelled_handles().is_empty());
    // Cancellation before the first poll dispatches zero backend work
    // (AIR-004: drop before first poll performs zero source work).
    assert_eq!(backend.fetch_count(), 0);

    // The remote job is untouched: the next explicit fetch still works.
    let pending = collection
        .fetch_deferred(&handle, DeferredFetchCall::default())
        .await
        .expect("still fetchable");
    assert_eq!(pending.stop_reason, StopReason::Deferred);
    assert!(backend.cancelled_handles().is_empty());
}

#[tokio::test]
async fn explicit_cancel_is_recorded_and_best_effort() {
    let backend = std::sync::Arc::new(ScriptedDeferredBackend::new("test", "mock-model"));
    let handle = backend.submit(test_support::text_response("late"), 3, None);
    let (collection, _record) = registered_collection(&backend);

    // A cancelled deferred response fails subsequent fetches typed.
    collection
        .cancel_deferred(
            &handle,
            opi_ai::api::contract::DeferredCancelCall::default(),
        )
        .await
        .expect("best-effort cancel succeeds");
    assert_eq!(backend.cancelled_handles(), vec![handle.clone()]);
    let failure = collection
        .fetch_deferred(&handle, DeferredFetchCall::default())
        .await
        .unwrap_err();
    assert_eq!(backend.fetch_count(), 1);
    assert_eq!(backend.cancelled_handles(), vec![handle.clone()]);
    assert_eq!(failure.message().provider, handle.provider);
    assert_eq!(failure.message().model, handle.model_id);
    assert_eq!(failure.message().stop_reason, StopReason::Error);
    assert!(matches!(
        failure.cause(),
        opi_ai::provider::ProviderError::Config(_)
    ));
    let diagnostic = format!(
        "invalid provider configuration: Scripted deferred response was cancelled: {}",
        handle.id
    );
    assert_eq!(
        failure.message().error_message.as_deref(),
        Some(diagnostic.as_str())
    );
    assert!(!failure.cause().is_retryable());
    assert_eq!(
        classify_whole_call_retry(failure.message()),
        WholeCallRetryClassification::Terminal
    );

    // Best-effort failure is reported, never swallowed.
    let other = backend.submit(test_support::text_response("late"), 3, None);
    backend.fail_cancels();
    let error = collection
        .cancel_deferred(&other, opi_ai::api::contract::DeferredCancelCall::default())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("best-effort"));

    // Cancelling an unsupported provider fails typed without dispatch.
    let mut stranger = handle.clone();
    stranger.provider = "other".into();
    let error = collection
        .cancel_deferred(
            &stranger,
            opi_ai::api::contract::DeferredCancelCall::default(),
        )
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "invalid provider configuration: no deferred backend is registered for provider 'other' and API 'scripted-deferred'"
    );
}

#[tokio::test]
async fn remote_cancel_observes_caller_abort_before_and_during_auth() {
    use opi_ai::auth::AuthResolver;
    use opi_ai::credential::BoxAuthFuture;
    use opi_ai::provider::ProviderError;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use std::time::Duration;

    #[derive(Default)]
    struct Auth {
        calls: AtomicUsize,
        entered: tokio::sync::Notify,
        release: tokio::sync::Notify,
    }
    impl AuthResolver for Auth {
        fn resolve(&self) -> BoxAuthFuture<'_, Result<ResolvedAuth, ProviderError>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async {
                self.entered.notify_one();
                self.release.notified().await;
                Ok(opi_ai::test_support::resolved_bearer_auth())
            })
        }
    }
    let auth = Arc::new(Auth::default());
    let backend = Arc::new(ScriptedDeferredBackend::new("test", "mock-model"));
    let handle = backend.submit(test_support::text_response("late"), 3, None);
    let mut collection = Models::new();
    collection
        .register_deferred_api(
            backend.clone() as Arc<dyn opi_ai::api::contract::DeferredApi<()>>,
            auth.clone(),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let collection = Arc::new(collection);

    let call = opi_ai::api::contract::DeferredCancelCall::default();
    call.cancel.cancel();
    let mut wrong_model = handle.clone();
    wrong_model.model_id = "wrong".into();
    let error = collection
        .cancel_deferred(&wrong_model, call.clone())
        .await
        .unwrap_err();
    assert!(
        matches!(error, ProviderError::Config(_)),
        "identity validation precedes abort"
    );
    assert_eq!(auth.calls.load(Ordering::SeqCst), 0);
    let error = tokio::time::timeout(
        Duration::from_millis(100),
        collection.cancel_deferred(&handle, call),
    )
    .await
    .expect("preaborted cancel must not await auth")
    .unwrap_err();
    assert!(matches!(error, ProviderError::Cancelled));
    assert_eq!(auth.calls.load(Ordering::SeqCst), 0);
    assert!(backend.cancelled_handles().is_empty());

    let call = opi_ai::api::contract::DeferredCancelCall::default();
    let abort = call.cancel.clone();
    let pending = tokio::spawn({
        let collection = collection.clone();
        let handle = handle.clone();
        async move { collection.cancel_deferred(&handle, call).await }
    });
    tokio::time::timeout(Duration::from_secs(1), auth.entered.notified())
        .await
        .unwrap();
    assert_eq!(auth.calls.load(Ordering::SeqCst), 1);
    abort.cancel();
    let error = tokio::time::timeout(Duration::from_millis(100), pending)
        .await
        .expect("abort must end caller auth wait")
        .unwrap()
        .unwrap_err();
    assert!(matches!(error, ProviderError::Cancelled));
    auth.release.notify_one();
    tokio::task::yield_now().await;
    assert!(
        backend.cancelled_handles().is_empty(),
        "late auth must not dispatch cancel"
    );

    collection
        .cancel_deferred(&handle, Default::default())
        .await
        .unwrap();
    assert_eq!(auth.calls.load(Ordering::SeqCst), 2);
    assert_eq!(backend.cancelled_handles(), vec![handle]);
}
