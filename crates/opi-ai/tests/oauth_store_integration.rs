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
    ttl: std::sync::atomic::AtomicI64,
}
impl Flow {
    fn new(gated: bool) -> Arc<Self> {
        Arc::new(Self {
            calls: AtomicUsize::new(0),
            entered: tokio::sync::Semaphore::new(0),
            release: tokio::sync::Semaphore::new(0),
            gated,
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
        model: "openai:gpt-4o".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![],
        })
        .unwrap(),
        max_tokens: Some(32),
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
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
