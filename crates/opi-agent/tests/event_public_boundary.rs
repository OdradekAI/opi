//! Public event-boundary secret scrubbing for conversation echo surfaces.
//!
//! The conversation stream (user prompts, tool results, tool execution
//! results) is intended product output echoed to the same client that
//! produced it, so `AgentEvent::redacted_for_public` deliberately keeps
//! ordinary content — including paths — intact. These tests pin the one hard
//! requirement layered on top of that decision: recognized credential
//! patterns must not cross into public JSON output byte-for-byte, and the
//! pre-existing argument/diagnostic redaction must keep behaving.

use opi_agent::AgentMessage;
use opi_agent::event::AgentEvent;
use opi_ai::message::{
    ImageSource, InputContent, MediaType, Message, OutputContent, ToolResultMessage, UserMessage,
};

const API_KEY_CANARY: &str = "sk-ant-canary-0123456789abcdef0123";
const JWT_CANARY: &str =
    "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJjYW5hcnkifQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

fn user_text_event(text: &str) -> AgentEvent {
    AgentEvent::AgentEnd {
        messages: vec![AgentMessage::Llm(Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: text.to_owned(),
            }],
            timestamp_ms: 0,
        }))],
    }
}

fn tool_result_text_event(text: &str) -> AgentEvent {
    AgentEvent::TurnEnd {
        message: AgentMessage::Llm(Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "summarize the credential scan".to_owned(),
            }],
            timestamp_ms: 0,
        })),
        tool_results: vec![ToolResultMessage {
            tool_call_id: "call-1".to_owned(),
            tool_name: "read".to_owned(),
            content: vec![OutputContent::Text {
                text: text.to_owned(),
            }],
            details: None,
            is_error: false,
            truncated: false,
            timestamp_ms: 0,
        }],
    }
}

fn serialized_for_public(event: &AgentEvent) -> String {
    serde_json::to_string(&event.redacted_for_public()).expect("agent event serializes")
}

#[test]
fn user_prompt_canary_is_scrubbed_at_the_public_boundary() {
    let rendered = serialized_for_public(&user_text_event(&format!(
        "my key leaked here {API_KEY_CANARY}"
    )));
    assert!(
        !rendered.contains(API_KEY_CANARY),
        "api-key canary crossed the public boundary: {rendered}"
    );
    assert!(rendered.contains("[REDACTED]"));
}

#[test]
fn tool_result_content_canary_is_scrubbed_at_the_public_boundary() {
    let rendered = serialized_for_public(&tool_result_text_event(&format!(
        "config file contains {JWT_CANARY}"
    )));
    assert!(
        !rendered.contains(JWT_CANARY),
        "bearer/JWT canary crossed the public boundary: {rendered}"
    );
    assert!(rendered.contains("[REDACTED]"));
}

#[test]
fn tool_execution_end_result_canary_is_scrubbed_at_the_public_boundary() {
    let event = AgentEvent::ToolExecutionEnd {
        tool_call_id: "call-2".to_owned(),
        tool_name: "read".to_owned(),
        result: serde_json::json!({
            "output": API_KEY_CANARY,
            "token": "opaque-value-without-a-recognizable-prefix",
            "count": 3,
        }),
        details: None,
        is_error: false,
        truncated: false,
        diagnostics: vec![],
    };
    let rendered = serialized_for_public(&event);
    assert!(
        !rendered.contains(API_KEY_CANARY),
        "api-key canary crossed the public boundary: {rendered}"
    );
    assert!(
        !rendered.contains("opaque-value-without-a-recognizable-prefix"),
        "sensitive field name must be redacted even without a token prefix: {rendered}"
    );
    assert!(
        rendered.contains("\"count\":3"),
        "ordinary structured values must survive scrubbing: {rendered}"
    );
}

#[test]
fn ordinary_conversation_content_passes_through_unchanged() {
    let prose = "read /home/user/notes.txt and summarized it; nothing sensitive";
    let rendered = serialized_for_public(&user_text_event(prose));
    assert!(
        rendered.contains(prose),
        "ordinary prose must pass through verbatim: {rendered}"
    );
    assert!(
        rendered.contains("/home/user/notes.txt"),
        "paths are intended echo content and must not be summary-redacted: {rendered}"
    );

    let image_event = AgentEvent::AgentEnd {
        messages: vec![AgentMessage::Llm(Message::User(UserMessage {
            content: vec![
                InputContent::Text {
                    text: "look at this".to_owned(),
                },
                InputContent::Image {
                    source: ImageSource::Base64 {
                        data: "aGVsbG8gaW1hZ2U=".to_owned(),
                    },
                    media_type: MediaType::Png,
                },
            ],
            timestamp_ms: 0,
        }))],
    };
    let rendered = serialized_for_public(&image_event);
    assert!(
        rendered.contains("aGVsbG8gaW1hZ2U="),
        "image payloads must pass through untouched: {rendered}"
    );

    let plain_result = serialized_for_public(&tool_result_text_event("3 matches, no credentials"));
    assert!(
        plain_result.contains("3 matches, no credentials"),
        "ordinary tool output must pass through verbatim: {plain_result}"
    );
}

#[test]
fn argument_redaction_surfaces_keep_behavior() {
    let event = AgentEvent::ToolExecutionStart {
        tool_call_id: "call-3".to_owned(),
        tool_name: "bash".to_owned(),
        args: serde_json::json!({
            "password": "hunter2",
            "note": "plain value",
        }),
    };
    let rendered = serialized_for_public(&event);
    assert!(
        !rendered.contains("hunter2"),
        "sensitive argument keys stay redacted: {rendered}"
    );
    assert!(
        rendered.contains("plain value"),
        "non-sensitive argument keys stay readable: {rendered}"
    );
}
