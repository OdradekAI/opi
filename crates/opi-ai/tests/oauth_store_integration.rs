//! Public SDK store/refresh/request assembly; all identities and HTTP are local.
use opi_ai::auth::{LoginPresenter, OAuthCredential, OAuthProvider};
use opi_ai::credential::{BoxAuthFuture, CredentialStore};
use opi_ai::models::{AuthResolutionOverrides, CompatMetadata, Models, drain_to_completion};
use opi_ai::mutation::InMemoryCredentialStore;
use opi_ai::openai_chat::OpenAiChatProvider;
use opi_ai::provider::{CacheRetention, ProviderError, Request, ThinkingConfig};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

fn credential(access: &str, expires: i64) -> OAuthCredential {
    OAuthCredential {
        access: access.to_owned().into(),
        refresh: "rotating-refresh".into(),
        expires_at: Some(time::OffsetDateTime::now_utc() + time::Duration::seconds(expires)),
        base_url: None,
        account_id: Some("selected-account".into()),
        client_id: Some("issued-client".into()),
        enterprise_domain: None,
        available_model_ids: None,
        scopes: Some(vec!["model.read".into()]),
    }
}
struct Flow {
    calls: AtomicUsize,
    entered: tokio::sync::Semaphore,
    release: tokio::sync::Semaphore,
    gated: bool,
    refresh_delay: Duration,
    ttl: std::sync::atomic::AtomicI64,
}
impl Flow {
    fn new(gated: bool) -> Arc<Self> {
        Self::with_timing(gated, Duration::ZERO)
    }

    fn with_timing(gated: bool, refresh_delay: Duration) -> Arc<Self> {
        Arc::new(Self {
            calls: AtomicUsize::new(0),
            entered: tokio::sync::Semaphore::new(0),
            release: tokio::sync::Semaphore::new(0),
            gated,
            refresh_delay,
            ttl: std::sync::atomic::AtomicI64::new(3600),
        })
    }
}
impl OAuthProvider for Flow {
    fn id(&self) -> &str {
        "openai"
    }
    fn login<'a>(
        &'a self,
        _: &'a dyn LoginPresenter,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        Box::pin(async { Ok(credential("login", 3600)) })
    }
    fn refresh<'a>(
        &'a self,
        c: &'a OAuthCredential,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        Box::pin(async move {
            assert_eq!(c.client_id.as_deref(), Some("issued-client"));
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.entered.add_permits(1);
            if self.gated {
                self.release.acquire().await.unwrap().forget();
            }
            if !self.refresh_delay.is_zero() {
                tokio::time::sleep(self.refresh_delay).await;
            }
            Ok(credential(
                "refreshed-selected-account",
                self.ttl.load(Ordering::SeqCst),
            ))
        })
    }
}
fn models(store: Arc<dyn CredentialStore>, flow: Arc<Flow>, endpoint: String) -> Models {
    let mut models = Models::new();
    models.set_credential_store(store);
    models
        .register_oauth_route(
            Box::new(OpenAiChatProvider::new(Some(endpoint))),
            flow,
            CompatMetadata::default(),
        )
        .unwrap();
    models
}
fn request() -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openai:gpt-4o".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![],
        })
        .unwrap(),
        max_tokens: Some(32),
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

#[tokio::test]
async fn public_route_refreshes_once_and_actual_adapter_consumes_selected_bearer() {
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{header, method, path},
    };
    let server = MockServer::start().await;
    Mock::given(method("POST")).and(path("/v1/chat/completions")).and(header("authorization","Bearer refreshed-selected-account"))
        .respond_with(ResponseTemplate::new(200).insert_header("content-type","text/event-stream")
        .set_body_string("data: {\"id\":\"local\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"ok\"},\"finish_reason\":null}]}\n\ndata: {\"id\":\"local\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n"))
        .expect(1).mount(&server).await;
    let store = Arc::new(InMemoryCredentialStore::new());
    store
        .write("openai", &credential("expired", -1).into())
        .await
        .unwrap();
    let flow = Flow::new(false);
    let models = models(store, flow.clone(), server.uri());
    let call = models
        .prepare_call("openai:gpt-4o", request())
        .await
        .unwrap();
    drain_to_completion(call.start_attempt().unwrap())
        .await
        .unwrap();
    assert_eq!(flow.calls.load(Ordering::SeqCst), 1);
    server.verify().await;
}

