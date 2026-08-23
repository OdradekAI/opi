//! Session round-trip tests for image content (task 3.4).
//!
//! Validates that UserMessages containing InputContent::Image survive
//! JSONL session write → read cycles with stable media type and source data.

use opi_agent::session::{MessageEntry, SessionEntry, SessionHeader, SessionReader, SessionWriter};
use opi_ai::message::{ImageSource, InputContent, MediaType, Message, UserMessage};
use tempfile::TempDir;

fn make_header() -> SessionHeader {
    SessionHeader::new_for_test(
        "test-session".into(),
        "2026-05-26T12:00:00Z".into(),
        "/test".into(),
        None,
    )
}

fn image_url_msg() -> Message {
    Message::User(UserMessage {
        content: vec![InputContent::Image {
            source: ImageSource::Url {
                url: "https://example.com/photo.png".into(),
            },
            media_type: MediaType::Png,
        }],
        timestamp_ms: 1000,
    })
}

fn image_base64_msg() -> Message {
    Message::User(UserMessage {
        content: vec![InputContent::Image {
            source: ImageSource::Base64 {
                data: "iVBORw0KGgo=".into(),
            },
            media_type: MediaType::Png,
        }],
        timestamp_ms: 1000,
    })
}

fn image_bytes_msg() -> Message {
    Message::User(UserMessage {
        content: vec![InputContent::Image {
            source: ImageSource::Bytes {
                data: (0u8..=255).collect(),
            },
            media_type: MediaType::Png,
        }],
        timestamp_ms: 1000,
    })
}

fn mixed_text_image_msg() -> Message {
    Message::User(UserMessage {
        content: vec![
            InputContent::Text {
                text: "Describe this image".into(),
            },
            InputContent::Image {
                source: ImageSource::Url {
                    url: "https://example.com/cat.jpg".into(),
                },
                media_type: MediaType::Jpeg,
            },
        ],
        timestamp_ms: 1000,
    })
}

fn write_and_read(messages: Vec<Message>) -> Vec<Message> {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("session.jsonl");
    let mut writer = SessionWriter::create(&path, make_header()).unwrap();
    for (i, msg) in messages.iter().enumerate() {
        writer
            .append(&SessionEntry::Message(MessageEntry {
                id: format!("e{i}"),
                parent_id: if i == 0 {
                    None
                } else {
                    Some(format!("e{}", i - 1))
                },
                timestamp: format!("2026-05-26T12:00:{:02}Z", i),
                message: msg.clone(),
            }))
            .unwrap();
    }
    drop(writer);
    let (_, entries) = SessionReader::read_all(&path).unwrap();
    entries
        .into_iter()
        .filter_map(|e| match e {
            SessionEntry::Message(me) => Some(me.message),
            _ => None,
        })
        .collect()
}

#[test]
fn session_roundtrip_image_url() {
    let messages = write_and_read(vec![image_url_msg()]);
    assert_eq!(messages.len(), 1);
    let msg = &messages[0];
    let Message::User(u) = msg else {
        panic!("expected User message")
    };
    assert_eq!(u.content.len(), 1);
    let InputContent::Image { source, media_type } = &u.content[0] else {
        panic!("expected Image")
    };
    assert_eq!(*media_type, MediaType::Png);
    assert!(matches!(source, ImageSource::Url { url } if url == "https://example.com/photo.png"));
}

#[test]
fn session_roundtrip_image_base64() {
    let messages = write_and_read(vec![image_base64_msg()]);
    let Message::User(u) = &messages[0] else {
        panic!("expected User")
    };
    let InputContent::Image { source, media_type } = &u.content[0] else {
        panic!("expected Image")
    };
    assert_eq!(*media_type, MediaType::Png);
    assert!(matches!(source, ImageSource::Base64 { data } if data == "iVBORw0KGgo="));
}

#[test]
fn session_roundtrip_image_bytes_preserves_binary() {
    let messages = write_and_read(vec![image_bytes_msg()]);
    let Message::User(u) = &messages[0] else {
        panic!("expected User")
    };
    let InputContent::Image { source, .. } = &u.content[0] else {
        panic!("expected Image")
    };
    let ImageSource::Bytes { data } = source else {
        panic!("expected Bytes")
    };
    assert_eq!(data.len(), 256);
    assert_eq!(data[0], 0);
    assert_eq!(data[255], 255);
}

#[test]
fn session_roundtrip_mixed_text_image() {
    let messages = write_and_read(vec![mixed_text_image_msg()]);
    let Message::User(u) = &messages[0] else {
        panic!("expected User")
    };
    assert_eq!(u.content.len(), 2);
    assert!(matches!(&u.content[0], InputContent::Text { text } if text == "Describe this image"));
    assert!(matches!(
        &u.content[1],
        InputContent::Image {
            media_type: MediaType::Jpeg,
            ..
        }
    ));
}

#[test]
fn session_roundtrip_stable_json_shape() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("session.jsonl");
    let mut writer = SessionWriter::create(&path, make_header()).unwrap();
    writer
        .append(&SessionEntry::Message(MessageEntry {
            id: "e0".into(),
            parent_id: None,
            timestamp: "2026-05-26T12:00:00Z".into(),
            message: image_url_msg(),
        }))
        .unwrap();
    drop(writer);

    let raw = std::fs::read_to_string(&path).unwrap();
    let line = raw.lines().nth(1).unwrap();
    let val: serde_json::Value = serde_json::from_str(line).unwrap();
    let content = &val["entry"]["message"]["content"][0];
    assert_eq!(content["type"], "image");
    assert_eq!(content["source"]["type"], "url");
    assert_eq!(content["source"]["url"], "https://example.com/photo.png");
    assert_eq!(content["media_type"], "image/png");
}
