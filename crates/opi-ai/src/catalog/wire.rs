//! Distribution-schema-v1 codec for dynamic model catalogs.
//!
//! The persisted [`crate::catalog::ModelsStore`] snapshot and documents served
//! by [`crate::catalog::HttpCatalogSource`] speak the Pi *distribution* schema
//! (`https://pi.dev/schemas/models.schema.json`, schema 1): a `providers` map
//! whose entries carry a `models` array of `{ id, name, api, contextWindow,
//! maxTokens, input, reasoning, cost, ... }` records. This is deliberately a
//! different artifact from the hydration data the builtin bundle embeds
//! (`DATA_SCHEMA_VERSION` 6), and the two must not be conflated.
//!
//! `ModelInfo` carries opi-only fidelity that distribution schema 1 cannot
//! express (streaming support, cache-control flags, pricing tiers, exact
//! thinking-level mappings, wire compat payloads). Those fields round-trip
//! through an explicit `"x-opi"` extension object on each model entry. The
//! distribution schema leaves additional properties open, so an `x-opi`
//! extension stays schema-compatible while keeping restored snapshots
//! observationally identical to what was committed. Documents without the
//! extension still decode: v1 fields win and opi-only capabilities take
//! closed defaults.

use std::collections::{BTreeMap, HashMap};

use serde_json::{Map, Value, json};

use crate::model_info::{
    AnthropicMessagesCompat, ModelCapabilities, ModelInfo, OpenAiCodexResponsesCompat,
    OpenAiResponsesCompat, ThinkingLevel, ThinkingLevelMap, ThinkingLevelMapping, WireApi,
    WireCompat,
};
use crate::stream::Pricing;

use super::store::CatalogPublisher;
use super::{CatalogRecord, ModelsStoreEntry, ModelsStoreError};

/// The distribution schema identity the store and HTTP documents declare.
pub(crate) const DISTRIBUTION_SCHEMA_ID: &str = "https://pi.dev/schemas/models.schema.json";

/// Extension namespace holding opi-only model fidelity alongside the
/// distribution-schema-v1 fields.
const EXTENSION_KEY: &str = "x-opi";

fn level_key(level: ThinkingLevel) -> &'static str {
    match level {
        ThinkingLevel::None => "off",
        ThinkingLevel::Minimal => "minimal",
        ThinkingLevel::Low => "low",
        ThinkingLevel::Medium => "medium",
        ThinkingLevel::High => "high",
        ThinkingLevel::XHigh => "xhigh",
        ThinkingLevel::Max => "max",
    }
}

fn level_from_key(key: &str) -> Option<ThinkingLevel> {
    ThinkingLevel::ALL
        .into_iter()
        .find(|level| level_key(*level) == key)
}

fn pricing_to_json(pricing: &Pricing) -> Value {
    json!({
        "input": pricing.input_cost_per_mtok,
        "output": pricing.output_cost_per_mtok,
        "cacheRead": pricing.cache_read_cost_per_mtok,
        "cacheWrite": pricing.cache_write_cost_per_mtok,
    })
}

fn pricing_from_json(value: &Value) -> Result<Pricing, String> {
    let field = |name: &str| -> Result<f64, String> {
        value
            .get(name)
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("pricing field '{name}' missing or not a number"))
    };
    Ok(Pricing {
        input_cost_per_mtok: field("input")?,
        output_cost_per_mtok: field("output")?,
        cache_read_cost_per_mtok: field("cacheRead")?,
        cache_write_cost_per_mtok: field("cacheWrite")?,
    })
}

fn known_pricing_from_json(value: &Value) -> Result<Option<Pricing>, String> {
    let object = value.as_object().ok_or("pricing must be an object")?;
    for key in ["input", "output", "cacheRead", "cacheWrite"] {
        match object.get(key) {
            None | Some(Value::Null) => return Ok(None),
            Some(value) => {
                let rate = value
                    .as_f64()
                    .ok_or_else(|| format!("pricing field '{key}' must be a number"))?;
                if !rate.is_finite() || rate < 0.0 {
                    return Ok(None);
                }
            }
        }
    }
    pricing_from_json(value).map(Some)
}

/// Best-effort v1 `thinkingLevelMap` rendering: mapped/identity levels with a
/// wire name render their name; everything else renders `null`.
fn thinking_map_to_v1(map: &ThinkingLevelMap) -> Value {
    let mut rendered = Map::new();
    for level in ThinkingLevel::ALL {
        let value = match map.resolve(level) {
            Ok(Some(name)) => Value::String(name),
            _ => Value::Null,
        };
        rendered.insert(level_key(level).to_owned(), value);
    }
    Value::Object(rendered)
}

/// Exact fidelity rendering under the `x-opi` extension: every level records
/// its precise mapping kind so `resolve` outcomes survive the round trip.
fn thinking_map_to_extension(map: &ThinkingLevelMap) -> Value {
    let mut rendered = Map::new();
    for level in ThinkingLevel::ALL {
        let value = match map.mapping(level) {
            ThinkingLevelMapping::Mapped(name) => json!({ "mapped": name }),
            ThinkingLevelMapping::Identity => json!({ "identity": true }),
            ThinkingLevelMapping::Unsupported => json!({ "unsupported": true }),
        };
        rendered.insert(level_key(level).to_owned(), value);
    }
    Value::Object(rendered)
}

fn thinking_map_from_extension(value: &Value) -> Result<ThinkingLevelMap, String> {
    let mut map = ThinkingLevelMap::disabled();
    let Some(entries) = value.as_object() else {
        return Err("x-opi.thinking must be an object".to_owned());
    };
    for (key, entry) in entries {
        let level =
            level_from_key(key).ok_or_else(|| format!("unknown thinking level key '{key}'"))?;
        let mapping = if entry.get("unsupported").is_some() {
            ThinkingLevelMapping::Unsupported
        } else if let Some(identity) = entry.get("identity") {
            if identity.as_bool() == Some(true) {
                ThinkingLevelMapping::Identity
            } else {
                return Err(format!("invalid x-opi.thinking entry for '{key}'"));
            }
        } else if let Some(name) = entry.get("mapped").and_then(Value::as_str) {
            ThinkingLevelMapping::Mapped(name.to_owned())
        } else {
            return Err(format!("invalid x-opi.thinking entry for '{key}'"));
        };
        map = map.with_mapping(level, mapping);
    }
    Ok(map)
}