#[tokio::test]
async fn two_models_share_rotation_and_waiter_drop_preserves_the_acknowledged_result() {
    let store = Arc::new(InMemoryCredentialStore::new());
    store
        .write("openai", &credential("expired", -1).into())
        .await
        .unwrap();
    let flow = Flow::new(true);
    let one = Arc::new(models(
        store.clone(),
        flow.clone(),
        "http://127.0.0.1:1".into(),
    ));
    let two = models(store.clone(), flow.clone(), "http://127.0.0.1:1".into());
    let first = {
        let one = one.clone();
        tokio::spawn(async move {
            one.get_auth("openai", AuthResolutionOverrides::default())
                .await
        })
    };
    flow.entered.acquire().await.unwrap().forget();
    let barrier = two.mutation_barrier().unwrap();
    assert!(
        barrier
            .report()
            .captured
            .iter()
            .any(|e| matches!(e, opi_ai::mutation::BarrierEntry::Pending { .. }))
    );
    first.abort();
    flow.release.add_permits(1);
    barrier.wait().await;
    let resolved = two
        .get_auth("openai", AuthResolutionOverrides::default())
        .await
        .unwrap()
        .unwrap();
    use secrecy::ExposeSecret;
    assert_eq!(
        resolved.secret.expose_secret(),
        "refreshed-selected-account"
    );
    assert_eq!(flow.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn refreshed_access_must_still_be_valid_after_ack_but_may_have_a_short_ttl() {
    for ttl in [-1, 120] {
        let store = Arc::new(InMemoryCredentialStore::new());
        store
            .write("openai", &credential("expired", -1).into())
            .await
            .unwrap();
        let flow = Flow::new(false);
        flow.ttl.store(ttl, Ordering::SeqCst);
        let models = models(store, flow, "http://127.0.0.1:1".into());
        let result = models
            .get_auth("openai", AuthResolutionOverrides::default())
            .await;
        assert_eq!(
            result.is_ok(),
            ttl > 0,
            "actual expiry governs the current call, not the eager window"
        );
    }
}

struct UnknownAckStore(InMemoryCredentialStore);
impl CredentialStore for UnknownAckStore {
    fn read<'a>(
        &'a self,
        k: &'a str,
    ) -> BoxAuthFuture<'a, Result<Option<opi_ai::Credential>, opi_ai::CredentialStoreError>> {
        self.0.read(k)
    }
    fn write<'a>(
        &'a self,
        k: &'a str,
        c: &'a opi_ai::Credential,
    ) -> BoxAuthFuture<'a, Result<(), opi_ai::CredentialStoreError>> {
        self.0.write(k, c)
    }
    fn delete<'a>(
        &'a self,
        k: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), opi_ai::CredentialStoreError>> {
        self.0.delete(k)
    }
    fn probe<'a>(&'a self, k: &'a str) -> BoxAuthFuture<'a, opi_ai::CredentialSource> {
        self.0.probe(k)
    }
    fn modify<'a>(
        &'a self,
        k: &'a str,
        op: opi_ai::mutation::CredentialMutationOp,
    ) -> BoxAuthFuture<'a, Result<opi_ai::mutation::MutationReceipt, opi_ai::CredentialStoreError>>
    {
        let pending = self.0.modify(k, op);
        Box::pin(async move {
            let mut receipt = pending.await?;
            receipt.outcome = opi_ai::mutation::MutationOutcome::Unknown {
                detail: "synthetic lost ACK".into(),
            };
            Ok(receipt)
        })
    }
}

#[tokio::test]
async fn committed_token_cannot_turn_an_unknown_ack_into_success() {
    let store = Arc::new(UnknownAckStore(InMemoryCredentialStore::new()));
    store
        .write("openai", &credential("expired", -1).into())
        .await
        .unwrap();
    let models = models(store, Flow::new(false), "http://127.0.0.1:1".into());
    let result = models
        .get_auth("openai", AuthResolutionOverrides::default())
        .await;
    assert!(
        result.is_err(),
        "a locally visible token does not prove the refresh ACK"
    );
}

