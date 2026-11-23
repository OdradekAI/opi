use super::*;
use opi_ai::RefreshStatus;
use opi_ai::catalog::{DynamicCatalogSource, HttpCatalogSource, ModelsStore};
use opi_ai::models::RefreshOutcome;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Provider whose `refresh_models` signals entry and blocks until released.
struct BarrierRefreshProvider {
    id: &'static str,
    builtin: Vec<ModelInfo>,
    fresh: Vec<ModelInfo>,
    entered: Arc<tokio::sync::Notify>,
    release: Mutex<Option<tokio::sync::oneshot::Receiver<()>>>,
}

impl BarrierRefreshProvider {
    fn new(
        id: &'static str,
        fresh: Vec<ModelInfo>,
        entered: Arc<tokio::sync::Notify>,
        release: tokio::sync::oneshot::Receiver<()>,
    ) -> Self {
        Self {
            id,
            builtin: Vec::new(),
            fresh,
            entered,
            release: Mutex::new(Some(release)),
        }
    }
}

impl Provider for BarrierRefreshProvider {
    fn id(&self) -> &str {
        self.id
    }
    fn models(&self) -> &[ModelInfo] {
        Box::leak(self.builtin.clone().into_boxed_slice())
    }
    fn stream_prepared(
        &self,
        _request: Request,
        _auth: opi_ai::auth::ResolvedAuth,
    ) -> opi_ai::provider::EventStream {
        Box::pin(futures_util::stream::empty())
    }
    fn refresh_models(
        &self,
        _context: opi_ai::CatalogRefreshContext,
    ) -> BoxAuthFuture<'_, Result<Option<Vec<ModelInfo>>, ProviderError>> {
        self.entered.notify_one();
        let release = self.release.lock().unwrap().take();
        let fresh = self.fresh.clone();
        Box::pin(async move {
            if let Some(release) = release {
                let _ = release.await;
            }
            Ok(Some(fresh))
        })
    }
}

/// Provider that counts refresh fetches, proving offline refresh never
/// touches the network path.
struct CountingRefreshProvider {
    id: &'static str,
    fresh: Vec<ModelInfo>,
    fetches: Arc<AtomicUsize>,
}

struct CountingOAuthResolver(Arc<AtomicUsize>);

impl AuthResolver for CountingOAuthResolver {
    fn resolve(&self) -> BoxAuthFuture<'_, Result<ResolvedAuth, ProviderError>> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {
            Err(ProviderError::CredentialNeeded {
                provider_id: "dyn".to_owned(),
            })
        })
    }
}

impl Provider for CountingRefreshProvider {
    fn id(&self) -> &str {
        self.id
    }
    fn models(&self) -> &[ModelInfo] {
        Box::leak(self.fresh.clone().into_boxed_slice())
    }
    fn stream_prepared(
        &self,
        _request: Request,
        _auth: opi_ai::auth::ResolvedAuth,
    ) -> opi_ai::provider::EventStream {
        Box::pin(futures_util::stream::empty())
    }
    fn refresh_models(
        &self,
        _context: opi_ai::CatalogRefreshContext,
    ) -> BoxAuthFuture<'_, Result<Option<Vec<ModelInfo>>, ProviderError>> {
        self.fetches.fetch_add(1, Ordering::SeqCst);
        let fresh = self.fresh.clone();
        Box::pin(async move { Ok(Some(fresh)) })
    }
}

/// User-plugged dynamic catalog source with fixed output.
struct FixedCatalogSource {
    id: String,
    catalogs: HashMap<String, opi_ai::catalog::ModelsStoreEntry>,
    providers: Vec<String>,
}

enum CancelAtCatalogResult {
    External {
        entered: Arc<std::sync::Barrier>,
        released: Arc<std::sync::Barrier>,
    },
    Source,
    BeforeRefresh,
}

struct CancellingCatalogSource {
    fetches: Arc<AtomicUsize>,
    cancellation: Arc<CancelAtCatalogResult>,
}

impl DynamicCatalogSource for CancellingCatalogSource {
    fn id(&self) -> &str {
        "cancel-at-result"
    }

