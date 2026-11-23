use super::*;
use opi_ai::RefreshStatus;
use opi_ai::catalog::{CatalogRefreshContext, DynamicCatalogSource, ModelsStore, ModelsStoreEntry};
use std::collections::{BTreeSet, HashMap};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32, Ordering},
};

struct Native {
    id: &'static str,
    models: Vec<ModelInfo>,
    seen: Arc<Mutex<Vec<CatalogRefreshContext>>>,
}
impl Provider for Native {
    fn id(&self) -> &str {
        self.id
    }
    fn models(&self) -> &[ModelInfo] {
        &self.models
    }
    fn stream_prepared(&self, _: Request, _: ResolvedAuth) -> opi_ai::provider::EventStream {
        panic!("refresh cannot generate")
    }
    fn refresh_models(
        &self,
        context: CatalogRefreshContext,
    ) -> opi_ai::BoxAuthFuture<'_, Result<Option<Vec<ModelInfo>>, ProviderError>> {
        self.seen.lock().unwrap().push(context);
        Box::pin(async { Ok(None) })
    }
}
struct Source {
    ids: Vec<String>,
    output: Mutex<HashMap<String, ModelsStoreEntry>>,
    seen: Mutex<Vec<CatalogRefreshContext>>,
    fail: AtomicBool,
}
impl Source {
    fn new(ids: &[&str], output: HashMap<String, ModelsStoreEntry>) -> Self {
        Self {
            ids: ids.iter().map(|id| (*id).into()).collect(),
            output: Mutex::new(output),
            seen: Mutex::default(),
            fail: AtomicBool::new(false),
        }
    }
}
impl DynamicCatalogSource for Source {
    fn id(&self) -> &str {
        "aggregate"
    }
    fn provider_ids(&self) -> &[String] {
        &self.ids
    }
    fn fetch(
        &self,
        context: CatalogRefreshContext,
    ) -> futures_util::future::BoxFuture<
        'static,
        Result<HashMap<String, ModelsStoreEntry>, ProviderError>,
    > {
        self.seen.lock().unwrap().push(context);
        let fail = self.fail.load(Ordering::SeqCst);
        let output = self.output.lock().unwrap().clone();
        Box::pin(async move {
            if fail {
                Err(ProviderError::Cancelled)
            } else {
                Ok(output)
            }
        })
    }
}
fn entry(id: &str) -> ModelsStoreEntry {
    catalog_fixture::chat_entry(vec![model_info(id, id)])
}
fn register(models: &mut Models, id: &'static str, seen: Arc<Mutex<Vec<CatalogRefreshContext>>>) {
    models
        .register(
            Box::new(Native {
                id,
                models: vec![model_info("base", "Base")],
                seen,
            }),
            AuthDescriptor::Resolved {
                source: "not consulted by refresh".into(),
            },
            Default::default(),
        )
        .unwrap();
}

#[tokio::test]
async fn selection_precedes_source_work_and_forwards_each_force_value_exactly() {
    for force in [None, Some(false), Some(true)] {
        let mut models = Models::new();
        let a = Arc::new(Mutex::default());
        let b = Arc::new(Mutex::default());
        register(&mut models, "a", a.clone());
        register(&mut models, "b", b.clone());
        models
            .register(
                Box::new(MockProvider::new("static", vec![])),
                AuthDescriptor::Resolved {
                    source: "unused".into(),
                },
                Default::default(),
            )
            .unwrap();
        let source = Arc::new(Source::new(
            &["a", "b"],
            HashMap::from([
                ("a".into(), entry("source-a")),
                ("b".into(), entry("unselected-b")),
            ]),
        ));
        models.register_catalog_source(source.clone());
        for ids in [&[][..], &["unknown"][..]] {
            let report = models
                .refresh(catalog_fixture::refresh_options(ids, true, force))
                .await
                .unwrap();
            assert!(report.is_empty());
        }
        models
            .refresh(catalog_fixture::refresh_options(&["static"], true, force))
            .await
            .unwrap();
        models
            .refresh(catalog_fixture::refresh_options(&["a"], false, force))
            .await
            .unwrap();
        assert!(a.lock().unwrap().is_empty());
        assert!(b.lock().unwrap().is_empty());
        assert!(source.seen.lock().unwrap().is_empty());
        models
            .refresh(catalog_fixture::refresh_options(&["a"], true, force))
            .await
            .unwrap();
        assert!(models.get_model("a", "source-a").is_some());
        assert!(models.get_model("b", "base").is_some());
        assert!(models.get_model("b", "unselected-b").is_none());
        let seen = a.lock().unwrap();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].force, force);
        assert!(seen[0].allow_network);
        assert_eq!(seen[0].providers, BTreeSet::from(["a".into()]));
        assert!(b.lock().unwrap().is_empty());
        let seen = source.seen.lock().unwrap();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].force, force);
        assert!(seen[0].allow_network);
        assert_eq!(seen[0].providers, BTreeSet::from(["a".into()]));
    }
}

