//! Catalog publication ownership, persisted restore, and source failures.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures_util::future::BoxFuture;
use opi_ai::auth::{AuthScheme, ResolvedAuth, StaticAuthResolver};
use opi_ai::catalog::{DynamicCatalogSource, ModelsStore};
use opi_ai::provider::{EventStream, Provider, ProviderError, Request};
use opi_ai::{AuthProvenanceSource, ModelCapabilities, ModelInfo, Models, WireApi};
use secrecy::SecretString;

#[path = "../support/catalog_store.rs"]
mod catalog_fixture;

struct SuccessThenFailureSource {
    calls: AtomicUsize,
}

impl DynamicCatalogSource for SuccessThenFailureSource {
    fn id(&self) -> &str {
        "explicit-source"
    }
    fn provider_ids(&self) -> &[String] {
        static IDS: std::sync::LazyLock<Vec<String>> =
            std::sync::LazyLock::new(|| vec!["openai".into()]);
        &IDS
    }
    fn fetch(
        &self,
        _context: opi_ai::catalog::CatalogRefreshContext,
    ) -> BoxFuture<'static, Result<HashMap<String, opi_ai::catalog::ModelsStoreEntry>, ProviderError>>
    {
        let first = self.calls.fetch_add(1, Ordering::SeqCst) == 0;
        Box::pin(async move {
            if first {
                Ok(catalog_fixture::catalogs(HashMap::from([(
                    "openai".into(),
                    vec![ModelInfo::new(
                        "source-only",
                        "Published source model",
                        WireApi::OpenAiCompletions,
                        ModelCapabilities::new(10_000, 1_000),
                    )],
                )])))
            } else {
                Err(ProviderError::ProviderSide(
                    opi_ai::provider::ProviderErrorSummary::from_untrusted("source unavailable"),
                ))
            }
        })
    }
}

#[tokio::test]
async fn failed_explicit_source_retains_catalog_when_static_provider_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let store_path = dir.path().join("catalog.json");
    let mut models = Models::new();
    models
        .register_route(
            Box::new(opi_ai::providers::openai::OpenAiChatProvider::new(None)),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("fake-key"),
            )),
            AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    models.set_models_store(ModelsStore::open(&store_path).unwrap());
    models.register_catalog_source(Arc::new(SuccessThenFailureSource {
        calls: AtomicUsize::new(0),
    }));
    models.refresh(Default::default()).await.unwrap();
    assert!(models.resolve("openai:source-only").is_ok());
    let report = models.refresh(Default::default()).await.unwrap();
    assert!(report.find("explicit-source").unwrap().outcome.is_failure());
    assert!(
        models.resolve("openai:source-only").is_ok(),
        "failed source retains memory revision"
    );
    let restored = ModelsStore::open(&store_path).unwrap().snapshot();
    assert_eq!(restored.catalogs["openai"].models[0].id(), "source-only");
}

struct AuthoritativeEmptyProvider {
    inner: opi_ai::providers::openai::OpenAiChatProvider,
    rounds: AtomicUsize,
}

impl Provider for AuthoritativeEmptyProvider {
    fn id(&self) -> &str {
        self.inner.id()
    }
    fn models(&self) -> &[ModelInfo] {
        self.inner.models()
    }
    fn stream_prepared(&self, request: Request, auth: ResolvedAuth) -> EventStream {
        self.inner.stream_prepared(request, auth)
    }
    fn refresh_models(
        &self,
        _context: opi_ai::catalog::CatalogRefreshContext,
    ) -> opi_ai::credential::BoxAuthFuture<'_, Result<Option<Vec<ModelInfo>>, ProviderError>> {
        let first = self.rounds.fetch_add(1, Ordering::SeqCst) == 0;
        Box::pin(async move { Ok(if first { None } else { Some(vec![]) }) })
    }
}

