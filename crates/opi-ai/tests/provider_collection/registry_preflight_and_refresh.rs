//! Collection registry lookup, preflight and in-memory refresh.

use super::minimal_request as request;
use super::{ErrorRefreshProvider, RefreshProvider, model_info, text_mock};
use std::sync::{Arc, Mutex};

use opi_ai::auth::ResolvedAuth;
use opi_ai::credential::BoxAuthFuture;
use opi_ai::model_info::ThinkingLevel;
use opi_ai::models::{AuthDescriptor, CompatMetadata, Models, RefreshOutcome, SecretKey};
use opi_ai::provider::{Provider, ProviderError, Request, ThinkingConfig};
use opi_ai::registry::ProviderRegistry;
use opi_ai::{ModelCapabilities, ModelInfo, WireApi};

struct CountingRoute {
    model: Vec<ModelInfo>,
    calls: Arc<std::sync::atomic::AtomicUsize>,
}

impl Provider for CountingRoute {
    fn id(&self) -> &str {
        "mapped"
    }

    fn models(&self) -> &[ModelInfo] {
        &self.model
    }

    fn stream_prepared(
        &self,
        _request: Request,
        _auth: opi_ai::auth::ResolvedAuth,
    ) -> opi_ai::provider::EventStream {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Box::pin(futures_util::stream::empty())
    }

    fn replace_model_catalog(&mut self, models: Vec<ModelInfo>) -> Result<(), ProviderError> {
        self.model = models;
        Ok(())
    }
}

fn unsupported_xhigh_model() -> ModelInfo {
    ModelInfo::new(
        "reasoning",
        "Reasoning",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(1000, 100).with_thinking(true),
    )
}
#[tokio::test]
async fn collection_preflights_registry_model_before_provider_dispatch() {
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut collection = Models::new();
    collection
        .register(
            Box::new(CountingRoute {
                model: vec![unsupported_xhigh_model()],
                calls: calls.clone(),
            }),
            AuthDescriptor::StaticApiKey {
                value: SecretKey::new("unused"),
            },
            CompatMetadata::default(),
        )
        .unwrap();
    let mut request = request("mapped:reasoning");
    request.thinking = ThinkingConfig {
        enabled: true,
        budget_tokens: None,
        level: ThinkingLevel::XHigh,
    };

    let error = collection
        .prepare_call("mapped:reasoning", request)
        .await
        .expect_err("unsupported thinking must fail");

    assert!(error.to_string().contains("unsupported"));
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
}
struct LoggedRefreshProvider {
    id: &'static str,
    log: Arc<Mutex<Vec<String>>>,
    result: Result<Option<Vec<ModelInfo>>, &'static str>,
}

impl Provider for LoggedRefreshProvider {
    fn id(&self) -> &str {
        self.id
    }

    fn models(&self) -> &[ModelInfo] {
        &[]
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
        self.log.lock().unwrap().push(self.id.into());
        let result = self.result.clone();
        Box::pin(async move {
            result.map_err(|message| {
                ProviderError::ProviderSide(opi_ai::provider::ProviderErrorSummary::from_untrusted(
                    message,
                ))
            })
        })
    }
}
#[tokio::test]
async fn refresh_collects_every_provider_after_the_first_error() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut collection = Models::new();
    for provider in [
        LoggedRefreshProvider {
            id: "a-error",
            log: log.clone(),
            result: Err("first"),
        },
        LoggedRefreshProvider {
            id: "b-success",
            log: log.clone(),
            result: Ok(Some(vec![ModelInfo::new(
                "fresh",
                "Fresh",
                WireApi::OpenAiCompletions,
                ModelCapabilities::new(1000, 100),
            )])),
        },
        LoggedRefreshProvider {
            id: "c-error",
            log: log.clone(),
            result: Err("later"),
        },
    ] {
        collection
            .register(
                Box::new(provider),
                AuthDescriptor::StaticApiKey {
                    value: SecretKey::new("unused"),
                },
                CompatMetadata::default(),
            )
            .unwrap();
    }

    // Refresh reports per-provider outcomes and still evaluates every
    // provider in deterministic (sorted) id order after an error.
    let report = collection.refresh(Default::default()).await.unwrap();
    assert_eq!(
        *log.lock().unwrap(),
        ["a-error", "b-success", "c-error"],
        "refresh must call every provider in sorted order"
    );
    // Successful providers publish despite sibling failures.
    assert!(collection.resolve("b-success:fresh").is_ok());
    assert!(report.find("a-error").unwrap().outcome.is_failure());
    assert!(report.find("c-error").unwrap().outcome.is_failure());
    assert!(matches!(
        report.find("b-success").unwrap().outcome,
        RefreshOutcome::Published { models: 1 }
    ));
}