#[tokio::test]
async fn subset_restore_and_publication_preserve_shared_unselected_owners_and_tombstones() {
    let temp = tempfile::tempdir().unwrap();
    let store = ModelsStore::open(temp.path().join("models.json")).unwrap();
    let mut original = Models::new();
    for id in ["a", "b"] {
        register(&mut original, id, Arc::default());
    }
    original.set_models_store(store.clone());
    original.register_catalog_source(Arc::new(Source::new(
        &["a", "b"],
        HashMap::from([
            ("a".into(), entry("cached-a")),
            ("b".into(), entry("cached-b")),
        ]),
    )));
    original.refresh(Default::default()).await.unwrap();
    // A publisher unknown to the follower must survive its selected commit.
    let (guard, snapshot) = (store.begin(), store.snapshot());
    let mut complete = snapshot.catalogs;
    complete.insert("foreign".into(), entry("foreign"));
    guard.commit(complete).await.unwrap();
    // Restore source publisher provenance for a/b through their real owner.
    original.refresh(Default::default()).await.unwrap();
    let mut follower = Models::new();
    for id in ["a", "b"] {
        register(&mut follower, id, Arc::default());
    }
    follower.set_models_store(store.clone());
    let bytes = std::fs::read(store.path()).unwrap();
    follower
        .refresh(catalog_fixture::refresh_options(&["a"], false, Some(true)))
        .await
        .unwrap();
    assert!(follower.get_model("a", "cached-a").is_some());
    assert!(follower.get_model("b", "base").is_some());
    follower
        .refresh(catalog_fixture::refresh_options(&["b"], false, None))
        .await
        .unwrap();
    assert!(follower.get_model("b", "cached-b").is_some());
    assert_eq!(std::fs::read(store.path()).unwrap(), bytes);
    assert!(follower.delete_provider("a"));
    assert_eq!(std::fs::read(store.path()).unwrap(), bytes);
    follower
        .refresh(catalog_fixture::refresh_options(&["a"], false, None))
        .await
        .unwrap();
    assert!(follower.get_model("a", "cached-a").is_none());
    register(&mut follower, "a", Arc::default());
    follower
        .refresh(catalog_fixture::refresh_options(&["a"], false, None))
        .await
        .unwrap();
    assert!(follower.get_model("a", "cached-a").is_some());
    let source = Arc::new(Source::new(
        &["a", "b"],
        HashMap::from([
            ("a".into(), entry("new-a")),
            ("b".into(), entry("must-not-publish-b")),
        ]),
    ));
    follower.register_catalog_source(source.clone());
    follower
        .refresh(catalog_fixture::refresh_options(&["a"], true, Some(false)))
        .await
        .unwrap();
    let snapshot = store.snapshot();
    assert_eq!(snapshot.catalogs["a"].models[0].id(), "new-a");
    assert_eq!(snapshot.catalogs["b"].models[0].id(), "cached-b");
    assert_eq!(snapshot.catalogs["foreign"].models[0].id(), "foreign");
    assert!(follower.get_model("b", "cached-b").is_some());
    assert!(original.get_model("a", "cached-a").is_some());
    store.invalidate_source("aggregate").await.unwrap();
    source.fail.store(true, Ordering::SeqCst);
    follower
        .refresh(catalog_fixture::refresh_options(&["a"], true, None))
        .await
        .unwrap();
    assert!(follower.get_model("a", "new-a").is_none());
    assert!(follower.get_model("b", "cached-b").is_some());
    let snapshot = store.snapshot();
    assert!(!snapshot.catalogs.contains_key("a"));
    assert!(!snapshot.catalogs.contains_key("b"));
    assert!(snapshot.catalogs.contains_key("foreign"));
    follower
        .refresh(catalog_fixture::refresh_options(&["b"], false, None))
        .await
        .unwrap();
    assert!(follower.get_model("b", "cached-b").is_none());
    assert!(follower.get_model("b", "base").is_some());
}

