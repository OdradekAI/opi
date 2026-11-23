//! Pricing through catalog decode, store reopen and operation cost calculation.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use opi_ai::catalog::ModelsStore;
use opi_ai::classifier::{ClassifierContext, ClassifierStopReason};
use opi_ai::images::ImagesStopReason;
use opi_ai::{ChatModelRecord, WireApi};
use serde_json::{Value, json};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::fixtures::{FixtureAuth, document, registered_collection, request};

#[tokio::test]
async fn mixed_store_preserves_unknown_image_cost_and_classifier_tier_order() {
    let temp = tempfile::tempdir().unwrap();
    let store = ModelsStore::open(temp.path().join("models.json")).unwrap();
    let mut value = document("http://127.0.0.1:1");
    value["providers"]["fixture"]["models"][1]
        .as_object_mut()
        .unwrap()
        .remove("cost");
    value["providers"]["fixture"]["models"][2]["cost"]["tiers"] = json!([
        {"inputTokensAbove": 0, "input": 3, "output": 4, "cacheRead": 0, "cacheWrite": 0},
        {"inputTokensAbove": 0, "input": 5, "output": 6, "cacheRead": 0, "cacheWrite": 0}
    ]);
    let entries = opi_ai::catalog::decode_document(&value).unwrap();
    store.begin().commit(entries.clone()).await.unwrap();
    let restored = ModelsStore::open(store.path()).unwrap().snapshot();
    assert_eq!(restored.catalogs, entries);
    let opi_ai::catalog::CatalogRecord::Image(image) = &restored.catalogs["fixture"].models[1]
    else {
        panic!("image");
    };
    assert!(image.cost.is_none());
}

#[tokio::test]
async fn dynamic_unknown_prices_preserve_models_without_reporting_free_results() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/image/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"choices": [],
            "usage": {"prompt_tokens": 4, "completion_tokens": 2, "total_tokens": 6}})),
        )
        .expect(3)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/classifier/systemone"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"answers": {},
            "usage": {"input_tokens": 4, "output_tokens": 2}})),
        )
        .expect(3)
        .mount(&server)
        .await;
    Mock::given(method("POST")).and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).insert_header("content-type", "text/event-stream")
            .set_body_string("data: {\"id\":\"prices\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":4,\"completion_tokens\":2,\"total_tokens\":6}}\n\ndata: [DONE]\n\n"))
        .expect(3).mount(&server).await;
    let auth = Arc::new(FixtureAuth::default());
    for price in [Some(-1.0), None, Some(0.0)] {
        let mut value = document(&server.uri());
        for model in value["providers"]["fixture"]["models"]
            .as_array_mut()
            .unwrap()
        {
            model["cost"] = price.map_or(Value::Null, |rate| {
                json!({
                "input": rate, "output": rate, "cacheRead": rate, "cacheWrite": rate})
            });
        }
        let entry =
            opi_ai::catalog::decode_document(&value).expect("unknown prices keep usable models");
        let temp = tempfile::tempdir().unwrap();
        let store = ModelsStore::open(temp.path().join("models.json")).unwrap();
        store.begin().commit(entry).await.unwrap();
        let mut models = registered_collection(
            ModelsStore::open(store.path()).unwrap(),
            &server.uri(),
            auth.clone(),
        );
        models
            .refresh(opi_ai::ModelsRefreshOptions {
                allow_network: false,
                ..Default::default()
            })
            .await
            .unwrap();
        let image = models
            .generate_images(
                models.image_model("fixture", "shared").unwrap(),
                Default::default(),
                Default::default(),
            )
            .await;
        assert_eq!(image.stop_reason, ImagesStopReason::Stop);
        assert_eq!(
            image.cost.map(|cost| cost.total),
            (price == Some(0.0)).then_some(0.0)
        );
        let classifier = models
            .classify(
                models.classifier_model("fixture", "shared").unwrap(),
                ClassifierContext {
                    state: json!({}),
                    ..Default::default()
                },
                Default::default(),
            )
            .await;
        assert_eq!(classifier.stop_reason, ClassifierStopReason::Stop);
        assert_eq!(
            classifier.cost.map(|cost| cost.total_cost()),
            (price == Some(0.0)).then_some(0.0)
        );
        let chat = models.get_model("fixture", "shared").unwrap();
        let pricing = chat.pricing.clone();
        let result = models
            .complete_simple(
                ChatModelRecord::new("fixture", chat),
                request(),
                Default::default(),
            )
            .await
            .unwrap();
        let cost = pricing.and_then(|pricing| {
            opi_ai::model_info::calculate_cost_tiered(
                &result.usage,
                &pricing,
                "shared",
                WireApi::OpenAiCompletions,
            )
        });
        assert_eq!(
            cost.map(|cost| cost.total_cost()),
            (price == Some(0.0)).then_some(0.0)
        );
    }
    assert_eq!(auth.0.load(Ordering::SeqCst), 9);
}

