//! Validate the editable opi records before they become a published catalog.

use crate::profiles::{BuiltinEndpoint, BuiltinProviderProfile, builtin_profile};
use opi_ai::{
    WireApi,
    catalog::{Catalog, CatalogModel, CatalogOperation, decode_document},
};
use serde_json::{Map, Value, json};

#[path = "../../src/providers/catalog_routes.rs"]
mod catalog_routes;
#[path = "unique_json.rs"]
mod unique_json;

pub(super) fn parse(raw: &[u8]) -> serde_json::Result<Vec<Value>> {
    unique_json::parse(raw)
}

pub(super) fn validate(catalog: &Catalog) -> Result<(), String> {
    for provider in catalog.providers() {
        for operation in [
            CatalogOperation::Chat,
            CatalogOperation::Image,
            CatalogOperation::Classifier,
        ] {
            for entry in catalog.models(provider, operation).into_iter().flatten() {
                let check = || {
                    let profile = builtin_profile(provider)
                        .ok_or("provider: no registered builtin profile")?;
                    validate_entry(profile, entry)
                };
                check().map_err(|detail| format!("{provider}/{}: {detail}", entry.identity()))?;
            }
        }
    }
    Ok(())
}

fn validate_entry(profile: &BuiltinProviderProfile, entry: &CatalogModel) -> Result<(), String> {
    let api = entry.api.as_str();
    let route_exists = match entry.operation {
        CatalogOperation::Classifier => catalog_routes::CLASSIFIERS.contains(&(profile.id, api)),
        CatalogOperation::Image => (profile.id, api) == catalog_routes::IMAGE,
        CatalogOperation::Chat => api.parse::<WireApi>().is_ok_and(|wire| {
            use WireApi::*;
            match profile.endpoint {
                BuiltinEndpoint::AzureDeployment => {
                    matches!(wire, OpenAiCompletions | AzureOpenAiResponses)
                }
                BuiltinEndpoint::VertexLocation => wire == GoogleVertex,
                BuiltinEndpoint::BedrockRegion => wire == BedrockConverseStream,
                BuiltinEndpoint::Dynamic => wire == PiMessages,
                BuiltinEndpoint::ProviderBaseUrl | BuiltinEndpoint::PerModelBaseUrl => {
                    !profile.classifier_only
                        && matches!(
                            wire,
                            OpenAiCompletions
                                | AnthropicMessages
                                | OpenAiResponses
                                | MistralConversations
                                | OpenAiCodexResponses
                                | GoogleGenerativeAi
                        )
                }
            }
        }),
    };
    if !route_exists {
        return Err(format!(
            "api: no builtin {} route for '{api}'",
            entry.operation
        ));
    }

    let raw = &entry.raw;
    let object = raw.as_object().expect("Catalog validates object records");
    for (key, value) in object {
        match key.as_str() {
            "id" | "name" | "type" | "provider" | "api" | "baseUrl" | "input" | "headers" => {}
            "cost" => validate_cost(value)?,
            "inputLimits" => validate_input_limits(value)?,
            "contextWindow" if entry.operation != CatalogOperation::Image => {}
            "output" if entry.operation == CatalogOperation::Image => {}
            "compat" => validate_compat(entry, value)?,
            "reasoning" | "maxTokens" | "samplingParams" | "samplingParamsByThinkingLevel"
                if entry.operation == CatalogOperation::Chat => {}
            "thinkingLevelMap" if entry.operation == CatalogOperation::Chat => {
                let levels = object_at(value, key)?;
                if api == "mistral-conversations" {
                    for (level, effort) in levels {
                        if !effort.is_null()
                            && !matches!(
                                effort.as_str(),
                                Some("none" | "low" | "medium" | "high" | "max")
                            )
                        {
                            return Err(format!(
                                "thinkingLevelMap.{level}: unknown Mistral reasoning effort"
                            ));
                        }
                    }
                }
            }
            "promptCache" if entry.operation == CatalogOperation::Chat => {
                for (tier, seconds) in object_at(value, key)? {
                    if !matches!(tier.as_str(), "long" | "short")
                        || !seconds.as_f64().is_some_and(|n| n > 0.0)
                    {
                        return Err(format!(
                            "promptCache.{tier}: expected a short or long lifetime in positive seconds"
                        ));
                    }
                }
            }
            "lab" if profile.id == "radius" && value.is_string() => {}
            "enabled" if profile.id == "radius" && value.is_boolean() => {}
            "free" if profile.id == "radius" && value.is_boolean() => {}
            "providers" if profile.id == "radius" && value.is_array() => {}
            // x-opi is interpreted by the persistence codec. It cannot carry opaque author metadata.
            key if key.starts_with("x-") && key.len() > 2 && key != "x-opi" => {}
            _ => {
                return Err(format!(
                    "{key}: unsupported field or value for this operation"
                ));
            }
        }
    }

    if entry.operation == CatalogOperation::Chat {
        let model = entry
            .to_chat_model_info()
            .map_err(|error| error.to_string())?;
        model.validate().map_err(|error| error.to_string())?;
    } else {
        let provider = profile.id;
        decode_document(&json!({"providers": {(provider): {"models": [raw]}}}))
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn object_at<'a>(value: &'a Value, path: &str) -> Result<&'a Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{path}: expected an object"))
}