fn compat_to_extension(compat: &WireCompat) -> Value {
    match compat {
        WireCompat::AnthropicMessages(inner) => json!({
            "api": WireApi::AnthropicMessages.as_str(),
            "supportsEagerToolInputStreaming": inner.supports_eager_tool_input_streaming,
            "supportsMidConvoSystemMessages": inner.supports_mid_convo_system_messages,
            "supportsMidConvoToolChanges": inner.supports_mid_convo_tool_changes,
            "forceAdaptiveThinking": inner.force_adaptive_thinking,
            "supportsTemperature": inner.supports_temperature,
            "supportsStrictTools": inner.supports_strict_tools,
        }),
        WireCompat::OpenAiCompletions(inner) => json!({
            "api": WireApi::OpenAiCompletions.as_str(),
            "storeField": match &inner.store {
                Some(crate::model_info::ChatStoreField::Omit) => json!("omit"),
                Some(crate::model_info::ChatStoreField::SendFalse) => json!("send_false"),
                None => serde_json::Value::Null,
            },
            "instructionRole": match &inner.instruction_role {
                Some(crate::model_info::ChatInstructionRole::Auto) => json!("auto"),
                Some(crate::model_info::ChatInstructionRole::System) => json!("system"),
                Some(crate::model_info::ChatInstructionRole::DeveloperForReasoning) => {
                    json!("developer_for_reasoning")
                }
                Some(crate::model_info::ChatInstructionRole::Developer) => json!("developer"),
                None => serde_json::Value::Null,
            },
            "thinkingFormat": match &inner.thinking_format {
                Some(format) => json!(format.wire_name()),
                None => serde_json::Value::Null,
            },
            "supportsReasoningEffort": inner.supports_reasoning_effort,
            "maxTokensField": inner.max_tokens_field,
            "toolResultNameField": inner.tool_result_name_field,
            "usageInStream": inner.usage_in_stream,
            "strictToolSchema": inner.strict_tool_schema,
            "cacheKey": inner.cache_key,
            "sendSessionAffinityHeaders": inner.send_session_affinity_headers,
            "requireAssistantAfterToolResult": inner.require_assistant_after_tool_result,
            "requiresThinkingAsText": inner.requires_thinking_as_text,
            "requiresReasoningContentOnAssistantMessages":
                inner.requires_reasoning_content_on_assistant_messages,
            "zaiToolStream": inner.zai_tool_stream,
            "vllmPriority": inner.vllm_priority,
            "thinkingTokenBudgetField": inner
                .thinking_token_budget_field
                .as_ref()
                .map(|field| field.wire_name()),
            "supportsThinkingTokenBudget": inner.supports_thinking_token_budget,
            "chatTemplateArgs": template_values_to_extension(&inner.chat_template_args),
            "chatTemplateKwargs": template_values_to_extension(&inner.chat_template_kwargs),
            "chatCompletionsPath": inner.chat_completions_path,
        }),
        WireCompat::OpenAiResponses(inner) => json!({
            "api": WireApi::OpenAiResponses.as_str(),
            "store": inner.store,
            "reasoningEffort": inner.reasoning_effort,
            "strictTools": inner.strict_tools,
            "responsesPath": inner.responses_path,
            "sendSessionIdHeader": inner.send_session_id_header,
            "supportsMidConvoEffort": inner.supports_mid_convo_effort,
            "supportsOpenAIGrammarTools": inner.supports_grammar,
        }),
        WireCompat::AzureOpenAiResponses(inner) => json!({
            "api": WireApi::AzureOpenAiResponses.as_str(),
            "store": inner.store,
            "reasoningEffort": inner.reasoning_effort,
            "strictTools": inner.strict_tools,
            "responsesPath": inner.responses_path,
            "sendSessionIdHeader": inner.send_session_id_header,
            "supportsMidConvoEffort": inner.supports_mid_convo_effort,
            "supportsOpenAIGrammarTools": inner.supports_grammar,
        }),
        WireCompat::OpenAiCodexResponses(inner) => json!({
            "api": WireApi::OpenAiCodexResponses.as_str(),
            "responsesPath": inner.responses_path,
            "deriveAccountId": inner.derive_account_id,
        }),
        WireCompat::GoogleGenerativeAi => json!({ "api": WireApi::GoogleGenerativeAi.as_str() }),
        WireCompat::GoogleVertex => json!({ "api": WireApi::GoogleVertex.as_str() }),
        WireCompat::BedrockConverseStream(inner) => json!({
            "api": WireApi::BedrockConverseStream.as_str(),
            "supportsStrictTools": inner.supports_strict_tools,
        }),
        WireCompat::AzureOpenAiCompletions => {
            json!({ "api": WireApi::AzureOpenAiCompletions.as_str() })
        }
        WireCompat::PiMessages => {
            json!({ "api": WireApi::PiMessages.as_str() })
        }
        WireCompat::MistralConversations => {
            json!({ "api": WireApi::MistralConversations.as_str() })
        }
    }
}

fn opt_string(value: &Value, key: &str) -> Result<Option<String>, String> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(_) => Err(format!("field '{key}' must be a string when present")),
    }
}

fn opt_flag(value: &Value, key: &str) -> Result<Option<bool>, String> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(flag)) => Ok(Some(*flag)),
        Some(_) => Err(format!("field '{key}' must be a boolean when present")),
    }
}

fn thinking_budget_field(
    value: &Value,
) -> Result<Option<crate::model_info::ChatThinkingTokenBudgetField>, String> {
    use crate::model_info::ChatThinkingTokenBudgetField;
    match opt_string(value, "thinkingTokenBudgetField")?.as_deref() {
        None => Ok(None),
        Some("thinking_token_budget") => {
            Ok(Some(ChatThinkingTokenBudgetField::ThinkingTokenBudget))
        }
        Some("thinking_budget") => Ok(Some(ChatThinkingTokenBudgetField::ThinkingBudget)),
        Some("thinking_budget_tokens") => {
            Ok(Some(ChatThinkingTokenBudgetField::ThinkingBudgetTokens))
        }
        Some(other) => Err(format!(
            "field 'thinkingTokenBudgetField' carries unknown budget field '{other}'"
        )),
    }
}

fn template_values_to_extension(
    values: &BTreeMap<String, crate::model_info::ChatTemplateKwargValue>,
) -> Value {
    use crate::model_info::{ChatTemplateKwargValue, ChatTemplateKwargVar};
    Value::Object(
        values
            .iter()
            .map(|(key, value)| {
                let value = match value {
                    ChatTemplateKwargValue::Literal(value) => json!({"literal": value}),
                    ChatTemplateKwargValue::Var { var, omit_when_off } => json!({
                        "var": match var {
                            ChatTemplateKwargVar::ThinkingEnabled => "thinking.enabled",
                            ChatTemplateKwargVar::ThinkingEffort => "thinking.effort",
                            ChatTemplateKwargVar::ThinkingBudget => "thinking.budget",
                        },
                        "omitWhenOff": omit_when_off,
                    }),
                };
                (key.clone(), value)
            })
            .collect(),
    )
}