#[tokio::test]
async fn undeclared_source_output_rejects_its_selected_candidate_without_partial_publication() {
    let mut models = Models::new();
    register(&mut models, "a", Arc::default());
    register(&mut models, "b", Arc::default());
    let source = Arc::new(Source::new(
        &["a", "b"],
        HashMap::from([("a".into(), entry("known"))]),
    ));
    models.register_catalog_source(source.clone());
    models
        .refresh(catalog_fixture::refresh_options(&["a"], true, None))
        .await
        .unwrap();
    *source.output.lock().unwrap() = HashMap::from([
        ("a".into(), entry("candidate")),
        ("undeclared".into(), entry("poison")),
    ]);
    let report = models
        .refresh(catalog_fixture::refresh_options(&["a"], true, None))
        .await
        .unwrap();
    assert!(report.find("aggregate").unwrap().outcome.is_failure());
    assert!(models.get_model("a", "known").is_some());
    assert!(models.get_model("a", "candidate").is_none());
    assert!(models.get_model("b", "base").is_some());
    let token = CancellationToken::new();
    token.cancel();
    let before = source.seen.lock().unwrap().len();
    assert_eq!(
        models
            .refresh_with(
                catalog_fixture::refresh_options(&["a"], true, Some(true)),
                &token
            )
            .await
            .unwrap()
            .status(),
        RefreshStatus::Cancelled
    );
    assert_eq!(source.seen.lock().unwrap().len(), before);
}

fn mapped(models: &mut Models, wires: &[WireApi]) {
    let catalog: Vec<_> = wires
        .iter()
        .map(|wire| {
            ModelInfo::new(
                wire.as_str(),
                wire.as_str(),
                *wire,
                ModelCapabilities::new(1000, 100),
            )
        })
        .collect();
    let routes: Vec<_> = wires
        .iter()
        .map(|wire| {
            (
                *wire,
                Box::new(MockProvider::new_with_models(
                    "mapped",
                    catalog
                        .iter()
                        .filter(|model| model.wire_api == *wire)
                        .cloned()
                        .collect(),
                    vec![],
                )) as Box<dyn Provider>,
            )
        })
        .collect();
    models
        .register(
            Box::new(
                opi_ai::providers::mapped::ApiMappedProvider::try_new("mapped", catalog, routes)
                    .unwrap(),
            ),
            AuthDescriptor::Resolved {
                source: "unused".into(),
            },
            Default::default(),
        )
        .unwrap();
}
fn responses_entry() -> ModelsStoreEntry {
    catalog_fixture::chat_entry(vec![ModelInfo::new(
        "response-cached",
        "Response",
        WireApi::OpenAiResponses,
        ModelCapabilities::new(1000, 100),
    )])
}
#[tokio::test]
async fn restored_chat_requires_a_current_mapped_wire_without_using_seed_membership() {
    let temp = tempfile::tempdir().unwrap();
    let store = ModelsStore::open(temp.path().join("models.json")).unwrap();
    store
        .begin()
        .commit(HashMap::from([("mapped".into(), responses_entry())]))
        .await
        .unwrap();
    let bytes = std::fs::read(store.path()).unwrap();
    let mut models = Models::new();
    mapped(
        &mut models,
        &[WireApi::OpenAiCompletions, WireApi::OpenAiResponses],
    );
    models.set_models_store(store.clone());
    models
        .refresh(catalog_fixture::refresh_options(&["mapped"], false, None))
        .await
        .unwrap();
    assert!(
        models.get_model("mapped", "response-cached").is_some(),
        "new ID on an owned mapped wire is valid"
    );
    assert!(models.delete_provider("mapped"));
    mapped(&mut models, &[WireApi::OpenAiCompletions]);
    models
        .refresh(catalog_fixture::refresh_options(&["mapped"], false, None))
        .await
        .unwrap();
    assert!(
        models.get_model("mapped", "response-cached").is_none(),
        "retired Responses route cannot expose stale cached record"
    );
    assert!(
        models
            .get_model("mapped", WireApi::OpenAiCompletions.as_str())
            .is_some(),
        "incompatible nonempty cache does not manufacture an empty publication"
    );
    assert_eq!(std::fs::read(store.path()).unwrap(), bytes);
}
#[tokio::test]
async fn source_chat_requires_a_current_mapped_wire_before_publication() {
    let mut models = Models::new();
    mapped(&mut models, &[WireApi::OpenAiCompletions]);
    let source = Arc::new(Source::new(
        &["mapped"],
        HashMap::from([("mapped".into(), entry("known-chat"))]),
    ));
    models.register_catalog_source(source.clone());
    models
        .refresh(catalog_fixture::refresh_options(&["mapped"], true, None))
        .await
        .unwrap();
    assert!(models.get_model("mapped", "known-chat").is_some());
    *source.output.lock().unwrap() = HashMap::from([("mapped".into(), responses_entry())]);
    let report = models
        .refresh(catalog_fixture::refresh_options(&["mapped"], true, None))
        .await
        .unwrap();
    assert!(
        report.find("aggregate").unwrap().outcome.is_failure(),
        "missing mapped route must reject source candidate"
    );
    assert!(models.get_model("mapped", "known-chat").is_some());
    assert!(models.get_model("mapped", "response-cached").is_none());
}

