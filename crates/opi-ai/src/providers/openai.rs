//! OpenAI Chat Completions Provider profile.
//!
//! This module owns the OpenAI identity, endpoint, model catalog, compatibility
//! profile configuration, shared HTTP client and per-dispatch profile state.
//! The wire protocol implementation lives in crate::api::openai.

use std::collections::HashMap;
use std::sync::Arc;

use crate::api::shared::headers::ProviderHeaders;
use crate::auth::AuthInvalidPolicy;
use crate::http::HttpClient;
use crate::model_info::WireApi;
use crate::provider::ModelInfo;
use crate::provider::routes::{OpenAiModelCompatOverride, OpenAiRouteContext};
use crate::registry::ModelCapabilities;

// ---------------------------------------------------------------------------
// CompatConfig  - configuration points for OpenAI-compatible profiles
// ---------------------------------------------------------------------------

/// Configuration overrides for OpenAI-compatible provider profiles.
///
/// Task 20.36 (AIR-022/V-11) applies the single normalization rule: every
/// policy field is an explicit value layered over the adapter's frozen
/// provider inference (`detect_chat_baseline`). Downstream profiles set:
/// - `store`: `Omit`/`SendFalse` store-field policy
/// - `instruction_role`: `Auto`/`System`/`DeveloperForReasoning`/`Developer`
/// - `thinking_format` + `supports_reasoning_effort`: declared reasoning wire
///   format and standard effort-field support
/// - `max_tokens_field`: output-limit field name override
/// - `tool_result_name_field`: tool results carry a `name` field
/// - `usage_in_stream`: `stream_options.include_usage` policy
/// - `strict_tool_schema`: admit strict conversion for requested tool intents;
///   unconstrained tools carry `strict: false` on a capable wire
/// - `cache_key`: emit `prompt_cache_key` for OpenAI prompt-cache affinity
/// - `send_session_affinity_headers`: session-affinity header policy
/// - `require_assistant_after_tool_result`: legacy wire bridge projection
/// - `requires_thinking_as_text`: replay thinking as plain text
/// - `requires_reasoning_content_on_assistant_messages`: DeepSeek-style
///   empty `reasoning_content` on assistant messages
/// - `zai_tool_stream`/`vllm_priority`: explicit-only extras
/// - `thinking_token_budget_field`/`supports_thinking_token_budget`: the
///   reasoning token-budget request field
/// - `chat_template_kwargs`/`chat_template_args`: `$var` template values
/// - `chat_completions_path`: endpoint path override
///
/// The retired `system_role_override`, `supports_store`,
/// `supports_developer_role` and static `reasoning_effort` keys fail with
/// migration errors at every config position (catalog extension and product
/// TOML); request thinking intent and the model thinking-level map are the
/// only sources of reasoning wire output.
pub use crate::model_info::OpenAiCompletionsCompat as CompatConfig;

/// Per-model compatibility overrides.
///
/// A model entry may override a subset of the provider-level [`CompatConfig`].
/// At request-build time the provider resolves the effective config by layering
/// the frozen adapter inference, then the provider-level defaults, then a
/// model's override (model wins per field).
pub type ModelCompatOverride = OpenAiModelCompatOverride;

// OpenAiChatProvider
// ---------------------------------------------------------------------------

/// Canonical provider-facing name for the neutral OpenAI route context.
pub type OpenAiChatProvider = OpenAiRouteContext;

/// Built-in OpenAI Chat model metadata without credentials or HTTP construction.
pub fn model_catalog() -> Vec<ModelInfo> {
    vec![
        ModelInfo::new(
            "gpt-4o",
            "GPT-4o",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(128000, 16384)
                .with_images(true)
                .with_streaming(true),
        ),
        ModelInfo::new(
            "gpt-4o-mini",
            "GPT-4o Mini",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(128000, 16384)
                .with_images(true)
                .with_streaming(true),
        ),
        ModelInfo::new(
            "o3",
            "o3",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(200000, 100000)
                .with_images(true)
                .with_streaming(true)
                .with_thinking(true),
        ),
        ModelInfo::new(
            "o4-mini",
            "o4-mini",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(200000, 100000)
                .with_images(true)
                .with_streaming(true)
                .with_thinking(true),
        ),
    ]
}

impl OpenAiRouteContext {
    #[cfg(feature = "builtins")]
    pub(crate) fn with_wire_model(mut self, deployment: String) -> Self {
        self.wire_model_override = Some(deployment);
        self
    }

    pub fn new(base_url: Option<String>) -> Self {
        Self::new_with_compat(base_url, CompatConfig::default())
    }

    pub fn new_with_compat(base_url: Option<String>, compat: CompatConfig) -> Self {
        Self::with_auth(
            base_url,
            compat,
            "openai".into(),
            vec![],
            Arc::new(HttpClient::new()),
        )
        .with_auth_invalid_policy(AuthInvalidPolicy::Static)
    }