#[tokio::test]
async fn explicit_source_failure_does_not_revoke_a_providers_authoritative_empty() {
    let dir = tempfile::tempdir().unwrap();
    let store_path = dir.path().join("catalog.json");
    let mut models = Models::new();
    models
        .register_route(
            Box::new(AuthoritativeEmptyProvider {
                inner: opi_ai::providers::openai::OpenAiChatProvider::new(None),
                rounds: AtomicUsize::new(0),
            }),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("fake-key"),
            )),
            AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    models.set_models_store(ModelsStore::open(&store_path).unwrap());
    models.register_catalog_source(Arc::new(SuccessThenFailureSource {
        calls: AtomicUsize::new(0),
    }));
    models.refresh(Default::default()).await.unwrap();
    assert!(models.resolve("openai:source-only").is_ok());
    models.refresh(Default::default()).await.unwrap();
    assert!(models.resolve("openai:source-only").is_err());
    assert!(models.resolve("openai:gpt-4o").is_err());
    let snapshot = ModelsStore::open(&store_path).unwrap().snapshot();
    assert!(snapshot.catalogs["openai"].models.is_empty());
}

struct NativeThenAbsentProvider {
    id: &'static str,
    calls: AtomicUsize,
}

impl Provider for NativeThenAbsentProvider {
    fn id(&self) -> &str {
        self.id
    }
    fn models(&self) -> &[ModelInfo] {
        &[]
    }
    fn stream_prepared(&self, _: Request, _: ResolvedAuth) -> EventStream {
        unreachable!("catalog-only provider")
    }
    fn refresh_models(
        &self,
        _context: opi_ai::catalog::CatalogRefreshContext,
    ) -> opi_ai::BoxAuthFuture<'_, Result<Option<Vec<ModelInfo>>, ProviderError>> {
        let publish = self.calls.fetch_add(1, Ordering::SeqCst) == 0;
        Box::pin(async move {
            Ok(publish.then(|| {
                vec![ModelInfo::new(
                    "native-old",
                    "Native old",
                    WireApi::OpenAiCompletions,
                    ModelCapabilities::new(10_000, 1_000),
                )]
            }))
        })
    }
}

struct IndependentSource {
    fail: bool,
}

impl DynamicCatalogSource for IndependentSource {
    fn id(&self) -> &str {
        "source-b"
    }
    fn provider_ids(&self) -> &[String] {
        static IDS: std::sync::LazyLock<Vec<String>> =
            std::sync::LazyLock::new(|| vec!["provider-b".into()]);
        &IDS
    }
    fn fetch(
        &self,
        _: opi_ai::catalog::CatalogRefreshContext,
    ) -> BoxFuture<'static, Result<HashMap<String, opi_ai::catalog::ModelsStoreEntry>, ProviderError>>
    {
        let fail = self.fail;
        Box::pin(async move {
            if fail {
                Err(ProviderError::Network(
                    opi_ai::provider::ProviderErrorSummary::from_untrusted("unavailable"),
                ))
            } else {
                Ok(catalog_fixture::catalogs(HashMap::from([(
                    "provider-b".into(),
                    vec![ModelInfo::new(
                        "source-b-model",
                        "Source B",
                        WireApi::OpenAiCompletions,
                        ModelCapabilities::new(10_000, 1_000),
                    )],
                )])))
            }
        })
    }
}

fn register_native_catalog_provider(
    models: &mut Models,
    id: &'static str,
    already_published: bool,
) {
    models
        .register_route(
            Box::new(NativeThenAbsentProvider {
                id,
                calls: AtomicUsize::new(usize::from(already_published)),
            }),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("fake-key"),
            )),
            AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
}

#[tokio::test]
async fn unrelated_explicit_source_does_not_block_native_none_clear() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("catalog.json");
    let mut models = Models::new();
    register_native_catalog_provider(&mut models, "provider-a", false);
    register_native_catalog_provider(&mut models, "provider-b", true);
    models.set_models_store(ModelsStore::open(&path).unwrap());
    models.register_catalog_source(Arc::new(IndependentSource { fail: false }));
    models.refresh(Default::default()).await.unwrap();
    assert!(models.resolve("provider-a:native-old").is_ok());
    assert!(models.resolve("provider-b:source-b-model").is_ok());
    let persisted: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(
        persisted["providers"]["provider-a"]["x-opi"]["publisher"],
        serde_json::json!({"kind":"native"})
    );
    assert_eq!(
        persisted["providers"]["provider-b"]["x-opi"]["publisher"],
        serde_json::json!({"kind":"source","id":"source-b"})
    );
    models.refresh(Default::default()).await.unwrap();
    assert!(
        models.resolve("provider-a:native-old").is_err(),
        "source B cannot retain native A's revoked snapshot"
    );
    assert!(models.resolve("provider-b:source-b-model").is_ok());
    assert!(
        !ModelsStore::open(&path)
            .unwrap()
            .snapshot()
            .catalogs
            .contains_key("provider-a")
    );
}