struct CleanupOwner(Arc<AtomicU32>);
impl opi_ai::codex_resources::SessionResourceOwner for CleanupOwner {
    fn new_namespace(&self) -> Arc<dyn opi_ai::codex_resources::SessionResourceOwner> {
        Arc::new(Self(self.0.clone()))
    }
    fn cleanup_session_resources(&self, _: Option<&str>) -> opi_ai::codex_resources::CleanupHandle {
        let calls = self.0.clone();
        opi_ai::codex_resources::CleanupHandle::from_future(async move {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(Default::default())
        })
    }
}
struct ResourceProvider {
    inner: MockProvider,
    owner: CleanupOwner,
}
impl Provider for ResourceProvider {
    fn id(&self) -> &str {
        self.inner.id()
    }
    fn models(&self) -> &[ModelInfo] {
        self.inner.models()
    }
    fn stream_prepared(
        &self,
        request: Request,
        auth: ResolvedAuth,
    ) -> opi_ai::provider::EventStream {
        self.inner.stream_prepared(request, auth)
    }
    fn session_resources(&self) -> Option<&dyn opi_ai::codex_resources::SessionResourceOwner> {
        Some(&self.owner)
    }
}
#[tokio::test]
async fn removal_and_clear_retain_retired_resource_namespaces_until_explicit_cleanup() {
    let mut models = Models::new();
    let old = Arc::new(AtomicU32::new(0));
    let current = Arc::new(AtomicU32::new(0));
    for (counter, remove) in [(old.clone(), true), (current.clone(), false)] {
        models
            .register(
                Box::new(ResourceProvider {
                    inner: MockProvider::new("resources", vec![]),
                    owner: CleanupOwner(counter),
                }),
                AuthDescriptor::Resolved {
                    source: "unused".into(),
                },
                Default::default(),
            )
            .unwrap();
        if remove {
            assert!(models.delete_provider("resources"));
        } else {
            models.clear_providers();
        }
    }
    assert!(models.provider_ids().is_empty());
    assert_eq!(old.load(Ordering::SeqCst), 0);
    assert_eq!(current.load(Ordering::SeqCst), 0);
    assert!(models.cleanup_session_resources(Some("")).await.is_err());
    assert_eq!(old.load(Ordering::SeqCst), 0);
    assert_eq!(current.load(Ordering::SeqCst), 0);
    models.cleanup_session_resources(None).await.unwrap();
    assert_eq!(old.load(Ordering::SeqCst), 1);
    assert_eq!(current.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn removal_and_reregistration_keep_old_chat_captures_and_other_instances_separate() {
    let count = Arc::new(AtomicU32::new(0));
    let mut models = Models::new();
    let model = model_info("chat", "Chat");
    models
        .register_route(
            Box::new(MockProvider::new_with_models(
                "managed",
                vec![model.clone()],
                vec![text_response("old")],
            )),
            Arc::new(opi_ai::test_support::CountingAuthResolver::new(
                count.clone(),
            )),
            AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    let captured = models.complete_simple(
        opi_ai::ChatModelRecord::new("managed", model.clone()),
        minimal_request("managed:chat"),
        Default::default(),
    );
    assert_eq!(count.load(Ordering::SeqCst), 0);
    assert!(!models.delete_provider("missing"));
    assert!(models.delete_provider("managed"));
    let new = models.complete_simple(
        opi_ai::ChatModelRecord::new("managed", model.clone()),
        minimal_request("managed:chat"),
        Default::default(),
    );
    assert!(new.await.is_err());
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while count.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("captured execution resolves its original auth");
    assert_eq!(
        count.load(Ordering::SeqCst),
        1,
        "only the original captured call reaches auth"
    );
    let mut other = Models::new();
    register(&mut other, "managed", Arc::default());
    models
        .register_route(
            Box::new(MockProvider::new_with_models(
                "managed",
                vec![model.clone()],
                vec![text_response("replacement")],
            )),
            Arc::new(opi_ai::test_support::CountingAuthResolver::new(
                count.clone(),
            )),
            AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    assert_eq!(assistant_text(&captured.await.unwrap()), "old");
    assert_eq!(
        assistant_text(
            &models
                .complete_simple(
                    opi_ai::ChatModelRecord::new("managed", model),
                    minimal_request("managed:chat"),
                    Default::default()
                )
                .await
                .unwrap()
        ),
        "replacement"
    );
    assert_eq!(count.load(Ordering::SeqCst), 2);
    models.clear_providers();
    assert!(models.provider_ids().is_empty());
    assert!(other.get_model("managed", "base").is_some());
}
