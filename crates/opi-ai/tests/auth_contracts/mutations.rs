use super::fixtures::stored_api_key;
use super::*;

use std::sync::Arc;

use opi_ai::auth::credential::{Credential, CredentialStore};
use opi_ai::{
    BarrierEntry, InMemoryCredentialStore, MutationDecision, MutationOutcome, MutationQueue,
};

pub(crate) fn write_op(value: &'static str) -> opi_ai::CredentialMutationOp {
    opi_ai::write_mutation(stored_api_key(value))
}

#[test]
fn sync_entry_mutations_preserve_fifo_after_runtime_handoff() {
    let store = InMemoryCredentialStore::new();
    let (release, released) = tokio::sync::oneshot::channel::<()>();
    let first = store.modify(
        "prov",
        opi_ai::CredentialMutationOp::Update(Box::new(move |_view, _current| {
            Box::pin(async move {
                released.await.unwrap();
                Ok(MutationDecision::Write(stored_api_key("rotated")))
            })
        })),
    );
    let deletion = store.modify("prov", opi_ai::delete_mutation());
    let barrier = store.mutation_barrier();
    assert_eq!(barrier.cutoff(), 2);
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let mut first = first;
        let mut deletion = deletion;
        assert!(futures_util::poll!(&mut first).is_pending());
        assert!(
            futures_util::poll!(&mut deletion).is_pending(),
            "delete must wait for the earlier admitted rotation"
        );
        release.send(()).unwrap();
        assert!(matches!(
            first.await.unwrap().outcome,
            MutationOutcome::Written { revision: 1 }
        ));
        assert!(matches!(
            deletion.await.unwrap().outcome,
            MutationOutcome::Deleted { revision: 2 }
        ));
        assert!(store.read("prov").await.unwrap().is_none());
        assert!(barrier.report().is_complete());
    });
}

#[test]
fn sync_entry_started_mutation_survives_waiter_drop() {
    let store = InMemoryCredentialStore::new();
    let (started, start) = tokio::sync::oneshot::channel::<()>();
    let (release, released) = tokio::sync::oneshot::channel::<()>();
    let mutation = store.modify(
        "prov",
        opi_ai::CredentialMutationOp::Update(Box::new(move |_view, _current| {
            Box::pin(async move {
                started.send(()).unwrap();
                released.await.unwrap();
                Ok(MutationDecision::Write(stored_api_key("committed")))
            })
        })),
    );
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let mut mutation = mutation;
        assert!(futures_util::poll!(&mut mutation).is_pending());
        start.await.unwrap();
        drop(mutation);
        assert!(
            release.send(()).is_ok(),
            "store must retain the running operation"
        );
        let report = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            store.mutation_barrier().wait(),
        )
        .await
        .expect("store-owned operation must settle");
        assert!(report.is_complete());
        assert!(matches!(
            store.read("prov").await.unwrap(),
            Some(Credential::ApiKey(_))
        ));
    });
}

#[test]
fn off_runtime_submission_waits_for_explicit_runtime_execution_entry() {
    let queue = Arc::new(MutationQueue::new());
    let calls = Arc::new(AtomicUsize::new(0));
    let submitted = queue.clone();
    let worker_calls = calls.clone();
    let mut handle = std::thread::spawn(move || {
        submitted.submit(
            "prov",
            Box::new(move || {
                Box::pin(async move {
                    worker_calls.fetch_add(1, Ordering::SeqCst);
                    Ok(opi_ai::MutationApplied::Write)
                })
            }),
        )
    })
    .join()
    .unwrap();
    let waker = futures_util::task::noop_waker();
    let mut cx = std::task::Context::from_waker(&waker);
    assert!(std::future::Future::poll(std::pin::Pin::new(&mut handle), &mut cx).is_pending());
    drop(handle);
    let barrier = queue.barrier();
    assert!(!barrier.report().is_complete());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(30), barrier.wait())
                .await
                .is_err(),
            "barrier waiting must not start an admitted auth operation"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(!barrier.report().is_complete());
        let later = queue.submit(
            "prov",
            Box::new(|| Box::pin(async { Ok(opi_ai::MutationApplied::Delete) })),
        );
        assert!(matches!(
            later.await.unwrap().outcome,
            MutationOutcome::Deleted { revision: 2 }
        ));
        assert!(barrier.report().is_complete());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    });
}

