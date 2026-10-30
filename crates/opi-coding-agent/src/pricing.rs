//! Built-in model pricing lookup table.
//!
//! Used by `SessionCoordinator::cost_summary` to convert accumulated token
//! usage into a USD cost breakdown. Prices are per-million-tokens (USD) and
//! reflect public list prices at time of writing. The lookup is best-effort:
//! unknown models return `None`, in which case the runtime simply shows
//! token totals without a cost figure.
//!
//! The table is intentionally small — covering only the default models for
//! each supported provider. Users wanting full coverage can supply their own
//! pricing externally.

use opi_ai::stream::Pricing;

/// Look up pricing for a model spec of the form `provider:model`.
///
/// Returns `None` for unknown specs or models without published pricing.
pub fn lookup_pricing(model_spec: &str) -> Option<Pricing> {
    let (provider, model) = model_spec.split_once(':')?;
    match provider {
        "anthropic" => anthropic_pricing(model),
        "openai" | "openai-responses" => openai_pricing(model),
        "openrouter" => openrouter_pricing(model),
        "google" => gemini_pricing(model),
        "mistral" => mistral_pricing(model),
        _ => None,
    }
}

fn anthropic_pricing(model: &str) -> Option<Pricing> {
    if model.contains("opus") {
        Some(Pricing {
            input_cost_per_mtok: 15.0,
            output_cost_per_mtok: 75.0,
            cache_read_cost_per_mtok: 1.5,
            cache_write_cost_per_mtok: 18.75,
        })
    } else if model.contains("sonnet") {
        Some(Pricing {
            input_cost_per_mtok: 3.0,
            output_cost_per_mtok: 15.0,
            cache_read_cost_per_mtok: 0.3,
            cache_write_cost_per_mtok: 3.75,
        })
    } else if model.contains("haiku") {
        Some(Pricing {
            input_cost_per_mtok: 0.8,
            output_cost_per_mtok: 4.0,
            cache_read_cost_per_mtok: 0.08,
            cache_write_cost_per_mtok: 1.0,
        })
    } else {
        None
    }
}

fn openai_pricing(model: &str) -> Option<Pricing> {
    if model.starts_with("gpt-4o-mini") {
        Some(Pricing {
            input_cost_per_mtok: 0.15,
            output_cost_per_mtok: 0.60,
            cache_read_cost_per_mtok: 0.075,
            cache_write_cost_per_mtok: 0.0,
        })
    } else if model.starts_with("gpt-4o") {
        Some(Pricing {
            input_cost_per_mtok: 2.50,
            output_cost_per_mtok: 10.0,
            cache_read_cost_per_mtok: 1.25,
            cache_write_cost_per_mtok: 0.0,
        })
    } else if model.starts_with("gpt-4-turbo") {
        Some(Pricing {
            input_cost_per_mtok: 10.0,
            output_cost_per_mtok: 30.0,
            cache_read_cost_per_mtok: 0.0,
            cache_write_cost_per_mtok: 0.0,
        })
    } else if model.starts_with("gpt-3.5") {
        Some(Pricing {
            input_cost_per_mtok: 0.50,
            output_cost_per_mtok: 1.50,
            cache_read_cost_per_mtok: 0.0,
            cache_write_cost_per_mtok: 0.0,
        })
    } else {
        None
    }
}

fn openrouter_pricing(model: &str) -> Option<Pricing> {
    // OpenRouter forwards to many backends; try common prefixes.
    if let Some(stripped) = model.strip_prefix("anthropic/") {
        return anthropic_pricing(stripped);
    }
    if let Some(stripped) = model.strip_prefix("openai/") {
        return openai_pricing(stripped);
    }
    if let Some(stripped) = model.strip_prefix("google/") {
        return gemini_pricing(stripped);
    }
    if let Some(stripped) = model.strip_prefix("mistralai/") {
        return mistral_pricing(stripped);
    }
    None
}

fn gemini_pricing(model: &str) -> Option<Pricing> {
    if model.contains("flash") {
        Some(Pricing {
            input_cost_per_mtok: 0.075,
            output_cost_per_mtok: 0.30,
            cache_read_cost_per_mtok: 0.01875,
            cache_write_cost_per_mtok: 0.0,
        })
    } else if model.contains("pro") {
        Some(Pricing {
            input_cost_per_mtok: 1.25,
            output_cost_per_mtok: 5.0,
            cache_read_cost_per_mtok: 0.3125,
            cache_write_cost_per_mtok: 0.0,
        })
    } else {
        None
    }
}

fn mistral_pricing(model: &str) -> Option<Pricing> {
    if model.contains("large") {
        Some(Pricing {
            input_cost_per_mtok: 2.0,
            output_cost_per_mtok: 6.0,
            cache_read_cost_per_mtok: 0.0,
            cache_write_cost_per_mtok: 0.0,
        })
    } else if model.contains("medium") {
        Some(Pricing {
            input_cost_per_mtok: 2.7,
            output_cost_per_mtok: 8.1,
            cache_read_cost_per_mtok: 0.0,
            cache_write_cost_per_mtok: 0.0,
        })
    } else if model.contains("small") {
        Some(Pricing {
            input_cost_per_mtok: 0.20,
            output_cost_per_mtok: 0.60,
            cache_read_cost_per_mtok: 0.0,
            cache_write_cost_per_mtok: 0.0,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anthropic_sonnet_resolves() {
        let p = lookup_pricing("anthropic:claude-sonnet-4").unwrap();
        assert_eq!(p.input_cost_per_mtok, 3.0);
        assert_eq!(p.output_cost_per_mtok, 15.0);
    }

    #[test]
    fn openai_gpt4o_mini_resolves() {
        let p = lookup_pricing("openai:gpt-4o-mini").unwrap();
        assert_eq!(p.input_cost_per_mtok, 0.15);
    }

    #[test]
    fn gemini_flash_resolves() {
        let p = lookup_pricing("google:gemini-1.5-flash").unwrap();
        assert_eq!(p.input_cost_per_mtok, 0.075);
    }

    #[test]
    fn mistral_large_resolves() {
        let p = lookup_pricing("mistral:mistral-large-latest").unwrap();
        assert_eq!(p.input_cost_per_mtok, 2.0);
    }

    #[test]
    fn openrouter_forwards_to_underlying() {
        let p = lookup_pricing("openrouter:anthropic/claude-sonnet-4").unwrap();
        assert_eq!(p.input_cost_per_mtok, 3.0);
    }

    #[test]
    fn unknown_model_returns_none() {
        assert!(lookup_pricing("anthropic:not-a-real-model").is_none());
        assert!(lookup_pricing("malformed").is_none());
        assert!(lookup_pricing("future-provider:foo").is_none());
    }
}
