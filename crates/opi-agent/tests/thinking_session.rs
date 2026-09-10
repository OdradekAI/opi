//! Signed and opaque thinking survive public events and durable tool continuation.

use opi_agent::AgentMessage;
use opi_agent::event::AgentEvent;
use opi_agent::session::{
    LeafEntry, MessageEntry, SessionEntry, SessionHeader, SessionReader, SessionWriter,
};
use opi_agent::session_context::reconstruct_context;
use opi_ai::message::{AssistantMessage, Message, OutputContent, ToolResultMessage};
use opi_ai::stream::{AssistantStreamEvent, StopReason};
use serde_json::json;

fn thinking_content() -> serde_json::Value {
    json!([
        {
            "type": "thinking",
            "thinking": "Inspect the local fixture.",
            "signature": "signed-thinking-A+/=="
        },
        { "type": "redacted_thinking", "data": "opaque-thinking+/==" },
        { "type": "thinking", "thinking": "", "signature": "signed-thinking-B+/==" },
        {
            "type": "tool_call",
            "tool_call": { "id": "call-1", "name": "inspect", "arguments": "{}" }
        }
    ])
}

fn assistant() -> AssistantMessage {
    AssistantMessage {
        content: serde_json::from_value(thinking_content()).expect("thinking blocks deserialize"),
        api: opi_ai::ApiKind::Anthropic,
        provider: "anthropic".into(),
        model: "claude-sonnet-4".into(),
        response_model: None,
        response_id: None,
        usage: Default::default(),
        stop_reason: StopReason::ToolUse,
        error_message: None,
        timestamp_ms: 0,
    }
}

#[test]
fn session_roundtrip_preserves_thinking_for_tool_continuation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("thinking.jsonl");
    let messages = vec![
        Message::Assistant(assistant()),
        Message::ToolResult(ToolResultMessage {
            tool_call_id: "call-1".into(),
            tool_name: "inspect".into(),
            content: vec![OutputContent::Text {
                text: "found".into(),
            }],
            details: None,
            is_error: false,
            truncated: false,
            timestamp_ms: 1,
        }),
    ];
    let header = SessionHeader::new_for_test(
        "thinking".into(),
        "2026-09-10T00:00:00Z".into(),
        "/test".into(),
        None,
    );
    let mut writer = SessionWriter::create(&path, header).unwrap();
    for (index, message) in messages.iter().enumerate() {
        writer
            .append(&SessionEntry::Message(MessageEntry {
                id: format!("m{index}"),
                parent_id: index.checked_sub(1).map(|parent| format!("m{parent}")),
                timestamp: "2026-09-10T00:00:00Z".into(),
                message: message.clone(),
            }))
            .unwrap();
    }
    writer
        .append(&SessionEntry::Leaf(LeafEntry {
            id: "leaf".into(),
            parent_id: Some("m1".into()),
            timestamp: "2026-09-10T00:00:00Z".into(),
            entry_id: "m1".into(),
        }))
        .unwrap();
    drop(writer);

    let raw = std::fs::read_to_string(&path).unwrap();
    let stored: serde_json::Value = serde_json::from_str(raw.lines().nth(1).unwrap()).unwrap();
    assert_eq!(stored["entry"]["message"]["content"], thinking_content());

    let (_, entries, recovery) = SessionReader::read_with_recovery(&path).unwrap();
    assert!(recovery.is_clean());
    let context = reconstruct_context(&entries, &recovery);
    assert!(context.diagnostics.is_empty());
    assert_eq!(context.active_tip_entry_id.as_deref(), Some("m1"));
    let resumed: Vec<_> = context
        .messages
        .into_iter()
        .map(|message| match message {
            AgentMessage::Llm(message) => message,
            other => panic!("unexpected continuation message: {other:?}"),
        })
        .collect();
    assert_eq!(resumed, messages);
    assert_eq!(
        serde_json::to_value(&resumed[0]).unwrap()["content"],
        thinking_content()
    );
}

#[test]
fn public_events_preserve_signed_and_opaque_thinking_content() {
    let assistant = assistant();
    let message = AgentMessage::Llm(Message::Assistant(assistant.clone()));
    let events = [
        (
            AgentEvent::MessageStart {
                message: message.clone(),
            },
            "/message/content",
        ),
        (
            AgentEvent::MessageEnd {
                message: message.clone(),
            },
            "/message/content",
        ),
        (
            AgentEvent::TurnEnd {
                message: message.clone(),
                tool_results: vec![],
            },
            "/message/content",
        ),
        (
            AgentEvent::AgentEnd {
                messages: vec![message.clone()],
            },
            "/messages/0/content",
        ),
        (
            AgentEvent::MessageUpdate {
                message,
                assistant_event: Box::new(AssistantStreamEvent::Done {
                    reason: StopReason::ToolUse,
                    message: assistant,
                }),
            },
            "/assistant_event/message/content",
        ),
    ];
    for (event, pointer) in events {
        let public = event.redacted_for_public();
        let encoded = serde_json::to_value(&public).unwrap();
        assert_eq!(encoded.pointer(pointer), Some(&thinking_content()));
        if encoded["type"] == "MessageUpdate" {
            assert_eq!(encoded["message"]["content"], thinking_content());
        }
        let decoded: AgentEvent = serde_json::from_value(encoded.clone()).unwrap();
        assert_eq!(serde_json::to_value(decoded).unwrap(), encoded);
    }
}
