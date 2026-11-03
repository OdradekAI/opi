//! Public model routing, capability, thinking, compatibility, and pricing metadata.
//!
//! [`WireApi`] is the exact request protocol, separate from the normalized
//! assistant-message `ApiKind`. Every [`ModelInfo`] has one wire, a matching
//! tagged [`WireCompat`], a thinking-level map, and optional pricing with
//! deterministic input-token threshold tiers.

use std::cmp::Ordering;
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
    PiMessages,
    #[serde(rename = "azure-openai-completions")]
    AzureOpenAiCompletions,
    #[serde(rename = "azure-openai-responses")]
    AzureOpenAiResponses,
    #[serde(rename = "mistral-conversations")]
    MistralConversations,
}

impl WireApi {
    pub const ALL: [Self; 11] = [
        Self::AnthropicMessages,
        Self::OpenAiCompletions,
        Self::OpenAiResponses,
        Self::OpenAiCodexResponses,
        Self::GoogleGenerativeAi,
        Self::GoogleVertex,
        Self::BedrockConverseStream,
        Self::PiMessages,
        Self::AzureOpenAiCompletions,
        Self::AzureOpenAiResponses,
        Self::MistralConversations,
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
            Self::PiMessages => "pi-messages",
            Self::AzureOpenAiCompletions => "azure-openai-completions",
            Self::AzureOpenAiResponses => "azure-openai-responses",
            Self::MistralConversations => "mistral-conversations",
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

    pub(crate) fn mapping(&self, level: ThinkingLevel) -> &ThinkingLevelMapping {
        self.mappings
            .get(&level)
            .unwrap_or(&ThinkingLevelMapping::Unsupported)
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

    /// Whether any level carries an explicit (source-provided) mapping, as
    /// opposed to the synthetic identity default. Adapters whose wire keeps
    /// distinct branches for mapped versus mapless reasoning models branch
    /// on this (task 20.43).
    pub fn has_explicit_mappings(&self) -> bool {
        self.mappings
            .values()
            .any(|mapping| matches!(mapping, ThinkingLevelMapping::Mapped(_)))
    }

    /// Clamp one requested level to the nearest supported level: exact match
    /// first, then the nearest higher level, then the nearest lower level
    /// (Pi `clampThinkingLevel`; used by the neutral simple entry so an
    /// unavailable level degrades instead of failing).
    pub fn nearest_supported(&self, level: ThinkingLevel) -> Option<ThinkingLevel> {
        let supported = |level: ThinkingLevel| self.resolve(level).is_ok();
        if supported(level) {
            return Some(level);
        }
        let order = ThinkingLevel::ALL;
        let requested = order.iter().position(|candidate| *candidate == level)?;
        for candidate in &order[requested..] {
            if supported(*candidate) {
                return Some(*candidate);
            }
        }
        order[..requested]
            .iter()
            .rev()
            .find(|candidate| supported(**candidate))
            .copied()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("thinking level '{level}' is unsupported")]
pub struct UnsupportedThinkingLevel {
    pub level: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BedrockConverseStreamCompat {
    /// Whether the wire accepts strict tool schemas (task 20.42, V-34,
    /// pi `supportsStrictMode`): `json_schema` require fails typed before
    /// dispatch when false, and strict tool specs carry `"strict": true`.
    pub supports_strict_tools: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnthropicMessagesCompat {
    pub supports_eager_tool_input_streaming: bool,
    pub force_adaptive_thinking: bool,
    pub supports_temperature: bool,
    /// Whether the wire accepts strict tool schemas (task 20.11, V-34):
    /// `json_schema` require fails typed before dispatch when false.
    pub supports_strict_tools: bool,
    /// Whether system messages stay native mid-conversation (task 20.39,
    /// pi `supportsMidConvoSystemMessages`, default false).
    pub supports_mid_convo_system_messages: bool,
    /// Whether the model accepts `tool_addition`/`tool_removal` blocks
    /// (task 20.39, pi `supportsMidConvoToolChanges`, default false);
    /// requires `supports_mid_convo_system_messages`.
    pub supports_mid_convo_tool_changes: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatStoreField {
    Omit,
    SendFalse,
}

impl std::str::FromStr for ChatStoreField {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "omit" => Ok(Self::Omit),
            "send_false" => Ok(Self::SendFalse),
            other => Err(format!(
                "unknown chat store policy '{other}' (expected omit|send_false)"
            )),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatInstructionRole {
    Auto,
    System,
    DeveloperForReasoning,
    Developer,
}

impl std::str::FromStr for ChatInstructionRole {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "auto" => Ok(Self::Auto),
            "system" => Ok(Self::System),
            "developer_for_reasoning" => Ok(Self::DeveloperForReasoning),
            "developer" => Ok(Self::Developer),
            other => Err(format!(
                "unknown chat instruction role '{other}' \
                 (expected auto|system|developer_for_reasoning|developer)"
            )),
        }
    }
}

impl ChatThinkingFormat {
    pub fn wire_name(&self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::OpenRouter => "openrouter",
            Self::DeepSeek => "deepseek",
            Self::Together => "together",
            Self::Baseten => "baseten",
            Self::Zai => "zai",
            Self::Qwen => "qwen",
            Self::QwenChatTemplate => "qwen-chat-template",
            Self::ChatTemplate => "chat-template",
            Self::StringThinking => "string-thinking",
            Self::AntLing => "ant-ling",
        }
    }
}

impl std::str::FromStr for ChatThinkingFormat {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "openai" => Self::OpenAi,
            "openrouter" => Self::OpenRouter,
            "deepseek" => Self::DeepSeek,
            "together" => Self::Together,
            "baseten" => Self::Baseten,
            "zai" => Self::Zai,
            "qwen" => Self::Qwen,
            "qwen-chat-template" => Self::QwenChatTemplate,
            "chat-template" => Self::ChatTemplate,
            "string-thinking" => Self::StringThinking,
            "ant-ling" => Self::AntLing,
            other => {
                return Err(format!("unknown chat thinking format '{other}'"));
            }
        })
    }
}
/// Declared Chat reasoning wire formats (task 20.36, AIR-022/V-11). The
/// closed set is frozen from the pinned Pi 1.3.0 compat schema; each variant
/// owns its on/off/null/omission wire projection in the Chat adapter.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum ChatThinkingFormat {
    #[default]
    OpenAi,
    OpenRouter,
    DeepSeek,
    Together,
    Baseten,
    Zai,
    Qwen,
    QwenChatTemplate,
    ChatTemplate,
    StringThinking,
    AntLing,
}