    fn provider_ids(&self) -> &[String] {
        static IDS: std::sync::LazyLock<Vec<String>> =
            std::sync::LazyLock::new(|| vec!["dyn".into()]);
        &IDS
    }
    fn fetch(
        &self,
        context: opi_ai::CatalogRefreshContext,
    ) -> futures_util::future::BoxFuture<
        'static,
        Result<HashMap<String, opi_ai::catalog::ModelsStoreEntry>, ProviderError>,
    > {
        let token = context.cancel;
        let later = self.fetches.fetch_add(1, Ordering::SeqCst) > 0;
        let cancellation = self.cancellation.clone();
        Box::pin(async move {
            if later {
                match cancellation.as_ref() {
                    CancelAtCatalogResult::External { entered, released } => {
                        // Keep this poll pending synchronously until the external
                        // thread cancels, then return a successful late result.
                        entered.wait();
                        released.wait();
                    }
                    CancelAtCatalogResult::Source => token.cancel(),
                    CancelAtCatalogResult::BeforeRefresh => {}
                }
            }
            let id = if later { "late" } else { "old" };
            Ok(catalog_fixture::catalogs(HashMap::from([(
                "dyn".to_owned(),
                vec![model_info(id, id)],
            )])))
        })
    }
}

async fn assert_cancelled_catalog_retains_publication(
    with_store: bool,
    cancellation: CancelAtCatalogResult,
) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("models-store.json");
    let mut collection = Models::new();
    register(
        &mut collection,
        Box::new(RefreshProvider::new("dyn", vec![])),
    );
    if with_store {
        collection.set_models_store(ModelsStore::open(&path).unwrap());
    }
    let fetches = Arc::new(AtomicUsize::new(0));
    let cancellation = Arc::new(cancellation);
    collection.register_catalog_source(Arc::new(CancellingCatalogSource {
        fetches: fetches.clone(),
        cancellation: cancellation.clone(),
    }));
    collection.refresh(Default::default()).await.unwrap();
    assert!(collection.resolve("dyn:old").is_ok());
    let persisted = with_store.then(|| std::fs::read(&path).unwrap());

    let token = CancellationToken::new();
    let external = match cancellation.as_ref() {
        CancelAtCatalogResult::External { entered, released } => {
            let (entered, released) = (entered.clone(), released.clone());
            let token = token.clone();
            Some(std::thread::spawn(move || {
                entered.wait();
                token.cancel();
                released.wait();
            }))
        }
        CancelAtCatalogResult::BeforeRefresh => {
            token.cancel();
            None
        }
        CancelAtCatalogResult::Source => None,
    };
    let report = collection
        .refresh_with(Default::default(), &token)
        .await
        .unwrap();
    if let Some(external) = external {
        external.join().unwrap();
    }
    assert!(token.is_cancelled());
    assert_eq!(
        report.status(),
        RefreshStatus::Cancelled,
        "store={with_store}"
    );
    assert!(collection.resolve("dyn:old").is_ok(), "store={with_store}");
    assert!(
        collection.resolve("dyn:late").is_err(),
        "store={with_store}"
    );
    let expected_fetches = if matches!(cancellation.as_ref(), CancelAtCatalogResult::BeforeRefresh)
    {
        1
    } else {
        2
    };
    assert_eq!(fetches.load(Ordering::SeqCst), expected_fetches);
    if let Some(persisted) = persisted {
        assert_eq!(std::fs::read(&path).unwrap(), persisted);
        let reopened = ModelsStore::open(&path).unwrap();
        assert_eq!(reopened.revision(), 1);
        assert_eq!(reopened.snapshot().catalogs["dyn"].models[0].id(), "old");
    }
}

#[tokio::test]
async fn cancelled_catalog_external_result_preserves_old_revision() {
    for with_store in [true, false] {
        assert_cancelled_catalog_retains_publication(
            with_store,
            CancelAtCatalogResult::External {
                entered: Arc::new(std::sync::Barrier::new(2)),
                released: Arc::new(std::sync::Barrier::new(2)),
            },
        )
        .await;
    }
}

#[tokio::test]
async fn cancelled_catalog_source_result_preserves_old_revision() {
    for with_store in [true, false] {
        assert_cancelled_catalog_retains_publication(with_store, CancelAtCatalogResult::Source)
            .await;
    }
}

#[tokio::test]
async fn cancelled_catalog_before_refresh_preserves_old_revision_without_fetch() {
    for with_store in [true, false] {
        assert_cancelled_catalog_retains_publication(
            with_store,
            CancelAtCatalogResult::BeforeRefresh,
        )
        .await;
    }
}