#[tokio::test]
async fn source_unknown_base_retains_known_tiers_and_prices_only_above_the_boundary() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/image/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"choices": [],
            "usage": {"prompt_tokens": 4, "completion_tokens": 2, "total_tokens": 6}})),
        )
        .expect(3)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/classifier/systemone"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"answers": {},
            "usage": {"input_tokens": 4, "output_tokens": 2}})),
        )
        .expect(3)
        .mount(&server)
        .await;
    Mock::given(method("POST")).and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).insert_header("content-type", "text/event-stream")
            .set_body_string("data: {\"id\":\"prices\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":4,\"completion_tokens\":2,\"total_tokens\":6}}\n\ndata: [DONE]\n\n"))
        .expect(3).mount(&server).await;
    for (threshold, unknown_tier) in [(4, false), (3, false), (3, true)] {
        let mut value = document(&server.uri());
        for model in value["providers"]["fixture"]["models"]
            .as_array_mut()
            .unwrap()
        {
            let base = if unknown_tier { 1.0 } else { -1.0 };
            let tier_input = if unknown_tier { -1.0 } else { 5.0 };
            model["cost"] = json!({"input": base, "output": base, "cacheRead": base, "cacheWrite": base,
            "tiers": [
                {"inputTokensAbove": threshold, "input": tier_input, "output": 7, "cacheRead": 0, "cacheWrite": 0},
                {"inputTokensAbove": threshold, "input": 99, "output": 99, "cacheRead": 0, "cacheWrite": 0}
            ]});
        }
        let temp = tempfile::tempdir().unwrap();
        let store = ModelsStore::open(temp.path().join("models.json")).unwrap();
        store
            .begin()
            .commit(opi_ai::catalog::decode_document(&value).unwrap())
            .await
            .unwrap();
        let mut models = registered_collection(
            ModelsStore::open(store.path()).unwrap(),
            &server.uri(),
            Arc::new(FixtureAuth::default()),
        );
        models
            .refresh(opi_ai::ModelsRefreshOptions {
                allow_network: false,
                ..Default::default()
            })
            .await
            .unwrap();
        let classifier_record = models.classifier_model("fixture", "shared").unwrap();
        let pricing = classifier_record.cost.as_ref().unwrap();
        assert_eq!(
            pricing.base.input_cost_per_mtok,
            if unknown_tier { 1.0 } else { -1.0 }
        );
        assert_eq!(pricing.tiers.len(), 2);
        assert_eq!(
            pricing.tiers[0].pricing.input_cost_per_mtok,
            if unknown_tier { -1.0 } else { 5.0 }
        );
        let image_record = models.image_model("fixture", "shared").unwrap();
        assert_eq!(image_record.cost_tiers, pricing.tiers);
        assert_eq!(
            models
                .get_model("fixture", "shared")
                .unwrap()
                .pricing
                .as_ref(),
            Some(pricing)
        );
        let image = models
            .generate_images(
                models.image_model("fixture", "shared").unwrap(),
                Default::default(),
                Default::default(),
            )
            .await;
        let classifier = models
            .classify(
                models.classifier_model("fixture", "shared").unwrap(),
                ClassifierContext {
                    state: json!({}),
                    ..Default::default()
                },
                Default::default(),
            )
            .await;
        let chat = models.get_model("fixture", "shared").unwrap();
        let pricing = chat.pricing.clone();
        let result = models
            .complete_simple(
                ChatModelRecord::new("fixture", chat),
                request(),
                Default::default(),
            )
            .await
            .unwrap();
        let chat_cost = pricing.and_then(|pricing| {
            opi_ai::model_info::calculate_cost_tiered(
                &result.usage,
                &pricing,
                "shared",
                WireApi::OpenAiCompletions,
            )
        });
        assert_eq!(image.stop_reason, ImagesStopReason::Stop);
        assert_eq!(classifier.stop_reason, ClassifierStopReason::Stop);
        assert_eq!(
            image.cost.map(|cost| cost.total),
            unknown_tier.then_some(0.000_006),
            "image always uses its base price, including when the selected metadata tier is unknown"
        );
        for cost in [chat_cost, classifier.cost] {
            if threshold == 4 || unknown_tier {
                assert!(cost.is_none());
            } else {
                assert!(
                    (cost.expect("known selected tier").total_cost() - 0.000_034).abs() < 1e-12
                );
            }
        }
    }
}