fn template_values_from_extension(
    value: &Value,
    key: &str,
) -> Result<BTreeMap<String, crate::model_info::ChatTemplateKwargValue>, String> {
    use crate::model_info::{ChatTemplateKwargValue, ChatTemplateKwargVar};
    let Some(values) = value.get(key) else {
        return Ok(BTreeMap::new());
    };
    values
        .as_object()
        .ok_or_else(|| format!("x-opi.compat.{key} must be an object"))?
        .iter()
        .map(|(name, entry)| {
            let value = match (entry.get("literal"), entry.get("var")) {
                (Some(value), None) => ChatTemplateKwargValue::Literal(value.clone()),
                (None, Some(value)) => ChatTemplateKwargValue::Var {
                    var: match value.as_str() {
                        Some("thinking.enabled") => ChatTemplateKwargVar::ThinkingEnabled,
                        Some("thinking.effort") => ChatTemplateKwargVar::ThinkingEffort,
                        Some("thinking.budget") => ChatTemplateKwargVar::ThinkingBudget,
                        _ => return Err(format!("invalid x-opi.compat.{key} variable")),
                    },
                    omit_when_off: opt_flag(entry, "omitWhenOff")?.unwrap_or(false),
                },
                _ => return Err(format!("invalid x-opi.compat.{key} value")),
            };
            Ok((name.clone(), value))
        })
        .collect()
}

fn parse_chat_thinking_format(
    value: &Value,
    key: &str,
) -> Result<Option<crate::model_info::ChatThinkingFormat>, String> {
    match value.get(key).and_then(Value::as_str) {
        None => Ok(None),
        Some(name) => Ok(Some(name.parse().map_err(|_| {
            format!("field '{key}' carries unknown thinking format '{name}'")
        })?)),
    }
}

fn reject_retired_chat_compat_keys(value: &Value) -> Result<(), String> {
    for (old, replacement) in [
        (
            "systemRoleOverride",
            "instructionRole (auto|system|developer_for_reasoning|developer)",
        ),
        ("supportsStore", "storeField (omit|send_false)"),
        (
            "supportsDeveloperRole",
            "instructionRole (auto|system|developer_for_reasoning|developer)",
        ),
        (
            "reasoningEffort",
            "per-call thinking intent plus the model thinking-level map",
        ),
    ] {
        if value.get(old).is_some_and(|v| !v.is_null()) {
            return Err(format!(
                "retired x-opi.compat key '{old}': use {replacement} instead"
            ));
        }
    }
    Ok(())
}

fn compat_from_extension(value: &Value) -> Result<Option<WireCompat>, String> {
    let Some(api) = value.get("api").and_then(Value::as_str) else {
        return Ok(None);
    };
    // Task 20.38: both responses wires share the responses compat flag
    // surface; only the default path differs.
    fn parse_responses_compat(
        value: &Value,
        default_path: &str,
    ) -> Result<OpenAiResponsesCompat, String> {
        let flag = |key: &str| -> Result<bool, String> {
            value
                .get(key)
                .and_then(Value::as_bool)
                .ok_or_else(|| format!("x-opi.compat field '{key}' must be a boolean"))
        };
        Ok(OpenAiResponsesCompat {
            store: value.get("store").and_then(Value::as_bool),
            reasoning_effort: opt_string(value, "reasoningEffort")?,
            strict_tools: flag("strictTools")?,
            responses_path: value
                .get("responsesPath")
                .and_then(Value::as_str)
                .unwrap_or(default_path)
                .to_owned(),
            send_session_id_header: flag("sendSessionIdHeader")?,
            supports_mid_convo_effort: flag("supportsMidConvoEffort")?,
            supports_grammar: opt_flag(value, "supportsOpenAIGrammarTools")?.unwrap_or(false),
        })
    }

    let wire: WireApi = api
        .parse()
        .map_err(|_| format!("x-opi.compat carries unknown api '{api}'"))?;
    let flag = |key: &str| -> Result<bool, String> {
        value
            .get(key)
            .and_then(Value::as_bool)
            .ok_or_else(|| format!("x-opi.compat field '{key}' must be a boolean"))
    };
    match wire {
        WireApi::AnthropicMessages => Ok(Some(WireCompat::AnthropicMessages(
            AnthropicMessagesCompat {
                supports_eager_tool_input_streaming: flag("supportsEagerToolInputStreaming")?,
                force_adaptive_thinking: flag("forceAdaptiveThinking")?,
                supports_temperature: flag("supportsTemperature")?,
                supports_strict_tools: match value.get("supportsStrictTools") {
                    None => false,
                    Some(value) => value
                        .as_bool()
                        .ok_or("x-opi.compat field 'supportsStrictTools' must be a boolean")?,
                },
                supports_mid_convo_system_messages: match value
                    .get("supportsMidConvoSystemMessages")
                {
                    None => false,
                    Some(value) => value.as_bool().ok_or(
                        "x-opi.compat field 'supportsMidConvoSystemMessages' must be a boolean",
                    )?,
                },
                supports_mid_convo_tool_changes: match value.get("supportsMidConvoToolChanges") {
                    None => false,
                    Some(value) => value.as_bool().ok_or(
                        "x-opi.compat field 'supportsMidConvoToolChanges' must be a boolean",
                    )?,
                },
            },
        ))),
        WireApi::OpenAiCompletions => {
            reject_retired_chat_compat_keys(value)?;
            let thinking_token_budget_field = thinking_budget_field(value)?;
            Ok(Some(WireCompat::OpenAiCompletions(
                crate::model_info::OpenAiCompletionsCompat {
                    store: match value.get("storeField").and_then(Value::as_str) {
                        None | Some("auto") => None,
                        Some("omit") => Some(crate::model_info::ChatStoreField::Omit),
                        Some("send_false") => Some(crate::model_info::ChatStoreField::SendFalse),
                        Some(other) => {
                            return Err(format!(
                                "field 'storeField' carries unknown store policy '{other}'"
                            ));
                        }
                    },
                    instruction_role: match value.get("instructionRole").and_then(Value::as_str) {
                        None => None,
                        Some("auto") => Some(crate::model_info::ChatInstructionRole::Auto),
                        Some("system") => Some(crate::model_info::ChatInstructionRole::System),
                        Some("developer_for_reasoning") => {
                            Some(crate::model_info::ChatInstructionRole::DeveloperForReasoning)
                        }
                        Some("developer") => {
                            Some(crate::model_info::ChatInstructionRole::Developer)
                        }
                        Some(other) => {
                            return Err(format!(
                                "field 'instructionRole' carries unknown role policy '{other}'"
                            ));
                        }
                    },
                    thinking_format: parse_chat_thinking_format(value, "thinkingFormat")?,
                    supports_reasoning_effort: opt_flag(value, "supportsReasoningEffort")?,
                    usage_in_stream: opt_flag(value, "usageInStream")?,
                    max_tokens_field: opt_string(value, "maxTokensField")?,
                    tool_result_name_field: flag("toolResultNameField")?,
                    strict_tool_schema: flag("strictToolSchema")?,
                    cache_key: opt_string(value, "cacheKey")?,
                    send_session_affinity_headers: opt_flag(value, "sendSessionAffinityHeaders")?,
                    require_assistant_after_tool_result: flag("requireAssistantAfterToolResult")?,
                    requires_thinking_as_text: flag("requiresThinkingAsText")?,
                    requires_reasoning_content_on_assistant_messages: opt_flag(
                        value,
                        "requiresReasoningContentOnAssistantMessages",
                    )?,
                    zai_tool_stream: flag("zaiToolStream")?,
                    vllm_priority: value.get("vllmPriority").and_then(Value::as_i64),
                    thinking_token_budget_field,
                    supports_thinking_token_budget: flag("supportsThinkingTokenBudget")?,
                    chat_template_kwargs: template_values_from_extension(
                        value,
                        "chatTemplateKwargs",
                    )?,
                    chat_template_args: template_values_from_extension(value, "chatTemplateArgs")?,
                    chat_completions_path: value
                        .get("chatCompletionsPath")
                        .and_then(Value::as_str)
                        .unwrap_or("/v1/chat/completions")
                        .to_owned(),
                },
            )))
        }
        WireApi::OpenAiResponses => Ok(Some(WireCompat::OpenAiResponses(parse_responses_compat(
            value,
            "/v1/responses",
        )?))),
        WireApi::AzureOpenAiResponses => Ok(Some(WireCompat::AzureOpenAiResponses(
            parse_responses_compat(value, "/responses")?,
        ))),
        WireApi::OpenAiCodexResponses => Ok(Some(WireCompat::OpenAiCodexResponses(
            OpenAiCodexResponsesCompat {
                responses_path: value
                    .get("responsesPath")
                    .and_then(Value::as_str)
                    .unwrap_or("/codex/responses")
                    .to_owned(),
                derive_account_id: flag("deriveAccountId")?,
            },
        ))),
        WireApi::GoogleGenerativeAi => Ok(Some(WireCompat::GoogleGenerativeAi)),
        WireApi::GoogleVertex => Ok(Some(WireCompat::GoogleVertex)),
        WireApi::BedrockConverseStream => Ok(Some(WireCompat::BedrockConverseStream(
            crate::model_info::BedrockConverseStreamCompat {
                supports_strict_tools: match value.get("supportsStrictTools") {
                    None => false,
                    Some(value) => value
                        .as_bool()
                        .ok_or("x-opi.compat field 'supportsStrictTools' must be a boolean")?,
                },
            },
        ))),
        WireApi::AzureOpenAiCompletions => Ok(Some(WireCompat::AzureOpenAiCompletions)),
        WireApi::MistralConversations => Ok(Some(WireCompat::MistralConversations)),
        WireApi::PiMessages => Ok(Some(WireCompat::PiMessages)),
    }
}

