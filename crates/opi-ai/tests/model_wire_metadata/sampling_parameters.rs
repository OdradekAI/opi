//! Sampling parameter presence and shallow model, level and request precedence.

use opi_ai::registry::ModelCapabilities;
use opi_ai::types::model::{ModelInfo, ThinkingLevel, WireApi, resolve_sampling_params};
use serde_json::json;

use super::fixtures::sampling_json;

#[test]
fn sampling_metadata_defaults_absent_and_builders_preserve_presence() {
    let model = ModelInfo::new(
        "sampling-model",
        "Sampling Model",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(100_000, 10_000).with_streaming(true),
    );
    assert!(model.sampling_params.is_none());
    assert!(model.sampling_params_by_thinking_level.is_empty());

    let explicit_empty = model.clone().with_sampling_params(Default::default());
    assert_eq!(explicit_empty.sampling_params, Some(Default::default()));

    let model = model
        .with_sampling_params(sampling_json(json!({"top_k": 40})))
        .with_sampling_level_params(
            ThinkingLevel::High,
            sampling_json(json!({"temperature": 0.5})),
        );
    assert_eq!(model.sampling_params.as_ref().unwrap()["top_k"], json!(40));
    assert_eq!(
        model.sampling_params_by_thinking_level[&ThinkingLevel::High]["temperature"],
        json!(0.5)
    );
    model.validate().unwrap();
}

#[test]
fn resolve_sampling_params_applies_model_level_request_precedence() {
    let model = ModelInfo::new(
        "sampling-model",
        "Sampling Model",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(100_000, 10_000).with_streaming(true),
    )
    .with_sampling_params(sampling_json(json!({"a": 1, "b": 1})))
    .with_sampling_level_params(ThinkingLevel::High, sampling_json(json!({"b": 2})));

    let resolved = resolve_sampling_params(
        &model,
        ThinkingLevel::High,
        Some(&sampling_json(json!({"c": 3}))),
    )
    .expect("non-empty sources resolve");
    assert_eq!(resolved["a"], json!(1));
    assert_eq!(resolved["b"], json!(2));
    assert_eq!(resolved["c"], json!(3));

    let request_wins = resolve_sampling_params(
        &model,
        ThinkingLevel::High,
        Some(&sampling_json(json!({"a": 9, "b": 9}))),
    )
    .expect("non-empty sources resolve");
    assert_eq!(request_wins["a"], json!(9));
    assert_eq!(request_wins["b"], json!(9));

    let empty = ModelInfo::new(
        "empty",
        "Empty",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(100_000, 10_000).with_streaming(true),
    );
    assert!(resolve_sampling_params(&empty, ThinkingLevel::High, None).is_none());
}

#[test]
fn sampling_presence_shallow_merge_matrix() {
    let empty = sampling_json(json!({}));
    for model_present in [false, true] {
        for level_present in [false, true] {
            for request_present in [false, true] {
                let mut model = ModelInfo::new(
                    "presence",
                    "Presence",
                    WireApi::OpenAiCompletions,
                    ModelCapabilities::new(8192, 1024),
                );
                if model_present {
                    model = model.with_sampling_params(empty.clone());
                }
                if level_present {
                    model = model.with_sampling_level_params(ThinkingLevel::High, empty.clone());
                }
                let result = resolve_sampling_params(
                    &model,
                    ThinkingLevel::High,
                    request_present.then_some(&empty),
                );
                assert_eq!(
                    result,
                    (model_present || level_present || request_present).then(|| empty.clone()),
                    "model={model_present}, level={level_present}, request={request_present}"
                );
                assert_eq!(
                    resolve_sampling_params(&model, ThinkingLevel::Low, None).is_some(),
                    model_present
                );
            }
        }
    }
    let model = ModelInfo::new(
        "precedence",
        "Precedence",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(8192, 1024),
    )
    .with_sampling_params(sampling_json(json!({"a":1,"b":1,"nested":{"base":1}})))
    .with_sampling_level_params(
        ThinkingLevel::High,
        sampling_json(json!({"b":2,"nested":{"x":2}})),
    );
    let request = sampling_json(json!({"c":3,"nested":{"y":3}}));
    assert_eq!(
        resolve_sampling_params(&model, ThinkingLevel::High, Some(&request)),
        Some(sampling_json(json!({"a":1,"b":2,"c":3,"nested":{"y":3}})))
    );
    assert_eq!(
        resolve_sampling_params(&model, ThinkingLevel::High, Some(&empty)),
        Some(sampling_json(json!({"a":1,"b":2,"nested":{"x":2}})))
    );
}
