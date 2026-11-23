//! Radius gateway and catalog lifecycle assertions.

use super::*;

// ---------------------------------------------------------------------------
// identity and endpoint
// ---------------------------------------------------------------------------

mod gateway_baseline {
    use super::*;

    #[test]
    fn radius_nondefault_gateway_starts_without_default_baseline() {
        let provider = PiMessagesProvider::new(Some("https://caller-radius.invalid".into()));
        println!("nondefault model count={}", provider.models().len());
        assert!(
            provider.models().is_empty(),
            "nondefault gateway must not advertise default gateway baseline"
        );
    }

    #[test]
    fn radius_default_gateway_baseline_control() {
        for endpoint in [None, Some(BASE_URL.into())] {
            let provider = PiMessagesProvider::new(endpoint);
            assert!(!provider.models().is_empty());
            assert_eq!(provider.id(), "radius");
        }
    }
}

mod gateway_catalog {
    use super::*;
    use opi_ai::catalog::{ModelsStore, ModelsStoreEntry};
    use opi_ai::registry::ProviderRegistry;

    fn lookup(
        provider: PiMessagesProvider,
        model_override: Option<opi_ai::ModelInfo>,
    ) -> opi_ai::Models {
        let mut registry = ProviderRegistry::new();
        registry.register_provider(Box::new(provider)).unwrap();
        if let Some(model) = model_override {
            registry.register_model("radius", model).unwrap();
        }
        opi_ai::Models::from_registry(registry)
    }

    async fn empty_store(path: &std::path::Path) -> ModelsStore {
        let store = ModelsStore::open(path).unwrap();
        store
            .begin()
            .commit([("radius".into(), ModelsStoreEntry::default())].into())
            .await
            .unwrap();
        assert!(store.snapshot().catalogs["radius"].models.is_empty());
        store
    }

    #[tokio::test]
    async fn custom_local_gateway_accepts_complete_caller_model_upsert() {
        let server = MockServer::start().await;
        mount_44(&server, sse_ok_44()).await;
        let provider = PiMessagesProvider::new(Some(server.uri()));
        assert_eq!(provider.id(), "radius");
        assert!(provider.models().is_empty());
        let auth = std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
            opi_ai::auth::AuthScheme::Bearer,
            SecretString::from("test-key"),
        ));
        let mut models = opi_ai::Models::new();
        models
            .register_route(
                Box::new(provider.clone()),
                auth.clone(),
                opi_ai::AuthProvenanceSource::Static,
                Default::default(),
            )
            .unwrap();
        assert!(models.get_models(Some("radius")).is_empty());
        assert!(server.received_requests().await.unwrap().is_empty());

        let mut caller = model_44("caller-private-model").with_base_url(server.uri());
        caller.display_name = "Caller private model".into();
        caller.sampling_params = Some(Default::default());
        caller.input_limits = Some(serde_json::json!({"maxCharacters":4096}));
        models
            .register_route(
                Box::new(provider.with_models(vec![caller.clone()])),
                auth,
                opi_ai::AuthProvenanceSource::Static,
                Default::default(),
            )
            .unwrap();
        let selected = models.get_model("radius", &caller.id).unwrap();
        assert_eq!(selected, caller);
        assert_eq!(models.get_models(Some("radius")), vec![caller.clone()]);
        let mut request = req_44("hi");
        request.model = format!("radius:{}", caller.id);
        let message = models
            .complete_simple(
                opi_ai::ChatModelRecord::new("radius", selected),
                request,
                Default::default(),
            )
            .await
            .unwrap();
        assert_eq!(message.model, caller.id);
        assert_eq!(message.stop_reason, StopReason::Stop);
        assert_eq!(message.usage.input_tokens, 10);
        assert_eq!(message.usage.output_tokens, 5);
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(body["model"], caller.id);
        server.verify().await;
    }

    #[tokio::test]
    async fn acknowledged_empty_catalog_survives_reopen_without_baseline_resurrection() {
        let server = MockServer::start().await;
        for endpoint in [None, Some(server.uri())] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("models.json");
            let store = empty_store(&path).await;
            let provider = match endpoint {
                None => PiMessagesProvider::new(None),
                Some(base) => {
                    PiMessagesProvider::new(Some(base)).with_models(vec![model_44(MODEL_44)])
                }
            };
            for store in [store, ModelsStore::open(&path).unwrap()] {
                assert!(store.snapshot().catalogs["radius"].models.is_empty());
                let mut models = lookup(provider.clone(), None);
                assert!(models.get_model("radius", MODEL_44).is_some());
                models.set_models_store(store);
                for _ in 0..2 {
                    models
                        .refresh(opi_ai::ModelsRefreshOptions {
                            allow_network: false,
                            ..Default::default()
                        })
                        .await
                        .unwrap();
                    assert!(models.get_model("radius", MODEL_44).is_none());
                    assert!(models.get_models(Some("radius")).is_empty());
                }
            }
        }
        assert!(server.received_requests().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn empty_catalog_preserves_explicit_registry_override_precedence() {
        let server = MockServer::start().await;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("models.json");
        let store = empty_store(&path).await;
        let provider =
            PiMessagesProvider::new(Some(server.uri())).with_models(vec![model_44(MODEL_44)]);
        let mut caller = model_44(MODEL_44);
        caller.display_name = "Caller override".into();
        for store in [store, ModelsStore::open(&path).unwrap()] {
            let mut models = lookup(provider.clone(), Some(caller.clone()));
            models.set_models_store(store);
            models
                .refresh(opi_ai::ModelsRefreshOptions {
                    allow_network: false,
                    ..Default::default()
                })
                .await
                .unwrap();
            assert_eq!(models.get_model("radius", MODEL_44), Some(caller.clone()));
            assert_eq!(models.get_models(Some("radius")), vec![caller.clone()]);
        }
        assert!(server.received_requests().await.unwrap().is_empty());
    }
}

#[test]
fn provider_identity_and_curated_catalog() {
    let provider = PiMessagesProvider::new(None);
    assert_eq!(provider.id(), "radius");
    let models = provider.models();
    assert!(
        !models.is_empty(),
        "curated baseline covers pre-catalog time"
    );
    assert!(
        models
            .iter()
            .all(|model| model.wire_api == WireApi::PiMessages)
    );
}