/// Encode one model entry in distribution-schema-v1 shape with the `x-opi`
/// fidelity extension.
pub(crate) fn encode_model_entry(model: &ModelInfo) -> Value {
    let mut input = vec!["text"];
    if model.capabilities.supports_images {
        input.push("image");
    }

    let mut entry = json!({
        "id": model.id,
        "name": model.display_name,
        "api": model.wire_api.as_str(),
        "contextWindow": model.capabilities.context_window,
        "maxTokens": model.capabilities.max_output_tokens,
        "input": input,
        "reasoning": model.capabilities.supports_thinking,
        "thinkingLevelMap": thinking_map_to_v1(&model.thinking_level_map),
    });
    if let Some(base_url) = &model.base_url {
        entry["baseUrl"] = Value::String(base_url.clone());
    }
    if let Some(pricing) = &model.pricing {
        entry["cost"] = pricing_to_json(&pricing.base);
    }

    if let Some(limits) = &model.input_limits {
        entry["inputLimits"] = limits.clone();
    }
    if !model.headers.is_empty() {
        entry["headers"] = Value::Object(
            model
                .headers
                .iter()
                .map(|(name, value)| {
                    (
                        name.clone(),
                        value
                            .as_ref()
                            .map_or(Value::Null, |value| Value::String(value.clone())),
                    )
                })
                .collect(),
        );
    }
    let mut extension = Map::new();
    extension.insert(
        "streaming".to_owned(),
        Value::Bool(model.capabilities.supports_streaming),
    );
    extension.insert(
        "cacheControl".to_owned(),
        Value::Bool(model.capabilities.supports_cache_control),
    );
    extension.insert(
        "longCacheRetention".to_owned(),
        Value::Bool(model.capabilities.supports_long_cache_retention),
    );
    extension.insert(
        "thinking".to_owned(),
        thinking_map_to_extension(&model.thinking_level_map),
    );
    extension.insert("compat".to_owned(), compat_to_extension(&model.compat));
    // A JSON object cannot retain repeated header operations. The fidelity
    // extension keeps their order, including case-insensitive set/remove pairs.
    if !model.headers.is_empty() {
        extension.insert("headers".to_owned(), json!(model.headers));
    }
    extension.insert(
        "samplingParams".to_owned(),
        Value::Object(model.sampling_params.clone()),
    );
    extension.insert(
        "samplingParamsByThinkingLevel".to_owned(),
        Value::Object(
            model
                .sampling_params_by_thinking_level
                .iter()
                .map(|(level, params)| {
                    (level_key(*level).to_owned(), Value::Object(params.clone()))
                })
                .collect(),
        ),
    );
    if let Some(pricing) = &model.pricing
        && let Some(tiers) = (!pricing.tiers.is_empty()).then(|| {
            Value::Array(
                pricing
                    .tiers
                    .iter()
                    .map(|tier| {
                        let mut rendered = pricing_to_json(&tier.pricing);
                        rendered["above"] = json!(tier.input_tokens_above);
                        rendered
                    })
                    .collect(),
            )
        })
    {
        extension.insert("pricingTiers".to_owned(), tiers);
    }
    entry[EXTENSION_KEY] = Value::Object(extension);
    entry
}

