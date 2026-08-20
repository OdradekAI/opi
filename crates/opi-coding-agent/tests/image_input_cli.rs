//! CLI image attachment tests for task 3.4.
//!
//! Validates the --image CLI flag reads files from isolated temp directories
//! and constructs correct InputContent::Image messages. Never reads user
//! runtime data.

mod common;

use clap::Parser;
use opi_ai::message::{ImageSource, InputContent, MediaType};
use opi_coding_agent::cli::Cli;
use std::path::PathBuf;
use std::sync::Arc;

// --- CLI flag parsing ---

#[test]
fn cli_accepts_image_flag() {
    let args = Cli::try_parse_from(["opi", "--image", "photo.png", "Describe this"]);
    assert!(args.is_ok(), "failed to parse --image: {:?}", args.err());
    let cli = args.unwrap();
    assert_eq!(cli.image.len(), 1);
    assert_eq!(cli.image[0], PathBuf::from("photo.png"));
}

#[test]
fn cli_accepts_multiple_image_flags() {
    let args = Cli::try_parse_from([
        "opi",
        "--image",
        "a.png",
        "--image",
        "b.jpg",
        "Compare these",
    ]);
    assert!(
        args.is_ok(),
        "failed to parse multiple --image: {:?}",
        args.err()
    );
    let cli = args.unwrap();
    assert_eq!(cli.image.len(), 2);
}

#[test]
fn cli_image_flag_optional() {
    let args = Cli::try_parse_from(["opi", "Hello"]);
    assert!(args.is_ok());
    let cli = args.unwrap();
    assert!(cli.image.is_empty());
}

// --- Image file reading and media type detection ---

#[test]
fn detect_media_type_from_png_extension() {
    assert_eq!(
        opi_coding_agent::image::detect_media_type(PathBuf::from("photo.png")),
        Some(MediaType::Png)
    );
}

#[test]
fn detect_media_type_from_jpg_extension() {
    assert_eq!(
        opi_coding_agent::image::detect_media_type(PathBuf::from("photo.jpg")),
        Some(MediaType::Jpeg)
    );
}

#[test]
fn detect_media_type_from_jpeg_extension() {
    assert_eq!(
        opi_coding_agent::image::detect_media_type(PathBuf::from("photo.jpeg")),
        Some(MediaType::Jpeg)
    );
}

#[test]
fn detect_media_type_from_gif_extension() {
    assert_eq!(
        opi_coding_agent::image::detect_media_type(PathBuf::from("photo.gif")),
        Some(MediaType::Gif)
    );
}

#[test]
fn detect_media_type_from_webp_extension() {
    assert_eq!(
        opi_coding_agent::image::detect_media_type(PathBuf::from("photo.webp")),
        Some(MediaType::WebP)
    );
}

#[test]
fn detect_media_type_unknown_returns_none() {
    assert_eq!(
        opi_coding_agent::image::detect_media_type(PathBuf::from("photo.bmp")),
        None
    );
}

#[test]
fn detect_media_type_case_insensitive() {
    assert_eq!(
        opi_coding_agent::image::detect_media_type(PathBuf::from("photo.PNG")),
        Some(MediaType::Png)
    );
}

// --- Load image from temp file ---

#[test]
fn load_image_from_temp_file() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.png");
    let data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A];
    std::fs::write(&file_path, &data).unwrap();

    let content = opi_coding_agent::image::load_image(&file_path).unwrap();
    let InputContent::Image { source, media_type } = content else {
        panic!("expected Image variant");
    };
    assert_eq!(media_type, MediaType::Png);
    let ImageSource::Bytes { data: loaded } = source else {
        panic!("expected Bytes source");
    };
    assert_eq!(loaded, data);
}

#[test]
fn load_image_nonexistent_file_returns_error() {
    let result = opi_coding_agent::image::load_image(&PathBuf::from("/nonexistent/photo.png"));
    assert!(result.is_err());
}

#[test]
fn load_image_unsupported_extension_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("photo.bmp");
    std::fs::write(&file_path, b"BM").unwrap();

    let result = opi_coding_agent::image::load_image(&file_path);
    assert!(result.is_err());
}