fn validate_cost(value: &Value) -> Result<(), String> {
    let rates = object_at(value, "cost")?;
    for key in rates.keys() {
        if !matches!(
            key.as_str(),
            "input" | "output" | "cacheRead" | "cacheWrite" | "tiers"
        ) {
            return Err(format!("cost.{key}: unknown rate"));
        }
    }
    if let Some(tiers) = rates.get("tiers") {
        let tiers = tiers.as_array().ok_or("cost.tiers: expected an array")?;
        for (i, tier) in tiers.iter().enumerate() {
            let path = format!("cost.tiers[{i}]");
            for (key, value) in object_at(tier, &path)? {
                if !matches!(
                    key.as_str(),
                    "input" | "output" | "cacheRead" | "cacheWrite" | "inputTokensAbove"
                ) || !value.is_number()
                {
                    return Err(format!(
                        "{path}.{key}: expected a numeric rate or threshold"
                    ));
                }
            }
        }
    }
    Ok(())
}

fn validate_input_limits(value: &Value) -> Result<(), String> {
    for (key, value) in object_at(value, "inputLimits")? {
        match key.as_str() {
            "maxRequestBytes" if positive(value) => {}
            "images" => {
                for (key, value) in object_at(value, "inputLimits.images")? {
                    match key.as_str() {
                        "maxPerMessage" | "maxPerRequest" if positive(value) => {}
                        "resize" => {
                            for (key, value) in object_at(value, "inputLimits.images.resize")? {
                                let valid = match key.as_str() {
                                    "maxWidth" | "maxHeight" | "maxBytes" => positive(value),
                                    "jpegQuality" => {
                                        value.as_u64().is_some_and(|n| (1..=100).contains(&n))
                                    }
                                    _ => false,
                                };
                                if !valid {
                                    return Err(format!(
                                        "inputLimits.images.resize.{key}: unknown field or invalid limit"
                                    ));
                                }
                            }
                        }
                        _ => {
                            return Err(format!(
                                "inputLimits.images.{key}: unknown field or invalid limit"
                            ));
                        }
                    }
                }
            }
            _ => return Err(format!("inputLimits.{key}: unknown field or invalid limit")),
        }
    }
    Ok(())
}

fn positive(value: &Value) -> bool {
    value.as_u64().is_some_and(|n| n > 0)
}

