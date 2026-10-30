//! Explicit, key-scoped OpenRouter catalog personalization.

use std::collections::HashMap;
use std::time::Duration;

use secrecy::{ExposeSecret, SecretString};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use super::{CatalogCost, CatalogModel, CatalogOperation};
use crate::http::HttpClient;
use crate::provider::{ProviderError, ProviderErrorSummary};

/// Return owned records personalized for one explicitly supplied key and endpoint.
///
/// This helper has no cache, store, environment lookup or publication side effect.
/// It preserves nonchat entries and the input on missing/empty keys or HTTP
/// 401/403/404. Other failures return an error without publishing partial data.
/// The source's 15-second deadline includes response body acquisition. Callers
/// own refresh scheduling and must not share one key's returned records with
/// another account. The authenticated bundled [`super::Catalog`] is unchanged.
pub async fn personalize_openrouter_catalog(
    catalog: Vec<CatalogModel>,
    base_url: &str,
    key: Option<&SecretString>,
    client: &HttpClient,
    cancel: CancellationToken,
) -> Result<Vec<CatalogModel>, ProviderError> {
    let Some(key) = key.filter(|key| !key.expose_secret().is_empty()) else {
        return Ok(catalog);
    };
    let acquire = async {
        let response = client
            .client()
            .get(format!("{}/models/user", base_url.trim_end_matches('/')))
            .header("accept", "application/json")
            .bearer_auth(key.expose_secret())
            .send()
            .await
            .map_err(|_| failure("OpenRouter catalog transport failed"))?;
        if matches!(response.status().as_u16(), 401 | 403 | 404) {
            return Ok(None);
        }
        if !response.status().is_success() {
            return Err(failure(format!(
                "OpenRouter catalog HTTP {}",
                response.status().as_u16()
            )));
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|_| failure("OpenRouter catalog body failed"))?;
        let body: Value = serde_json::from_slice(&bytes)
            .map_err(|_| failure("Invalid OpenRouter model list response"))?;
        Ok(Some(body))
    };
    let body = tokio::select! {
        biased;
        _ = cancel.cancelled() => return Err(ProviderError::Cancelled),
        result = tokio::time::timeout(Duration::from_secs(15), acquire) => result.map_err(|_| failure("OpenRouter catalog timeout"))??,
    };
    let Some(body) = body else {
        return Ok(catalog);
    };
    let entries = body
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| failure("Invalid OpenRouter model list response"))?;
    let allowed: HashMap<_, _> = entries
        .iter()
        .filter_map(|entry| {
            entry
                .get("id")
                .and_then(Value::as_str)
                .map(|id| (id, entry))
        })
        .collect();
    let mut personalized = Vec::with_capacity(catalog.len());
    for mut model in catalog {
        if model.provider != "openrouter" || model.operation != CatalogOperation::Chat {
            personalized.push(model);
            continue;
        }
        let Some(entry) = allowed.get(model.id.as_str()) else {
            continue;
        };
        let positive =
            |value: Option<&Value>| value.and_then(Value::as_u64).filter(|value| *value > 0);
        if let Some(limit) = positive(entry.pointer("/top_provider/context_length"))
            .or_else(|| positive(entry.get("context_length")))
        {
            model.context_window = Some(limit);
            model.raw["contextWindow"] = json!(limit);
        }
        if let Some(limit) = positive(entry.pointer("/top_provider/max_completion_tokens")) {
            model.max_tokens = Some(limit);
            model.raw["maxTokens"] = json!(limit);
        }
        if let Some(pricing) = entry.get("pricing").filter(|value| !value.is_null()) {
            let rates = known_rates(pricing);
            model.cost = rates;
            model.raw["cost"] = rates.map_or(Value::Null, |base| {
                let mut cost = rates_json(base);
                let tiers: Vec<_> = pricing
                    .get("overrides")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|entry| {
                        let threshold = entry.get("min_prompt_tokens")?.as_u64()?;
                        if ["utc_start", "utc_end", "utc_days"]
                            .iter()
                            .any(|key| entry.get(key).is_some())
                        {
                            return None;
                        }
                        let mut tier = rates_json(CatalogCost {
                            input: rate(entry.get("prompt")).unwrap_or(base.input),
                            output: rate(entry.get("completion")).unwrap_or(base.output),
                            cache_read: rate(entry.get("input_cache_read"))
                                .unwrap_or(base.cache_read),
                            cache_write: rate(entry.get("input_cache_write"))
                                .unwrap_or(base.cache_write),
                        });
                        tier["inputTokensAbove"] = json!(threshold);
                        Some(tier)
                    })
                    .collect();
                if !tiers.is_empty() {
                    cost["tiers"] = Value::Array(tiers);
                }
                cost
            });
        }
        personalized.push(model);
    }
    if cancel.is_cancelled() {
        return Err(ProviderError::Cancelled);
    }
    Ok(personalized)
}

fn failure(message: impl Into<String>) -> ProviderError {
    ProviderError::RequestFailed(ProviderErrorSummary::sanitized(message.into()))
}

fn rate(value: Option<&Value>) -> Option<f64> {
    let value: f64 = value?.as_str()?.parse().ok()?;
    let value = (value * 1_000_000.0 * 1_000_000.0).round() / 1_000_000.0;
    (value.is_finite() && value >= 0.0).then_some(value)
}

fn known_rates(value: &Value) -> Option<CatalogCost> {
    Some(CatalogCost {
        input: rate(value.get("prompt"))?,
        output: rate(value.get("completion"))?,
        cache_read: rate(value.get("input_cache_read"))?,
        cache_write: rate(value.get("input_cache_write"))?,
    })
}

fn rates_json(rates: CatalogCost) -> Value {
    json!({"input": rates.input, "output": rates.output, "cacheRead": rates.cache_read, "cacheWrite": rates.cache_write})
}