/// Map the source model fields before applying the exact Rust store extension.
fn apply_source_metadata(model: &mut ModelInfo, entry: &Value) -> Result<(), String> {
    if let Some(map) = entry.get("thinkingLevelMap").and_then(Value::as_object) {
        for (name, value) in map {
            let level =
                level_from_key(name).ok_or_else(|| format!("unknown thinking level '{name}'"))?;
            let mapping = match value {
                Value::Null => ThinkingLevelMapping::Unsupported,
                Value::String(value) => ThinkingLevelMapping::Mapped(value.clone()),
                _ => return Err(format!("thinking level '{name}' must be a string or null")),
            };
            model.thinking_level_map = model
                .thinking_level_map
                .clone()
                .with_mapping(level, mapping);
        }
    }
    model.input_limits = entry
        .get("inputLimits")
        .filter(|value| !value.is_null())
        .cloned();
    if let Some(headers) = entry.get("headers") {
        for (name, value) in headers
            .as_object()
            .ok_or("model headers must be an object")?
        {
            let value = match value {
                Value::Null => None,
                Value::String(value) => Some(value.clone()),
                _ => return Err("model header must be a string or null".into()),
            };
            model.headers.push((name.clone(), value));
        }
    }
    let Some(compat) = entry.get("compat") else {
        return Ok(());
    };
    let flag = |key: &str| opt_flag(compat, key);
    match &mut model.compat {
        WireCompat::OpenAiCompletions(c) => {
            c.store = flag("supportsStore")?.map(|value| {
                if value {
                    crate::model_info::ChatStoreField::SendFalse
                } else {
                    crate::model_info::ChatStoreField::Omit
                }
            });
            c.instruction_role = match opt_string(compat, "systemRoleOverride")?.as_deref() {
                Some("system") => Some(crate::model_info::ChatInstructionRole::System),
                Some("developer") => Some(crate::model_info::ChatInstructionRole::Developer),
                Some(_) => return Err("invalid systemRoleOverride".into()),
                None => flag("supportsDeveloperRole")?.map(|value| {
                    if value {
                        crate::model_info::ChatInstructionRole::DeveloperForReasoning
                    } else {
                        crate::model_info::ChatInstructionRole::System
                    }
                }),
            };
            c.thinking_format = parse_chat_thinking_format(compat, "thinkingFormat")?;
            c.max_tokens_field = opt_string(compat, "maxTokensField")?;
            c.supports_reasoning_effort = flag("supportsReasoningEffort")?;
            c.usage_in_stream = flag("supportsUsageInStreaming")?;
            c.strict_tool_schema = flag("supportsStrictMode")?.unwrap_or(c.strict_tool_schema);
            c.tool_result_name_field =
                flag("requiresToolResultName")?.unwrap_or(c.tool_result_name_field);
            c.send_session_affinity_headers = flag("sendSessionAffinityHeaders")?;
            c.require_assistant_after_tool_result = flag("requiresAssistantAfterToolResult")?
                .unwrap_or(c.require_assistant_after_tool_result);
            c.requires_thinking_as_text =
                flag("requiresThinkingAsText")?.unwrap_or(c.requires_thinking_as_text);
            c.requires_reasoning_content_on_assistant_messages =
                flag("requiresReasoningContentOnAssistantMessages")?;
            c.zai_tool_stream = flag("zaiToolStream")?.unwrap_or(false);
            c.supports_thinking_token_budget =
                flag("supportsThinkingTokenBudget")?.unwrap_or(false);
            c.thinking_token_budget_field = thinking_budget_field(compat)?;
            c.vllm_priority = compat.get("vllmPriority").and_then(Value::as_i64);
            for (key, target) in [
                ("chatTemplateArgs", &mut c.chat_template_args),
                ("chatTemplateKwargs", &mut c.chat_template_kwargs),
            ] {
                if let Some(values) = compat.get(key).and_then(Value::as_object) {
                    for (name, value) in values {
                        let value = if let Some(var) = value.get("$var").and_then(Value::as_str) {
                            use crate::model_info::{ChatTemplateKwargValue, ChatTemplateKwargVar};
                            ChatTemplateKwargValue::Var {
                                var: match var {
                                    "thinking.enabled" => ChatTemplateKwargVar::ThinkingEnabled,
                                    "thinking.effort" => ChatTemplateKwargVar::ThinkingEffort,
                                    "thinking.budget" => ChatTemplateKwargVar::ThinkingBudget,
                                    _ => {
                                        return Err(format!(
                                            "unknown chat template variable '{var}'"
                                        ));
                                    }
                                },
                                omit_when_off: value
                                    .get("omitWhenOff")
                                    .and_then(Value::as_bool)
                                    .unwrap_or(false),
                            }
                        } else {
                            crate::model_info::ChatTemplateKwargValue::Literal(value.clone())
                        };
                        target.insert(name.clone(), value);
                    }
                }
            }
        }
        WireCompat::OpenAiResponses(c) | WireCompat::AzureOpenAiResponses(c) => {
            c.strict_tools = flag("supportsStrictMode")?.unwrap_or(c.strict_tools);
            c.supports_mid_convo_effort = flag("supportsMidConvoEffort")?.unwrap_or(false);
            c.supports_grammar = flag("supportsOpenAIGrammarTools")?.unwrap_or(false);
        }
        WireCompat::AnthropicMessages(c) => {
            c.supports_eager_tool_input_streaming = flag("supportsEagerToolInputStreaming")?
                .unwrap_or(c.supports_eager_tool_input_streaming);
            c.supports_mid_convo_system_messages = flag("supportsMidConvoSystemMessages")?
                .unwrap_or(c.supports_mid_convo_system_messages);
            c.supports_mid_convo_tool_changes =
                flag("supportsMidConvoToolChanges")?.unwrap_or(c.supports_mid_convo_tool_changes);
            c.force_adaptive_thinking =
                flag("forceAdaptiveThinking")?.unwrap_or(c.force_adaptive_thinking);
            c.supports_temperature = flag("supportsTemperature")?.unwrap_or(c.supports_temperature);
            c.supports_strict_tools =
                flag("supportsStrictTools")?.unwrap_or(c.supports_strict_tools);
        }
        WireCompat::BedrockConverseStream(c) => {
            c.supports_strict_tools = flag("supportsStrictMode")?.unwrap_or(c.supports_strict_tools)
        }
        _ => {}
    }
    Ok(())
}