#[tokio::test]
async fn queued_lookup_selects_its_identity_only_after_acquiring_the_store_slot() {
    use opi_ai::mutation::{CredentialMutationOp, MutationDecision};
    let store = Arc::new(InMemoryCredentialStore::new());
    store
        .write("openai", &credential("old-account", -1).into())
        .await
        .unwrap();
    let (release, released) = tokio::sync::oneshot::channel();
    let (entered, ready) = tokio::sync::oneshot::channel();
    let replacement = store.modify(
        "openai",
        CredentialMutationOp::Update(Box::new(move |_, _| {
            Box::pin(async move {
                let _ = entered.send(());
                released.await.unwrap();
                Ok(MutationDecision::Write(
                    credential("new-account", 3600).into(),
                ))
            })
        })),
    );
    ready.await.unwrap();
    let models = Arc::new(models(
        store.clone(),
        Flow::new(false),
        "http://127.0.0.1:1".into(),
    ));
    let mut pending = Box::pin(models.get_auth("openai", AuthResolutionOverrides::default()));
    assert!(futures_util::poll!(&mut pending).is_pending());
    assert_eq!(
        store.read_count(),
        0,
        "no credential identity may be selected from outside the owned slot"
    );
    release.send(()).unwrap();
    replacement.await.unwrap();
    use secrecy::ExposeSecret;
    assert_eq!(
        pending.await.unwrap().unwrap().secret.expose_secret(),
        "new-account"
    );
}

#[tokio::test]
async fn replacing_the_attached_store_rebinds_the_generated_oauth_route() {
    use secrecy::ExposeSecret;
    let old = Arc::new(InMemoryCredentialStore::new());
    old.write("openai", &credential("old-store-account", 3600).into())
        .await
        .unwrap();
    let new = Arc::new(InMemoryCredentialStore::new());
    new.write("openai", &credential("selected-new-store", 3600).into())
        .await
        .unwrap();
    let flow = Flow::new(false);
    let mut models = models(old.clone(), flow.clone(), "http://127.0.0.1:1".into());
    models.set_credential_store(new.clone());
    let auth = models
        .get_auth("openai", AuthResolutionOverrides::default())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(auth.secret.expose_secret(), "selected-new-store");
    assert_eq!(flow.calls.load(Ordering::SeqCst), 0);
    models.logout(new.as_ref(), "openai").await.unwrap();
    assert!(new.read("openai").await.unwrap().is_none());
    assert!(old.read("openai").await.unwrap().is_some());
}

struct LogoutBetweenReads {
    store: InMemoryCredentialStore,
    reads: AtomicUsize,
}
impl CredentialStore for LogoutBetweenReads {
    fn read<'a>(
        &'a self,
        k: &'a str,
    ) -> BoxAuthFuture<'a, Result<Option<opi_ai::Credential>, opi_ai::CredentialStoreError>> {
        if self.reads.fetch_add(1, Ordering::SeqCst) == 0 {
            self.store.read(k)
        } else {
            Box::pin(async { Ok(None) })
        }
    }
    fn write<'a>(
        &'a self,
        k: &'a str,
        c: &'a opi_ai::Credential,
    ) -> BoxAuthFuture<'a, Result<(), opi_ai::CredentialStoreError>> {
        self.store.write(k, c)
    }
    fn delete<'a>(
        &'a self,
        k: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), opi_ai::CredentialStoreError>> {
        self.store.delete(k)
    }
    fn probe<'a>(&'a self, k: &'a str) -> BoxAuthFuture<'a, opi_ai::CredentialSource> {
        self.store.probe(k)
    }
}

#[tokio::test]
async fn availability_never_combines_one_account_presence_with_later_missing_metadata() {
    let store = Arc::new(LogoutBetweenReads {
        store: InMemoryCredentialStore::new(),
        reads: AtomicUsize::new(0),
    });
    let mut credential = credential("selected-account", 3600);
    credential.available_model_ids = Some(vec!["gpt-4o".into()]);
    store.write("openai", &credential.into()).await.unwrap();
    let models = models(store, Flow::new(false), "http://127.0.0.1:1".into());
    let available = models.get_available(Some("openai")).await.unwrap();
    assert_eq!(
        available
            .iter()
            .map(|(_, m)| m.id.as_str())
            .collect::<Vec<_>>(),
        vec!["gpt-4o"],
        "neither the old account nor a logged-out store permits the whole catalog"
    );
}

/// Delay persistence inside the real queue slot, after the resolver decides
/// Write but before the credential is published or its receipt can settle.
struct DelayedCommitStore {
    inner: InMemoryCredentialStore,
    delay: Duration,
    persistence_started: Arc<tokio::sync::Semaphore>,
}

impl DelayedCommitStore {
    fn new(delay: Duration) -> Self {
        Self {
            inner: InMemoryCredentialStore::new(),
            delay,
            persistence_started: Arc::new(tokio::sync::Semaphore::new(0)),
        }
    }
}