#[tokio::test]
async fn finite_fractional_thresholds_survive_all_operation_store_reopen_and_costing() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/classifier/systemone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "answers": {}, "usage": {"input_tokens": 4, "output_tokens": 0}
        })))
        .expect(1)
        .mount(&server)
        .await;
    let mut value = document(&server.uri());
    for model in value["providers"]["fixture"]["models"]
        .as_array_mut()
        .unwrap()
    {
        model["cost"] = json!({"input": 1, "output": 0, "cacheRead": 0, "cacheWrite": 0,
        "tiers": [
            {"inputTokensAbove": -2, "input": 99, "output": 0, "cacheRead": 0, "cacheWrite": 0},
            {"inputTokensAbove": -1, "input": 88, "output": 0, "cacheRead": 0, "cacheWrite": 0},
            {"inputTokensAbove": -0.5, "input": 2, "output": 0, "cacheRead": 0, "cacheWrite": 0},
            {"inputTokensAbove": 0.5, "input": 3, "output": 0, "cacheRead": 0, "cacheWrite": 0},
            {"inputTokensAbove": 0.5, "input": 4, "output": 0, "cacheRead": 0, "cacheWrite": 0},
            {"inputTokensAbove": 1.5, "input": 5, "output": 0, "cacheRead": 0, "cacheWrite": 0}
        ]});
    }
    let decoded = opi_ai::catalog::decode_document(&value)
        .expect("finite source thresholds include negative and fractional numbers");
    let temp = tempfile::tempdir().unwrap();
    let store = ModelsStore::open(temp.path().join("models.json")).unwrap();
    store.begin().commit(decoded).await.unwrap();
    let mut models = registered_collection(
        ModelsStore::open(store.path()).unwrap(),
        &server.uri(),
        Arc::new(FixtureAuth::default()),
    );
    models
        .refresh(opi_ai::ModelsRefreshOptions {
            allow_network: false,
            ..Default::default()
        })
        .await
        .unwrap();
    let classifier = models.classifier_model("fixture", "shared").unwrap();
    let pricing = classifier.cost.as_ref().unwrap();
    assert_eq!(
        pricing.tiers.len(),
        6,
        "ignored negative tiers remain metadata"
    );
    assert_eq!(
        models
            .get_model("fixture", "shared")
            .unwrap()
            .pricing
            .as_ref(),
        Some(pricing)
    );
    assert_eq!(
        models.image_model("fixture", "shared").unwrap().cost_tiers,
        pricing.tiers
    );
    for (input, expected_rate) in [(0, 2.0), (1, 3.0), (2, 5.0)] {
        assert_eq!(pricing.effective(input).input_cost_per_mtok, expected_rate);
    }
    let result = models
        .classify(classifier, Default::default(), Default::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    assert_eq!(result.usage.unwrap().input_tokens, 4);
    assert!((result.cost.unwrap().total_cost() - 0.000_020).abs() < 1e-12);
}

