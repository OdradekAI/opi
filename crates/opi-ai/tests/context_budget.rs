use opi_ai::context::{estimate_context_tokens, estimate_message_tokens, estimate_text_tokens};
use opi_ai::message::{
    AssistantContent, AssistantMessage, InputContent, Message, ToolDef, UserMessage,
};
use opi_ai::stream::{StopReason, Usage};
use opi_ai::transcript::{SystemMessage, SystemSection, ToolReference};

fn user(text: &str, timestamp_ms: i64) -> Message {
    Message::User(UserMessage {
        content: vec![InputContent::Text { text: text.into() }],
        timestamp_ms,
    })
}

fn assistant(timestamp_ms: i64) -> Message {
    Message::Assistant(AssistantMessage {
        content: vec![AssistantContent::Text {
            text: "done".into(),
            signature: None,
        }],
        api: opi_ai::ApiKind::OpenAi,
        provider: "test".into(),
        model: "model".into(),
        response_model: None,
        wire: None,
        response_id: None,
        usage: Usage::reported(500, 20, 100, 50, Some(10), Some(15)),
        stop_reason: StopReason::Stop,
        error_message: None,
        timestamp_ms,
        diagnostics: Vec::new(),
        duration_ms: None,
        provider_thinking_level: None,

        deferred: None,
    })
}

/// Assistant whose usage carries an independently reported total.
fn assistant_with_total(timestamp_ms: i64, total: Option<u64>, reported: bool) -> Message {
    let mut message = assistant(timestamp_ms);
    if let Message::Assistant(assistant) = &mut message {
        assistant.usage.total_tokens = total;
        assistant.usage.reported = reported;
    }
    message
}

#[test]
fn latest_applicable_usage_and_new_tail_replace_lifetime_sum() {
    let messages = vec![
        user("task", 1),
        assistant(2),
        user("next", 3),
        assistant(4),
        user("abcdefgh", 5),
    ];
    let estimate = estimate_context_tokens(Some("system"), &messages, &[]);
    assert_eq!(estimate.tokens, 673);
    assert_eq!(
        estimate.usage_tokens, 670,
        "child cache/reasoning subsets are not added twice"
    );
    assert_eq!(estimate.trailing_tokens, 3);
    assert_eq!(estimate.last_usage_index, Some(3));
}

#[test]
fn compacted_prefix_invalidates_retained_usage_until_new_response() {
    let mut messages = vec![user("summary!", 10), assistant(2), user("tail", 3)];
    let compacted = estimate_context_tokens(None, &messages, &[]);
    assert_eq!(compacted.tokens, 7);
    assert_eq!(compacted.last_usage_index, None);
    messages.push(assistant(11));
    let fresh = estimate_context_tokens(None, &messages, &[]);
    assert_eq!(fresh.tokens, 670);
    assert_eq!(fresh.last_usage_index, Some(3));
}

#[test]
fn failed_or_unreported_usage_falls_back_to_content_estimate() {
    for reason in [StopReason::Error, StopReason::Aborted] {
        let mut response = assistant(2);
        if let Message::Assistant(message) = &mut response {
            message.stop_reason = reason;
        }
        assert_eq!(
            estimate_context_tokens(None, &[user("task", 1), response], &[]).tokens,
            4
        );
    }
    let mut response = assistant(2);
    if let Message::Assistant(message) = &mut response {
        message.usage.reported = false;
    }
    assert_eq!(estimate_context_tokens(None, &[response], &[]).tokens, 2);
}

#[test]
fn retained_usage_applies_across_model_identity() {
    // Q35 removes the same-model restriction: any applicable successful
    // usage anchors the estimate regardless of the recorded model identity.
    let mut other_model = assistant(2);
    if let Message::Assistant(message) = &mut other_model {
        message.provider = "other".into();
        message.model = "other-model".into();
    }
    let messages = vec![user("task", 1), other_model, user("abcdefgh", 3)];
    let estimate = estimate_context_tokens(None, &messages, &[]);
    assert_eq!(estimate.tokens, 673);
    assert_eq!(estimate.usage_tokens, 670);
    assert_eq!(estimate.last_usage_index, Some(1));
}

#[test]
fn initial_context_counts_system_images_and_tool_schemas() {
    let image = Message::User(UserMessage {
        content: vec![InputContent::Image {
            source: opi_ai::message::ImageSource::Url {
                url: "https://example.invalid/image.png".into(),
            },
            media_type: opi_ai::message::MediaType::Png,
        }],
        timestamp_ms: 1,
    });
    let tools = vec![ToolDef {
        name: "read".into(),
        description: "Read a file".into(),
        input_schema: serde_json::json!({"type":"object"}),
        constrained_sampling: None,
    }];
    let schema_tokens = estimate_text_tokens(&serde_json::to_string(&tools).unwrap());
    let estimate = estimate_context_tokens(Some("abcdefgh"), &[image], &tools);
    assert_eq!(estimate.tokens, 1_372 + 3 + schema_tokens);
    assert_eq!(estimate.last_usage_index, None);
    assert_eq!(estimate_text_tokens("🙂🙂🙂"), 2);
    assert_eq!(estimate_message_tokens(&user("abcde", 0)), 2);
}

#[test]
fn anchor_supersedes_full_text_lower_bound() {
    // Q35 removes the same-model/full-text lower bound: an applicable usage
    // anchor replaces the content estimate instead of bounding it. The
    // system/tool parameters count only when no anchor exists.
    let estimate = estimate_context_tokens(
        Some(&"s".repeat(4_000)),
        &[assistant(2)],
        &[ToolDef {
            name: "read".into(),
            description: "Read a file".into(),
            input_schema: serde_json::json!({"type":"object"}),
            constrained_sampling: None,
        }],
    );
    assert_eq!(estimate.tokens, 670);
    assert_eq!(estimate.usage_tokens, 670);
}