#[tokio::test]
async fn collection_refresh_is_a_documented_noop_extension_point() {
    let mut collection = Models::new();
    // Offline refresh on a store-less collection performs no network work and
    // reports an empty outcome (no pi.dev/default startup policy exists).
    let report = collection
        .refresh(opi_ai::ModelsRefreshOptions {
            allow_network: false,
            ..Default::default()
        })
        .await;
    assert!(report.is_ok());
}

#[test]
fn collection_wraps_existing_registry_via_from_registry() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(text_mock("wrapped", "wrapped response"))
        .unwrap();

    let collection = Models::from_registry(registry);
    // Underlying registry is accessible for list-models / overrides.
    assert_eq!(collection.registry().provider_ids(), vec!["wrapped"]);
    // Model lookup flows through the wrapped registry.
    let (provider, _) = collection.resolve("wrapped:mock-model").unwrap();
    assert_eq!(provider.id(), "wrapped");
    // Auth descriptor defaults to absent for pre-registered providers, and the
    // on-demand check reports them unconfigured.
    assert!(collection.auth_descriptor("wrapped").is_none());
}

/// Shared mutable refresh catalog behind an `Arc<Mutex<>>` so tests can
/// change the result between refresh calls.
struct MutableRefreshProvider {
    id: &'static str,
    builtin_models: Vec<ModelInfo>,
    refresh_result: Arc<std::sync::Mutex<Option<Vec<ModelInfo>>>>,
}

impl MutableRefreshProvider {
    fn new(
        id: &'static str,
        builtin_models: Vec<ModelInfo>,
        initial_refresh: Vec<ModelInfo>,
    ) -> Self {
        Self {
            id,
            builtin_models,
            refresh_result: Arc::new(std::sync::Mutex::new(Some(initial_refresh))),
        }
    }
}

impl Provider for MutableRefreshProvider {
    fn id(&self) -> &str {
        self.id
    }

    fn models(&self) -> &[ModelInfo] {
        // Leak is intentional and bounded — test-only.
        Box::leak(self.builtin_models.clone().into_boxed_slice())
    }

    fn stream_prepared(
        &self,
        _request: Request,
        _auth: ResolvedAuth,
    ) -> opi_ai::provider::EventStream {
        Box::pin(futures_util::stream::empty())
    }

    fn refresh_models(
        &self,
        _context: opi_ai::CatalogRefreshContext,
    ) -> opi_ai::credential::BoxAuthFuture<
        '_,
        Result<Option<Vec<ModelInfo>>, opi_ai::provider::ProviderError>,
    > {
        let result = self.refresh_result.lock().unwrap().clone();
        Box::pin(async move { Ok(result) })
    }
}

