//! Wire identities, model validation, catalog projections and routing metadata.

use std::str::FromStr;

use opi_ai::provider::Provider;
use opi_ai::providers::anthropic as anthropic_provider;
use opi_ai::providers::azure_openai::AzureOpenAIProvider;
use opi_ai::providers::bedrock;
use opi_ai::providers::gemini as gemini_provider;
use opi_ai::providers::mistral;
use opi_ai::providers::openai as openai_profile;
use opi_ai::providers::openai_responses as openai_responses_profile;
use opi_ai::providers::openrouter;
use opi_ai::providers::vertex;
use opi_ai::registry::ModelCapabilities;
use opi_ai::types::model::{AnthropicMessagesCompat, ModelInfo, WireApi, WireCompat};

#[test]
fn wire_api_serializes_exact_reviewed_names() {
    let cases = [
        (WireApi::AnthropicMessages, "anthropic-messages"),
        (WireApi::OpenAiCompletions, "openai-completions"),
        (WireApi::OpenAiResponses, "openai-responses"),
        (WireApi::OpenAiCodexResponses, "openai-codex-responses"),
        (WireApi::GoogleGenerativeAi, "google-generative-ai"),
        (WireApi::GoogleVertex, "google-vertex"),
        (WireApi::BedrockConverseStream, "bedrock-converse-stream"),
        (WireApi::AzureOpenAiCompletions, "azure-openai-completions"),
    ];

    for (wire, name) in cases {
        assert_eq!(wire.to_string(), name);
        assert_eq!(serde_json::to_value(wire).unwrap(), name);
        assert_eq!(WireApi::from_str(name).unwrap(), wire);
    }
}

#[test]
fn model_info_public_constructor_sets_required_wire() {
    let model = ModelInfo::new(
        "external-model",
        "External Model",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(128_000, 16_384).with_streaming(true),
    );

    assert_eq!(model.id, "external-model");
    assert_eq!(model.wire_api, WireApi::OpenAiCompletions);
    assert_eq!(model.compat.wire_api(), WireApi::OpenAiCompletions);
    model.validate().unwrap();
}

#[test]
fn model_info_rejects_wire_compat_mismatch() {
    let error = ModelInfo::new(
        "mismatch",
        "Mismatch",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(8_192, 1_024),
    )
    .with_compat(WireCompat::AnthropicMessages(
        AnthropicMessagesCompat::default(),
    ))
    .unwrap_err();

    assert!(error.to_string().contains("wire"));
    assert!(error.to_string().contains("mismatch"));
}

#[test]
fn routing_types_retain_explicit_empty_and_nested_partition_null() {
    use opi_ai::types::model::{OpenRouterRouting, VercelGatewayRouting};
    for routing in [
        serde_json::json!({}),
        serde_json::json!({"sort":"price"}),
        serde_json::json!({"sort":{}}),
        serde_json::json!({"sort":{"by":"price"}}),
        serde_json::json!({"sort":{"partition":null}}),
        serde_json::json!({"sort":{"partition":"model"}}),
        serde_json::json!({"data_collection":"allow","preferred_min_throughput":0,
            "preferred_max_latency":{"p50":0,"p75":1.25,"p90":2,"p99":3}}),
    ] {
        let typed: OpenRouterRouting = serde_json::from_value(routing.clone()).unwrap();
        assert_eq!(serde_json::to_value(typed).unwrap(), routing);
    }
    for routing in [
        serde_json::json!({}),
        serde_json::json!({"only":[]}),
        serde_json::json!({"order":[]}),
        serde_json::json!({"only":["chosen"],"order":[]}),
    ] {
        let typed: VercelGatewayRouting = serde_json::from_value(routing.clone()).unwrap();
        assert_eq!(serde_json::to_value(typed).unwrap(), routing);
    }
    for invalid in [
        serde_json::json!([]),
        serde_json::json!({"sort":[]}),
        serde_json::json!({"sort":{"partition":false}}),
        serde_json::json!({"max_price":[]}),
        serde_json::json!({"max_price":{"prompt":false}}),
        serde_json::json!({"preferred_min_throughput":[]}),
        serde_json::json!({"preferred_min_throughput":{"p99":"1"}}),
        serde_json::json!({"preferred_max_latency":[]}),
        serde_json::json!({"quantizations":[0]}),
    ] {
        assert!(
            serde_json::from_value::<OpenRouterRouting>(invalid.clone()).is_err(),
            "accepted invalid routing: {invalid}"
        );
    }
    assert!(serde_json::from_value::<VercelGatewayRouting>(serde_json::json!([])).is_err());
}

#[test]
fn single_wire_catalogs_and_configured_azure_declare_exact_wire() {
    let catalogs = [
        (
            anthropic_provider::model_catalog(),
            WireApi::AnthropicMessages,
        ),
        (openai_profile::model_catalog(), WireApi::OpenAiCompletions),
        (
            openai_responses_profile::model_catalog(),
            WireApi::OpenAiResponses,
        ),
        (openrouter::model_catalog(), WireApi::OpenAiCompletions),
        (mistral::model_catalog(), WireApi::OpenAiCompletions),
        (
            gemini_provider::model_catalog(),
            WireApi::GoogleGenerativeAi,
        ),
        (vertex::model_catalog(), WireApi::GoogleVertex),
        (bedrock::model_catalog(), WireApi::BedrockConverseStream),
    ];

    for (catalog, wire) in catalogs {
        assert!(!catalog.is_empty());
        for model in catalog {
            assert_eq!(model.wire_api, wire, "{}", model.id);
            model.validate().unwrap();
        }
    }

    let azure = AzureOpenAIProvider::from_config(
        Some("https://example.openai.azure.com".into()),
        vec!["deployment".into()],
        Some("2024-10-21".into()),
    )
    .unwrap();
    assert_eq!(azure.models()[0].wire_api, WireApi::AzureOpenAiCompletions);
}

#[test]
fn model_metadata_rejects_long_cache_without_cache_control() {
    let model = ModelInfo::new(
        "bad-cache",
        "Bad Cache",
        WireApi::AnthropicMessages,
        ModelCapabilities::new(1000, 100).with_long_cache_retention(true),
    );

    assert!(model.validate().is_err());
}