impl DynamicCatalogSource for FixedCatalogSource {
    fn id(&self) -> &str {
        &self.id
    }
    fn provider_ids(&self) -> &[String] {
        &self.providers
    }
    fn fetch(
        &self,
        _context: opi_ai::CatalogRefreshContext,
    ) -> futures_util::future::BoxFuture<
        'static,
        Result<HashMap<String, opi_ai::catalog::ModelsStoreEntry>, ProviderError>,
    > {
        let catalogs = self.catalogs.clone();
        Box::pin(async move { Ok(catalogs) })
    }
}

/// User-plugged dynamic catalog source that always fails.
struct FailingCatalogSource {
    id: String,
}

impl DynamicCatalogSource for FailingCatalogSource {
    fn id(&self) -> &str {
        &self.id
    }
    fn provider_ids(&self) -> &[String] {
        static IDS: std::sync::LazyLock<Vec<String>> =
            std::sync::LazyLock::new(|| vec!["custom-prov".into()]);
        &IDS
    }
    fn fetch(
        &self,
        _context: opi_ai::CatalogRefreshContext,
    ) -> futures_util::future::BoxFuture<
        'static,
        Result<HashMap<String, opi_ai::catalog::ModelsStoreEntry>, ProviderError>,
    > {
        Box::pin(async move {
            Err(ProviderError::ProviderSide(
                opi_ai::provider::ProviderErrorSummary::from_untrusted("source down"),
            ))
        })
    }
}

fn register(collection: &mut Models, provider: Box<dyn Provider>) {
    collection
        .register(
            provider,
            AuthDescriptor::StaticApiKey {
                value: SecretKey::new("key"),
            },
            CompatMetadata::default(),
        )
        .unwrap();
}

