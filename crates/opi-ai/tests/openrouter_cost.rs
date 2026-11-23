use opi_ai::api::anthropic::{
    AnthropicEvent, ParsedEvent as AnthropicParsedEvent,
    parse_sse_events as parse_anthropic_sse_events,
};
use opi_ai::api::openai_completions::{
    OpenAiChatEvent, ParsedEvent, parse_sse_events, parse_sse_events_for_openrouter,
};
use opi_ai::providers::openrouter::{
    OpenRouterCostDetails, OpenRouterUsageAccounting, apply_openrouter_reported_cost,
};
use opi_ai::stream::Usage;

fn usage() -> Usage {
    Usage::reported(1, 1, 0, 0, None, None).with_cost(opi_ai::CostBreakdown {
        input_cost: 0.25,
        output_cost: 0.75,
        ..opi_ai::CostBreakdown::default()
    })
}

#[test]
fn non_byok_uses_openrouter_charge_without_upstream_addition() {
    let mut value = usage();
    apply_openrouter_reported_cost(
        &mut value,
        &OpenRouterUsageAccounting {
            cost: Some(0.004),
            is_byok: Some(false),
            cost_details: Some(OpenRouterCostDetails {
                upstream_inference_cost: Some(0.003),
                upstream_inference_prompt_cost: Some(0.002),
                upstream_inference_completions_cost: Some(0.001),
                server_tool_cost: None,
            }),
        },
    );

    assert_eq!(value.cost.total, 0.004);
    assert!(value.cost.total_from_provider);
    assert_eq!(value.cost.catalog_total_cost(), 1.0);
}

#[test]
fn byok_adds_upstream_total_or_falls_back_to_components() {
    let mut value = usage();
    apply_openrouter_reported_cost(
        &mut value,
        &OpenRouterUsageAccounting {
            cost: Some(0.1),
            is_byok: Some(true),
            cost_details: Some(OpenRouterCostDetails {
                upstream_inference_cost: Some(0.9),
                upstream_inference_prompt_cost: Some(0.6),
                upstream_inference_completions_cost: Some(0.3),
                server_tool_cost: None,
            }),
        },
    );
    assert_eq!(value.cost.total, 1.0);
    assert!(value.cost.total_from_provider);

    let mut value = usage();
    apply_openrouter_reported_cost(
        &mut value,
        &OpenRouterUsageAccounting {
            cost: Some(0.1),
            is_byok: Some(true),
            cost_details: Some(OpenRouterCostDetails {
                upstream_inference_cost: None,
                upstream_inference_prompt_cost: Some(0.6),
                upstream_inference_completions_cost: Some(0.3),
                server_tool_cost: None,
            }),
        },
    );
    assert!((value.cost.total - 1.0).abs() < 1e-12);
    assert!(value.cost.total_from_provider);
}

#[test]
fn incomplete_or_invalid_openrouter_accounting_preserves_catalog_total() {
    for accounting in [
        OpenRouterUsageAccounting {
            cost: None,
            is_byok: Some(false),
            cost_details: None,
        },
        OpenRouterUsageAccounting {
            cost: Some(-0.1),
            is_byok: Some(false),
            cost_details: None,
        },
        OpenRouterUsageAccounting {
            cost: Some(0.1),
            is_byok: Some(true),
            cost_details: None,
        },
        OpenRouterUsageAccounting {
            cost: Some(0.1),
            is_byok: Some(true),
            cost_details: Some(OpenRouterCostDetails {
                upstream_inference_cost: None,
                upstream_inference_prompt_cost: None,
                upstream_inference_completions_cost: Some(0.3),
                server_tool_cost: None,
            }),
        },
    ] {
        let mut value = usage();
        apply_openrouter_reported_cost(&mut value, &accounting);
        assert_eq!(value.cost.total, 0.0);
        assert!(!value.cost.total_from_provider);
    }
}

#[test]
fn openai_chat_usage_maps_openrouter_cache_write_and_billed_total() {
    let input = concat!(
        "data: {\"id\":\"chatcmpl-cost\",\"model\":\"openrouter/auto\",",
        "\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],",
        "\"usage\":{\"prompt_tokens\":100,\"completion_tokens\":5,",
        "\"prompt_tokens_details\":{\"cached_tokens\":20,\"cache_write_tokens\":30},",
        "\"cost\":0.004,\"is_byok\":false,",
        "\"cost_details\":{\"upstream_inference_cost\":0.003}}}\n\n",
    );
    let usage = parse_sse_events_for_openrouter(input)
        .find_map(|event| match event {
            ParsedEvent::Valid(events) => events.into_iter().find_map(|event| match event {
                OpenAiChatEvent::Finish { usage, .. } => usage,
                _ => None,
            }),
            _ => None,
        })
        .expect("OpenRouter usage finish event");

    assert_eq!(usage.input_tokens, 50);
    assert_eq!(usage.cache_read_tokens, 20);
    assert_eq!(usage.cache_write_tokens, 30);
    assert_eq!(usage.cost.total, 0.004);
    assert!(usage.cost.total_from_provider);
}

#[test]
fn generic_openai_parser_does_not_treat_openrouter_fields_as_provider_billing() {
    let input = concat!(
        "data: {\"id\":\"chatcmpl-generic\",\"model\":\"custom\",",
        "\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],",
        "\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5,",
        "\"cost\":0.004,\"is_byok\":false}}\n\n",
    );
    let usage = parse_sse_events(input)
        .find_map(|event| match event {
            ParsedEvent::Valid(events) => events.into_iter().find_map(|event| match event {
                OpenAiChatEvent::Finish { usage, .. } => usage,
                _ => None,
            }),
            _ => None,
        })
        .expect("generic usage finish event");

    assert_eq!(usage.cost.total, 0.0);
    assert!(!usage.cost.total_from_provider);
}

#[test]
fn generic_anthropic_parser_does_not_treat_openrouter_fields_as_provider_billing() {
    let input = concat!(
        "event: message_start\n",
        "data: {\"type\":\"message_start\",\"message\":{",
        "\"id\":\"msg-generic\",\"model\":\"custom\",",
        "\"usage\":{\"input_tokens\":10,\"output_tokens\":5,",
        "\"cost\":0.004,\"is_byok\":false}}}\n\n",
    );
    let usage = parse_anthropic_sse_events(input)
        .find_map(|event| match event {
            AnthropicParsedEvent::Valid(AnthropicEvent::MessageStart { usage, .. }) => Some(usage),
            _ => None,
        })
        .expect("generic Anthropic usage event");

    assert_eq!(usage.cost.total, 0.0);
    assert!(!usage.cost.total_from_provider);
}