/// Top-level request field used by OpenAI-compatible servers to cap reasoning
/// tokens: `thinking_token_budget` (vLLM), `thinking_budget` (Qwen/DashScope/
/// SGLang) and `thinking_budget_tokens` (llama.cpp).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatThinkingTokenBudgetField {
    ThinkingTokenBudget,
    ThinkingBudget,
    ThinkingBudgetTokens,
}

impl ChatThinkingTokenBudgetField {
    pub fn wire_name(&self) -> &'static str {
        match self {
            Self::ThinkingTokenBudget => "thinking_token_budget",
            Self::ThinkingBudget => "thinking_budget",
            Self::ThinkingBudgetTokens => "thinking_budget_tokens",
        }
    }
}

/// One `chat_template_kwargs`/`chat_template_args` value: a wire literal or a
/// `$var` reference resolved at request build time against the request's
/// thinking intent (`thinking.enabled`, `thinking.budget` or the mapped
/// level/off value). `omit_when_off` drops the key when no effort is set.
#[derive(Clone, Debug, PartialEq)]
pub enum ChatTemplateKwargValue {
    Literal(serde_json::Value),
    Var {
        var: ChatTemplateKwargVar,
        omit_when_off: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChatTemplateKwargVar {
    ThinkingEnabled,
    ThinkingEffort,
    ThinkingBudget,
}

// Derived structs also accept positional arrays. Source controls require objects.
macro_rules! object_control_deserialize {
    ($name:ident { $($field:ident: $ty:ty,)* }) => {
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where D: serde::Deserializer<'de> {
                let value = serde_json::Value::deserialize(deserializer)?;
                if !value.is_object() {
                    return Err(serde::de::Error::custom("expected an object"));
                }
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Fields { $($field: $ty,)* }
                let fields: Fields = serde_json::from_value(value).map_err(serde::de::Error::custom)?;
                Ok(Self { $($field: fields.$field,)* })
            }
        }
    };
}