#[tokio::test]
async fn integer_threshold_above_f64_precision_survives_decode_store_and_reopen() {
    let threshold = 9_007_199_254_740_993_u64;
    let endpoint = "http://127.0.0.1:1";
    let mut value = document(endpoint);
    for model in value["providers"]["fixture"]["models"]
        .as_array_mut()
        .unwrap()
    {
        model["cost"] = json!({"input": 1, "output": 0, "cacheRead": 0, "cacheWrite": 0,
        "tiers": [{"inputTokensAbove": threshold, "input": 3, "output": 0, "cacheRead": 0, "cacheWrite": 0}]});
    }
    let temp = tempfile::tempdir().unwrap();
    let store = ModelsStore::open(temp.path().join("models.json")).unwrap();
    store
        .begin()
        .commit(opi_ai::catalog::decode_document(&value).unwrap())
        .await
        .unwrap();
    let persisted: Value = serde_json::from_slice(&std::fs::read(store.path()).unwrap()).unwrap();
    let records = &persisted["providers"]["fixture"]["models"];
    for number in [
        &records[0]["x-opi"]["pricingTiers"][0]["above"],
        &records[1]["cost"]["tiers"][0]["inputTokensAbove"],
        &records[2]["cost"]["tiers"][0]["inputTokensAbove"],
    ] {
        assert_eq!(number.as_u64(), Some(threshold));
    }
    let mut models = registered_collection(
        ModelsStore::open(store.path()).unwrap(),
        endpoint,
        Arc::new(FixtureAuth::default()),
    );
    models
        .refresh(opi_ai::ModelsRefreshOptions {
            allow_network: false,
            ..Default::default()
        })
        .await
        .unwrap();
    let pricing = models
        .get_model("fixture", "shared")
        .unwrap()
        .pricing
        .unwrap();
    assert_eq!(
        pricing.tiers[0].input_tokens_above.to_string(),
        threshold.to_string()
    );
    assert_eq!(pricing.effective(threshold).input_cost_per_mtok, 1.0);
    assert_eq!(pricing.effective(threshold + 1).input_cost_per_mtok, 3.0);
    assert_eq!(
        models.classifier_model("fixture", "shared").unwrap().cost,
        Some(pricing.clone())
    );
    assert_eq!(
        models.image_model("fixture", "shared").unwrap().cost_tiers,
        pricing.tiers
    );
}

mod anthropic_complete_metadata {
    use opi_ai::catalog::{CatalogRecord, ModelsStore};
    use serde_json::{Value, json};

    use crate::fixtures::anthropic::{compat, decode, entry, fallback, old_owned_compat, persist};

    #[tokio::test]
    async fn fallback_local_prices_survive_public_store_roundtrip() {
        let catalog = opi_ai::catalog::Catalog::builtin().unwrap();
        let source = catalog
            .model(
                "anthropic",
                opi_ai::catalog::CatalogOperation::Chat,
                "claude-fable-5",
            )
            .unwrap();
        let expected = source.raw["compat"]["allowedFallbackModels"].clone();
        assert_eq!(expected.as_array().unwrap().len(), 2);
        let model = source.to_chat_model_info().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let store = ModelsStore::open(temp.path().join("models.json")).unwrap();
        store
            .begin()
            .commit(
                [(
                    "anthropic".into(),
                    opi_ai::catalog::ModelsStoreEntry {
                        models: vec![opi_ai::catalog::CatalogRecord::from(model)],
                        ..Default::default()
                    },
                )]
                .into_iter()
                .collect(),
            )
            .await
            .unwrap();
        let document: Value =
            serde_json::from_slice(&std::fs::read(store.path()).unwrap()).unwrap();
        let entry = &document["providers"]["anthropic"]["models"][0];
        let actual = entry["compat"]
            .get("allowedFallbackModels")
            .or_else(|| entry["x-opi"]["compat"].get("allowedFallbackModels"));
        assert_eq!(actual, Some(&expected));
        let reopened = ModelsStore::open(store.path()).unwrap().snapshot();
        let CatalogRecord::Chat(restored) = &reopened.catalogs["anthropic"].models[0] else {
            panic!("restored fallback metadata")
        };
        assert_eq!(restored.as_ref(), &source.to_chat_model_info().unwrap());
    }