/// Decode one distribution-schema-v1 model entry. Fields under `x-opi` win
/// over the plain v1 fields; opi-only capabilities default closed when the
/// extension is absent.
pub(crate) fn decode_model_entry(entry: &Value) -> Result<ModelInfo, String> {
    let id = entry
        .get("id")
        .and_then(Value::as_str)
        .ok_or("model entry requires a string id")?
        .to_owned();
    if id.is_empty() {
        return Err("model entry id must not be empty".to_owned());
    }
    let display_name = entry
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(id.as_str())
        .to_owned();
    let wire_api = match entry.get("api").and_then(Value::as_str) {
        Some(api) => api
            .parse()
            .map_err(|_| format!("model '{id}' carries unknown api '{api}'"))?,
        None => WireApi::OpenAiCompletions,
    };
    let number = |key: &str, default: u64| -> u64 {
        match entry.get(key) {
            None => default,
            Some(value) => value
                .as_u64()
                .or_else(|| {
                    let float = value.as_f64()?;
                    (float.fract() == 0.0 && float >= 0.0).then_some(float as u64)
                })
                .unwrap_or(default),
        }
    };
    let supports_images = entry
        .get("input")
        .and_then(Value::as_array)
        .map(|values| values.iter().any(|value| value.as_str() == Some("image")))
        .unwrap_or(false);
    let supports_thinking = entry
        .get("reasoning")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let extension = entry.get(EXTENSION_KEY);
    let ext_flag = |key: &str| {
        extension
            .and_then(|ext| ext.get(key))
            .and_then(Value::as_bool)
    };
    let supports_streaming = ext_flag("streaming").unwrap_or(false);
    let supports_cache_control = ext_flag("cacheControl").unwrap_or(false);
    let supports_long_cache_retention = ext_flag("longCacheRetention").unwrap_or(false);

    let mut model = ModelInfo::new(
        id,
        display_name,
        wire_api,
        ModelCapabilities::new(number("contextWindow", 0), number("maxTokens", 0))
            .with_images(supports_images)
            .with_streaming(supports_streaming)
            .with_thinking(supports_thinking)
            .with_cache_control(supports_cache_control)
            .with_long_cache_retention(supports_long_cache_retention),
    );

    if let Some(base_url) = opt_string(entry, "baseUrl")? {
        model = model.with_base_url(base_url);
    }

    apply_source_metadata(&mut model, entry)?;

    if let Some(headers) = extension.and_then(|ext| ext.get("headers")) {
        model.headers = serde_json::from_value(headers.clone()).map_err(|_| {
            "x-opi.headers must be an ordered list of header name/value pairs".to_owned()
        })?;
    }

    if let Some(thinking) = extension.and_then(|ext| ext.get("thinking")) {
        model = model.with_thinking_level_map(thinking_map_from_extension(thinking)?);
    }
    if let Some(params) = extension
        .and_then(|ext| ext.get("samplingParams"))
        .or_else(|| entry.get("samplingParams"))
    {
        model.sampling_params = params
            .as_object()
            .ok_or("x-opi.samplingParams must be an object")?
            .clone();
    }
    if let Some(levels) = extension
        .and_then(|ext| ext.get("samplingParamsByThinkingLevel"))
        .or_else(|| entry.get("samplingParamsByThinkingLevel"))
    {
        for (key, params) in levels
            .as_object()
            .ok_or("x-opi.samplingParamsByThinkingLevel must be an object")?
        {
            let level = level_from_key(key)
                .ok_or_else(|| format!("unknown sampling thinking level '{key}'"))?;
            model.sampling_params_by_thinking_level.insert(
                level,
                params
                    .as_object()
                    .ok_or_else(|| {
                        format!("sampling parameters for thinking level '{key}' must be an object")
                    })?
                    .clone(),
            );
        }
    }

    if let Some(cost) = entry.get("cost").filter(|value| !value.is_null()) {
        let base = pricing_from_json(cost)?;
        let tiers = extension
            .and_then(|ext| ext.get("pricingTiers"))
            .or_else(|| cost.get("tiers"))
            .and_then(Value::as_array)
            .map(|tiers| {
                tiers
                    .iter()
                    .map(|tier| {
                        Ok(crate::model_info::PricingTier {
                            input_tokens_above: tier
                                .get("above")
                                .or_else(|| tier.get("inputTokensAbove"))
                                .and_then(Value::as_number)
                                .filter(|threshold| threshold.as_f64().is_some_and(f64::is_finite))
                                .cloned()
                                .ok_or("pricing tier requires an 'above' threshold")?,
                            pricing: pricing_from_json(tier)?,
                        })
                    })
                    .collect::<Result<Vec<_>, String>>()
            })
            .transpose()?
            .unwrap_or_default();
        model.pricing = Some(crate::model_info::ModelPricing { base, tiers });
    }

    if let Some(compat) = extension
        .and_then(|ext| ext.get("compat"))
        .and_then(|compat| compat_from_extension(compat).transpose())
    {
        model = model
            .with_compat(compat?)
            .map_err(|error| error.to_string())?;
    }

    Ok(model)
}

pub(crate) fn decode_record_headers(
    entry: &Value,
) -> Result<Vec<(String, Option<String>)>, String> {
    if let Some(headers) = entry.pointer("/x-opi/headers") {
        return serde_json::from_value(headers.clone()).map_err(|_| {
            "x-opi.headers must be an ordered list of header name/value pairs".into()
        });
    }
    let Some(headers) = entry.get("headers") else {
        return Ok(Vec::new());
    };
    headers
        .as_object()
        .ok_or("model headers must be an object")?
        .iter()
        .map(|(name, value)| {
            Ok((
                name.clone(),
                match value {
                    Value::Null => None,
                    Value::String(value) => Some(value.clone()),
                    _ => return Err("model header must be a string or null".into()),
                },
            ))
        })
        .collect()
}

fn encode_record_headers(
    entry: &mut Value,
    headers: &[(String, Option<String>)],
    limits: &Option<Value>,
) {
    if !headers.is_empty() {
        entry["headers"] = Value::Object(
            headers
                .iter()
                .map(|(key, value)| {
                    (
                        key.clone(),
                        value.as_ref().map_or(Value::Null, |value| json!(value)),
                    )
                })
                .collect(),
        );
        if entry.get(EXTENSION_KEY).is_none() {
            entry[EXTENSION_KEY] = json!({});
        }
        entry[EXTENSION_KEY]["headers"] = json!(headers);
    }
    if let Some(limits) = limits {
        entry["inputLimits"] = limits.clone();
    }
}

fn encode_pricing_tiers(tiers: &[crate::model_info::PricingTier]) -> Value {
    json!(
        tiers
            .iter()
            .map(|tier| {
                let mut value = pricing_to_json(&tier.pricing);
                value["inputTokensAbove"] = json!(tier.input_tokens_above);
                value
            })
            .collect::<Vec<_>>()
    )
}

pub(crate) fn decode_pricing_tiers(
    value: Option<&Value>,
) -> Result<Vec<crate::model_info::PricingTier>, String> {
    value
        .map(|value| {
            value
                .as_array()
                .ok_or("pricing tiers must be an array")?
                .iter()
                .map(|tier| {
                    Ok(crate::model_info::PricingTier {
                        input_tokens_above: tier
                            .get("inputTokensAbove")
                            .and_then(serde_json::Value::as_number)
                            .filter(|threshold| threshold.as_f64().is_some_and(f64::is_finite))
                            .cloned()
                            .ok_or("pricing tier requires inputTokensAbove")?,
                        pricing: pricing_from_json(tier)?,
                    })
                })
                .collect::<Result<Vec<_>, String>>()
        })
        .transpose()
        .map(Option::unwrap_or_default)
}