    /// Create with a shared HTTP client.
    pub fn with_client(
        base_url: Option<String>,
        provider_id: String,
        extra_headers: Vec<(String, String)>,
        client: Arc<HttpClient>,
    ) -> Self {
        Self::with_auth(
            base_url,
            CompatConfig::default(),
            provider_id,
            extra_headers,
            client,
        )
        .with_auth_invalid_policy(AuthInvalidPolicy::Static)
    }

    /// Build with provider route config. The secret arrives per-call via
    /// [`crate::auth::ResolvedAuth`] on `Provider::stream_prepared`.
    pub fn with_auth(
        base_url: Option<String>,
        compat: CompatConfig,
        provider_id: String,
        extra_headers: Vec<(String, String)>,
        client: Arc<HttpClient>,
    ) -> Self {
        let base_url = base_url.unwrap_or_else(|| "https://api.openai.com".into());
        let models = model_catalog()
            .into_iter()
            .map(|mut model| {
                if let crate::WireCompat::OpenAiCompletions(source) = &mut model.compat {
                    source.supports_mid_convo_system_messages = source
                        .supports_mid_convo_system_messages
                        .or(compat.supports_mid_convo_system_messages);
                    source.supports_mid_convo_tool_additions = source
                        .supports_mid_convo_tool_additions
                        .or(compat.supports_mid_convo_tool_additions);
                }
                model
            })
            .collect();
        let direct_prompt_cache_key = provider_id == "openai";
        Self {
            wire: crate::WireApi::OpenAiCompletions,
            base_url,
            models,
            compat,
            model_overrides: HashMap::new(),
            provider_id,
            headers: ProviderHeaders::default(),
            route_headers: extra_headers,
            catalog_compat: false,
            auth_invalid_policy: AuthInvalidPolicy::CredentialManaged,
            direct_prompt_cache_key,
            copilot_initiator: false,
            wire_model_override: None,
            client,
        }
    }

    /// Create a provider for an OpenAI-compatible profile (OpenRouter, Mistral, etc.).
    pub fn new_for_profile(
        base_url: String,
        provider_id: String,
        compat: CompatConfig,
        extra_headers: Vec<(String, String)>,
        models: Vec<ModelInfo>,
    ) -> Self {
        let models = models
            .into_iter()
            .map(|mut model| {
                if let crate::WireCompat::OpenAiCompletions(source) = &mut model.compat {
                    source.supports_mid_convo_system_messages = source
                        .supports_mid_convo_system_messages
                        .or(compat.supports_mid_convo_system_messages);
                    source.supports_mid_convo_tool_additions = source
                        .supports_mid_convo_tool_additions
                        .or(compat.supports_mid_convo_tool_additions);
                }
                model
            })
            .collect();
        Self {
            wire: crate::WireApi::OpenAiCompletions,
            base_url,
            models,
            compat,
            model_overrides: HashMap::new(),
            provider_id,
            headers: ProviderHeaders::default(),
            route_headers: extra_headers,
            catalog_compat: false,
            auth_invalid_policy: AuthInvalidPolicy::Static,
            direct_prompt_cache_key: false,
            copilot_initiator: false,
            wire_model_override: None,
            client: Arc::new(HttpClient::new()),
        }
    }

    /// Build a Chat Completions route for a mapped provider.
    pub fn for_route(
        default_base_url: Option<String>,
        provider_id: String,
        headers: ProviderHeaders,
        models: Vec<ModelInfo>,
        client: Arc<HttpClient>,
    ) -> Self {
        Self {
            wire: crate::WireApi::OpenAiCompletions,
            base_url: default_base_url.unwrap_or_else(|| "https://api.openai.com".into()),
            models,
            compat: CompatConfig::default(),
            model_overrides: HashMap::new(),
            provider_id,
            headers,
            route_headers: Vec::new(),
            catalog_compat: true,
            auth_invalid_policy: AuthInvalidPolicy::Static,
            direct_prompt_cache_key: false,
            copilot_initiator: false,
            wire_model_override: None,
            client,
        }
    }

    /// Override how this route classifies provider 401/403 responses.
    pub fn with_auth_invalid_policy(mut self, policy: AuthInvalidPolicy) -> Self {
        self.auth_invalid_policy = policy;
        self
    }

    /// Enable the GitHub Copilot `X-Initiator` request header. The value is
    /// derived inside each stream from the last conversation message: user
    /// prompts map to `user`; assistant/tool continuations map to `agent`.
    pub fn with_copilot_initiator(mut self) -> Self {
        self.copilot_initiator = true;
        self
    }

    /// Replace the HTTP client with a shared one (for proxy configuration
    /// and connection pooling).
    pub fn with_shared_client(self, client: Arc<HttpClient>) -> Self {
        Self { client, ..self }
    }

    /// Attach per-model compatibility overrides. Model-level
    /// overrides win over the provider-level [`CompatConfig`] for the models
    /// they declare; models without an entry inherit the provider default.
    pub fn with_model_overrides(mut self, overrides: HashMap<String, ModelCompatOverride>) -> Self {
        self.model_overrides = overrides;
        self
    }
}
