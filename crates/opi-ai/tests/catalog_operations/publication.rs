//! Catalog decode, source freshness, publication atomicity and stored route metadata.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use opi_ai::catalog::ModelsStore;
use opi_ai::models::Models;
use opi_ai::provider::ProviderError;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use super::fixtures::{FixtureAuth, document, registered_collection};

#[test]
fn catalog_document_rejects_unknown_operation() {
    let mut document = document("http://127.0.0.1:1");
    document["providers"]["fixture"]["models"] = json!([{
        "type": "audio", "id": "unsupported", "api": "openai-completions",
        "contextWindow": 8192, "maxTokens": 512,
    }]);
    assert!(opi_ai::catalog::decode_document(&document).is_err());
}

#[tokio::test]
async fn provider_freshness_is_preserved_without_fabricating_missing_values() {
    let mut document = document("http://127.0.0.1:1");
    let chat = document["providers"]["fixture"]["models"][0].clone();
    document["providers"]["fixture"]["models"] = json!([chat]);
    document["providers"]["fixture"]["lastModified"] = json!(0);
    document["providers"]["fixture"]
        .as_object_mut()
        .unwrap()
        .remove("checkedAt");
    document["providers"]["fixture"]["etag"] = json!("\"opaque-validator\"");
    let decoded = opi_ai::catalog::decode_document(&document).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let store = ModelsStore::open(temp.path().join("models.json")).unwrap();
    store.begin().commit(decoded).await.unwrap();
    let reopened = ModelsStore::open(store.path()).unwrap();
    let persisted: Value =
        serde_json::from_slice(&std::fs::read(reopened.path()).unwrap()).unwrap();
    let entry = &persisted["providers"]["fixture"];
    assert_eq!(entry["lastModified"], 0);
    assert!(entry.get("checkedAt").is_none());
    assert_eq!(entry["etag"], "\"opaque-validator\"");
}

#[derive(Clone)]
enum CatalogAction {
    Publish(Value),
    Fail,
    Cancel(Value),
    Pending,
    InvalidateThenPublish(Value, ModelsStore),
}

struct ControlledSource(std::sync::Mutex<CatalogAction>);

impl ControlledSource {
    fn set(&self, action: CatalogAction) {
        *self.0.lock().unwrap() = action;
    }
}

impl opi_ai::catalog::DynamicCatalogSource for ControlledSource {
    fn id(&self) -> &str {
        "controlled"
    }

    fn provider_ids(&self) -> &[String] {
        static IDS: std::sync::LazyLock<Vec<String>> =
            std::sync::LazyLock::new(|| vec!["fixture".into()]);
        &IDS
    }

    fn fetch(
        &self,
        context: opi_ai::catalog::CatalogRefreshContext,
    ) -> futures_util::future::BoxFuture<
        'static,
        Result<std::collections::HashMap<String, opi_ai::catalog::ModelsStoreEntry>, ProviderError>,
    > {
        let action = self.0.lock().unwrap().clone();
        Box::pin(async move {
            let value = match action {
                CatalogAction::Publish(value) => value,
                CatalogAction::Fail => {
                    return Err(ProviderError::Network(
                        opi_ai::provider::ProviderErrorSummary::sanitized("fixture source failed"),
                    ));
                }
                CatalogAction::Cancel(value) => {
                    context.cancel.cancel();
                    value
                }
                CatalogAction::Pending => return std::future::pending().await,
                CatalogAction::InvalidateThenPublish(value, store) => {
                    store.invalidate_source("controlled").await.unwrap();
                    value
                }
            };
            opi_ai::catalog::decode_document(&value).map_err(|error| {
                ProviderError::Config(opi_ai::provider::ProviderErrorSummary::sanitized(
                    error.to_string(),
                ))
            })
        })
    }
}

fn assert_selected(models: &Models, id: &str, expected: bool) {
    assert_eq!(models.get_model("fixture", id).is_some(), expected);
    assert_eq!(models.image_model("fixture", id).is_ok(), expected);
    assert_eq!(models.classifier_model("fixture", id).is_ok(), expected);
}

