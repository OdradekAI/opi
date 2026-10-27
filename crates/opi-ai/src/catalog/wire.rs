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
    OpenAiCompletionsCompat, OpenAiResponsesCompat, ThinkingLevel, ThinkingLevelMap,
    ThinkingLevelMapping, WireApi, WireCompat,
};
use crate::stream::Pricing;

use super::ModelsStoreError;
use super::store::CatalogPublisher;

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
        let value = match map.resolve(level) {
            Ok(Some(name)) => json!({ "mapped": name }),
            Ok(None) => json!({ "identity": true }),
            Err(_) => json!({ "unsupported": true }),
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
            "forceAdaptiveThinking": inner.force_adaptive_thinking,
            "supportsTemperature": inner.supports_temperature,
            "supportsStrictTools": inner.supports_strict_tools,
        }),
        WireCompat::OpenAiCompletions(inner) => json!({
            "api": WireApi::OpenAiCompletions.as_str(),
            "systemRoleOverride": inner.system_role_override,
            "maxTokensField": inner.max_tokens_field,
            "toolResultNameField": inner.tool_result_name_field,
            "usageInStream": inner.usage_in_stream,
            "strictToolSchema": inner.strict_tool_schema,
            "reasoningEffort": inner.reasoning_effort,
            "cacheKey": inner.cache_key,
            "sendSessionAffinityHeaders": inner.send_session_affinity_headers,
            "requireAssistantAfterToolResult": inner.require_assistant_after_tool_result,
            "chatCompletionsPath": inner.chat_completions_path,
            "supportsStore": inner.supports_store,
            "supportsDeveloperRole": inner.supports_developer_role,
            "supportsReasoningEffort": inner.supports_reasoning_effort,
        }),
        WireCompat::OpenAiResponses(inner) => json!({
            "api": WireApi::OpenAiResponses.as_str(),
            "store": inner.store,
            "reasoningEffort": inner.reasoning_effort,
            "strictTools": inner.strict_tools,
            "responsesPath": inner.responses_path,
            "sendSessionIdHeader": inner.send_session_id_header,
        }),
        WireCompat::OpenAiCodexResponses(inner) => json!({
            "api": WireApi::OpenAiCodexResponses.as_str(),
            "responsesPath": inner.responses_path,
            "deriveAccountId": inner.derive_account_id,
        }),
        WireCompat::GoogleGenerativeAi => json!({ "api": WireApi::GoogleGenerativeAi.as_str() }),
        WireCompat::GoogleVertex => json!({ "api": WireApi::GoogleVertex.as_str() }),
        WireCompat::BedrockConverseStream => {
            json!({ "api": WireApi::BedrockConverseStream.as_str() })
        }
        WireCompat::AzureOpenAiCompletions => {
            json!({ "api": WireApi::AzureOpenAiCompletions.as_str() })
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

fn compat_from_extension(value: &Value) -> Result<Option<WireCompat>, String> {
    let Some(api) = value.get("api").and_then(Value::as_str) else {
        return Ok(None);
    };
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
            },
        ))),
        WireApi::OpenAiCompletions => Ok(Some(WireCompat::OpenAiCompletions(
            OpenAiCompletionsCompat {
                system_role_override: opt_string(value, "systemRoleOverride")?,
                max_tokens_field: value
                    .get("maxTokensField")
                    .and_then(Value::as_str)
                    .unwrap_or("max_tokens")
                    .to_owned(),
                tool_result_name_field: flag("toolResultNameField")?,
                usage_in_stream: flag("usageInStream")?,
                strict_tool_schema: flag("strictToolSchema")?,
                reasoning_effort: opt_string(value, "reasoningEffort")?,
                cache_key: opt_string(value, "cacheKey")?,
                send_session_affinity_headers: flag("sendSessionAffinityHeaders")?,
                require_assistant_after_tool_result: flag("requireAssistantAfterToolResult")?,
                chat_completions_path: value
                    .get("chatCompletionsPath")
                    .and_then(Value::as_str)
                    .unwrap_or("/v1/chat/completions")
                    .to_owned(),
                supports_store: flag("supportsStore")?,
                supports_developer_role: flag("supportsDeveloperRole")?,
                supports_reasoning_effort: flag("supportsReasoningEffort")?,
            },
        ))),
        WireApi::OpenAiResponses => Ok(Some(WireCompat::OpenAiResponses(OpenAiResponsesCompat {
            store: value.get("store").and_then(Value::as_bool),
            reasoning_effort: opt_string(value, "reasoningEffort")?,
            strict_tools: flag("strictTools")?,
            responses_path: value
                .get("responsesPath")
                .and_then(Value::as_str)
                .unwrap_or("/v1/responses")
                .to_owned(),
            send_session_id_header: flag("sendSessionIdHeader")?,
        }))),
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
        WireApi::BedrockConverseStream => Ok(Some(WireCompat::BedrockConverseStream)),
        WireApi::AzureOpenAiCompletions => Ok(Some(WireCompat::AzureOpenAiCompletions)),
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

    if let Some(thinking) = extension.and_then(|ext| ext.get("thinking")) {
        model = model.with_thinking_level_map(thinking_map_from_extension(thinking)?);
    }
    if let Some(params) = extension.and_then(|ext| ext.get("samplingParams")) {
        model.sampling_params = params
            .as_object()
            .ok_or("x-opi.samplingParams must be an object")?
            .clone();
    }
    if let Some(levels) = extension.and_then(|ext| ext.get("samplingParamsByThinkingLevel")) {
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

    if let Some(cost) = entry.get("cost") {
        let base = pricing_from_json(cost)?;
        let tiers = extension
            .and_then(|ext| ext.get("pricingTiers"))
            .and_then(Value::as_array)
            .map(|tiers| {
                tiers
                    .iter()
                    .map(|tier| {
                        Ok(crate::model_info::PricingTier {
                            input_tokens_above: tier
                                .get("above")
                                .and_then(Value::as_u64)
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

/// Encode a full store document: schema identity, revision, providers.
pub(crate) fn encode_store_document(
    revision: u64,
    catalogs: &BTreeMap<String, Vec<ModelInfo>>,
    publishers: &HashMap<String, CatalogPublisher>,
) -> Value {
    let mut providers = Map::new();
    for (provider_id, models) in catalogs {
        let mut provider = json!({
            "models": models
                .iter()
                .map(encode_model_entry)
                .collect::<Vec<_>>(),
        });
        if let Some(publisher) = publishers.get(provider_id) {
            provider[EXTENSION_KEY] = json!({"publisher": publisher});
        }
        providers.insert(provider_id.clone(), provider);
    }
    json!({
        "$schema": DISTRIBUTION_SCHEMA_ID,
        "revision": revision,
        "providers": Value::Object(providers),
    })
}

/// Decode a store document previously written by
/// [`encode_store_document`].
pub(crate) struct DecodedStore {
    pub revision: u64,
    pub catalogs: BTreeMap<String, Vec<ModelInfo>>,
    pub publishers: BTreeMap<String, CatalogPublisher>,
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
        let Some(models) = provider.get("models").and_then(Value::as_array) else {
            return Err(corrupt(format!(
                "provider '{provider_id}' requires a models array"
            )));
        };
        let mut decoded = Vec::with_capacity(models.len());
        for model in models {
            decoded.push(
                decode_model_entry(model)
                    .map_err(|message| corrupt(format!("provider '{provider_id}': {message}")))?,
            );
        }
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
    Ok(DecodedStore {
        revision,
        catalogs,
        publishers,
    })
}
