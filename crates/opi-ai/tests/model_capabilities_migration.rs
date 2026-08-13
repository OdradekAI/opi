//! Task 14.4 migration guard: proves ModelInfo capability fields are nested,
//! ModelCapabilities is non-exhaustive with constructor/builder, cache fields
//! exist, and Anthropic built-in models have cache capabilities enabled.

use opi_ai::anthropic::AnthropicProvider;
use opi_ai::provider::Provider;
use opi_ai::registry::{ModelCapabilities, ProviderRegistry};

// ---------------------------------------------------------------------------
// Non-exhaustive construction surface (external crate compile test)
// ---------------------------------------------------------------------------

#[test]
fn model_capabilities_new_and_builder_api() {
    let caps = ModelCapabilities::new(200_000, 8_192)
        .with_images(true)
        .with_streaming(true)
        .with_thinking(true)
        .with_cache_control(true)
        .with_long_cache_retention(true);

    assert_eq!(caps.context_window, 200_000);
    assert_eq!(caps.max_output_tokens, 8_192);
    assert!(caps.supports_images);
    assert!(caps.supports_streaming);
    assert!(caps.supports_thinking);
    assert!(caps.supports_cache_control);
    assert!(caps.supports_long_cache_retention);
}

#[test]
fn model_capabilities_default_is_all_off() {
    let caps = ModelCapabilities::default();
    assert_eq!(caps.context_window, 0);
    assert_eq!(caps.max_output_tokens, 0);
    assert!(!caps.supports_images);
    assert!(!caps.supports_streaming);
    assert!(!caps.supports_thinking);
    assert!(!caps.supports_cache_control);
    assert!(!caps.supports_long_cache_retention);
}

#[test]
fn model_capabilities_builder_defaults_all_off() {
    let caps = ModelCapabilities::new(128_000, 4_096);
    assert_eq!(caps.context_window, 128_000);
    assert_eq!(caps.max_output_tokens, 4_096);
    assert!(!caps.supports_images);
    assert!(!caps.supports_streaming);
    assert!(!caps.supports_thinking);
    assert!(!caps.supports_cache_control);
    assert!(!caps.supports_long_cache_retention);
}

// ---------------------------------------------------------------------------
// Migration guard: ModelInfo has capabilities field, NOT flattened fields
// ---------------------------------------------------------------------------

#[test]
fn model_info_has_capabilities_not_flattened() {
    // The migration moved context_window, max_output_tokens, supports_images,
    // supports_streaming, and supports_thinking into `capabilities`, so the
    // model-level fields no longer exist. The following compiles only because
    // we access through `capabilities`:
    let provider = AnthropicProvider::new(None);
    let models = provider.models();
    let model = &models[0];

    // Compile-time proof: model.capabilities exists and has the 5 original
    // fields PLUS the two new cache fields.
    let _ctx: u64 = model.capabilities.context_window;
    let _max: u64 = model.capabilities.max_output_tokens;
    let _img: bool = model.capabilities.supports_images;
    let _str: bool = model.capabilities.supports_streaming;
    let _thk: bool = model.capabilities.supports_thinking;
    let _cc: bool = model.capabilities.supports_cache_control;
    let _lc: bool = model.capabilities.supports_long_cache_retention;
}

// ---------------------------------------------------------------------------
// Built-in Anthropic cache capability defaults
// ---------------------------------------------------------------------------

#[test]
fn anthropic_builtin_models_have_cache_capabilities() {
    // All three Claude 4.5 models must advertise cache-control and
    // long-cache-retention support so the provider can emit the
    // anthropic-beta prompt-caching header and cache_control markers.
    let provider = AnthropicProvider::new(None);
    for model in provider.models() {
        assert!(
            model.capabilities.supports_cache_control,
            "{} should support cache control",
            model.id
        );
        assert!(
            model.capabilities.supports_long_cache_retention,
            "{} should support long cache retention",
            model.id
        );
    }
}

// ---------------------------------------------------------------------------
// Registry capabilities() returns the model's capabilities directly
// ---------------------------------------------------------------------------

#[test]
fn registry_capabilities_returns_nested_capabilities() {
    let mut registry = ProviderRegistry::new();
    let provider = AnthropicProvider::new(None);
    registry.register(Box::new(provider));

    let caps = registry
        .capabilities("anthropic:claude-sonnet-4-5-20250514")
        .expect("resolve");
    assert_eq!(caps.context_window, 200_000);
    assert!(caps.supports_cache_control);
    assert!(caps.supports_long_cache_retention);
}

#[test]
fn non_anthropic_models_default_cache_to_false() {
    // Test that a custom model (non-Anthropic) defaults cache to false,
    // as verified through registry resolution.
    use opi_ai::provider::ModelInfo;
    use opi_ai::test_support::MockProvider;

    use opi_ai::test_support::text_response;

    let responses: Vec<Vec<opi_ai::stream::AssistantStreamEvent>> = vec![text_response("ok")];
    let mock = MockProvider::new_with_models(
        "test-prov",
        vec![ModelInfo::new(
            "test-model",
            "Test",
            opi_ai::WireApi::OpenAiCompletions,
            ModelCapabilities::new(100_000, 4_096).with_streaming(true),
        )],
        responses,
    );
    let mut registry = ProviderRegistry::new();
    registry.register(Box::new(mock));

    let caps = registry
        .capabilities("test-prov:test-model")
        .expect("resolve");
    assert!(
        !caps.supports_cache_control,
        "non-Anthropic should default false"
    );
    assert!(
        !caps.supports_long_cache_retention,
        "non-Anthropic should default false"
    );
}