fn validate_compat(entry: &CatalogModel, value: &Value) -> Result<(), String> {
    let fields = object_at(value, "compat")?;
    for (key, value) in fields {
        let key = key.as_str();
        let valid = match entry.api.as_str() {
            "openai-completions" => match key {
                "supportsMidConvoSystemMessages"
                | "supportsMidConvoToolAdditions"
                | "supportsStore"
                | "supportsDeveloperRole"
                | "supportsReasoningEffort"
                | "supportsUsageInStreaming"
                | "supportsFinishReason"
                | "supportsOpenAIGrammarTools"
                | "supportsLongCacheRetention"
                | "supportsStrictMode"
                | "requiresToolResultName"
                | "sendSessionAffinityHeaders"
                | "requiresAssistantAfterToolResult"
                | "requiresThinkingAsText"
                | "requiresReasoningContentOnAssistantMessages"
                | "zaiToolStream"
                | "supportsThinkingTokenBudget" => value.is_boolean() || value.is_null(),
                "thinkingFormat"
                | "systemRoleOverride"
                | "thinkingTokenBudgetField"
                | "cacheControlFormat"
                | "sessionAffinityFormat" => value.is_string() || value.is_null(),
                "maxTokensField" => {
                    value.is_null()
                        || matches!(value.as_str(), Some("max_tokens" | "max_completion_tokens"))
                }
                "openRouterRouting"
                | "vercelGatewayRouting"
                | "chatTemplateArgs"
                | "chatTemplateKwargs" => value.is_object() || value.is_null(),
                "vllmPriority" => value.is_number() || value.is_null(),
                _ => false,
            },
            "anthropic-messages" => match key {
                "allowEmptySignature"
                | "supportsEagerToolInputStreaming"
                | "supportsLongCacheRetention"
                | "supportsCacheControlOnTools"
                | "supportsMidConvoSystemMessages"
                | "supportsMidConvoToolChanges"
                | "forceAdaptiveThinking"
                | "supportsTemperature"
                | "supportsStrictTools"
                | "supportsMidConvoEffort"
                | "sendSessionAffinityHeaders" => value.is_boolean() || value.is_null(),
                "sessionAffinityFormat" => value.is_string() || value.is_null(),
                "allowedFallbackModels" => value.is_array() || value.is_null(),
                _ => false,
            },
            "openai-responses" | "azure-openai-responses" | "openai-codex-responses" => match key {
                "supportsStrictMode"
                | "supportsOpenAIGrammarTools"
                | "supportsMidConvoEffort"
                | "supportsMidConvoSystemMessages"
                | "supportsAdditionalTools"
                | "supportsToolSearch"
                | "supportsDeveloperRole"
                | "supportsLongCacheRetention"
                | "supportsExplicitPromptCacheMode"
                | "supportsMaxOutputTokens" => value.is_boolean() || value.is_null(),
                "sessionAffinityFormat" => value.is_string() || value.is_null(),
                // This shipped source observation has no Responses interpretation.
                "supportsReasoningEffort" => {
                    entry.provider == "opencode"
                        && entry.id == "grok-build-0.1"
                        && entry.api == "openai-responses"
                        && value.as_bool() == Some(false)
                }
                _ => false,
            },
            "bedrock-converse-stream" => {
                key == "supportsStrictMode" && (value.is_boolean() || value.is_null())
            }
            // System One image transport is provider-specific. Record input
            // modalities still decide whether a model can use it.
            "typesafe-system-one" => match entry.provider.as_str() {
                "openrouter" => match key {
                    "imageInput" => value.as_str() == Some("state-image-urls"),
                    "sendImageOptInHeader" => value.as_bool() == Some(false),
                    _ => false,
                },
                "typesafe" => match key {
                    "imageInput" => value.as_str() == Some("assets"),
                    "sendImageOptInHeader" => value.as_bool() == Some(true),
                    _ => false,
                },
                _ => false,
            },
            "cloudflare-workers-ai-system-one" => match key {
                "imageInput" => matches!(value.as_str(), Some("assets") | Some("images")),
                "sendImageOptInHeader" => value.as_bool() == Some(false),
                _ => false,
            },
            _ => false,
        };
        if !valid {
            return Err(format!(
                "compat.{key}: unsupported field or value for {}",
                entry.api
            ));
        }
    }
    Ok(())
}
