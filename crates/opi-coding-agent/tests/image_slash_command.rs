//! Test /image slash command queue behavior.

use opi_ai::message::InputContent;
use opi_ai::test_support::{MockProvider, text_response};
use opi_coding_agent::config::OpiConfig;
use opi_coding_agent::harness::CodingHarness;

#[test]
fn image_slash_command_queues_for_next_prompt() {
    let provider = Box::new(MockProvider::new("mock", vec![text_response("ok")]));
    let config = OpiConfig::default();
    let mut harness = CodingHarness::new(
        provider,
        "mock:test".into(),
        config,
        std::env::current_dir().unwrap(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    // Create a minimal valid PNG file.
    let dir = tempfile::tempdir().unwrap();
    let png_path = dir.path().join("test.png");
    // Minimal PNG: 8-byte signature + IHDR + IEND
    let minimal_png_bytes: Vec<u8> = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, // IHDR length
        0x49, 0x48, 0x44, 0x52, // IHDR
        0x00, 0x00, 0x00, 0x01, // width: 1
        0x00, 0x00, 0x00, 0x01, // height: 1
        0x08, 0x02, // bit depth, color type
        0x00, 0x00, 0x00, // compression, filter, interlace
        0x90, 0x77, 0x53, 0xDE, // CRC
        0x00, 0x00, 0x00, 0x00, // IEND length
        0x49, 0x45, 0x4E, 0x44, // IEND
        0xAE, 0x42, 0x60, 0x82, // CRC
    ];
    std::fs::write(&png_path, &minimal_png_bytes).unwrap();

    // Simulate what the /image handler does.
    let img = opi_coding_agent::image::load_image_with_limit(
        &png_path,
        opi_coding_agent::image::DEFAULT_MAX_IMAGE_BYTES,
    )
    .unwrap();
    harness.queue_images(vec![img]);

    let pending = harness.take_pending_images();
    assert_eq!(pending.len(), 1);
    match &pending[0] {
        InputContent::Image { media_type, .. } => {
            assert_eq!(*media_type, opi_ai::message::MediaType::Png);
        }
        other => panic!("expected Image content, got: {other:?}"),
    }
}