#[test]
fn reported_total_differs_from_bucket_sum() {
    // An independently reported positive total wins over the bucket sum.
    let messages = vec![
        assistant_with_total(2, Some(999), true),
        user("abcdefgh", 3),
    ];
    let estimate = estimate_context_tokens(None, &messages, &[]);
    assert_eq!(estimate.usage_tokens, 999);
    assert_eq!(estimate.trailing_tokens, 3);
    assert_eq!(estimate.tokens, 1_002);
}

#[test]
fn explicit_zero_total_falls_back_to_fully_known_positive_sum() {
    // total=0 falls back to the bucket sum only when the parent buckets are
    // fully known (reported); unknown stays distinct from zero.
    let messages = vec![assistant_with_total(2, Some(0), true), user("abcdefgh", 3)];
    let estimate = estimate_context_tokens(None, &messages, &[]);
    assert_eq!(estimate.usage_tokens, 670);
    assert_eq!(estimate.tokens, 673);

    let unreported = vec![
        assistant_with_total(2, Some(999), false),
        user("abcdefgh", 3),
    ];
    let estimate = estimate_context_tokens(None, &unreported, &[]);
    assert_eq!(estimate.usage_tokens, 0);
    assert_eq!(estimate.tokens, 2 + 3);
}

#[test]
fn message_granularity_rounding_aggregates_before_ceil() {
    // One user message with two one-character blocks rounds once: ceil(2/3.5)
    // is 1, not two per-block rounds of 1 each.
    let two_blocks = Message::User(UserMessage {
        content: vec![
            InputContent::Text { text: "a".into() },
            InputContent::Text { text: "a".into() },
        ],
        timestamp_ms: 0,
    });
    assert_eq!(estimate_message_tokens(&two_blocks), 1);

    let three_blocks = Message::Assistant(AssistantMessage {
        content: vec![
            AssistantContent::Text {
                text: "a".into(),
                signature: None,
            },
            AssistantContent::Thinking {
                thinking: "a".into(),
                signature: None,
            },
            AssistantContent::ToolCall {
                tool_call: opi_ai::message::ToolCall {
                    id: "t".into(),
                    name: "n".into(),
                    arguments: "".into(),
                    arguments_error: None,
                    signature: None,
                },
            },
        ],
        api: opi_ai::ApiKind::OpenAi,
        provider: "test".into(),
        model: "model".into(),
        response_model: None,
        wire: None,
        response_id: None,
        usage: Usage::unknown(),
        stop_reason: StopReason::Stop,
        error_message: None,
        timestamp_ms: 0,
        diagnostics: Vec::new(),
        duration_ms: None,
        provider_thinking_level: None,

        deferred: None,
    });
    assert_eq!(estimate_message_tokens(&three_blocks), 1);

    assert_eq!(estimate_text_tokens(""), 0);
    assert_eq!(estimate_text_tokens("abcdefg"), 2);
    assert_eq!(estimate_text_tokens("abcdefgh"), 3);
}

#[test]
fn system_message_rounds_prompt_sections_and_tool_lists_separately() {
    let system = Message::System(SystemMessage {
        content: "a".into(),
        sections: vec![SystemSection {
            name: "s".into(),
            value: Some("a".into()),
        }],
        tools_added: vec![ToolDef {
            name: "tool".into(),
            description: "tool".into(),
            input_schema: serde_json::json!({"type":"object"}),
            constrained_sampling: None,
        }],
        tools_removed: vec![ToolReference {
            name: "gone".into(),
        }],
        timestamp_ms: 0,
    });
    let body_and_sections = estimate_text_tokens("a\n\na");
    let added = estimate_text_tokens(&serde_json::to_string(&system_tools_added(&system)).unwrap());
    let removed =
        estimate_text_tokens(&serde_json::to_string(&system_tools_removed(&system)).unwrap());
    let expected = body_and_sections + added + removed;
    assert_eq!(estimate_message_tokens(&system), expected);
    // Separate rounding must not collapse into one round over the raw sum:
    // the rendered body and the added-tool JSON each leave unused capacity, so
    // their separate ceils exceed the single ceil over the concatenation.
    let combined = estimate_text_tokens(&format!(
        "a\n\na{}{}",
        serde_json::to_string(&system_tools_added(&system)).unwrap(),
        serde_json::to_string(&system_tools_removed(&system)).unwrap(),
    ));
    assert_ne!(estimate_message_tokens(&system), combined);
}

#[test]
fn trailing_system_message_after_anchor_counts_separately() {
    let messages = vec![
        assistant(2),
        Message::System(SystemMessage {
            content: "abcdefgh".into(),
            sections: Vec::new(),
            tools_added: Vec::new(),
            tools_removed: Vec::new(),
            timestamp_ms: 3,
        }),
    ];
    let estimate = estimate_context_tokens(None, &messages, &[]);
    assert_eq!(estimate.usage_tokens, 670);
    assert_eq!(estimate.trailing_tokens, 3);
    assert_eq!(estimate.tokens, 673);
}

fn system_tools_added(message: &Message) -> &Vec<ToolDef> {
    match message {
        Message::System(system) => &system.tools_added,
        _ => unreachable!(),
    }
}

fn system_tools_removed(message: &Message) -> &Vec<ToolReference> {
    match message {
        Message::System(system) => &system.tools_removed,
        _ => unreachable!(),
    }
}