#[tokio::test]
async fn refresh_false_performs_no_network_and_restores_store_first() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("models-store.json");

    // A prior publication exists in the store.
    let store = ModelsStore::open(&path).unwrap();
    store
        .begin()
        .commit(catalog_fixture::catalogs(HashMap::from([(
            "dyn".to_owned(),
            vec![model_info("stored-1", "Stored 1")],
        )])))
        .await
        .unwrap();

    let fetches = Arc::new(AtomicUsize::new(0));
    let auth_resolutions = Arc::new(AtomicUsize::new(0));
    let source_fetches = Arc::new(AtomicUsize::new(0));
    let mut collection = Models::new();
    collection
        .register_route(
            Box::new(CountingRefreshProvider {
                id: "dyn",
                fresh: vec![model_info("fresh-1", "Fresh 1")],
                fetches: fetches.clone(),
            }),
            Arc::new(CountingOAuthResolver(auth_resolutions.clone())),
            AuthProvenanceSource::OAuth {
                kind: "offline-fixture".to_owned(),
            },
            CompatMetadata::default(),
        )
        .unwrap();
    collection.register_catalog_source(Arc::new(CancellingCatalogSource {
        fetches: source_fetches.clone(),
        cancellation: Arc::new(CancelAtCatalogResult::BeforeRefresh),
    }));
    collection.set_models_store(store);

    // Offline refresh: restore the persisted cache, never fetch.
    let report = collection
        .refresh(opi_ai::ModelsRefreshOptions {
            allow_network: false,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(
        report.status(),
        RefreshStatus::OfflineRestored,
        "refresh(false) restores without network"
    );
    assert_eq!(fetches.load(Ordering::SeqCst), 0, "no provider fetch");
    assert_eq!(source_fetches.load(Ordering::SeqCst), 0, "no source fetch");
    assert_eq!(auth_resolutions.load(Ordering::SeqCst), 0, "no OAuth work");
    assert!(collection.resolve("dyn:stored-1").is_ok());
    assert!(
        collection.resolve("dyn:fresh-1").is_err(),
        "offline refresh must not publish unfetched models"
    );
    assert!(matches!(
        report.find("dyn").map(|entry| &entry.outcome),
        Some(RefreshOutcome::Restored { models: 1 })
    ));
}

#[tokio::test]
async fn late_superseded_refresh_cannot_publish_or_rollback() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("models-store.json");

    let mut collection = Models::new();
    let store = ModelsStore::open(&path).unwrap();
    store
        .begin()
        .commit(catalog_fixture::catalogs(HashMap::from([(
            "winner".to_owned(),
            vec![model_info("w1", "W1")],
        )])))
        .await
        .unwrap();
    collection.set_models_store(store.clone());

    // Refresh A hangs inside its fetch; while it hangs, a second publisher
    // commits a newer revision directly through the store.
    let entered = Arc::new(tokio::sync::Notify::new());
    let (release_tx, release_rx) = tokio::sync::oneshot::channel::<()>();
    register(
        &mut collection,
        Box::new(BarrierRefreshProvider::new(
            "racer",
            vec![model_info("racer-fresh", "Racer Fresh")],
            entered.clone(),
            release_rx,
        )),
    );

    let mut refresh_a = collection;
    let task = tokio::spawn(async move { refresh_a.refresh(Default::default()).await.unwrap() });
    entered.notified().await;

    // Publisher B commits revision 2 while A is suspended in fetch.
    store
        .begin()
        .commit(catalog_fixture::catalogs(HashMap::from([(
            "winner".to_owned(),
            vec![model_info("w2", "W2")],
        )])))
        .await
        .unwrap();

    release_tx.send(()).unwrap();
    let report = task.await.unwrap();
    assert_eq!(
        report.status(),
        RefreshStatus::Superseded,
        "the late refresh must be dropped, not published"
    );

    // Store: B's revision wins; A's racer write never lands.
    assert_eq!(store.revision(), 2);
    assert!(
        !store.snapshot().catalogs.contains_key("racer"),
        "superseded write must not persist"
    );

    // Reopened store shows B's publication (no rollback).
    let reopened = ModelsStore::open(&path).unwrap();
    assert_eq!(reopened.revision(), 2);
    assert!(reopened.snapshot().catalogs["winner"].models[0].id() == "w2");

    // A fresh collection restoring from the store sees B's world.
    let mut collection_b = Models::new();
    register(
        &mut collection_b,
        Box::new(RefreshProvider::new("winner", vec![])),
    );
    collection_b.set_models_store(ModelsStore::open(&path).unwrap());
    collection_b
        .refresh(opi_ai::ModelsRefreshOptions {
            allow_network: false,
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(collection_b.resolve("winner:w2").is_ok());
    assert!(collection_b.resolve("racer:racer-fresh").is_err());
}

#[tokio::test]
async fn cancelled_refresh_never_publishes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("models-store.json");
    let mut collection = Models::new();
    collection.set_models_store(ModelsStore::open(&path).unwrap());

    let entered = Arc::new(tokio::sync::Notify::new());
    let (release_tx, release_rx) = tokio::sync::oneshot::channel::<()>();
    register(
        &mut collection,
        Box::new(BarrierRefreshProvider::new(
            "racer",
            vec![model_info("racer-fresh", "Racer Fresh")],
            entered.clone(),
            release_rx,
        )),
    );
    // Hold the sender for the whole test: the fetch stays suspended until
    // cancellation drops it, and a released-late write cannot publish.
    let _hold_release = release_tx;

    let token = CancellationToken::new();
    let task_token = token.clone();
    let task = tokio::spawn(async move {
        let report = collection
            .refresh_with(Default::default(), &task_token)
            .await
            .unwrap();
        (report, collection)
    });
    entered.notified().await;
    token.cancel();
    let (report, collection) = task.await.unwrap();
    assert_eq!(report.status(), RefreshStatus::Cancelled);

    // Nothing persisted, nothing published, and a later release of the
    // suspended fetch cannot publish after cancellation.
    assert!(!path.exists(), "cancelled refresh persists nothing");
    assert!(
        collection.resolve("racer:racer-fresh").is_err(),
        "cancelled work must never appear in the in-memory catalog"
    );
}

#[tokio::test]
async fn custom_catalog_source_publishes_and_retains_failed_sources() {
    let mut collection = Models::new();
    // Catalog sources publish dynamic catalogs for registered providers.
    register(
        &mut collection,
        Box::new(RefreshProvider::new("custom-prov", vec![])),
    );
    collection.register_catalog_source(Arc::new(FixedCatalogSource {
        id: "custom-src".to_owned(),
        providers: vec!["custom-prov".into()],
        catalogs: catalog_fixture::catalogs(HashMap::from([(
            "custom-prov".to_owned(),
            vec![model_info("src-model", "Source Model")],
        )])),
    }));
    collection.register_catalog_source(Arc::new(FailingCatalogSource {
        id: "broken-src".to_owned(),
    }));

    let report = collection.refresh(Default::default()).await.unwrap();
    assert_eq!(report.status(), RefreshStatus::Published);
    assert!(
        collection.resolve("custom-prov:src-model").is_ok(),
        "custom source models publish through refresh"
    );
    assert!(matches!(
        report.find("custom-src").unwrap().outcome,
        RefreshOutcome::Published { models: 1 }
    ));
    assert!(
        report.find("broken-src").unwrap().outcome.is_failure(),
        "failing source reports its error"
    );
}

/// Minimal raw HTTP server returning queued responses, one per request.
async fn spawn_catalog_server(
    responses: Vec<(u16, String)>,
) -> (std::net::SocketAddr, Arc<Mutex<VecDeque<(u16, String)>>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind catalog server");
    let addr = listener.local_addr().expect("catalog server addr");
    let queue = Arc::new(Mutex::new(responses.into_iter().collect::<VecDeque<_>>()));
    let server_queue = queue.clone();
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let mut request = Vec::new();
            let mut buf = [0_u8; 1024];
            loop {
                let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                    .await
                    .expect("read catalog request");
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buf[..read]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            let (code, body) = server_queue
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or((200, "{}".to_owned()));
            let reason = match code {
                200 => "OK",
                500 => "Internal Server Error",
                _ => "Status",
            };
            let response = format!(
                "HTTP/1.1 {code} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            use tokio::io::AsyncWriteExt;
            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.shutdown().await;
        }
    });
    (addr, queue)
}