#[tokio::test]
async fn refresh_models_is_atomic_substrate() {
    // Mixed static + dynamic providers.
    let mut collection = Models::new();

    // Static provider — default refresh_models returns Ok(None).
    let static_prov = text_mock("staticprov", "static");
    collection
        .register(
            static_prov,
            AuthDescriptor::StaticApiKey {
                value: SecretKey::new("key"),
            },
            CompatMetadata::default(),
        )
        .unwrap();

    // Dynamic provider A — refresh returns a new catalog.
    let dyn_a =
        RefreshProvider::new("dyna", vec![model_info("old-a", "Old A")]).with_refresh(vec![
            model_info("fresh-a1", "Fresh A1"),
            model_info("fresh-a2", "Fresh A2"),
        ]);
    collection
        .register(
            Box::new(dyn_a),
            AuthDescriptor::StaticApiKey {
                value: SecretKey::new("key"),
            },
            CompatMetadata::default(),
        )
        .unwrap();

    // Dynamic provider B — refresh returns a different catalog.
    let dyn_b = RefreshProvider::new("dynb", vec![model_info("old-b", "Old B")])
        .with_refresh(vec![model_info("fresh-b", "Fresh B")]);
    collection
        .register(
            Box::new(dyn_b),
            AuthDescriptor::StaticApiKey {
                value: SecretKey::new("key"),
            },
            CompatMetadata::default(),
        )
        .unwrap();

    // Before refresh: built-in models are visible.
    assert!(collection.resolve("dyna:old-a").is_ok());
    assert!(collection.resolve("dynb:old-b").is_ok());
    // Fresh models not yet visible.
    assert!(collection.resolve("dyna:fresh-a1").is_err());

    // Refresh succeeds.
    collection.refresh(Default::default()).await.unwrap();

    // After refresh: dynamic catalogs replace built-in models.
    // Old models gone, fresh models visible.
    assert!(
        collection.resolve("dyna:old-a").is_err(),
        "old built-in model should be replaced by dynamic catalog"
    );
    let (prov_a1, model_a1) = collection.resolve("dyna:fresh-a1").unwrap();
    assert_eq!(prov_a1.id(), "dyna");
    assert_eq!(model_a1.id, "fresh-a1");
    assert!(collection.resolve("dyna:fresh-a2").is_ok());

    let (prov_b, model_b) = collection.resolve("dynb:fresh-b").unwrap();
    assert_eq!(prov_b.id(), "dynb");
    assert_eq!(model_b.id, "fresh-b");

    // Static provider is unaffected.
    assert!(collection.resolve("staticprov:mock-model").is_ok());

    // all_models includes dynamic catalogs.
    let all: Vec<(&str, &str)> = collection
        .registry()
        .all_models()
        .into_iter()
        .map(|(pid, m)| (pid, m.id.as_str()))
        .collect();
    // Dynamic provider A: old-a gone, fresh-a1 + fresh-a2 present.
    assert!(!all.contains(&("dyna", "old-a")));
    assert!(all.contains(&("dyna", "fresh-a1")));
    assert!(all.contains(&("dyna", "fresh-a2")));
    // Dynamic provider B: old-b gone, fresh-b present.
    assert!(!all.contains(&("dynb", "old-b")));
    assert!(all.contains(&("dynb", "fresh-b")));
}

#[tokio::test]
async fn refresh_models_reports_per_provider_and_publishes_successes() {
    const STATIC_PROVIDER_CANARY: &str = "static-refresh-provider-canary";
    let mut collection = Models::new();

    // Dynamic provider that succeeds on refresh.
    let good =
        RefreshProvider::new("good", vec![]).with_refresh(vec![model_info("good-model", "Good")]);
    collection
        .register(
            Box::new(good),
            AuthDescriptor::StaticApiKey {
                value: SecretKey::new("key"),
            },
            CompatMetadata::default(),
        )
        .unwrap();

    // Dynamic provider that fails on refresh.
    collection
        .register(
            Box::new(ErrorRefreshProvider {
                id: "bad",
                error_msg: STATIC_PROVIDER_CANARY,
            }),
            AuthDescriptor::StaticApiKey {
                value: SecretKey::new("key"),
            },
            CompatMetadata::default(),
        )
        .unwrap();

    // Verify initial state: no models yet (empty built-in).
    assert!(collection.resolve("good:good-model").is_err());

    // Refresh reports per-provider outcomes: a failing provider retains its
    // prior data and does not erase other providers' fresh catalogs.
    let report = collection.refresh(Default::default()).await.unwrap();
    let good_entry = report
        .find("good")
        .expect("good provider appears in the refresh report");
    assert_eq!(good_entry.outcome, RefreshOutcome::Published { models: 1 });
    let bad_entry = report
        .find("bad")
        .expect("bad provider appears in the refresh report");
    let RefreshOutcome::Failed(bad_error) = &bad_entry.outcome else {
        panic!(
            "bad provider must report its error, got {:?}",
            bad_entry.outcome
        )
    };
    assert_eq!(bad_error.to_string(), "provider error: [REDACTED]");
    assert!(!format!("{bad_error:?}").contains(STATIC_PROVIDER_CANARY));

    // Per-provider isolation: good's fresh catalog IS published despite bad.
    assert!(
        collection.resolve("good:good-model").is_ok(),
        "good's fresh catalog must publish even when bad fails"
    );
}

