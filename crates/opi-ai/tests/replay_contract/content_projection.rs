//! Project thinking, visible assistant content and tool-result images through real adapters.

use std::sync::Arc;

use opi_ai::message::{
    AssistantContent, AssistantMessage, InputContent, MediaType, Message, OutputContent,
    ToolResultMessage, UserMessage,
};
use opi_ai::providers::bedrock::BedrockProvider;
use opi_ai::providers::openai::OpenAiChatProvider;
use opi_ai::stream::StopReason;
use serde_json::json;

use super::fixtures::{anthropic, assistant, call, request, result, user};

/// Thinking with unknown or different identity stays out of replay.
/// The same identity preserves signed and redacted thinking, including signed empty blocks.
#[test]
fn cross_identity_drops_all_thinking_and_same_identity_preserves_opaque() {
    let thinking = vec![
        AssistantContent::Thinking {
            thinking: "readable reasoning".into(),
            signature: Some("private-signature".into()),
        },
        AssistantContent::RedactedThinking {
            data: "private-opaque-data".into(),
        },
        AssistantContent::Text {
            text: "answer".into(),
            signature: None,
        },
    ];
    let mut foreign = assistant(thinking.clone(), StopReason::Stop);
    if let Message::Assistant(a) = &mut foreign {
        a.provider = "other-provider".into();
    }
    let anthropic_body = anthropic()
        .build_request_body(&request(
            "anthropic:claude-sonnet-4-5-20250514",
            vec![user("q"), foreign.clone()],
        ))
        .unwrap();
    let bedrock_body = BedrockProvider::new(None, Arc::new(opi_ai::http::HttpClient::new()))
        .build_converse_body(&request(
            "bedrock:us-anthropic.claude-sonnet-4-5-20250514-v1:0",
            vec![user("q"), foreign],
        ))
        .unwrap();
    for body in [&anthropic_body, &bedrock_body] {
        let serialized = body.to_string();
        assert!(!serialized.contains("readable reasoning"));
        assert!(!serialized.contains("private-signature"));
        assert!(!serialized.contains("private-opaque-data"));
        assert!(serialized.contains("answer"));
    }

    // Same identity: signed-empty and redacted blocks round-trip onto the
    // Anthropic wire; blank unsigned thinking is dropped.
    let same = assistant(
        vec![
            AssistantContent::Thinking {
                thinking: String::new(),
                signature: Some("empty-signed".into()),
            },
            AssistantContent::RedactedThinking {
                data: "opaque-kept".into(),
            },
            AssistantContent::Thinking {
                thinking: "   ".into(),
                signature: None,
            },
            AssistantContent::Text {
                text: "answer".into(),
                signature: None,
            },
        ],
        StopReason::Stop,
    );
    let body = anthropic()
        .build_request_body(&request(
            "anthropic:claude-sonnet-4-5-20250514",
            vec![user("q"), same],
        ))
        .unwrap();
    let content = &body["messages"][1]["content"];
    assert_eq!(content[0]["signature"], json!("empty-signed"));
    assert_eq!(
        content[1],
        json!({"type": "redacted_thinking", "data": "opaque-kept"})
    );
    assert_eq!(content[2]["type"], "text");
    assert_eq!(content[2]["text"], "answer");

    // Legacy-unknown provenance (no stored wire) is never same-identity.
    let mut legacy = assistant(thinking, StopReason::Stop);
    if let Message::Assistant(a) = &mut legacy {
        a.wire = None;
    }
    let body = anthropic()
        .build_request_body(&request(
            "anthropic:claude-sonnet-4-5-20250514",
            vec![user("q"), legacy],
        ))
        .unwrap();
    assert!(!body.to_string().contains("private-"));
}