#[tokio::test]
async fn store_mutations_serialize_in_submission_order_per_provider() {
    let store = InMemoryCredentialStore::new();
    let first = store
        .modify("prov", write_op("one"))
        .await
        .expect("first ACK");
    let second = store
        .modify("prov", write_op("two"))
        .await
        .expect("second ACK");
    let third = store
        .modify("prov", opi_ai::delete_mutation())
        .await
        .expect("third ACK");

    // Store-wide generations admit in submission order and per-provider FIFO
    // executes them in that order; revisions advance per applied mutation.
    assert_eq!(first.generation, 1);
    assert_eq!(second.generation, 2);
    assert_eq!(third.generation, 3);
    assert!(matches!(
        first.outcome,
        MutationOutcome::Written { revision: 1 }
    ));
    assert!(matches!(
        second.outcome,
        MutationOutcome::Written { revision: 2 }
    ));
    assert!(matches!(
        third.outcome,
        MutationOutcome::Deleted { revision: 3 }
    ));
    assert!(
        store.read("prov").await.expect("read").is_none(),
        "final state is the serialized delete"
    );
}

#[tokio::test]
async fn started_mutation_survives_waiter_cancellation() {
    let store = Arc::new(InMemoryCredentialStore::new());
    let (release, released) = tokio::sync::oneshot::channel::<()>();
    let op: opi_ai::CredentialMutationOp =
        opi_ai::CredentialMutationOp::Update(Box::new(move |_view, _current| {
            let released = released;
            Box::pin(async move {
                let _ = released.await;
                Ok(MutationDecision::Write(stored_api_key("committed")))
            })
        }));
    let handle = store.modify("prov", op);

    // Cancel the wait while the mutation is still running: dropping the
    // handle stops only the observation, not the mutation.
    drop(handle);
    // The store owns the mutation: releasing the gate commits it even though
    // no waiter remains.
    release.send(()).unwrap();
    let barrier = store.mutation_barrier();
    let report = barrier.wait().await;
    assert!(report.is_complete());
    match report.captured.as_slice() {
        [BarrierEntry::Settled(receipt)] => {
            assert_eq!(receipt.provider_id, "prov");
            assert!(matches!(
                receipt.outcome,
                MutationOutcome::Written { revision: 1 }
            ));
        }
        other => panic!("expected one settled receipt, got {other:?}"),
    }
    match store.read("prov").await.unwrap() {
        Some(Credential::ApiKey(secret)) => {
            assert_eq!(secret.key().unwrap().expose_secret(), "committed");
        }
        other => panic!("expected committed api key, got {other:?}"),
    }
}

