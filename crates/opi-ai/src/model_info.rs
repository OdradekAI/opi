//! Public model routing, capability, thinking, compatibility, and pricing metadata.
//!
//! [`WireApi`] is the exact request protocol, separate from the normalized
//! assistant-message `ApiKind`. Every [`ModelInfo`] has one wire, a matching
//! tagged [`WireCompat`], a thinking-level map, and optional pricing with
//! deterministic input-token threshold tiers.

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::stream::{CostBreakdown, Pricing, Usage, calculate_cost};

/// Arbitrary sampling parameters merged into the request body as-is.
///
/// Keys are provider-specific (for example `top_p`, `top_k`, `min_p`);
/// resolution order is per key: model base < effective thinking level <
/// request (see [`resolve_sampling_params`]). Only OpenAI-compatible
/// adapters apply the merged map; other adapters ignore it.
pub type SamplingParams = serde_json::Map<String, serde_json::Value>;

/// Capabilities of a resolved model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ModelCapabilities {
    pub context_window: u64,
    pub max_output_tokens: u64,
    pub supports_images: bool,
    pub supports_streaming: bool,
    pub supports_thinking: bool,
    pub supports_cache_control: bool,
    pub supports_long_cache_retention: bool,
}

impl ModelCapabilities {
    pub fn new(context_window: u64, max_output_tokens: u64) -> Self {
        Self {
            context_window,
            max_output_tokens,
            supports_images: false,
            supports_streaming: false,
            supports_thinking: false,
            supports_cache_control: false,
            supports_long_cache_retention: false,
        }
    }

    pub fn with_images(mut self, value: bool) -> Self {
        self.supports_images = value;
        self
    }

    pub fn with_streaming(mut self, value: bool) -> Self {
        self.supports_streaming = value;
        self
    }

    pub fn with_thinking(mut self, value: bool) -> Self {
        self.supports_thinking = value;
        self
    }

    pub fn with_cache_control(mut self, value: bool) -> Self {
        self.supports_cache_control = value;
        self
    }

    pub fn with_long_cache_retention(mut self, value: bool) -> Self {
        self.supports_long_cache_retention = value;
        self
    }
}

impl Default for ModelCapabilities {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
/// Exact request wire used to dispatch a model.
///
/// Custom mapped providers may select Anthropic Messages, OpenAI Completions,
/// or OpenAI Responses. `openai-codex-responses` is subscription-specific and
/// reserved for the built-in OpenAI Codex provider.
pub enum WireApi {
    #[serde(rename = "anthropic-messages")]
    AnthropicMessages,
    #[serde(rename = "openai-completions")]
    OpenAiCompletions,
    #[serde(rename = "openai-responses")]
    OpenAiResponses,
    #[serde(rename = "openai-codex-responses")]
    OpenAiCodexResponses,
    #[serde(rename = "google-generative-ai")]
    GoogleGenerativeAi,
    #[serde(rename = "google-vertex")]
    GoogleVertex,
    #[serde(rename = "bedrock-converse-stream")]
    BedrockConverseStream,
    #[serde(rename = "azure-openai-completions")]
    AzureOpenAiCompletions,
}

impl WireApi {
    pub const ALL: [Self; 8] = [
        Self::AnthropicMessages,
        Self::OpenAiCompletions,
        Self::OpenAiResponses,
        Self::OpenAiCodexResponses,
        Self::GoogleGenerativeAi,
        Self::GoogleVertex,
        Self::BedrockConverseStream,
        Self::AzureOpenAiCompletions,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AnthropicMessages => "anthropic-messages",
            Self::OpenAiCompletions => "openai-completions",
            Self::OpenAiResponses => "openai-responses",
            Self::OpenAiCodexResponses => "openai-codex-responses",
            Self::GoogleGenerativeAi => "google-generative-ai",
            Self::GoogleVertex => "google-vertex",
            Self::BedrockConverseStream => "bedrock-converse-stream",
            Self::AzureOpenAiCompletions => "azure-openai-completions",
        }
    }
}

impl fmt::Display for WireApi {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for WireApi {
    type Err = ParseWireApiError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|wire| wire.as_str() == value)
            .ok_or_else(|| ParseWireApiError(value.to_owned()))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("unknown wire API '{0}'")]