/// Preserve tool-result image ordering, failure status and URL sources on the wire.
#[test]
fn tool_result_images_preserve_association_order_and_failure_flag() {
    let image = || OutputContent::Image {
        source: opi_ai::message::ImageSource::Base64 {
            data: "AA==".into(),
        },
        media_type: MediaType::Png,
    };
    // Success result: image blocks ride the tool_result content array.
    let messages = vec![
        user("q"),
        assistant(vec![call("img_1", "screenshot")], StopReason::ToolUse),
        Message::ToolResult(ToolResultMessage {
            tool_call_id: "img_1".into(),
            tool_name: "screenshot".into(),
            content: vec![
                OutputContent::Text {
                    text: "captured".into(),
                },
                image(),
            ],
            details: Some(json!({"secret": "private-detail"})),
            is_error: false,
            not_executed: None,
            truncated: false,
            timestamp_ms: 0,
        }),
    ];
    let body = anthropic()
        .build_request_body(&request("anthropic:claude-sonnet-4-5-20250514", messages))
        .unwrap();
    let block = &body["messages"][2]["content"][0];
    assert_eq!(block["type"], "tool_result");
    assert_eq!(block["tool_use_id"], "img_1");
    assert_eq!(block["content"][0]["type"], "text");
    assert_eq!(block["content"][0]["text"], "captured");
    assert_eq!(block["content"][1]["type"], "image");
    assert_eq!(block["content"][1]["source"]["media_type"], "image/png");
    assert!(!body.to_string().contains("private-detail"));

    // Error result: images split after the turn's results with a caption.
    let messages = vec![
        user("q"),
        assistant(vec![call("img_2", "screenshot")], StopReason::ToolUse),
        Message::ToolResult(ToolResultMessage {
            tool_call_id: "img_2".into(),
            tool_name: "screenshot".into(),
            content: vec![image()],
            details: None,
            is_error: true,
            not_executed: None,
            truncated: false,
            timestamp_ms: 0,
        }),
    ];
    let body = anthropic()
        .build_request_body(&request("anthropic:claude-sonnet-4-5-20250514", messages))
        .unwrap();
    let messages = body["messages"].as_array().unwrap();
    // The placeholder replaces the image run. The caption and image follow
    // all tool results in the same user message.
    let error_block = &messages[2]["content"][0];
    assert_eq!(error_block["type"], "tool_result");
    assert_eq!(error_block["is_error"], json!(true));
    assert_eq!(
        error_block["content"],
        json!([{"type": "text", "text": "(image attached after the tool results)"}])
    );
    let follow_up = &messages[2]["content"];
    assert_eq!(follow_up[1]["type"], "text");
    assert!(
        follow_up[1]["text"]
            .as_str()
            .unwrap()
            .contains("Images from the error result of tool call img_2")
    );
    assert_eq!(follow_up[2]["type"], "image");

    // URL sources pass through, never downloaded.
    let messages = vec![user("see"), user_url_image()];
    let body = anthropic()
        .build_request_body(&request("anthropic:claude-sonnet-4-5-20250514", messages))
        .unwrap();
    assert_eq!(
        body["messages"][1]["content"][0]["source"]["url"],
        "https://example.test/i.png"
    );
}

/// Preserve visible text and tool calls together in the same Chat turn.
#[test]
fn chat_keeps_turn_text_alongside_tool_calls() {
    let chat = OpenAiChatProvider::new(None);
    let messages = vec![
        user("q"),
        assistant_with_openai_identity(vec![
            AssistantContent::Text {
                text: "I will read the file.".into(),
                signature: None,
            },
            call("c_1", "read"),
        ]),
        result("c_1", "read", "contents"),
    ];
    let body = chat
        .build_request_body(&request("openai:gpt-4o", messages))
        .unwrap();
    let assistant = body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["role"] == "assistant")
        .unwrap();
    assert_eq!(assistant["content"], "I will read the file.");
    assert_eq!(assistant["tool_calls"][0]["function"]["name"], "read");
}

fn user_url_image() -> Message {
    Message::User(UserMessage {
        content: vec![InputContent::Image {
            source: opi_ai::message::ImageSource::Url {
                url: "https://example.test/i.png".into(),
            },
            media_type: MediaType::Png,
        }],
        timestamp_ms: 0,
    })
}

fn assistant_with_openai_identity(content: Vec<AssistantContent>) -> Message {
    Message::Assistant(AssistantMessage {
        projection: None,
        content,
        api: opi_ai::ApiKind::OpenAi,
        provider: "openai".into(),
        model: "gpt-4o".into(),
        response_model: None,
        wire: Some(opi_ai::model_info::WireApi::OpenAiCompletions),
        response_id: None,
        usage: Default::default(),
        stop_reason: StopReason::ToolUse,
        error_message: None,
        timestamp_ms: 0,
        diagnostics: Vec::new(),
        duration_ms: None,
        raw_stop_reason: None,
        provider_thinking_level: None,

        deferred: None,
    })
}
