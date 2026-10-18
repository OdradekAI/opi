//! Image tool result session round-trip tests for task 3.5.
//!
//! Validates that image-bearing ToolResult messages round-trip through
//! session JSONL entries, preserving media type and binary source data.

use opi_agent::session::{MessageEntry, SessionEntry};
use opi_ai::message::{ImageSource, MediaType, Message, OutputContent, ToolResultMessage};

fn image_tool_result_msg() -> Message {
    Message::ToolResult(ToolResultMessage {
        not_executed: None,
        tool_call_id: "call_img_1".into(),
        tool_name: "screenshot".into(),
        content: vec![
            OutputContent::Text {
                text: "Screenshot captured".into(),
            },
            OutputContent::Image {
                source: ImageSource::Base64 {
                    data: "iVBORw0KGgo=".into(),
                },
                media_type: MediaType::Png,
            },
        ],
        details: None,
        is_error: false,
        truncated: false,
        timestamp_ms: 1000,
    })
}

fn image_bytes_tool_result_msg() -> Message {
    Message::ToolResult(ToolResultMessage {
        not_executed: None,
        tool_call_id: "call_img_2".into(),
        tool_name: "capture".into(),
        content: vec![OutputContent::Image {
            source: ImageSource::Bytes {
                data: vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A],
            },
            media_type: MediaType::Png,
        }],
        details: None,
        is_error: false,
        truncated: false,
        timestamp_ms: 2000,
    })
}

// --- Session entry serde ---

#[test]
fn message_entry_with_image_tool_result_serde() {
    let entry = SessionEntry::Message(MessageEntry {
        id: "e1".into(),
        parent_id: None,
        timestamp: "2026-01-01T00:00:00Z".into(),
        message: image_tool_result_msg(),
    });
    let json = serde_json::to_string(&entry).unwrap();
    let back: SessionEntry = serde_json::from_str(&json).unwrap();

    if let SessionEntry::Message(me) = back {
        if let Message::ToolResult(tr) = me.message {
            assert_eq!(tr.tool_name, "screenshot");
            assert_eq!(tr.content.len(), 2);
            assert!(
                matches!(&tr.content[0], OutputContent::Text { text } if text == "Screenshot captured")
            );
            assert!(matches!(
                &tr.content[1],
                OutputContent::Image {
                    media_type: MediaType::Png,
                    ..
                }
            ));
        } else {
            panic!("expected ToolResult message");
        }
    } else {
        panic!("expected Message entry");
    }
}

#[test]
fn message_entry_image_bytes_roundtrip() {
    let entry = SessionEntry::Message(MessageEntry {
        id: "e2".into(),
        parent_id: Some("e1".into()),
        timestamp: "2026-01-01T00:00:01Z".into(),
        message: image_bytes_tool_result_msg(),
    });
    let json = serde_json::to_string(&entry).unwrap();
    let back: SessionEntry = serde_json::from_str(&json).unwrap();

    if let SessionEntry::Message(me) = back {
        if let Message::ToolResult(tr) = me.message {
            assert_eq!(tr.content.len(), 1);
            if let OutputContent::Image {
                source: ImageSource::Bytes { data },
                ..
            } = &tr.content[0]
            {
                assert_eq!(data, &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A]);
            } else {
                panic!("expected Image with Bytes source");
            }
        } else {
            panic!("expected ToolResult message");
        }
    } else {
        panic!("expected Message entry");
    }
}

#[test]
fn session_entry_json_shape_image_tool_result() {
    let entry = SessionEntry::Message(MessageEntry {
        id: "e3".into(),
        parent_id: None,
        timestamp: "2026-01-01T00:00:00Z".into(),
        message: image_tool_result_msg(),
    });
    let val: serde_json::Value = serde_json::to_value(&entry).unwrap();

    assert_eq!(val["type"], "message");
    assert_eq!(val["message"]["role"], "tool_result");
    assert_eq!(val["message"]["content"][0]["type"], "text");
    assert_eq!(val["message"]["content"][1]["type"], "image");
    assert_eq!(val["message"]["content"][1]["source"]["type"], "base64");
    assert_eq!(val["message"]["content"][1]["media_type"], "image/png");
}

#[test]
fn tool_result_with_image_url_source() {
    let msg = Message::ToolResult(ToolResultMessage {
        not_executed: None,
        tool_call_id: "call_3".into(),
        tool_name: "web_capture".into(),
        content: vec![OutputContent::Image {
            source: ImageSource::Url {
                url: "https://example.com/capture.png".into(),
            },
            media_type: MediaType::Png,
        }],
        details: None,
        is_error: false,
        truncated: false,
        timestamp_ms: 3000,
    });
    let json = serde_json::to_string(&msg).unwrap();
    let val: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(val["role"], "tool_result");
    assert_eq!(val["content"][0]["type"], "image");
    assert_eq!(val["content"][0]["source"]["type"], "url");
    assert_eq!(
        val["content"][0]["source"]["url"],
        "https://example.com/capture.png"
    );
}

#[test]
fn tool_result_image_stable_json_shape() {
    let msg = Message::ToolResult(ToolResultMessage {
        not_executed: None,
        tool_call_id: "call_stable".into(),
        tool_name: "snapshot".into(),
        content: vec![OutputContent::Image {
            source: ImageSource::Base64 {
                data: "abc123".into(),
            },
            media_type: MediaType::Jpeg,
        }],
        details: Some(serde_json::json!({"format": "jpeg"})),
        is_error: false,
        truncated: false,
        timestamp_ms: 4000,
    });
    let val: serde_json::Value = serde_json::to_value(&msg).unwrap();

    assert_eq!(val["role"], "tool_result");
    assert_eq!(val["tool_call_id"], "call_stable");
    assert_eq!(val["content"][0]["type"], "image");
    assert_eq!(val["content"][0]["media_type"], "image/jpeg");
    assert_eq!(val["details"]["format"], "jpeg");
}