    #[tokio::test]
    async fn fallback_cardinality_identity_and_finite_source_prices_roundtrip() {
        for count in [0, 1, 3] {
            let mut target = fallback();
            target["cost"]["input"] = json!(-1);
            target["cost"]["tiers"] = json!([
                {"inputTokensAbove": 9007199254740993_u64, "input": 5.0,
                    "output": 1.25, "cacheRead": -1.5, "cacheWrite": 0},
                {"inputTokensAbove": 0.5, "input": 1e20,
                    "output": -1, "cacheRead": 0.5, "cacheWrite": 6.25}
            ]);
            let model = decode(entry(Some(
                json!({"allowedFallbackModels": vec![target; count]}),
            )))
            .unwrap();
            let original = compat(&model).allowed_fallback_models.clone();
            let (_, restored) = persist(model).await;
            assert_eq!(compat(&restored).allowed_fallback_models, original);
        }
        let mut bad = vec![json!(false), json!({})];
        for key in ["provider", "model", "cost"] {
            let mut target = fallback();
            target.as_object_mut().unwrap().remove(key);
            bad.push(target);
        }
        for key in ["provider", "model"] {
            for invalid in [json!(""), json!(0), Value::Null] {
                let mut target = fallback();
                target[key] = invalid;
                bad.push(target);
            }
        }
        for key in ["input", "output", "cacheRead", "cacheWrite"] {
            let mut target = fallback();
            target["cost"].as_object_mut().unwrap().remove(key);
            bad.push(target);
        }
        let mut invalid_tiers = fallback();
        invalid_tiers["cost"]["tiers"] = json!({});
        bad.push(invalid_tiers);
        for target in bad {
            for owned in [false, true] {
                let mut value = entry(Some(json!({"allowedFallbackModels": [target.clone()]})));
                if owned {
                    value["x-opi"]["compat"] = old_owned_compat();
                    value["x-opi"]["compat"]["allowedFallbackModels"] = json!([target.clone()]);
                    value.as_object_mut().unwrap().remove("compat");
                }
                assert!(decode(value).is_err(), "owned={owned}, target={target}");
            }
        }
        for owned in [false, true] {
            let mut value = entry(Some(json!({"allowedFallbackModels": vec![fallback(); 4]})));
            if owned {
                value["x-opi"]["compat"] = old_owned_compat();
                value["x-opi"]["compat"]["allowedFallbackModels"] = json!(vec![fallback(); 4]);
                value.as_object_mut().unwrap().remove("compat");
            }
            assert!(decode(value).is_err(), "owned={owned}, four targets");
        }
    }

    #[tokio::test]
    async fn fallback_rate_boundaries_preserve_numeric_values_without_saturation() {
        let mut target = fallback();
        target["provider"] = json!(" ");
        target["model"] = json!(" ");
        target["cost"] = json!({
            "input": 9.223372036854776e18,
            "output": 1.8446744073709552e19,
            "cacheRead": -9.223372036854776e18,
            "cacheWrite": -1.25
        });
        let model = decode(entry(Some(json!({"allowedFallbackModels": [target]})))).unwrap();
        let (document, restored) = persist(model).await;
        let rates = &document["providers"]["anthropic"]["models"][0]["x-opi"]["compat"]["allowedFallbackModels"]
            [0]["cost"];
        assert_eq!(rates["input"].as_f64(), Some(9.223372036854776e18));
        assert_eq!(rates["output"].as_f64(), Some(1.8446744073709552e19));
        assert_eq!(rates["cacheRead"].as_f64(), Some(-9.223372036854776e18));
        assert_eq!(rates["cacheWrite"].as_f64(), Some(-1.25));
        let target = &compat(&restored).allowed_fallback_models.as_ref().unwrap()[0];
        assert_eq!(target.provider, " ");
        assert_eq!(target.model, " ");
    }
}