/// Header dialect used for a caller's full session identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionAffinityFormat {
    #[serde(rename = "openai")]
    OpenAi,
    #[serde(rename = "openai-nosession")]
    OpenAiNoSession,
    #[serde(rename = "openrouter")]
    OpenRouter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CacheControlFormat {
    Anthropic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OpenRouterDataCollection {
    Deny,
    Allow,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NumberOrString {
    Number(serde_json::Number),
    String(String),
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PercentileCutoffs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p50: Option<serde_json::Number>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p75: Option<serde_json::Number>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p90: Option<serde_json::Number>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p99: Option<serde_json::Number>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NumberOrPercentileCutoffs {
    Number(serde_json::Number),
    Percentiles(PercentileCutoffs),
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum OpenRouterSort {
    String(String),
    Structured {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        by: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        partition: Option<Option<String>>,
    },
}

impl<'de> Deserialize<'de> for OpenRouterSort {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        if let serde_json::Value::String(value) = value {
            return Ok(Self::String(value));
        }
        let serde_json::Value::Object(mut object) = value else {
            return Err(serde::de::Error::custom("expected a string or sort object"));
        };
        let by = object
            .remove("by")
            .map(serde_json::from_value::<Option<String>>)
            .transpose()
            .map_err(serde::de::Error::custom)?
            .flatten();
        let partition = object
            .remove("partition")
            .map(serde_json::from_value::<Option<String>>)
            .transpose()
            .map_err(serde::de::Error::custom)?;
        if !object.is_empty() {
            return Err(serde::de::Error::custom("unknown sort field"));
        }
        Ok(Self::Structured { by, partition })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OpenRouterMaxPrice {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<NumberOrString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion: Option<NumberOrString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<NumberOrString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio: Option<NumberOrString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request: Option<NumberOrString>,
}

/// Supplied routing fields are forwarded without filling schema-hint defaults.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OpenRouterRouting {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_fallbacks: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub require_parameters: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_collection: Option<OpenRouterDataCollection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zdr: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enforce_distillable_text: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub only: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantizations: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort: Option<OpenRouterSort>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_price: Option<OpenRouterMaxPrice>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_min_throughput: Option<NumberOrPercentileCutoffs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_max_latency: Option<NumberOrPercentileCutoffs>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VercelGatewayRouting {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub only: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<Vec<String>>,
}

object_control_deserialize!(PercentileCutoffs {
    p50: Option<serde_json::Number>,
    p75: Option<serde_json::Number>,
    p90: Option<serde_json::Number>,
    p99: Option<serde_json::Number>,
});

object_control_deserialize!(OpenRouterMaxPrice {
    prompt: Option<NumberOrString>,
    completion: Option<NumberOrString>,
    image: Option<NumberOrString>,
    audio: Option<NumberOrString>,
    request: Option<NumberOrString>,
});

object_control_deserialize!(OpenRouterRouting {
    allow_fallbacks: Option<bool>,
    require_parameters: Option<bool>,
    data_collection: Option<OpenRouterDataCollection>,
    zdr: Option<bool>,
    enforce_distillable_text: Option<bool>,
    order: Option<Vec<String>>,
    only: Option<Vec<String>>,
    ignore: Option<Vec<String>>,
    quantizations: Option<Vec<String>>,
    sort: Option<OpenRouterSort>,
    max_price: Option<OpenRouterMaxPrice>,
    preferred_min_throughput: Option<NumberOrPercentileCutoffs>,
    preferred_max_latency: Option<NumberOrPercentileCutoffs>,
});

object_control_deserialize!(VercelGatewayRouting {
    only: Option<Vec<String>>,
    order: Option<Vec<String>>,
});

#[derive(Clone, Debug, PartialEq)]
pub struct OpenAiCompletionsCompat {
    pub supports_mid_convo_system_messages: Option<bool>,
    pub supports_mid_convo_tool_additions: Option<bool>,
    pub supports_finish_reason: Option<bool>,
    pub supports_grammar_tools: Option<bool>,
    pub open_router_routing: Option<OpenRouterRouting>,
    pub vercel_gateway_routing: Option<VercelGatewayRouting>,
    pub cache_control_format: Option<CacheControlFormat>,
    pub session_affinity_format: Option<SessionAffinityFormat>,
    pub supports_long_cache_retention: Option<bool>,
    /// Normalized store-field policy (`Omit`/`SendFalse`); `None` follows the
    /// adapter's frozen provider inference. Replaces retired `supports_store`.
    pub store: Option<ChatStoreField>,
    /// Normalized instruction-role policy; `None` (Auto) follows inference.
    /// Replaces retired `supports_developer_role` + `system_role_override`.
    pub instruction_role: Option<ChatInstructionRole>,
    /// Declared reasoning wire format; `None` follows inference.
    pub thinking_format: Option<ChatThinkingFormat>,
    /// Whether the endpoint accepts the standard effort field for the active
    /// format; `None` follows inference. Its `false` never disables a format's
    /// own native switch (zai/qwen keep their toggles).
    pub supports_reasoning_effort: Option<bool>,
    /// Whether `stream_options.include_usage` is sent; `None` follows
    /// inference (enabled on every frozen baseline).
    pub usage_in_stream: Option<bool>,
    /// Explicit output-limit field name; `None` follows inference
    /// (`max_completion_tokens` on standard endpoints, `max_tokens` on the
    /// frozen legacy-compatible set).
    pub max_tokens_field: Option<String>,
    pub tool_result_name_field: bool,
    pub strict_tool_schema: bool,
    pub cache_key: Option<String>,
    pub send_session_affinity_headers: Option<bool>,
    pub require_assistant_after_tool_result: bool,
    pub requires_thinking_as_text: bool,
    pub requires_reasoning_content_on_assistant_messages: Option<bool>,
    /// Explicit-only zai `tool_stream` flag; never inferred.
    pub zai_tool_stream: bool,
    /// Explicit-only vLLM priority number; never inferred.
    pub vllm_priority: Option<serde_json::Number>,
    pub thinking_token_budget_field: Option<ChatThinkingTokenBudgetField>,
    /// Alias for `thinking_token_budget_field = ThinkingTokenBudget` (vLLM).
    pub supports_thinking_token_budget: bool,
    pub chat_template_kwargs: Option<std::collections::BTreeMap<String, ChatTemplateKwargValue>>,
    pub chat_template_args: Option<std::collections::BTreeMap<String, ChatTemplateKwargValue>>,
    pub chat_completions_path: String,
}

impl Default for OpenAiCompletionsCompat {
    fn default() -> Self {
        Self {
            supports_mid_convo_system_messages: None,
            supports_mid_convo_tool_additions: None,
            supports_finish_reason: None,
            supports_grammar_tools: None,
            open_router_routing: None,
            vercel_gateway_routing: None,
            cache_control_format: None,
            session_affinity_format: None,
            supports_long_cache_retention: None,
            store: None,
            instruction_role: None,
            thinking_format: None,
            supports_reasoning_effort: None,
            usage_in_stream: None,
            max_tokens_field: None,
            tool_result_name_field: false,
            strict_tool_schema: false,
            cache_key: None,
            send_session_affinity_headers: None,
            require_assistant_after_tool_result: false,
            requires_thinking_as_text: false,
            requires_reasoning_content_on_assistant_messages: None,
            zai_tool_stream: false,
            vllm_priority: None,
            thinking_token_budget_field: None,
            supports_thinking_token_budget: false,
            chat_template_kwargs: None,
            chat_template_args: None,
            chat_completions_path: "/v1/chat/completions".into(),
        }
    }
}

/// Presence-aware source controls shared by the Responses wire families.
/// Each concrete wire resolves omitted values at its own boundary.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OpenAiResponsesSourceCompat {
    pub supports_developer_role: Option<bool>,
    pub session_affinity_format: Option<SessionAffinityFormat>,
    pub supports_long_cache_retention: Option<bool>,
    pub supports_explicit_prompt_cache_mode: Option<bool>,
    pub supports_max_output_tokens: Option<bool>,
    pub supports_strict_mode: Option<bool>,
    pub supports_grammar_tools: Option<bool>,
    pub supports_mid_convo_effort: Option<bool>,
    pub supports_mid_convo_system_messages: Option<bool>,
    pub supports_additional_tools: Option<bool>,
    pub supports_tool_search: Option<bool>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OpenAiResponsesCompat {
    pub source: OpenAiResponsesSourceCompat,
    pub store: Option<bool>,
    pub reasoning_effort: Option<String>,
    pub responses_path: String,
    pub send_session_id_header: bool,
}

impl Default for OpenAiResponsesCompat {
    fn default() -> Self {
        Self {
            source: OpenAiResponsesSourceCompat::default(),
            store: None,
            reasoning_effort: None,
            responses_path: "/v1/responses".into(),
            send_session_id_header: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OpenAiCodexResponsesCompat {
    pub source: OpenAiResponsesSourceCompat,
    pub responses_path: String,
    pub derive_account_id: bool,
}

impl Default for OpenAiCodexResponsesCompat {
    fn default() -> Self {
        Self {
            source: OpenAiResponsesSourceCompat::default(),
            responses_path: "/codex/responses".into(),
            derive_account_id: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
#[expect(
    clippy::large_enum_variant,
    reason = "Wire compatibility retains its public inline records, including typed routing controls"
)]
pub enum WireCompat {
    AnthropicMessages(AnthropicMessagesCompat),
    OpenAiCompletions(OpenAiCompletionsCompat),
    OpenAiResponses(OpenAiResponsesCompat),
    OpenAiCodexResponses(OpenAiCodexResponsesCompat),
    GoogleGenerativeAi,
    GoogleVertex,
    BedrockConverseStream(BedrockConverseStreamCompat),
    AzureOpenAiCompletions,
    /// Azure Responses rows (task 20.38): the same responses compat flag
    /// surface under the azure wire identity.
    AzureOpenAiResponses(OpenAiResponsesCompat),
    /// Mistral Conversations rows (task 20.43): the wire carries no
    /// mistral-specific compat flags today.
    MistralConversations,
    /// pi-messages rows (task 20.44): pi's own message protocol (Radius
    /// gateway); no adapter-specific compat flags.
    PiMessages,
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
            Self::BedrockConverseStream(_) => WireApi::BedrockConverseStream,
            Self::MistralConversations => WireApi::MistralConversations,
            Self::PiMessages => WireApi::PiMessages,
            Self::AzureOpenAiCompletions => WireApi::AzureOpenAiCompletions,
            Self::AzureOpenAiResponses(_) => WireApi::AzureOpenAiResponses,
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
            WireApi::BedrockConverseStream => Self::BedrockConverseStream(Default::default()),
            WireApi::AzureOpenAiCompletions => Self::AzureOpenAiCompletions,
            // Task 20.38: azure responses records carry the same responses
            // flag surface under their own wire identity.
            WireApi::AzureOpenAiResponses => Self::AzureOpenAiResponses(Default::default()),
            WireApi::MistralConversations => Self::MistralConversations,
            WireApi::PiMessages => Self::PiMessages,
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
    /// Finite source threshold; integer precision and fractional values are retained.
    pub input_tokens_above: serde_json::Number,
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
            validate_pricing_threshold(&tier.input_tokens_above)?;
            validate_pricing(&tier.pricing)?;
        }
        Ok(())
    }

    /// Source catalogs may retain finite negative placeholders for unknown
    /// rates. Validate their representation without treating them as prices.
    pub(crate) fn validate_source_metadata(&self) -> Result<(), ModelInfoError> {
        for tier in &self.tiers {
            validate_pricing_threshold(&tier.input_tokens_above)?;
        }
        for pricing in
            std::iter::once(&self.base).chain(self.tiers.iter().map(|tier| &tier.pricing))
        {
            if [
                pricing.input_cost_per_mtok,
                pricing.output_cost_per_mtok,
                pricing.cache_read_cost_per_mtok,
                pricing.cache_write_cost_per_mtok,
            ]
            .into_iter()
            .any(|rate| !rate.is_finite())
            {
                return Err(ModelInfoError::InvalidPricing(
                    "source pricing metadata must be finite".into(),
                ));
            }
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
    /// request. Every finite threshold is retained, including negative and
    /// fractional values. Only thresholds above the source's initial `-1`
    /// boundary can match; equal duplicates keep the first source entry.
    pub fn effective_for_usage(&self, usage: &Usage) -> Pricing {
        let prompt_tokens = u64::from(usage.input_tokens)
            + u64::from(usage.cache_read_tokens)
            + u64::from(usage.cache_write_tokens);
        self.effective_for_prompt_tokens(prompt_tokens)
    }

    fn effective_for_prompt_tokens(&self, prompt_tokens: u64) -> Pricing {
        let mut rates = self.base;
        let mut matched_threshold = serde_json::Number::from(-1);
        let input = serde_json::Number::from(prompt_tokens);
        for tier in &self.tiers {
            let threshold = &tier.input_tokens_above;
            if compare_pricing_thresholds(threshold, &matched_threshold) == Some(Ordering::Greater)
                && compare_pricing_thresholds(&input, threshold) == Some(Ordering::Greater)
            {
                rates = tier.pricing;
                matched_threshold = threshold.clone();
            }
        }
        rates
    }
}

fn validate_pricing_threshold(threshold: &serde_json::Number) -> Result<(), ModelInfoError> {
    if !threshold.as_f64().is_some_and(f64::is_finite) {
        return Err(ModelInfoError::InvalidPricing(
            "pricing tier thresholds must be finite".into(),
        ));
    }
    Ok(())
}

/// Numeric ordering, independent of the JSON integer/float representation.
/// Integer paths never pass through f64, including matched-tier comparisons.
fn compare_pricing_thresholds(
    left: &serde_json::Number,
    right: &serde_json::Number,
) -> Option<Ordering> {
    if let Some(left) = left.as_u64() {
        if let Some(right) = right.as_u64() {
            return Some(left.cmp(&right));
        }
        if right.as_i64().is_some() {
            return Some(Ordering::Greater);
        }
        return compare_unsigned_to_float(left, right.as_f64()?);
    }
    if right.as_u64().is_some() {
        return compare_pricing_thresholds(right, left).map(Ordering::reverse);
    }
    if let Some(left) = left.as_i64() {
        if let Some(right) = right.as_i64() {
            return Some(left.cmp(&right));
        }
        let right = right.as_f64()?;
        if right >= 0.0 {
            return Some(Ordering::Less);
        }
        return compare_unsigned_to_float(left.unsigned_abs(), -right).map(Ordering::reverse);
    }
    if right.as_i64().is_some() {
        return compare_pricing_thresholds(right, left).map(Ordering::reverse);
    }
    left.as_f64()?.partial_cmp(&right.as_f64()?)
}

fn compare_unsigned_to_float(integer: u64, float: f64) -> Option<Ordering> {
    if !float.is_finite() {
        return None;
    }
    if float < 0.0 {
        return Some(Ordering::Greater);
    }
    // u64::MAX rounds up to 2^64; no integer input can reach that float.
    if float >= u64::MAX as f64 {
        return Some(Ordering::Less);
    }
    // Below 2^64 the truncating cast is the exact floor. Keep the integer
    // operand intact so 2^53+1 does not become equal to the preceding float.
    Some(match integer.cmp(&(float as u64)) {
        Ordering::Equal if float.fract() > 0.0 => Ordering::Less,
        ordering => ordering,
    })
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
/// counted), and scales every component by the adapter's effective service
/// tier under the explicit wire. Codex does not mark up `fast`; Azure has
/// no service-tier multiplier. Unknown usage, unrecognized pricing tiers,
/// unknown selected rates and non-finite results return `None`; finite
/// negative source placeholders in unselected tiers do not affect cost.
/// Known free usage is
/// `Some` zero. The explicit source exception: OpenRouter image `parse_usage` keeps
/// base model rates without tiers — image callers use
/// [`crate::stream::calculate_cost`] directly.
pub fn calculate_cost_tiered(
    usage: &Usage,
    pricing: &ModelPricing,
    model_id: &str,
    wire: WireApi,
) -> Option<CostBreakdown> {
    if !usage.reported || pricing.validate_source_metadata().is_err() {
        return None;
    }
    let rates = pricing.effective_for_usage(usage);
    validate_pricing(&rates).ok()?;
    let cost = calculate_cost(usage, &rates);
    let tier = usage.effective_service_tier.or(usage.service_tier);
    let multiplier = match wire {
        WireApi::OpenAiResponses | WireApi::OpenAiCodexResponses => match tier {
            Some(crate::stream::ServiceTier::Unknown) => return None,
            Some(crate::stream::ServiceTier::Fast) if wire == WireApi::OpenAiCodexResponses => 1.0,
            Some(tier) => tier.cost_multiplier(model_id),
            None => 1.0,
        },
        _ => 1.0,
    };
    let cost = CostBreakdown {
        input_cost: cost.input_cost * multiplier,
        output_cost: cost.output_cost * multiplier,
        cache_read_cost: cost.cache_read_cost * multiplier,
        cache_write_cost: cost.cache_write_cost * multiplier,
    };
    [
        cost.input_cost,
        cost.output_cost,
        cost.cache_read_cost,
        cost.cache_write_cost,
        cost.total_cost(),
    ]
    .into_iter()
    .all(f64::is_finite)
    .then_some(cost)
}

pub(crate) fn validate_pricing(pricing: &Pricing) -> Result<(), ModelInfoError> {
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
    /// Model header defaults; a null value removes an inherited default.
    pub headers: Vec<(String, Option<String>)>,
    /// Source input-limit metadata. Preservation does not enable a resize or
    /// rejection policy; callers choose whether to apply these limits.
    pub input_limits: Option<serde_json::Value>,
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
            headers: Vec::new(),
            input_limits: None,
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
        crate::provider_headers::validate_model_headers(&self.headers)
            .map_err(|error| ModelInfoError::InvalidHeaders(error.to_string()))?;
        if let Some(pricing) = &self.pricing {
            pricing.validate_source_metadata()?;
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
    #[error("invalid model headers: {0}")]
    InvalidHeaders(String),
    #[error("invalid model pricing: {0}")]
    InvalidPricing(String),
}
