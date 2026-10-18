//! OutputContent::Image tests for task 3.5.
//!
//! Validates that OutputContent::Image variant exists with correct serde
//! representation, matching the InputContent::Image pattern from task 3.4.

use opi_ai::message::{ImageSource, MediaType, OutputContent};

// --- Construction ---

#[test]
fn output_content_image_url_construction() {
    let content = OutputContent::Image {
        source: ImageSource::Url {
            url: "https://example.com/screenshot.png".into(),
        },
        media_type: MediaType::Png,
    };
    match content {
        OutputContent::Image { source, media_type } => {
            assert_eq!(
                source,
                ImageSource::Url {
                    url: "https://example.com/screenshot.png".into()
                }
            );
            assert_eq!(media_type, MediaType::Png);
        }
        _ => panic!("expected Image variant"),
    }
}

#[test]
fn output_content_image_base64_construction() {
    let content = OutputContent::Image {
        source: ImageSource::Base64 {
            data: "iVBORw0KGgo=".into(),
        },
        media_type: MediaType::Png,
    };
    match content {
        OutputContent::Image { source, .. } => {
            assert_eq!(
                source,
                ImageSource::Base64 {
                    data: "iVBORw0KGgo=".into()
                }
            );
        }
        _ => panic!("expected Image variant"),
    }
}

#[test]
fn output_content_image_bytes_construction() {
    let data = vec![0x89, 0x50, 0x4E, 0x47];
    let content = OutputContent::Image {
        source: ImageSource::Bytes { data: data.clone() },
        media_type: MediaType::Png,
    };
    match content {
        OutputContent::Image { source, media_type } => {
            assert_eq!(source, ImageSource::Bytes { data });
            assert_eq!(media_type, MediaType::Png);
        }
        _ => panic!("expected Image variant"),
    }
}

// --- Serde round-trip ---

#[test]
fn output_content_image_url_serde_roundtrip() {
    let content = OutputContent::Image {
        source: ImageSource::Url {
            url: "https://example.com/photo.png".into(),
        },
        media_type: MediaType::Png,
    };
    let json = serde_json::to_string(&content).unwrap();
    let back: OutputContent = serde_json::from_str(&json).unwrap();
    assert_eq!(content, back);
}

#[test]
fn output_content_image_base64_serde_roundtrip() {
    let content = OutputContent::Image {
        source: ImageSource::Base64 {
            data: "iVBORw0KGgo=".into(),
        },
        media_type: MediaType::Jpeg,
    };
    let json = serde_json::to_string(&content).unwrap();
    let back: OutputContent = serde_json::from_str(&json).unwrap();
    assert_eq!(content, back);
}

#[test]
fn output_content_image_bytes_serde_roundtrip() {
    let content = OutputContent::Image {
        source: ImageSource::Bytes {
            data: (0u8..=255).collect(),
        },
        media_type: MediaType::WebP,
    };
    let json = serde_json::to_string(&content).unwrap();
    let back: OutputContent = serde_json::from_str(&json).unwrap();
    assert_eq!(content, back);
}

// --- JSON shape ---

#[test]
fn output_content_image_json_shape() {
    let content = OutputContent::Image {
        source: ImageSource::Url {
            url: "https://example.com/photo.png".into(),
        },
        media_type: MediaType::Png,
    };
    let val: serde_json::Value = serde_json::to_value(&content).unwrap();
    assert_eq!(val["type"], "image");
    assert_eq!(val["source"]["type"], "url");
    assert_eq!(val["source"]["url"], "https://example.com/photo.png");
    assert_eq!(val["media_type"], "image/png");
}

#[test]
fn output_content_image_base64_json_shape() {
    let content = OutputContent::Image {
        source: ImageSource::Base64 {
            data: "abc123".into(),
        },
        media_type: MediaType::Gif,
    };
    let val: serde_json::Value = serde_json::to_value(&content).unwrap();
    assert_eq!(val["type"], "image");
    assert_eq!(val["source"]["type"], "base64");
    assert_eq!(val["source"]["data"], "abc123");
    assert_eq!(val["media_type"], "image/gif");
}

// --- Binary safety ---

#[test]
fn output_content_image_bytes_preserves_binary() {
    let data: Vec<u8> = (0u8..=255).collect();
    let content = OutputContent::Image {
        source: ImageSource::Bytes { data },
        media_type: MediaType::Png,
    };
    let json = serde_json::to_string(&content).unwrap();
    let back: OutputContent = serde_json::from_str(&json).unwrap();
    if let OutputContent::Image {
        source: ImageSource::Bytes { data },
        ..
    } = back
    {
        assert_eq!(data.len(), 256);
        assert_eq!(data[0], 0);
        assert_eq!(data[255], 255);
    } else {
        panic!("expected Image with Bytes source");
    }
}

// --- Media type coverage ---

#[test]
fn output_content_image_all_media_types() {
    for media_type in [
        MediaType::Png,
        MediaType::Jpeg,
        MediaType::Gif,
        MediaType::WebP,
    ] {
        let content = OutputContent::Image {
            source: ImageSource::Base64 {
                data: "test".into(),
            },
            media_type,
        };
        let json = serde_json::to_string(&content).unwrap();
        let back: OutputContent = serde_json::from_str(&json).unwrap();
        assert_eq!(content, back);
    }
}

// --- ToolResultMessage with image content ---

#[test]
fn tool_result_message_with_image_content_serde() {
    use opi_ai::message::{Message, ToolResultMessage};

    let msg = Message::ToolResult(ToolResultMessage {
        not_executed: None,
        tool_call_id: "call_1".into(),
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
    });
    let json = serde_json::to_string(&msg).unwrap();
    let val: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(val["role"], "tool_result");
    assert_eq!(val["tool_call_id"], "call_1");
    assert_eq!(val["content"][0]["type"], "text");
    assert_eq!(val["content"][1]["type"], "image");
    assert_eq!(val["content"][1]["source"]["type"], "base64");
    assert_eq!(val["content"][1]["media_type"], "image/png");

    let back: Message = serde_json::from_str(&json).unwrap();
    assert_eq!(msg, back);
}