#[test]
fn load_image_rejects_file_above_limit() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("oversized.png");
    std::fs::write(&file_path, vec![0u8; 16]).unwrap();

    let result = opi_coding_agent::image::load_image_with_limit(&file_path, 8);

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("max_image_bytes"), "unexpected error: {err}");
}

// --- prompt_with_content integration tests ---

use opi_agent::hooks::AgentHooks;
use opi_agent::loop_types::{AgentError, InferenceConfig};
use opi_agent::message::AgentMessage;
use opi_ai::message::Message;
use opi_ai::test_support::{MockProvider, single_route_collection};

struct TestHooks;

impl AgentHooks for TestHooks {
    fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
        let mut result = Vec::new();
        for msg in messages {
            if let AgentMessage::Llm(m) = msg {
                result.push(m.clone());
            }
        }
        Ok(result)
    }
}

fn make_mock_provider() -> MockProvider {
    MockProvider::new(
        "test-mock",
        vec![opi_ai::test_support::text_response("I see the image.")],
    )
}

#[test]
fn pending_images_injected_into_first_prompt() {
    use opi_coding_agent::config::OpiConfig;
    use opi_coding_agent::harness::CodingHarness;

    let provider = Box::new(MockProvider::new(
        "test-mock",
        vec![opi_ai::test_support::text_response("ok")],
    ));
    let config = OpiConfig::default();
    let mut harness = CodingHarness::new(
        provider,
        "test-mock:mock-model".into(),
        config,
        std::env::current_dir().unwrap(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    // Queue a synthetic image content.
    let fake_image = InputContent::Text {
        text: "[fake image]".into(),
    };
    harness.queue_images(vec![fake_image]);

    let pending = harness.take_pending_images();
    assert_eq!(pending.len(), 1);
    assert!(
        harness.take_pending_images().is_empty(),
        "images should be cleared after take"
    );
}

#[tokio::test]
async fn prompt_with_content_sends_image_to_provider() {
    let provider = make_mock_provider();
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.png");
    let png_header = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A];
    std::fs::write(&file_path, &png_header).unwrap();

    let image_content = opi_coding_agent::image::load_image(&file_path).unwrap();
    let content = vec![
        InputContent::Text {
            text: "Describe this image".into(),
        },
        image_content,
    ];

    let mut agent = opi_agent::Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        common::registrations_from(vec![]),
        Some(common::permissive_authorizer()),
        "test-mock:mock-model".into(),
        None,
        InferenceConfig::default(),
        Default::default(),
        Box::new(TestHooks),
    )
    .expect("agent");

    let messages = agent
        .prompt_with_content(content)
        .await
        .into_execution_result()
        .unwrap();

    // The first message should be a UserMessage with image content
    let first = &messages[0];
    if let AgentMessage::Llm(Message::User(user_msg)) = first {
        assert!(
            user_msg
                .content
                .iter()
                .any(|c| matches!(c, InputContent::Image { .. })),
            "user message should contain an image"
        );
        assert!(
            user_msg
                .content
                .iter()
                .any(|c| matches!(c, InputContent::Text { .. })),
            "user message should contain text"
        );
    } else {
        panic!("expected user message, got: {first:?}");
    }
}

#[tokio::test]
async fn harness_prompt_with_content_sends_image() {
    use opi_coding_agent::config::OpiConfig;
    use opi_coding_agent::harness::CodingHarness;

    let provider = make_mock_provider();
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("photo.jpg");
    let jpg_data = vec![0xFF, 0xD8, 0xFF, 0xE0];
    std::fs::write(&file_path, &jpg_data).unwrap();

    let image_content = opi_coding_agent::image::load_image(&file_path).unwrap();
    let content = vec![
        InputContent::Text {
            text: "What is in this image?".into(),
        },
        image_content,
    ];

    let mut harness = CodingHarness::new(
        Box::new(provider),
        "test-mock:mock-model".into(),
        OpiConfig::default(),
        dir.path().to_path_buf(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    let messages = harness.prompt_with_content(content).await.unwrap();

    let first = &messages[0];
    if let AgentMessage::Llm(Message::User(user_msg)) = first {
        let has_image = user_msg
            .content
            .iter()
            .any(|c| matches!(c, InputContent::Image { .. }));
        assert!(
            has_image,
            "harness should pass image content through to agent"
        );
    } else {
        panic!("expected user message, got: {first:?}");
    }
}