fn encode_catalog_record(record: &CatalogRecord) -> Value {
    use crate::classifier::ClassifierInputModality;
    use crate::images::ImageOutputModality;
    match record {
        CatalogRecord::Chat(model) => {
            let mut entry = encode_model_entry(model);
            entry["type"] = json!("chat");
            entry
        }
        CatalogRecord::Image(model) => {
            let input: Vec<_> = model
                .input
                .iter()
                .map(|value| match value {
                    super::CatalogInputModality::Text => "text",
                    super::CatalogInputModality::Image => "image",
                })
                .collect();
            let output: Vec<_> = model
                .output
                .iter()
                .map(|value| match value {
                    ImageOutputModality::Text => "text",
                    ImageOutputModality::Image => "image",
                })
                .collect();
            let mut entry = json!({"type": "image", "id": model.id, "name": model.name,
                "provider": model.provider_id, "api": model.api.as_str(), "baseUrl": model.base_url,
                "input": input, "output": output});
            if let Some(cost) = model.cost {
                entry["cost"] = json!({"input": cost.input, "output": cost.output,
                    "cacheRead": cost.cache_read, "cacheWrite": cost.cache_write});
            }
            if !model.cost_tiers.is_empty() {
                let tiers = encode_pricing_tiers(&model.cost_tiers);
                if model.cost.is_some() {
                    entry["cost"]["tiers"] = tiers;
                } else {
                    entry[EXTENSION_KEY] = json!({"imagePricingTiers": tiers});
                }
            }
            encode_record_headers(&mut entry, &model.headers, &model.input_limits);
            entry
        }
        CatalogRecord::Classifier(model) => {
            let input: Vec<_> = model
                .input
                .iter()
                .map(|value| match value {
                    ClassifierInputModality::Text => "text",
                    ClassifierInputModality::Image => "image",
                })
                .collect();
            let cost = model.cost.as_ref().map(|pricing| {
                let mut cost = pricing_to_json(&pricing.base);
                if !pricing.tiers.is_empty() {
                    cost["tiers"] = encode_pricing_tiers(&pricing.tiers);
                }
                cost
            });
            let mut entry = json!({"type": "classifier", "id": model.id, "name": model.name,
                "provider": model.provider_id, "api": model.api.as_str(), "baseUrl": model.base_url,
                "input": input, "contextWindow": model.context_window, "cost": cost});
            encode_record_headers(&mut entry, &model.headers, &model.input_limits);
            entry
        }
    }
}