#[tokio::test]
async fn second_waiter_reuses_committed_result_without_second_operation() {
    let store = InMemoryCredentialStore::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_for_op = calls.clone();
    let op: opi_ai::CredentialMutationOp =
        opi_ai::CredentialMutationOp::Update(Box::new(move |_view, current| {
            let calls = calls_for_op.clone();
            Box::pin(async move {
                calls.fetch_add(1, Ordering::SeqCst);
                match current {
                    Some(_) => Ok(MutationDecision::Keep),
                    None => Ok(MutationDecision::Write(stored_api_key("once"))),
                }
            })
        }));
    let r1 = store.modify(
        "prov",
        opi_ai::CredentialMutationOp::Update(Box::new(move |_view, current| {
            // Distinct op object with identical single-flight semantics.
            let _ = current;
            Box::pin(async move { Ok(MutationDecision::Write(stored_api_key("once"))) })
        })),
    );
    let r1 = r1.await.expect("first ACK");
    let r2 = store.modify("prov", op).await.expect("second ACK");
    assert!(matches!(r1.outcome, MutationOutcome::Written { .. }));
    // The second serialized slot re-reads the committed state and keeps it:
    // one operation ran for the second waiter.
    assert!(matches!(
        r2.outcome,
        MutationOutcome::Unchanged { revision: 1 }
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn barrier_captures_cutoff_across_models_sharing_store() {
    let store = Arc::new(InMemoryCredentialStore::new());
    let mut models_a = opi_ai::Models::new();
    models_a.set_credential_store(store.clone());
    let mut models_b = opi_ai::Models::new();
    models_b.set_credential_store(store.clone());

    store.modify("prov-a", write_op("a")).await.expect("ACK a");
    store.modify("prov-b", write_op("b")).await.expect("ACK b");

    let from_a = models_a.mutation_barrier().expect("store attached to a");
    let from_b = models_b.mutation_barrier().expect("store attached to b");
    // The cutoff lives in the store: both Models observe the same history.
    assert_eq!(from_a.cutoff(), from_b.cutoff());
    assert_eq!(from_a.cutoff(), 2);
    let report = from_b.report();
    assert_eq!(report.captured.len(), 2);
    assert!(report.late.is_empty());
    assert!(report.is_complete());
}

#[tokio::test]
async fn constructor_owned_mutation_barriers_start_empty_and_remain_isolated() {
    let mut models = opi_ai::Models::new();
    let isolated = [
        opi_ai::Models::new(),
        opi_ai::Models::from_registry(opi_ai::ProviderRegistry::new()),
    ];
    let before = models.mutation_barrier().expect("constructor-owned store");
    for model in std::iter::once(&models).chain(isolated.iter()) {
        let barrier = model.mutation_barrier().expect("constructor-owned store");
        assert_eq!(barrier.cutoff(), 0);
        assert!(barrier.report().captured.is_empty());
        assert!(barrier.report().late.is_empty());
        assert!(barrier.report().is_complete());
    }
    models.register_oauth_flow("mock", Arc::new(MockOAuthProvider { id: "mock".into() }));
    models
        .login(
            "mock",
            &MockLoginPresenter {
                manual_code: "local-store".into(),
                url_calls: AtomicUsize::new(0),
            },
        )
        .await
        .expect("fake login is acknowledged by local memory");
    let after = models.mutation_barrier().unwrap();
    assert_eq!(after.cutoff(), 1);
    assert_eq!(after.report().captured.len(), 1);
    assert!(after.report().is_complete());
    assert_eq!(before.cutoff(), 0);
    assert!(before.report().captured.is_empty());
    assert_eq!(before.report().late.len(), 1);
    for model in isolated {
        let barrier = model.mutation_barrier().unwrap();
        assert_eq!(barrier.cutoff(), 0);
        assert!(barrier.report().captured.is_empty());
        assert!(barrier.report().late.is_empty());
    }
}

#[tokio::test]
async fn barrier_reports_pending_failure_unknown_and_late_writes() {
    let store = InMemoryCredentialStore::new();
    let (release, released) = tokio::sync::oneshot::channel::<()>();
    let gated: opi_ai::CredentialMutationOp =
        opi_ai::CredentialMutationOp::Update(Box::new(move |_view, _current| {
            let released = released;
            Box::pin(async move {
                let _ = released.await;
                Ok(MutationDecision::Keep)
            })
        }));
    let _pending = store.modify("prov-pending", gated);

    let failing: opi_ai::CredentialMutationOp =
        opi_ai::CredentialMutationOp::Update(Box::new(|_view, _current| {
            Box::pin(async move {
                Err(opi_ai::MutationError::Operation(ProviderError::AuthFailed(
                    opi_ai::provider::ProviderErrorSummary::from_untrusted("refresh failed"),
                )))
            })
        }));
    let failed = store
        .modify("prov-fail", failing)
        .await
        .expect("failed mutations still ACK with a receipt");
    assert!(matches!(failed.outcome, MutationOutcome::Failed { .. }));

    let panicking: opi_ai::CredentialMutationOp =
        opi_ai::CredentialMutationOp::Update(Box::new(|_view, _current| {
            Box::pin(async move { panic!("worker lost") })
        }));
    // Drive the panicking slot to its Unknown receipt by awaiting the handle.
    let unknown = store
        .modify("prov-panic", panicking)
        .await
        .expect("panic mutations settle as Unknown, not lost handles");
    assert!(matches!(unknown.outcome, MutationOutcome::Unknown { .. }));

    let barrier = store.mutation_barrier();
    let report = barrier.report();
    assert_eq!(barrier.cutoff(), 3);
    // Pending, failed, and unknown are all reported inside the cutoff.
    let mut pending = 0;
    let mut failures = 0;
    let mut unknowns = 0;
    for entry in &report.captured {
        match entry {
            BarrierEntry::Pending { .. } => pending += 1,
            BarrierEntry::Settled(receipt) => match receipt.outcome {
                MutationOutcome::Failed { .. } => failures += 1,
                MutationOutcome::Unknown { .. } => unknowns += 1,
                _ => {}
            },
        }
    }
    assert_eq!(pending, 1);
    assert_eq!(failures, 1);
    assert_eq!(unknowns, 1);
    assert!(!report.is_complete());

    // A post-cutoff admission is a fenced late write: reported, never aborted.
    let late = store
        .modify("prov-late", write_op("late"))
        .await
        .expect("late write completes");
    assert!(matches!(late.outcome, MutationOutcome::Written { .. }));
    let report = barrier.report();
    assert_eq!(report.late.len(), 1);
    assert_eq!(report.late[0].provider_id(), "prov-late");

    // Waiting resolves once the pending mutation settles (no default
    // deadline stopped it).
    release.send(()).unwrap();
    let report = barrier.wait().await;
    assert!(report.is_complete());
}

#[tokio::test]
async fn unknown_outcome_for_one_key_survives_another_keys_success() {
    let store = InMemoryCredentialStore::new();
    let panicking: opi_ai::CredentialMutationOp =
        opi_ai::CredentialMutationOp::Update(Box::new(|_view, _current| {
            Box::pin(async move { panic!("lost") })
        }));
    let _ = store
        .modify("prov-old", panicking)
        .await
        .expect("Unknown receipt");
    let ok = store
        .modify("prov-new", write_op("fresh"))
        .await
        .expect("ACK");
    assert!(matches!(ok.outcome, MutationOutcome::Written { .. }));

    let report = store.mutation_barrier().report();
    // Receipts are retained by key: the new key's success did not clear the
    // old key's unconfirmed outcome.
    let old = report
        .captured
        .iter()
        .find(|e| e.provider_id() == "prov-old")
        .expect("old key retained");
    match old {
        BarrierEntry::Settled(receipt) => {
            assert!(matches!(receipt.outcome, MutationOutcome::Unknown { .. }))
        }
        other => panic!("old key should be settled, got {other:?}"),
    }
}

#[tokio::test]
async fn mutation_waits_and_receipts_have_no_default_caps() {
    let store = InMemoryCredentialStore::new();
    // More mutations than any historical default cap (256-key style
    // admission limits): all admit and all receipts are retained.
    let mut handles = Vec::new();
    for i in 0..300 {
        let value: &'static str = Box::leak(format!("key-{i}").into_boxed_str());
        handles.push(store.modify("prov-cap", write_op(value)));
    }
    for handle in handles {
        handle.await.expect("every mutation ACKs");
    }
    let report = store.mutation_barrier().report();
    assert_eq!(report.captured.len(), 300);
    assert!(
        report.captured.iter().all(|entry| matches!(
            entry,
            BarrierEntry::Settled(receipt)
                if matches!(receipt.outcome, MutationOutcome::Written { .. })
        )),
        "every receipt retained"
    );
    assert!(report.is_complete());
}

#[tokio::test]
async fn network_timeout_scope_excludes_lock_wait_and_ack() {
    let store = InMemoryCredentialStore::new();
    // A slow prior slot occupies the (per-provider) serialization for far
    // longer than the second operation's own network bound.
    let (release, released) = tokio::sync::oneshot::channel::<()>();
    let slow: opi_ai::CredentialMutationOp =
        opi_ai::CredentialMutationOp::Update(Box::new(move |_view, _current| {
            let released = released;
            Box::pin(async move {
                let _ = released.await;
                Ok(MutationDecision::Keep)
            })
        }));
    let _slow = store.modify("prov", slow);

    let bounded: opi_ai::CredentialMutationOp =
        opi_ai::CredentialMutationOp::Update(Box::new(|_view, _current| {
            Box::pin(async move {
                // The operation's own network bound (50ms) applies only to the
                // simulated exchange, not to the slot's lock wait or the ACK.
                match tokio::time::timeout(
                    std::time::Duration::from_millis(50),
                    std::future::pending::<()>(),
                )
                .await
                {
                    Err(_elapsed) => Ok(MutationDecision::Write(stored_api_key("after-wait"))),
                    Ok(()) => unreachable!("never resolves"),
                }
            })
        }));
    let bounded = store.modify("prov", bounded);
    // Let the queue admit both before releasing the slow slot.
    tokio::task::yield_now().await;
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    release.send(()).unwrap();
    // The bounded operation's own timeout elapsed while waiting for the
    // serialization, yet the mutation completes: lock waits and ACK are
    // outside the operation's network bound.
    let receipt = bounded.await.expect("ACK after slow slot");
    assert!(matches!(receipt.outcome, MutationOutcome::Written { .. }));
    match store.read("prov").await.unwrap() {
        Some(Credential::ApiKey(secret)) => {
            assert_eq!(secret.key().unwrap().expose_secret(), "after-wait");
        }
        other => panic!("expected committed api key, got {other:?}"),
    }
}

#[tokio::test]
async fn mutation_queue_admits_synchronously_before_any_await() {
    let queue = MutationQueue::new();
    let barrier_before = queue.barrier();
    assert_eq!(barrier_before.cutoff(), 0);
    let (release, released) = tokio::sync::oneshot::channel::<()>();
    let slot: opi_ai::MutationSlot = Box::new(move || {
        let released = released;
        Box::pin(async move {
            let _ = released.await;
            Ok(opi_ai::MutationApplied::Write)
        })
    });
    let _handle = queue.submit("prov", slot);
    // Admission was synchronous: the barrier captured the running mutation
    // even though nothing has been awaited yet.
    let barrier_after = queue.barrier();
    assert_eq!(barrier_after.cutoff(), 1);
    assert!(barrier_before.report().is_complete());
    release.send(()).unwrap();
    let report = barrier_after.wait().await;
    assert!(report.is_complete());
}

// ---------------------------------------------------------------------------
// Task 20.19.3: source near-expiry refresh registers through the shared
// credential mutation owner (P20-STORE consumer closure).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn source_passive_refresh_registers_through_the_mutation_owner() {
    use opi_ai::auth::aws::cache::{
        AwsSourceCache, CacheIdentity, PassiveRefreshDeps, SourceCacheValue, SsoRefreshMaterial,
        SsoTokenSnapshot,
    };
    use opi_ai::auth::aws::source_http::{
        AwsSourceTransport, SourceHttpRequest, SourceHttpResponse,
    };
    use secrecy::SecretString;

    struct DenyingTransport;
    impl AwsSourceTransport for DenyingTransport {
        fn send(
            &self,
            _request: SourceHttpRequest,
        ) -> opi_ai::BoxAuthFuture<
            'static,
            Result<SourceHttpResponse, opi_ai::auth::aws::source_http::SourceHttpError>,
        > {
            Box::pin(async { Err(opi_ai::auth::aws::source_http::SourceHttpError::Transport) })
        }
    }

    struct FixedClock(time::OffsetDateTime);
    impl opi_ai::auth::aws::cache::SourceClock for FixedClock {
        fn now(&self) -> time::OffsetDateTime {
            self.0
        }
    }
    struct FixedJti;
    impl opi_ai::auth::aws::cache::DpopJtiSource for FixedJti {
        fn next_jti(&self) -> String {
            "jti".to_owned()
        }
    }

    let queue = std::sync::Arc::new(opi_ai::auth::mutation::MutationQueue::new());
    let cache = AwsSourceCache::with_mutation_queue(queue.clone()).with_passive_deps(
        std::sync::Arc::new(PassiveRefreshDeps {
            transport: std::sync::Arc::new(DenyingTransport),
            clock: std::sync::Arc::new(FixedClock(
                time::OffsetDateTime::from_unix_timestamp(0).unwrap(),
            )),
            jti: std::sync::Arc::new(FixedJti),
        }),
    );
    let identity = CacheIdentity::SsoToken {
        cache_key: "session".to_owned(),
    };
    cache.store(
        identity.clone(),
        SourceCacheValue::SsoToken(SsoTokenSnapshot {
            access_token: SecretString::from("token"),
            expires_at: time::OffsetDateTime::from_unix_timestamp(120).unwrap(),
            refresh: Some(SsoRefreshMaterial {
                client_id: "client".to_owned(),
                client_secret: SecretString::from("secret"),
                refresh_token: SecretString::from("refresh"),
            }),
            modern: true,
        }),
    );

    let barrier_before = queue.barrier();
    let spawned = cache.maybe_spawn_passive(
        identity,
        time::OffsetDateTime::from_unix_timestamp(0).unwrap(),
        Box::new(|| {
            Box::pin(async { Ok(opi_ai::auth::mutation::MutationApplied::Keep) })
                as opi_ai::BoxAuthFuture<
                    'static,
                    Result<
                        opi_ai::auth::mutation::MutationApplied,
                        opi_ai::auth::mutation::MutationError,
                    >,
                >
        }),
    );
    assert!(spawned, "near-expiry refresh registers with the owner");
    let barrier_after = queue.barrier();
    assert!(barrier_after.cutoff() > barrier_before.cutoff());
    // The original caller may leave; the barrier still observes completion.
    let report = barrier_after.wait().await;
    assert!(report.is_complete());
}

// ---------------------------------------------------------------------------
// ADC native families (task 20.21.1)
// ---------------------------------------------------------------------------
