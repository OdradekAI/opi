use opi_ai::context::{estimate_context_tokens, estimate_message_tokens, estimate_text_tokens};
use opi_ai::message::{
    AssistantContent, AssistantMessage, InputContent, Message, ToolDef, UserMessage,
};
use opi_ai::stream::{StopReason, Usage};

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
        }],
        api: opi_ai::ApiKind::OpenAi,
        provider: "test".into(),
        model: "model".into(),
        response_model: None,
        response_id: None,
        usage: Usage::reported(500, 20, 100, 50, Some(10), Some(15)),
        stop_reason: StopReason::Stop,
        error_message: None,
        timestamp_ms,
    })
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
    let estimate = estimate_context_tokens(Some("system"), &messages, &[], "test:model");
    assert_eq!(estimate.tokens, 672);
    assert_eq!(
        estimate.usage_tokens, 670,
        "child cache/reasoning subsets are not added twice"
    );
    assert_eq!(estimate.trailing_tokens, 2);
    assert_eq!(estimate.last_usage_index, Some(3));
}

#[test]
fn compacted_prefix_invalidates_retained_usage_until_new_response() {
    let mut messages = vec![user("summary!", 10), assistant(2), user("tail", 3)];
    let compacted = estimate_context_tokens(None, &messages, &[], "test:model");
    assert_eq!(compacted.tokens, 4);
    assert_eq!(compacted.last_usage_index, None);
    messages.push(assistant(11));
    let fresh = estimate_context_tokens(None, &messages, &[], "test:model");
    assert_eq!(fresh.tokens, 670);
    assert_eq!(fresh.last_usage_index, Some(3));
}

#[test]
fn changed_model_and_failed_or_unreported_usage_fall_back_to_content() {
    for reason in [StopReason::Error, StopReason::Aborted] {
        let mut response = assistant(2);
        if let Message::Assistant(message) = &mut response {
            message.stop_reason = reason;
        }
        assert_eq!(
            estimate_context_tokens(None, &[user("task", 1), response], &[], "test:model").tokens,
            2
        );
    }
    let messages = vec![user("task", 1), assistant(2)];
    assert_eq!(
        estimate_context_tokens(None, &messages, &[], "test:other").tokens,
        2
    );
    let mut response = assistant(2);
    if let Message::Assistant(message) = &mut response {
        message.usage.reported = false;
    }
    assert_eq!(
        estimate_context_tokens(None, &[response], &[], "test:model").tokens,
        1
    );
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
    }];
    let schema_tokens = estimate_text_tokens(&serde_json::to_string(&tools).unwrap());
    let estimate = estimate_context_tokens(Some("abcdefgh"), &[image], &tools, "test:model");
    assert_eq!(estimate.tokens, 1_202 + schema_tokens);
    assert_eq!(estimate.last_usage_index, None);
    assert_eq!(estimate_text_tokens("🙂🙂🙂"), 2);
    assert_eq!(estimate_message_tokens(&user("abcde", 0)), 2);
}

#[test]
fn larger_edited_system_or_tool_schema_is_not_hidden_by_old_usage() {
    let estimate =
        estimate_context_tokens(Some(&"s".repeat(4_000)), &[assistant(2)], &[], "test:model");
    assert_eq!(estimate.tokens, 1_001);
    assert_eq!(estimate.usage_tokens, 670);
}