fn decode_catalog_record(provider: &str, entry: &Value) -> Result<CatalogRecord, String> {
    let kind = match entry.get("type") {
        None => "chat",
        Some(Value::String(kind)) => kind.as_str(),
        Some(_) => return Err("model type must be a string".into()),
    };
    if let Some(owner) = entry.get("provider")
        && owner.as_str() != Some(provider)
    {
        return Err("model provider does not match its catalog owner".into());
    }
    if kind == "chat" {
        return decode_model_entry(entry).map(CatalogRecord::from);
    }
    if kind != "image" && kind != "classifier" {
        return Err(format!("unsupported model type '{kind}'"));
    }
    let string = |key: &str| {
        entry
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| format!("model requires string '{key}'"))
    };
    let id = string("id")?;
    let name = entry
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(&id)
        .to_owned();
    let api = crate::models::ApiId::new(string("api")?);
    let base_url = string("baseUrl")?;
    let input = entry
        .get("input")
        .and_then(Value::as_array)
        .ok_or("model requires input array")?
        .iter()
        .map(|value| match value.as_str() {
            Some("text") => Ok(super::CatalogInputModality::Text),
            Some("image") => Ok(super::CatalogInputModality::Image),
            _ => Err("unknown model input modality".to_owned()),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let headers = decode_record_headers(entry)?;
    let input_limits = entry
        .get("inputLimits")
        .filter(|value| !value.is_null())
        .cloned();
    if kind == "image" {
        let output = entry
            .get("output")
            .and_then(Value::as_array)
            .ok_or("image model requires output array")?
            .iter()
            .map(|value| match value.as_str() {
                Some("text") => Ok(crate::images::ImageOutputModality::Text),
                Some("image") => Ok(crate::images::ImageOutputModality::Image),
                _ => Err("unknown image output modality".to_owned()),
            })
            .collect::<Result<Vec<_>, _>>()?;
        let cost = entry
            .get("cost")
            .filter(|value| !value.is_null())
            .map(|cost| {
                Ok::<_, String>(known_pricing_from_json(cost)?.map(|pricing| {
                    crate::images::ImageCostRates {
                        input: pricing.input_cost_per_mtok,
                        output: pricing.output_cost_per_mtok,
                        cache_read: pricing.cache_read_cost_per_mtok,
                        cache_write: pricing.cache_write_cost_per_mtok,
                    }
                }))
            })
            .transpose()?
            .flatten();
        Ok(CatalogRecord::Image(crate::images::ImageModelRecord {
            provider_id: provider.into(),
            id,
            name,
            api,
            base_url,
            input,
            headers,
            input_limits,
            output,
            cost,
            cost_tiers: decode_pricing_tiers(
                entry
                    .pointer("/x-opi/imagePricingTiers")
                    .or_else(|| entry.pointer("/cost/tiers")),
            )?,
        }))
    } else {
        let cost = entry.get("cost").filter(|value| !value.is_null());
        let tiers = cost
            .and_then(|value| value.get("tiers"))
            .map(|value| {
                value
                    .as_array()
                    .ok_or("pricing tiers must be an array")?
                    .iter()
                    .map(|tier| {
                        Ok(crate::model_info::PricingTier {
                            input_tokens_above: tier
                                .get("inputTokensAbove")
                                .and_then(serde_json::Value::as_number)
                                .filter(|threshold| threshold.as_f64().is_some_and(f64::is_finite))
                                .cloned()
                                .ok_or("pricing tier requires inputTokensAbove")?,
                            pricing: pricing_from_json(tier)?,
                        })
                    })
                    .collect::<Result<Vec<_>, String>>()
            })
            .transpose()?
            .unwrap_or_default();
        let cost = cost
            .map(pricing_from_json)
            .transpose()?
            .map(|base| crate::model_info::ModelPricing { base, tiers });
        Ok(CatalogRecord::Classifier(
            crate::classifier::ClassifierModelRecord {
                provider_id: provider.into(),
                id,
                name,
                api,
                base_url,
                headers,
                input_limits,
                input: input
                    .into_iter()
                    .map(|value| match value {
                        super::CatalogInputModality::Text => {
                            crate::classifier::ClassifierInputModality::Text
                        }
                        super::CatalogInputModality::Image => {
                            crate::classifier::ClassifierInputModality::Image
                        }
                    })
                    .collect(),
                context_window: entry
                    .get("contextWindow")
                    .and_then(Value::as_u64)
                    .ok_or("classifier model requires contextWindow")?,
                cost,
            },
        ))
    }
}

pub(crate) fn validate_provider_entry(
    provider: &str,
    entry: &ModelsStoreEntry,
) -> Result<(), String> {
    if provider.is_empty() {
        return Err("empty provider id".into());
    }
    let mut seen = std::collections::HashSet::new();
    for model in &entry.models {
        if model.id().is_empty() || !seen.insert((model.operation(), model.id())) {
            return Err("empty or duplicate operation-qualified model id".into());
        }
        let (owner, api, headers) = match model {
            CatalogRecord::Chat(model) => {
                model.validate().map_err(|error| error.to_string())?;
                continue;
            }
            CatalogRecord::Image(model) => {
                let base = if let Some(cost) = model.cost {
                    let base = Pricing {
                        input_cost_per_mtok: cost.input,
                        output_cost_per_mtok: cost.output,
                        cache_read_cost_per_mtok: cost.cache_read,
                        cache_write_cost_per_mtok: cost.cache_write,
                    };
                    crate::model_info::validate_pricing(&base)
                        .map_err(|error| error.to_string())?;
                    base
                } else {
                    Pricing::default()
                };
                crate::model_info::ModelPricing {
                    base,
                    tiers: model.cost_tiers.clone(),
                }
                .validate_source_metadata()
                .map_err(|error| error.to_string())?;
                (&model.provider_id, &model.api, &model.headers)
            }
            CatalogRecord::Classifier(model) => {
                if let Some(cost) = &model.cost {
                    cost.validate_source_metadata()
                        .map_err(|error| error.to_string())?;
                }
                (&model.provider_id, &model.api, &model.headers)
            }
        };
        if owner != provider || api.as_str().is_empty() {
            return Err("model route identity does not match its catalog".into());
        }
        crate::provider_headers::validate_model_headers(headers)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub(crate) fn decode_provider_entry(
    provider: &str,
    value: &Value,
) -> Result<ModelsStoreEntry, String> {
    let models = value
        .get("models")
        .and_then(Value::as_array)
        .ok_or("provider requires models array")?
        .iter()
        .map(|model| decode_catalog_record(provider, model))
        .collect::<Result<Vec<_>, _>>()?;
    let timestamp = |key: &str| {
        value
            .get(key)
            .map(|value| {
                value
                    .as_i64()
                    .ok_or_else(|| format!("{key} must be Unix milliseconds as an integer"))
            })
            .transpose()
    };
    let entry = ModelsStoreEntry {
        models,
        last_modified: timestamp("lastModified")?,
        checked_at: timestamp("checkedAt")?,
        etag: value
            .get("etag")
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .ok_or("etag must be a string")
            })
            .transpose()?,
    };
    validate_provider_entry(provider, &entry)?;
    Ok(entry)
}

fn encode_provider_entry(entry: &ModelsStoreEntry) -> Value {
    let mut value =
        json!({"models": entry.models.iter().map(encode_catalog_record).collect::<Vec<_>>()});
    if let Some(timestamp) = entry.last_modified {
        value["lastModified"] = json!(timestamp);
    }
    if let Some(timestamp) = entry.checked_at {
        value["checkedAt"] = json!(timestamp);
    }
    if let Some(etag) = &entry.etag {
        value["etag"] = json!(etag);
    }
    value
}

/// Encode a full store document: schema identity, revision, providers.
pub(crate) fn encode_store_document(
    revision: u64,
    catalogs: &BTreeMap<String, ModelsStoreEntry>,
    publishers: &HashMap<String, CatalogPublisher>,
    invalidated_sources: &std::collections::BTreeSet<String>,
) -> Value {
    let mut providers = Map::new();
    for (provider_id, entry) in catalogs {
        let mut provider = encode_provider_entry(entry);
        if let Some(publisher) = publishers.get(provider_id) {
            provider[EXTENSION_KEY] = json!({"publisher": publisher});
        }
        providers.insert(provider_id.clone(), provider);
    }
    json!({
        "$schema": DISTRIBUTION_SCHEMA_ID,
        "revision": revision,
        "providers": Value::Object(providers),
        "x-opi": { "invalidatedSources": invalidated_sources },
    })
}

/// Decode a store document previously written by
/// [`encode_store_document`].
pub(crate) struct DecodedStore {
    pub revision: u64,
    pub catalogs: BTreeMap<String, ModelsStoreEntry>,
    pub publishers: BTreeMap<String, CatalogPublisher>,
    pub invalidated_sources: std::collections::BTreeSet<String>,
}

pub(crate) fn decode_store_document(document: &Value) -> Result<DecodedStore, ModelsStoreError> {
    let corrupt = |message: String| ModelsStoreError::Corrupt { message };
    let providers = document
        .get("providers")
        .and_then(Value::as_object)
        .ok_or_else(|| corrupt("store document requires a providers object".to_owned()))?;
    let revision = document
        .get("revision")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let mut catalogs = BTreeMap::new();
    let mut publishers = BTreeMap::new();
    for (provider_id, provider) in providers {
        let decoded = decode_provider_entry(provider_id, provider)
            .map_err(|message| corrupt(format!("provider '{provider_id}': {message}")))?;
        catalogs.insert(provider_id.clone(), decoded);
        if let Some(extension) = provider.get(EXTENSION_KEY) {
            let extension = extension.as_object().ok_or_else(|| {
                corrupt(format!(
                    "provider '{provider_id}' x-opi extension must be an object"
                ))
            })?;
            if let Some(value) = extension.get("publisher") {
                let publisher: CatalogPublisher =
                    serde_json::from_value(value.clone()).map_err(|_| {
                        corrupt(format!(
                            "provider '{provider_id}' has invalid publisher metadata"
                        ))
                    })?;
                publishers.insert(provider_id.clone(), publisher);
            }
        }
    }
    let invalidated_sources: std::collections::BTreeSet<String> = document
        .pointer("/x-opi/invalidatedSources")
        .map(|value| {
            serde_json::from_value(value.clone())
                .map_err(|_| corrupt("invalid source tombstones".into()))
        })
        .transpose()?
        .unwrap_or_default();
    for (provider, publisher) in &publishers {
        if matches!(publisher, CatalogPublisher::Source { id } if invalidated_sources.contains(id))
        {
            catalogs.remove(provider);
        }
    }
    publishers.retain(|provider, _| catalogs.contains_key(provider));
    Ok(DecodedStore {
        invalidated_sources,
        revision,
        catalogs,
        publishers,
    })
}