impl CredentialStore for DelayedCommitStore {
    fn read<'a>(
        &'a self,
        key: &'a str,
    ) -> BoxAuthFuture<'a, Result<Option<opi_ai::Credential>, opi_ai::CredentialStoreError>> {
        self.inner.read(key)
    }

    fn write<'a>(
        &'a self,
        key: &'a str,
        credential: &'a opi_ai::Credential,
    ) -> BoxAuthFuture<'a, Result<(), opi_ai::CredentialStoreError>> {
        self.inner.write(key, credential)
    }

    fn delete<'a>(
        &'a self,
        key: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), opi_ai::CredentialStoreError>> {
        self.inner.delete(key)
    }

    fn probe<'a>(&'a self, key: &'a str) -> BoxAuthFuture<'a, opi_ai::CredentialSource> {
        self.inner.probe(key)
    }

    fn modify<'a>(
        &'a self,
        key: &'a str,
        op: opi_ai::mutation::CredentialMutationOp,
    ) -> BoxAuthFuture<'a, Result<opi_ai::mutation::MutationReceipt, opi_ai::CredentialStoreError>>
    {
        use opi_ai::mutation::{CredentialMutationOp, MutationDecision};
        let delay = self.delay;
        let started = self.persistence_started.clone();
        let op = match op {
            CredentialMutationOp::Update(update) => {
                CredentialMutationOp::Update(Box::new(move |view, current| {
                    Box::pin(async move {
                        let decision = update(view, current).await?;
                        if matches!(&decision, MutationDecision::Write(_)) {
                            started.add_permits(1);
                            tokio::time::sleep(delay).await;
                        }
                        Ok(decision)
                    })
                }))
            }
            other => other,
        };
        self.inner.modify(key, op)
    }

    fn mutation_barrier(&self) -> opi_ai::mutation::MutationBarrier {
        self.inner.mutation_barrier()
    }

    fn mutation_queue(&self) -> Option<Arc<opi_ai::mutation::MutationQueue>> {
        self.inner.mutation_queue()
    }
}

async fn assert_stored_access(store: &dyn CredentialStore, expected: &str) {
    use secrecy::ExposeSecret;
    let Some(opi_ai::Credential::OAuthToken { access, .. }) = store.read("openai").await.unwrap()
    else {
        panic!("selected stored OAuth identity");
    };
    assert_eq!(access.expose_secret(), expected);
}

async fn assert_public_default_budget_excludes_store_time(lock_wait: Duration) {
    use opi_ai::mutation::{BarrierEntry, CredentialMutationOp, MutationDecision, MutationOutcome};
    use secrecy::ExposeSecret;
    let store = Arc::new(DelayedCommitStore::new(Duration::from_secs(6)));
    store
        .write("openai", &credential("expired-selected-account", -1).into())
        .await
        .unwrap();
    let blocked = if lock_wait.is_zero() {
        None
    } else {
        let (release, released) = tokio::sync::oneshot::channel();
        let (entered, ready) = tokio::sync::oneshot::channel();
        let preceding = store.modify(
            "openai",
            CredentialMutationOp::Update(Box::new(move |_, _| {
                Box::pin(async move {
                    entered.send(()).unwrap();
                    released.await.unwrap();
                    Ok(MutationDecision::Keep)
                })
            })),
        );
        ready.await.unwrap();
        Some((release, preceding))
    };
    let flow = Flow::with_timing(false, Duration::from_secs(10));
    // register_oauth_route supplies the production default, with no override.
    let models = models(store.clone(), flow.clone(), "http://127.0.0.1:1".into());
    let start = tokio::time::Instant::now();
    let mut pending = Box::pin(models.get_auth("openai", AuthResolutionOverrides::default()));
    assert!(futures_util::poll!(&mut pending).is_pending());
    let barrier = models.mutation_barrier().unwrap();
    let refresh_generation = barrier.cutoff();
    if let Some((release, preceding)) = blocked {
        tokio::time::advance(lock_wait).await;
        assert!(futures_util::poll!(&mut pending).is_pending());
        assert_eq!(flow.calls.load(Ordering::SeqCst), 0);
        assert!(!barrier.report().is_complete());
        release.send(()).unwrap();
        preceding.await.unwrap();
    }
    flow.entered.acquire().await.unwrap().forget();
    assert_eq!(start.elapsed(), lock_wait);
    tokio::time::advance(Duration::from_secs(10)).await;
    tokio::select! {
        permit = store.persistence_started.acquire() => permit.unwrap().forget(),
        outcome = &mut pending => panic!("auth returned before persistence ACK: {outcome:?}"),
    }
    assert_eq!(start.elapsed(), lock_wait + Duration::from_secs(10));
    assert!(futures_util::poll!(&mut pending).is_pending());
    assert_stored_access(store.as_ref(), "expired-selected-account").await;
    tokio::time::advance(Duration::from_secs(5)).await;
    assert!(futures_util::poll!(&mut pending).is_pending());
    assert_stored_access(store.as_ref(), "expired-selected-account").await;
    assert!(barrier.report().captured.iter().any(
        |entry| matches!(entry, BarrierEntry::Pending { generation, provider_id }
            if *generation == refresh_generation && provider_id == "openai")
    ));
    tokio::time::advance(Duration::from_secs(1)).await;
    let auth = pending.await.unwrap().unwrap();
    assert_eq!(start.elapsed(), lock_wait + Duration::from_secs(16));
    assert_eq!(auth.secret.expose_secret(), "refreshed-selected-account");
    assert_eq!(auth.account_id.as_deref(), Some("selected-account"));
    assert_eq!(flow.calls.load(Ordering::SeqCst), 1);
    assert_stored_access(store.as_ref(), "refreshed-selected-account").await;
    let report = barrier.report();
    assert!(report.is_complete());
    assert!(matches!(
        report.captured.last(),
        Some(BarrierEntry::Settled(receipt))
            if receipt.generation == refresh_generation
                && matches!(receipt.outcome, MutationOutcome::Written { .. })
    ));
}

