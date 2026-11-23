//! OpenAI-compatible choice-level usage fallback.
//!
//! Pi 1.5.0 accepts `choices[0].usage` when a provider omits the standard
//! chunk-level usage member. Keep the public Rust stream result equivalent.

use opi_ai::api::openai_completions::{
    OpenAiChatEvent, OpenAiChatMapper, ParsedEvent, parse_sse_events,
};
use opi_ai::stream::AssistantStreamEvent;

#[test]
fn choice_level_usage_is_used_when_chunk_usage_is_absent() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("data/openai-choice-usage.json")).unwrap();
    let input = fixture["sse"].as_str().unwrap();
    let events: Vec<OpenAiChatEvent> = parse_sse_events(input)
        .flat_map(|event| match event {
            ParsedEvent::Valid(events) => events,
            ParsedEvent::Done | ParsedEvent::UsageError(_) | ParsedEvent::Malformed { .. } => {
                Vec::new()
            }
        })
        .collect();
    let mut mapper = OpenAiChatMapper::new(
        opi_ai::ApiKind::OpenAi,
        "moonshotai",
        opi_ai::model_info::WireApi::OpenAiCompletions,
        "",
    );
    let mut mapped: Vec<AssistantStreamEvent> = events
        .into_iter()
        .flat_map(|event| mapper.process(event))
        .map(|event| event.expect("choice usage fixture maps"))
        .collect();
    mapped.extend(
        mapper
            .finish(true)
            .into_iter()
            .map(|event| event.expect("choice usage fixture finishes")),
    );

    let done = mapped
        .into_iter()
        .find_map(|event| match event {
            AssistantStreamEvent::Done { message, .. } => Some(message),
            _ => None,
        })
        .expect("choice usage emits Done");
    assert!(done.usage.is_reported());
    assert_eq!(
        done.usage.input_tokens,
        fixture["expected"]["input_tokens"].as_u64().unwrap() as u32
    );
    assert_eq!(
        done.usage.cache_read_tokens,
        fixture["expected"]["cache_read_tokens"].as_u64().unwrap() as u32
    );
    assert_eq!(
        done.usage.output_tokens,
        fixture["expected"]["output_tokens"].as_u64().unwrap() as u32
    );
    assert_eq!(
        done.usage.total_tokens,
        Some(fixture["expected"]["total_tokens"].as_u64().unwrap())
    );
    println!(
        "{}",
        serde_json::json!({
            "case": fixture["id"],
            "stop": "stop",
            "raw_stop": "stop",
            "usage": {
                "input": done.usage.input_tokens,
                "output": done.usage.output_tokens,
                "cache_read": done.usage.cache_read_tokens,
                "cache_write": done.usage.cache_write_tokens,
                "total": done.usage.total_tokens,
            },
            "cost": done.usage.cost.total,
            "requests": 1,
        })
    );
}

#[test]
fn choice_level_usage_subset_is_validated() {
    let input = concat!(
        "data: {\"id\":\"choice-invalid\",\"model\":\"moonshot\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\",\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":2,\"completion_tokens_details\":{\"reasoning_tokens\":3}}}]}\n\n",
        "data: [DONE]\n\n",
    );
    assert!(
        parse_sse_events(input)
            .into_iter()
            .any(|event| matches!(event, ParsedEvent::UsageError(_)))
    );
}

#[test]
fn chunk_level_usage_wins_over_choice_level_fallback() {
    let input = concat!(
        "data: {\"id\":\"precedence\",\"model\":\"moonshot\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\"},\"finish_reason\":\"stop\",\"usage\":{\"prompt_tokens\":100,\"completion_tokens\":20}}],\"usage\":{\"prompt_tokens\":4,\"completion_tokens\":2,\"total_tokens\":6}}\n\n",
        "data: [DONE]\n\n",
    );
    let events: Vec<OpenAiChatEvent> = parse_sse_events(input)
        .flat_map(|event| match event {
            ParsedEvent::Valid(events) => events,
            ParsedEvent::Done | ParsedEvent::UsageError(_) | ParsedEvent::Malformed { .. } => {
                Vec::new()
            }
        })
        .collect();
    let mut mapper = OpenAiChatMapper::new(
        opi_ai::ApiKind::OpenAi,
        "moonshotai",
        opi_ai::model_info::WireApi::OpenAiCompletions,
        "",
    );
    let mut mapped: Vec<AssistantStreamEvent> = events
        .into_iter()
        .flat_map(|event| mapper.process(event))
        .map(|event| event.expect("precedence fixture maps"))
        .collect();
    mapped.extend(
        mapper
            .finish(true)
            .into_iter()
            .map(|event| event.expect("precedence fixture finishes")),
    );
    let done = mapped
        .into_iter()
        .find_map(|event| match event {
            AssistantStreamEvent::Done { message, .. } => Some(message),
            _ => None,
        })
        .expect("precedence emits Done");
    assert_eq!(done.usage.input_tokens, 4);
    assert_eq!(done.usage.output_tokens, 2);
    assert_eq!(done.usage.total_tokens, Some(6));
}