#[tokio::test]
async fn restored_native_catalog_clears_despite_unrelated_source_failure() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("catalog.json");
    let mut first = Models::new();
    register_native_catalog_provider(&mut first, "provider-a", false);
    register_native_catalog_provider(&mut first, "provider-b", true);
    first.set_models_store(ModelsStore::open(&path).unwrap());
    first.register_catalog_source(Arc::new(IndependentSource { fail: false }));
    first.refresh(Default::default()).await.unwrap();
    drop(first);

    let mut restored = Models::new();
    register_native_catalog_provider(&mut restored, "provider-a", true);
    register_native_catalog_provider(&mut restored, "provider-b", true);
    restored.set_models_store(ModelsStore::open(&path).unwrap());
    restored.register_catalog_source(Arc::new(IndependentSource { fail: true }));
    restored
        .refresh(opi_ai::ModelsRefreshOptions {
            allow_network: false,
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(restored.resolve("provider-a:native-old").is_ok());
    assert!(restored.resolve("provider-b:source-b-model").is_ok());
    restored.refresh(Default::default()).await.unwrap();
    assert!(restored.resolve("provider-a:native-old").is_err());
    assert!(
        restored.resolve("provider-b:source-b-model").is_ok(),
        "source B's persisted publication keeps its owner"
    );
    let snapshot = ModelsStore::open(&path).unwrap().snapshot();
    assert!(!snapshot.catalogs.contains_key("provider-a"));
    assert_eq!(
        snapshot.catalogs["provider-b"].models[0].id(),
        "source-b-model"
    );
}

#[tokio::test]
async fn restored_source_catalog_retains_on_same_provider_failure() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("catalog.json");
    let mut first = Models::new();
    register_native_catalog_provider(&mut first, "openai", true);
    first.set_models_store(ModelsStore::open(&path).unwrap());
    first.register_catalog_source(Arc::new(SuccessThenFailureSource {
        calls: AtomicUsize::new(0),
    }));
    first.refresh(Default::default()).await.unwrap();
    drop(first);

    let mut restored = Models::new();
    register_native_catalog_provider(&mut restored, "openai", true);
    restored.set_models_store(ModelsStore::open(&path).unwrap());
    restored.register_catalog_source(Arc::new(SuccessThenFailureSource {
        calls: AtomicUsize::new(1),
    }));
    restored.refresh(Default::default()).await.unwrap();
    assert!(restored.resolve("openai:source-only").is_ok());
    assert_eq!(
        ModelsStore::open(&path).unwrap().snapshot().catalogs["openai"].models[0].id(),
        "source-only"
    );
}

#[tokio::test]
async fn legacy_catalog_has_unknown_publisher_and_malformed_publisher_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("catalog.json");
    let store = ModelsStore::open(&path).unwrap();
    store
        .begin()
        .commit(catalog_fixture::catalogs(HashMap::from([(
            "provider-a".into(),
            vec![ModelInfo::new(
                "legacy",
                "Legacy",
                WireApi::OpenAiCompletions,
                ModelCapabilities::new(10_000, 1_000),
            )],
        )])))
        .await
        .unwrap();
    let mut persisted: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert!(persisted["providers"]["provider-a"].get("x-opi").is_none());
    let mut models = Models::new();
    register_native_catalog_provider(&mut models, "provider-a", true);
    models.set_models_store(ModelsStore::open(&path).unwrap());
    models.register_catalog_source(Arc::new(IndependentSource { fail: true }));
    models
        .refresh(opi_ai::ModelsRefreshOptions {
            allow_network: false,
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(models.resolve("provider-a:legacy").is_ok());
    models.refresh(Default::default()).await.unwrap();
    assert!(
        models.resolve("provider-a:legacy").is_err(),
        "unknown ownership is not attributed to an unrelated source"
    );
    for malformed in [
        serde_json::json!(false),
        serde_json::json!({"kind":"source","id":0}),
        serde_json::json!({"kind":"unexpected"}),
    ] {
        persisted["providers"]["provider-a"]["x-opi"] = serde_json::json!({"publisher":malformed});
        std::fs::write(&path, serde_json::to_vec(&persisted).unwrap()).unwrap();
        assert!(matches!(
            ModelsStore::open(&path),
            Err(opi_ai::catalog::ModelsStoreError::Corrupt { .. })
        ));
    }
}