#[tokio::test(start_paused = true)]
async fn public_resolver_default_budget_allows_ten_second_refresh_and_six_second_ack() {
    assert_public_default_budget_excludes_store_time(Duration::ZERO).await;
}

#[tokio::test(start_paused = true)]
async fn public_resolver_default_budget_excludes_twenty_second_store_lock_wait() {
    assert_public_default_budget_excludes_store_time(Duration::from_secs(20)).await;
}

#[tokio::test(start_paused = true)]
async fn public_resolver_default_budget_times_out_refresh_network_without_a_late_commit() {
    use opi_ai::models::ModelsError;
    use opi_ai::mutation::{BarrierEntry, MutationError, MutationOutcome};
    let store = Arc::new(DelayedCommitStore::new(Duration::from_secs(6)));
    store
        .write("openai", &credential("expired-selected-account", -1).into())
        .await
        .unwrap();
    let flow = Flow::with_timing(false, Duration::from_secs(16));
    let models = models(store.clone(), flow.clone(), "http://127.0.0.1:1".into());
    let start = tokio::time::Instant::now();
    let mut pending = Box::pin(models.get_auth("openai", AuthResolutionOverrides::default()));
    assert!(futures_util::poll!(&mut pending).is_pending());
    flow.entered.acquire().await.unwrap().forget();
    assert_eq!(start.elapsed(), Duration::ZERO);
    let barrier = models.mutation_barrier().unwrap();
    let refresh_generation = barrier.cutoff();
    tokio::time::advance(Duration::from_millis(14_999)).await;
    assert!(futures_util::poll!(&mut pending).is_pending());
    assert!(!barrier.report().is_complete());
    tokio::time::advance(Duration::from_millis(2)).await;
    let error = pending.await.expect_err("default refresh network deadline");
    assert!(matches!(
        error,
        ModelsError::AuthResolution { provider, error }
            if provider == "openai" && matches!(*error, ProviderError::Timeout)
    ));
    assert_eq!(start.elapsed(), Duration::from_millis(15_001));
    let report = barrier.report();
    assert!(report.is_complete());
    assert!(report.captured.iter().any(|entry| matches!(
        entry,
        BarrierEntry::Settled(receipt) if receipt.generation == refresh_generation && matches!(
            &receipt.outcome,
            MutationOutcome::Failed { error } if matches!(error.as_ref(), MutationError::Operation(ProviderError::Timeout))
        )
    )));
    tokio::time::advance(Duration::from_secs(2)).await;
    assert_stored_access(store.as_ref(), "expired-selected-account").await;
    assert_eq!(store.persistence_started.available_permits(), 0);
    assert_eq!(flow.calls.load(Ordering::SeqCst), 1);
}