#[tokio::test]
async fn http_catalog_source_publishes_and_retains_on_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("models-store.json");

    let document = serde_json::json!({
        "providers": {
            "http-prov": {
                "models": [
                    {
                        "id": "s1",
                        "name": "Server One",
                        "api": "openai-completions",
                        "contextWindow": 32_000,
                        "maxTokens": 2_048
                    }
                ]
            }
        }
    })
    .to_string();
    let (addr, _queue) =
        spawn_catalog_server(vec![(200, document), (500, "boom".to_owned())]).await;
    let url = format!("http://127.0.0.1:{}", addr.port());

    let mut collection = Models::new();
    let store = ModelsStore::open(&path).unwrap();
    collection.set_models_store(store);
    // The provider exposes no native listing endpoint (its refresh fails);
    // the HTTP catalog source owns its dynamic catalog, and failures on
    // either side retain prior data.
    register(
        &mut collection,
        Box::new(ErrorRefreshProvider {
            id: "http-prov",
            error_msg: "no native models endpoint",
        }),
    );
    let source = HttpCatalogSource::new(url, ["http-prov".into()]);
    collection.register_catalog_source(Arc::new(source));

    // Round 1: the controlled HTTP source publishes and persists.
    let report = collection.refresh(Default::default()).await.unwrap();
    assert_eq!(report.status(), RefreshStatus::Published);
    assert!(collection.resolve("http-prov:s1").is_ok());
    let persisted = ModelsStore::open(&path).unwrap();
    assert!(
        persisted.snapshot().catalogs.contains_key("http-prov"),
        "published source catalog persists through the store"
    );

    // Round 2: the server fails; prior data is retained, error reported.
    let report = collection.refresh(Default::default()).await.unwrap();
    assert!(
        collection.resolve("http-prov:s1").is_ok(),
        "failed source refresh retains prior valid data"
    );
    assert!(report.find("http-catalog").unwrap().outcome.is_failure());
}

#[tokio::test]
async fn prepared_call_keeps_binding_across_refresh_replacement() {
    let mut collection = Models::new();
    let mut bound = RefreshProvider::new("dyn", vec![model_info("m1", "M1")]);
    bound.events = Mutex::new(Some(
        text_response("bound response")
            .into_iter()
            .map(Ok)
            .collect(),
    ));
    collection
        .register_route(
            Box::new(bound),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                secrecy::SecretString::from("sk-bound"),
            )),
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .expect("register dispatchable route");

    // Prepare against the current catalog, then replace it wholesale.
    let prepared = collection
        .prepare_call("dyn:m1", minimal_request("dyn:m1"))
        .await
        .expect("prepare binds while the model exists");
    collection
        .register_route(
            Box::new(
                RefreshProvider::new("dyn", vec![]).with_refresh(vec![model_info("m2", "M2")]),
            ),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                secrecy::SecretString::from("sk-bound"),
            )),
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .unwrap();
    collection.refresh(Default::default()).await.unwrap();
    assert!(
        collection.resolve("dyn:m1").is_err(),
        "refresh replaced the catalog wholesale"
    );

    // The already-prepared call keeps its old binding and still drains.
    let attempt = prepared.start_attempt().expect("old binding survives");
    let message = drain_to_completion(attempt)
        .await
        .expect("prepared call completes on the frozen binding");
    assert_eq!(assistant_text(&message), "bound response");
}