pub struct ParseWireApiError(String);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThinkingLevel {
    None,
    Minimal,
    Low,
    Medium,
    High,
    #[serde(rename = "xhigh")]
    XHigh,
    Max,
}

impl ThinkingLevel {
    pub const ALL: [Self; 7] = [
        Self::None,
        Self::Minimal,
        Self::Low,
        Self::Medium,
        Self::High,
        Self::XHigh,
        Self::Max,
    ];

    pub const fn wire_name(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Minimal => Some("minimal"),
            Self::Low => Some("low"),
            Self::Medium => Some("medium"),
            Self::High => Some("high"),
            Self::XHigh => Some("xhigh"),
            Self::Max => Some("max"),
        }
    }
}

impl FromStr for ThinkingLevel {
    type Err = UnsupportedThinkingLevel;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "off" | "none" => Ok(Self::None),
            "minimal" => Ok(Self::Minimal),
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "xhigh" => Ok(Self::XHigh),
            "max" => Ok(Self::Max),
            _ => Err(UnsupportedThinkingLevel {
                level: value.to_owned(),
            }),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ThinkingLevelMapping {
    Identity,
    Mapped(String),
    Unsupported,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThinkingLevelMap {
    mappings: BTreeMap<ThinkingLevel, ThinkingLevelMapping>,
}

impl ThinkingLevelMap {
    pub fn disabled() -> Self {
        Self {
            mappings: ThinkingLevel::ALL
                .into_iter()
                .map(|level| {
                    let mapping = if level == ThinkingLevel::None {
                        ThinkingLevelMapping::Identity
                    } else {
                        ThinkingLevelMapping::Unsupported
                    };
                    (level, mapping)
                })
                .collect(),
        }
    }

    pub fn reasoning_default() -> Self {
        Self {
            mappings: ThinkingLevel::ALL
                .into_iter()
                .map(|level| {
                    let mapping = match level {
                        ThinkingLevel::None
                        | ThinkingLevel::Minimal
                        | ThinkingLevel::Low
                        | ThinkingLevel::Medium
                        | ThinkingLevel::High => ThinkingLevelMapping::Identity,
                        ThinkingLevel::XHigh | ThinkingLevel::Max => {
                            ThinkingLevelMapping::Unsupported
                        }
                    };
                    (level, mapping)
                })
                .collect(),
        }
    }

    pub fn with_mapping(mut self, level: ThinkingLevel, mapping: ThinkingLevelMapping) -> Self {
        self.mappings.insert(level, mapping);
        self
    }

    pub fn resolve(
        &self,
        level: ThinkingLevel,
    ) -> Result<Option<String>, UnsupportedThinkingLevel> {
        match self
            .mappings
            .get(&level)
            .unwrap_or(&ThinkingLevelMapping::Unsupported)
        {
            ThinkingLevelMapping::Identity => Ok(level.wire_name().map(str::to_owned)),
            ThinkingLevelMapping::Mapped(value) => Ok(Some(value.clone())),
            ThinkingLevelMapping::Unsupported => Err(UnsupportedThinkingLevel {
                level: format!("{level:?}").to_ascii_lowercase(),
            }),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("thinking level '{level}' is unsupported")]
pub struct UnsupportedThinkingLevel {
    pub level: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnthropicMessagesCompat {
    pub supports_eager_tool_input_streaming: bool,
    pub force_adaptive_thinking: bool,
    pub supports_temperature: bool,
    /// Whether the wire accepts strict tool schemas (task 20.11, V-34):
    /// `json_schema` require fails typed before dispatch when false.
    pub supports_strict_tools: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OpenAiCompletionsCompat {
    pub system_role_override: Option<String>,
    pub max_tokens_field: String,
    pub tool_result_name_field: bool,
    pub usage_in_stream: bool,
    pub strict_tool_schema: bool,
    pub reasoning_effort: Option<String>,
    pub cache_key: Option<String>,
    pub send_session_affinity_headers: bool,
    pub require_assistant_after_tool_result: bool,
    pub chat_completions_path: String,
    pub supports_store: bool,
    pub supports_developer_role: bool,
    pub supports_reasoning_effort: bool,
}

impl Default for OpenAiCompletionsCompat {
    fn default() -> Self {
        Self {
            system_role_override: None,
            max_tokens_field: "max_tokens".into(),
            tool_result_name_field: false,
            usage_in_stream: false,
            strict_tool_schema: false,
            reasoning_effort: None,
            cache_key: None,
            send_session_affinity_headers: false,
            require_assistant_after_tool_result: false,
            chat_completions_path: "/v1/chat/completions".into(),
            supports_store: false,
            supports_developer_role: false,
            supports_reasoning_effort: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OpenAiResponsesCompat {
    pub store: Option<bool>,
    pub reasoning_effort: Option<String>,
    pub strict_tools: bool,
    pub responses_path: String,
    pub send_session_id_header: bool,
}

impl Default for OpenAiResponsesCompat {
    fn default() -> Self {
        Self {
            store: None,
            reasoning_effort: None,
            strict_tools: false,
            responses_path: "/v1/responses".into(),
            send_session_id_header: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OpenAiCodexResponsesCompat {
    pub responses_path: String,
    pub derive_account_id: bool,
}

impl Default for OpenAiCodexResponsesCompat {
    fn default() -> Self {
        Self {
            responses_path: "/codex/responses".into(),
            derive_account_id: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum WireCompat {
    AnthropicMessages(AnthropicMessagesCompat),
    OpenAiCompletions(OpenAiCompletionsCompat),
    OpenAiResponses(OpenAiResponsesCompat),
    OpenAiCodexResponses(OpenAiCodexResponsesCompat),
    GoogleGenerativeAi,
    GoogleVertex,
    BedrockConverseStream,
    AzureOpenAiCompletions,
}

impl WireCompat {
    pub fn wire_api(&self) -> WireApi {
        match self {
            Self::AnthropicMessages(_) => WireApi::AnthropicMessages,
            Self::OpenAiCompletions(_) => WireApi::OpenAiCompletions,
            Self::OpenAiResponses(_) => WireApi::OpenAiResponses,
            Self::OpenAiCodexResponses(_) => WireApi::OpenAiCodexResponses,
            Self::GoogleGenerativeAi => WireApi::GoogleGenerativeAi,
            Self::GoogleVertex => WireApi::GoogleVertex,
            Self::BedrockConverseStream => WireApi::BedrockConverseStream,
            Self::AzureOpenAiCompletions => WireApi::AzureOpenAiCompletions,
        }
    }

    fn default_for(wire_api: WireApi) -> Self {
        match wire_api {
            WireApi::AnthropicMessages => Self::AnthropicMessages(Default::default()),
            WireApi::OpenAiCompletions => Self::OpenAiCompletions(Default::default()),
            WireApi::OpenAiResponses => Self::OpenAiResponses(Default::default()),
            WireApi::OpenAiCodexResponses => Self::OpenAiCodexResponses(Default::default()),
            WireApi::GoogleGenerativeAi => Self::GoogleGenerativeAi,
            WireApi::GoogleVertex => Self::GoogleVertex,
            WireApi::BedrockConverseStream => Self::BedrockConverseStream,
            WireApi::AzureOpenAiCompletions => Self::AzureOpenAiCompletions,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelPricing {
    pub base: Pricing,
    pub tiers: Vec<PricingTier>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PricingTier {
    pub input_tokens_above: u64,
    pub pricing: Pricing,
}

impl ModelPricing {
    pub fn try_new(base: Pricing, tiers: Vec<PricingTier>) -> Result<Self, ModelInfoError> {
        let pricing = Self { base, tiers };
        pricing.validate()?;
        Ok(pricing)
    }

    pub fn validate(&self) -> Result<(), ModelInfoError> {
        validate_pricing(&self.base)?;
        for tier in &self.tiers {
            validate_pricing(&tier.pricing)?;
        }
        Ok(())
    }

    pub fn effective(&self, input_tokens: u64) -> Pricing {
        self.effective_for_prompt_tokens(input_tokens)
    }

    /// Tier selection over one request's usage (AIR-062).
    ///
    /// Counts `input + cache_read + cache_write` tokens only, picks the
    /// highest threshold that was strictly exceeded, and keeps the first of
    /// equal duplicate thresholds; the chosen rates apply to the whole
    /// request. Zero thresholds, duplicates and arbitrary input ordering are
    /// accepted; only the individual rates are validated.
    pub fn effective_for_usage(&self, usage: &Usage) -> Pricing {
        let prompt_tokens = u64::from(usage.input_tokens)
            + u64::from(usage.cache_read_tokens)
            + u64::from(usage.cache_write_tokens);
        self.effective_for_prompt_tokens(prompt_tokens)
    }

    fn effective_for_prompt_tokens(&self, prompt_tokens: u64) -> Pricing {
        let mut rates = self.base;
        let mut matched_threshold = None;
        for tier in &self.tiers {
            if prompt_tokens > tier.input_tokens_above
                && matched_threshold.is_none_or(|matched| tier.input_tokens_above > matched)
            {
                rates = tier.pricing;
                matched_threshold = Some(tier.input_tokens_above);
            }
        }
        rates
    }
}

/// Resolve sampling parameters for one request (AIR-062).
///
/// Per-key merge with model base < effective thinking level < request
/// precedence. Returns `None` only when every source is empty.
pub fn resolve_sampling_params(
    model: &ModelInfo,
    effective_level: ThinkingLevel,
    request: &SamplingParams,
) -> Option<SamplingParams> {
    let level_params = model
        .sampling_params_by_thinking_level
        .get(&effective_level);
    if model.sampling_params.is_empty()
        && level_params.is_none_or(|params| params.is_empty())
        && request.is_empty()
    {
        return None;
    }
    let mut merged = model.sampling_params.clone();
    if let Some(level_params) = level_params {
        for (key, value) in level_params {
            merged.insert(key.clone(), value.clone());
        }
    }
    for (key, value) in request {
        merged.insert(key.clone(), value.clone());
    }
    Some(merged)
}

/// Tiered request cost with service-tier multiplier (AIR-062).
///
/// Selects rates through [`ModelPricing::effective_for_usage`], applies the
/// base arithmetic (1-hour subset at twice the input rate, never double
/// counted), and scales every component by the response-reported service
/// tier. The explicit source exception: OpenRouter image `parse_usage` keeps
/// base model rates without tiers — image callers use
/// [`crate::stream::calculate_cost`] directly.
pub fn calculate_cost_tiered(
    usage: &Usage,
    pricing: &ModelPricing,
    model_id: &str,
) -> CostBreakdown {
    let rates = pricing.effective_for_usage(usage);
    let cost = calculate_cost(usage, &rates);
    let multiplier = usage
        .service_tier
        .map_or(1.0, |tier| tier.cost_multiplier(model_id));
    if multiplier == 1.0 {
        return cost;
    }
    CostBreakdown {
        input_cost: cost.input_cost * multiplier,
        output_cost: cost.output_cost * multiplier,
        cache_read_cost: cost.cache_read_cost * multiplier,
        cache_write_cost: cost.cache_write_cost * multiplier,
    }
}

fn validate_pricing(pricing: &Pricing) -> Result<(), ModelInfoError> {
    let rates = [
        pricing.input_cost_per_mtok,
        pricing.output_cost_per_mtok,
        pricing.cache_read_cost_per_mtok,
        pricing.cache_write_cost_per_mtok,
    ];
    if rates.iter().any(|rate| !rate.is_finite() || *rate < 0.0) {
        return Err(ModelInfoError::InvalidPricing(
            "pricing rates must be finite and non-negative".into(),
        ));
    }
    Ok(())
}

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
/// Complete public metadata for one model in a provider catalog.
///
/// `wire_api` selects the concrete route. `compat` must carry the same wire
/// tag, `thinking_level_map` maps or rejects each public thinking level, and
/// `pricing` chooses a tier only when input tokens are strictly greater than
/// that tier's threshold.
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
    pub wire_api: WireApi,
    pub capabilities: ModelCapabilities,
    pub base_url: Option<String>,
    pub thinking_level_map: ThinkingLevelMap,
    pub compat: WireCompat,
    pub pricing: Option<ModelPricing>,
    /// Model-base sampling parameters merged under the effective level and
    /// per-request values.
    pub sampling_params: SamplingParams,
    /// Sampling parameter overrides selected by the effective thinking level.
    pub sampling_params_by_thinking_level: BTreeMap<ThinkingLevel, SamplingParams>,
}

impl ModelInfo {
    pub fn new(
        id: impl Into<String>,
        display_name: impl Into<String>,
        wire_api: WireApi,
        capabilities: ModelCapabilities,
    ) -> Self {
        let thinking_level_map = if capabilities.supports_thinking {
            ThinkingLevelMap::reasoning_default()
        } else {
            ThinkingLevelMap::disabled()
        };
        Self {
            id: id.into(),
            display_name: display_name.into(),
            wire_api,
            capabilities,
            base_url: None,
            thinking_level_map,
            compat: WireCompat::default_for(wire_api),
            pricing: None,
            sampling_params: SamplingParams::new(),
            sampling_params_by_thinking_level: BTreeMap::new(),
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    pub fn with_thinking_level_map(mut self, map: ThinkingLevelMap) -> Self {
        self.thinking_level_map = map;
        self
    }

    pub fn with_compat(mut self, compat: WireCompat) -> Result<Self, ModelInfoError> {
        let compat_wire = compat.wire_api();
        if compat_wire != self.wire_api {
            return Err(ModelInfoError::WireCompatMismatch {
                model_id: self.id,
                wire_api: self.wire_api,
                compat_wire,
            });
        }
        self.compat = compat;
        Ok(self)
    }

    pub fn with_pricing(mut self, pricing: ModelPricing) -> Result<Self, ModelInfoError> {
        pricing.validate()?;
        self.pricing = Some(pricing);
        Ok(self)
    }

    /// Attach model-base sampling parameters.
    pub fn with_sampling_params(mut self, params: SamplingParams) -> Self {
        self.sampling_params = params;
        self
    }

    /// Attach sampling parameter overrides for one thinking level.
    pub fn with_sampling_level_params(
        mut self,
        level: ThinkingLevel,
        params: SamplingParams,
    ) -> Self {
        self.sampling_params_by_thinking_level.insert(level, params);
        self
    }

    pub fn validate(&self) -> Result<(), ModelInfoError> {
        if self.capabilities.context_window == 0 {
            return Err(ModelInfoError::InvalidCapabilities {
                model_id: self.id.clone(),
                field: "context_window",
            });
        }
        if self.capabilities.max_output_tokens == 0 {
            return Err(ModelInfoError::InvalidCapabilities {
                model_id: self.id.clone(),
                field: "max_output_tokens",
            });
        }
        if !self.capabilities.supports_thinking
            && ThinkingLevel::ALL
                .into_iter()
                .filter(|level| *level != ThinkingLevel::None)
                .any(|level| self.thinking_level_map.resolve(level).is_ok())
        {
            return Err(ModelInfoError::IncoherentCapabilities {
                model_id: self.id.clone(),
                detail: "thinking levels are enabled while supports_thinking is false",
            });
        }
        if self.capabilities.supports_long_cache_retention
            && !self.capabilities.supports_cache_control
        {
            return Err(ModelInfoError::IncoherentCapabilities {
                model_id: self.id.clone(),
                detail: "long cache retention requires cache-control support",
            });
        }
        let compat_wire = self.compat.wire_api();
        if compat_wire != self.wire_api {
            return Err(ModelInfoError::WireCompatMismatch {
                model_id: self.id.clone(),
                wire_api: self.wire_api,
                compat_wire,
            });
        }
        if let Some(pricing) = &self.pricing {
            pricing.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ModelInfoError {
    #[error("model '{model_id}' capability {field} must be greater than zero")]
    InvalidCapabilities {
        model_id: String,
        field: &'static str,
    },
    #[error("model '{model_id}' has incoherent capabilities: {detail}")]
    IncoherentCapabilities {
        model_id: String,
        detail: &'static str,
    },
    #[error("model '{model_id}' wire {wire_api} does not match compatibility wire {compat_wire}")]
    WireCompatMismatch {
        model_id: String,
        wire_api: WireApi,
        compat_wire: WireApi,
    },
    #[error("invalid model pricing: {0}")]
    InvalidPricing(String),
}