#[tokio::test]
async fn mixed_catalog_failure_cancel_drop_and_empty_preserve_publication_rules() {
    let temp = tempfile::tempdir().unwrap();
    let store = ModelsStore::open(temp.path().join("models.json")).unwrap();
    let auth = Arc::new(FixtureAuth::default());
    let endpoint = "http://127.0.0.1:1";
    let original = document(endpoint);
    let source = Arc::new(ControlledSource(std::sync::Mutex::new(
        CatalogAction::Publish(original.clone()),
    )));
    let mut models = registered_collection(store.clone(), endpoint, auth.clone());
    models.register_catalog_source(source.clone());
    models.refresh(Default::default()).await.unwrap();
    assert_selected(&models, "shared", true);
    assert_selected(&models, "baseline", false);
    assert_eq!(
        models.get_available_classifiers(None).await.unwrap().len(),
        1
    );
    let confirmed = store.snapshot().catalogs;

    source.set(CatalogAction::Fail);
    assert!(
        models
            .refresh(Default::default())
            .await
            .unwrap()
            .find("controlled")
            .unwrap()
            .outcome
            .is_failure()
    );
    assert_eq!(store.snapshot().catalogs, confirmed);
    assert_selected(&models, "shared", true);

    let mut empty = original;
    empty["providers"]["fixture"]["models"] = json!([]);
    source.set(CatalogAction::Cancel(empty.clone()));
    let before = std::fs::read(store.path()).unwrap();
    let token = CancellationToken::new();
    assert_eq!(
        models
            .refresh_with(Default::default(), &token)
            .await
            .unwrap()
            .status(),
        opi_ai::RefreshStatus::Cancelled
    );
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    assert_selected(&models, "shared", true);

    source.set(CatalogAction::Pending);
    assert!(
        tokio::time::timeout(
            std::time::Duration::from_millis(10),
            models.refresh(Default::default())
        )
        .await
        .is_err()
    );
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    assert_selected(&models, "shared", true);

    source.set(CatalogAction::Publish(empty));
    models.refresh(Default::default()).await.unwrap();
    assert_selected(&models, "shared", false);
    assert_selected(&models, "baseline", false);
    assert!(
        models
            .get_available_classifiers(None)
            .await
            .unwrap()
            .is_empty()
    );
    let mut reopened = registered_collection(
        ModelsStore::open(store.path()).unwrap(),
        endpoint,
        auth.clone(),
    );
    reopened
        .refresh(opi_ai::ModelsRefreshOptions {
            allow_network: false,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_selected(&reopened, "shared", false);
    assert_selected(&reopened, "baseline", false);
    assert_eq!(auth.0.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn mixed_catalog_late_source_cannot_undo_invalidation_or_reopen_stale_records() {
    let temp = tempfile::tempdir().unwrap();
    let store = ModelsStore::open(temp.path().join("models.json")).unwrap();
    let auth = Arc::new(FixtureAuth::default());
    let endpoint = "http://127.0.0.1:1";
    let original = document(endpoint);
    let source = Arc::new(ControlledSource(std::sync::Mutex::new(
        CatalogAction::Publish(original.clone()),
    )));
    let mut models = registered_collection(store.clone(), endpoint, auth.clone());
    models.register_catalog_source(source.clone());
    models.refresh(Default::default()).await.unwrap();
    source.set(CatalogAction::InvalidateThenPublish(
        original,
        store.clone(),
    ));
    assert_eq!(
        models.refresh(Default::default()).await.unwrap().status(),
        opi_ai::RefreshStatus::Superseded
    );
    assert!(store.snapshot().catalogs.is_empty());
    models
        .refresh(opi_ai::ModelsRefreshOptions {
            allow_network: false,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_selected(&models, "shared", false);
    let reopened_store = ModelsStore::open(store.path()).unwrap();
    assert!(
        reopened_store
            .snapshot()
            .invalidated_sources
            .contains("controlled")
    );
    let mut reopened = registered_collection(reopened_store, endpoint, auth.clone());
    reopened
        .refresh(opi_ai::ModelsRefreshOptions {
            allow_network: false,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_selected(&reopened, "shared", false);
    assert_eq!(auth.0.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn malformed_operation_route_does_not_replace_any_record_in_provider_entry() {
    let temp = tempfile::tempdir().unwrap();
    let store = ModelsStore::open(temp.path().join("models.json")).unwrap();
    let auth = Arc::new(FixtureAuth::default());
    let endpoint = "http://127.0.0.1:1";
    let mut value = document(endpoint);
    let source = Arc::new(ControlledSource(std::sync::Mutex::new(
        CatalogAction::Publish(value.clone()),
    )));
    let mut models = registered_collection(store.clone(), endpoint, auth.clone());
    models.register_catalog_source(source.clone());
    models.refresh(Default::default()).await.unwrap();
    let confirmed = store.snapshot().catalogs;
    value["providers"]["fixture"]["models"][1]["api"] = json!("unregistered-image-api");
    value["providers"]["fixture"]["models"][0]["maxTokens"] = json!(17);
    source.set(CatalogAction::Publish(value));
    assert!(
        models
            .refresh(Default::default())
            .await
            .unwrap()
            .find("controlled")
            .unwrap()
            .outcome
            .is_failure()
    );
    assert_eq!(store.snapshot().catalogs, confirmed);
    assert_eq!(
        models
            .get_model("fixture", "shared")
            .unwrap()
            .capabilities
            .max_output_tokens,
        512
    );
    assert_selected(&models, "shared", true);
    assert_eq!(auth.0.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn mixed_catalog_persist_failure_keeps_all_three_visible_records() {
    let temp = tempfile::tempdir().unwrap();
    let store = ModelsStore::open(temp.path().join("models.json")).unwrap();
    let auth = Arc::new(FixtureAuth::default());
    let endpoint = "http://127.0.0.1:1";
    let original = document(endpoint);
    let source = Arc::new(ControlledSource(std::sync::Mutex::new(
        CatalogAction::Publish(original.clone()),
    )));
    let mut models = registered_collection(store.clone(), endpoint, auth.clone());
    models.register_catalog_source(source.clone());
    models.refresh(Default::default()).await.unwrap();
    let confirmed = store.snapshot().catalogs;
    let before = std::fs::read(store.path()).unwrap();
    // Isolated filesystem failure at the store's write-before-rename step.
    std::fs::create_dir(store.path().with_extension("json.committing")).unwrap();
    let mut empty = original;
    empty["providers"]["fixture"]["models"] = json!([]);
    source.set(CatalogAction::Publish(empty));
    assert!(models.refresh(Default::default()).await.is_err());
    assert_eq!(store.snapshot().catalogs, confirmed);
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    assert_selected(&models, "shared", true);
    assert_eq!(auth.0.load(Ordering::SeqCst), 0);
}

mod anthropic_complete_metadata {
    use opi_ai::ModelInfo;
    use opi_ai::catalog::{CatalogRecord, ModelsStore, ModelsStoreEntry};
    use opi_ai::model_info::{AnthropicMessagesCompat, AnthropicSessionAffinityFormat, WireCompat};
    use serde_json::{Value, json};

    use crate::fixtures::anthropic::{compat, decode, entry, fallback, old_owned_compat, persist};

    #[tokio::test]
    async fn source_defaults_optional_presence_and_generic_cache_opt_out_roundtrip() {
        for source in [None, Some(json!({}))] {
            let model = decode(entry(source)).unwrap();
            assert_eq!(compat(&model), &AnthropicMessagesCompat::default());
            assert!(model.capabilities.supports_cache_control);
            assert!(model.capabilities.supports_long_cache_retention);
            let (document, _) = persist(model).await;
            let encoded = &document["providers"]["anthropic"]["models"][0]["x-opi"]["compat"];
            for absent in [
                "sendSessionAffinityHeaders",
                "sessionAffinityFormat",
                "allowedFallbackModels",
            ] {
                assert!(encoded.get(absent).is_none(), "{absent}");
            }
        }
        let mut value = entry(None);
        value["x-opi"]["cacheControl"] = json!(false);
        let model = decode(value).unwrap();
        assert!(!model.capabilities.supports_cache_control);
        assert!(!model.capabilities.supports_long_cache_retention);
        persist(model).await;

        let model = decode(entry(Some(json!({
            "sendSessionAffinityHeaders": false,
            "sessionAffinityFormat": "openrouter", "allowedFallbackModels": []
        }))))
        .unwrap();
        assert_eq!(compat(&model).send_session_affinity_headers, Some(false));
        assert_eq!(
            compat(&model).session_affinity_format,
            Some(AnthropicSessionAffinityFormat::OpenRouter)
        );
        assert_eq!(compat(&model).allowed_fallback_models, Some(vec![]));
        let (document, _) = persist(model).await;
        let encoded = &document["providers"]["anthropic"]["models"][0]["x-opi"]["compat"];
        assert_eq!(encoded["sendSessionAffinityHeaders"], false);
        assert_eq!(encoded["sessionAffinityFormat"], "openrouter");
        assert_eq!(encoded["allowedFallbackModels"], json!([]));

        let model = decode(entry(Some(json!({
            "sendSessionAffinityHeaders": null,
            "sessionAffinityFormat": null, "allowedFallbackModels": null
        }))))
        .unwrap();
        assert_eq!(compat(&model), &AnthropicMessagesCompat::default());
        persist(model).await;
    }

    #[tokio::test]
    async fn all_boolean_branches_and_owned_precedence_survive_restore() {
        for enabled in [false, true] {
            let source = json!({
                "supportsEagerToolInputStreaming": enabled,
                "supportsLongCacheRetention": enabled,
                "supportsCacheControlOnTools": enabled,
                "forceAdaptiveThinking": enabled,
                "supportsTemperature": enabled,
                "allowEmptySignature": enabled,
                "supportsStrictTools": enabled,
                "supportsMidConvoEffort": enabled,
                "supportsMidConvoSystemMessages": enabled,
                "supportsMidConvoToolChanges": enabled,
                "sendSessionAffinityHeaders": enabled,
                "sessionAffinityFormat": "openrouter",
                "allowedFallbackModels": [fallback()]
            });
            let model = decode(entry(Some(source.clone()))).unwrap();
            let c = compat(&model);
            assert_eq!(
                [
                    c.supports_eager_tool_input_streaming,
                    c.supports_long_cache_retention,
                    c.supports_cache_control_on_tools,
                    c.force_adaptive_thinking,
                    c.supports_temperature,
                    c.allow_empty_signature,
                    c.supports_strict_tools,
                    c.supports_mid_convo_effort,
                    c.supports_mid_convo_system_messages,
                    c.supports_mid_convo_tool_changes
                ],
                [enabled; 10]
            );
            assert_eq!(c.send_session_affinity_headers, Some(enabled));
            assert_eq!(c.allowed_fallback_models.as_ref().unwrap().len(), 1);
            persist(model).await;

            let mut value = entry(Some(source));
            value["x-opi"]["compat"] = old_owned_compat();
            value["x-opi"]["cacheControl"] = json!(false);
            value["x-opi"]["longCacheRetention"] = json!(false);
            let model = decode(value).unwrap();
            assert_eq!(
                compat(&model),
                &AnthropicMessagesCompat {
                    supports_eager_tool_input_streaming: false,
                    force_adaptive_thinking: false,
                    supports_temperature: false,
                    ..Default::default()
                }
            );
            assert!(!model.capabilities.supports_cache_control);
            assert!(!model.capabilities.supports_long_cache_retention);
            persist(model).await;
        }
    }

    #[tokio::test]
    async fn each_boolean_key_changes_only_its_own_typed_field_and_survives_restore() {
        fn fields(c: &AnthropicMessagesCompat) -> [bool; 10] {
            [
                c.supports_eager_tool_input_streaming,
                c.supports_long_cache_retention,
                c.supports_cache_control_on_tools,
                c.force_adaptive_thinking,
                c.supports_temperature,
                c.allow_empty_signature,
                c.supports_strict_tools,
                c.supports_mid_convo_effort,
                c.supports_mid_convo_system_messages,
                c.supports_mid_convo_tool_changes,
            ]
        }
        let keys = [
            "supportsEagerToolInputStreaming",
            "supportsLongCacheRetention",
            "supportsCacheControlOnTools",
            "forceAdaptiveThinking",
            "supportsTemperature",
            "allowEmptySignature",
            "supportsStrictTools",
            "supportsMidConvoEffort",
            "supportsMidConvoSystemMessages",
            "supportsMidConvoToolChanges",
        ];
        for owned in [false, true] {
            let baseline = if owned {
                [
                    false, true, true, false, false, false, false, false, false, false,
                ]
            } else {
                [
                    true, true, true, false, true, false, false, false, false, false,
                ]
            };
            for (index, key) in keys.into_iter().enumerate() {
                let mut expected = baseline;
                expected[index] = !expected[index];
                let mut value = entry(Some(json!({})));
                if owned {
                    value["x-opi"]["compat"] = old_owned_compat();
                    value["x-opi"]["compat"][key] = json!(expected[index]);
                } else {
                    value["compat"][key] = json!(expected[index]);
                }
                let model = decode(value).unwrap();
                let c = compat(&model);
                assert_eq!(fields(c), expected, "owned={owned}, key={key}");
                assert!(c.send_session_affinity_headers.is_none());
                assert!(c.session_affinity_format.is_none());
                assert!(c.allowed_fallback_models.is_none());
                let (_, restored) = persist(model).await;
                assert_eq!(
                    fields(compat(&restored)),
                    expected,
                    "owned={owned}, key={key}"
                );
            }
        }
    }

    #[test]
    fn source_and_owned_wrong_types_and_old_required_fields_are_rejected() {
        for owned in [false, true] {
            for key in [
                "supportsEagerToolInputStreaming",
                "supportsLongCacheRetention",
                "supportsCacheControlOnTools",
                "forceAdaptiveThinking",
                "supportsTemperature",
                "allowEmptySignature",
                "supportsStrictTools",
                "supportsMidConvoEffort",
                "supportsMidConvoSystemMessages",
                "supportsMidConvoToolChanges",
            ] {
                for invalid in [Value::Null, json!(0), json!("false"), json!([]), json!({})] {
                    let mut value = entry(Some(json!({})));
                    if owned {
                        value["x-opi"]["compat"] = old_owned_compat();
                        value["x-opi"]["compat"][key] = invalid;
                    } else {
                        value["compat"][key] = invalid;
                    }
                    assert!(decode(value).is_err(), "owned={owned}, key={key}");
                }
            }
            for (key, invalid) in [
                ("sendSessionAffinityHeaders", json!(0)),
                ("sendSessionAffinityHeaders", json!("false")),
                ("sessionAffinityFormat", json!("openai")),
                ("sessionAffinityFormat", json!(true)),
                ("allowedFallbackModels", json!({})),
            ] {
                let mut value = entry(Some(json!({})));
                if owned {
                    value["x-opi"]["compat"] = old_owned_compat();
                    value["x-opi"]["compat"][key] = invalid;
                } else {
                    value["compat"][key] = invalid;
                }
                assert!(decode(value).is_err(), "owned={owned}, key={key}");
            }
        }
        for missing in [
            "supportsEagerToolInputStreaming",
            "forceAdaptiveThinking",
            "supportsTemperature",
        ] {
            let mut owned = old_owned_compat();
            owned.as_object_mut().unwrap().remove(missing);
            let mut value = entry(None);
            value["x-opi"]["compat"] = owned;
            assert!(decode(value).is_err(), "required old field={missing}");
        }
        assert!(decode(entry(Some(json!(false)))).is_err());
    }

    #[tokio::test]
    async fn invalid_typed_fallback_publication_preserves_the_previous_store() {
        let good = decode(entry(Some(json!({"allowedFallbackModels": [fallback()]})))).unwrap();
        let temp = tempfile::tempdir().unwrap();
        let store = ModelsStore::open(temp.path().join("models.json")).unwrap();
        let entries = |model: ModelInfo| {
            [(
                "anthropic".into(),
                ModelsStoreEntry {
                    models: vec![CatalogRecord::from(model)],
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect()
        };
        store.begin().commit(entries(good.clone())).await.unwrap();
        let before = store.snapshot().catalogs;
        let bytes = std::fs::read(store.path()).unwrap();
        for invalid in [
            "too-many",
            "empty-provider",
            "empty-model",
            "nonfinite-base",
            "nonfinite-tier",
        ] {
            let mut model = good.clone();
            let WireCompat::AnthropicMessages(compat) = &mut model.compat else {
                unreachable!()
            };
            let target = compat.allowed_fallback_models.as_ref().unwrap()[0].clone();
            match invalid {
                "too-many" => compat.allowed_fallback_models = Some(vec![target; 4]),
                "empty-provider" => compat.allowed_fallback_models.as_mut().unwrap()[0]
                    .provider
                    .clear(),
                "empty-model" => compat.allowed_fallback_models.as_mut().unwrap()[0]
                    .model
                    .clear(),
                "nonfinite-base" => {
                    compat.allowed_fallback_models.as_mut().unwrap()[0]
                        .cost
                        .base
                        .input_cost_per_mtok = f64::INFINITY
                }
                "nonfinite-tier" => compat.allowed_fallback_models.as_mut().unwrap()[0]
                    .cost
                    .tiers
                    .push(opi_ai::PricingTier {
                        input_tokens_above: 0.into(),
                        pricing: opi_ai::Pricing {
                            input_cost_per_mtok: f64::NAN,
                            ..Default::default()
                        },
                    }),
                _ => unreachable!(),
            }
            assert!(
                store.begin().commit(entries(model)).await.is_err(),
                "{invalid}"
            );
            assert_eq!(store.snapshot().catalogs, before, "{invalid}");
            assert_eq!(std::fs::read(store.path()).unwrap(), bytes, "{invalid}");
        }
    }
}