#[tokio::test]
async fn refresh_models_deterministic_ordering() {
    // Refresh evaluates providers in deterministic (sorted) id order.
    // Test: the result is the same regardless of registration order.
    let mut collection = Models::new();

    // Register in non-alphabetical order.
    for id in ["zulu", "alpha", "mike"] {
        let prov = RefreshProvider::new(id, vec![])
            .with_refresh(vec![model_info(&format!("{id}-refreshed"), "refreshed")]);
        collection
            .register(
                Box::new(prov),
                AuthDescriptor::StaticApiKey {
                    value: SecretKey::new("key"),
                },
                CompatMetadata::default(),
            )
            .unwrap();
    }

    collection.refresh(Default::default()).await.unwrap();

    // Provider ids are exposed in deterministic sorted order regardless of
    // registration order, so a regression to insertion order would be caught.
    assert_eq!(
        collection.provider_ids(),
        vec!["alpha", "mike", "zulu"],
        "provider ids must be sorted regardless of registration order"
    );

    // All refreshed models are present.
    assert!(collection.resolve("alpha:alpha-refreshed").is_ok());
    assert!(collection.resolve("mike:mike-refreshed").is_ok());
    assert!(collection.resolve("zulu:zulu-refreshed").is_ok());
}

#[tokio::test]
async fn refresh_models_repeated_refresh_replaces() {
    let mut collection = Models::new();

    let prov = MutableRefreshProvider::new("dyn", vec![], vec![model_info("v1", "Version 1")]);
    let refresh_result = Arc::clone(&prov.refresh_result);
    collection
        .register(
            Box::new(prov),
            AuthDescriptor::StaticApiKey {
                value: SecretKey::new("key"),
            },
            CompatMetadata::default(),
        )
        .unwrap();

    // First refresh.
    collection.refresh(Default::default()).await.unwrap();
    assert!(collection.resolve("dyn:v1").is_ok());
    assert!(collection.resolve("dyn:v2").is_err());

    // Change the refresh result for the next call.
    *refresh_result.lock().unwrap() = Some(vec![
        model_info("v2", "Version 2"),
        model_info("v3", "Version 3"),
    ]);

    // Second refresh — replaces v1 with v2+v3.
    collection.refresh(Default::default()).await.unwrap();
    assert!(
        collection.resolve("dyn:v1").is_err(),
        "v1 should be replaced"
    );
    assert!(collection.resolve("dyn:v2").is_ok());
    assert!(collection.resolve("dyn:v3").is_ok());
}

#[tokio::test]
async fn refresh_models_none_clears_prior_dynamic_snapshot() {
    let mut collection = Models::new();

    let provider = MutableRefreshProvider::new(
        "dyn",
        vec![model_info("builtin", "Built In")],
        vec![model_info("dynamic", "Dynamic")],
    );
    let refresh_result = Arc::clone(&provider.refresh_result);
    collection
        .register(
            Box::new(provider),
            AuthDescriptor::StaticApiKey {
                value: SecretKey::new("key"),
            },
            CompatMetadata::default(),
        )
        .unwrap();

    collection.refresh(Default::default()).await.unwrap();
    assert!(collection.resolve("dyn:dynamic").is_ok());
    assert!(collection.resolve("dyn:builtin").is_err());

    *refresh_result.lock().unwrap() = None;
    collection.refresh(Default::default()).await.unwrap();

    assert!(collection.resolve("dyn:dynamic").is_err());
    assert!(collection.resolve("dyn:builtin").is_ok());
}

#[tokio::test]
async fn refresh_models_empty_catalog_is_valid() {
    let mut collection = Models::new();

    let dyn_prov = RefreshProvider::new("dyn", vec![model_info("old", "Old")]).with_refresh(vec![]); // Empty catalog.
    collection
        .register(
            Box::new(dyn_prov),
            AuthDescriptor::StaticApiKey {
                value: SecretKey::new("key"),
            },
            CompatMetadata::default(),
        )
        .unwrap();

    assert!(collection.resolve("dyn:old").is_ok());

    collection.refresh(Default::default()).await.unwrap();

    // Empty dynamic catalog: old model gone, no new models.
    assert!(collection.resolve("dyn:old").is_err());
    let dyn_models: Vec<_> = collection
        .registry()
        .all_models()
        .into_iter()
        .filter(|(pid, _)| *pid == "dyn")
        .collect();
    assert!(
        dyn_models.is_empty(),
        "empty dynamic catalog should yield no models: got {dyn_models:?}"
    );
}
