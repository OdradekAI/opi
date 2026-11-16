//! Bedrock pending-terminal and repeated-metadata contract.
//!
//! Pi consumes every Converse stream item before publishing `done`. Keep the
//! Rust pending `Done` unpublished until EOF so a later metadata frame remains
//! authoritative and a tail observer failure cannot arrive after a terminal.

use std::sync::Arc;

use futures_util::StreamExt;
use opi_ai::bedrock::BedrockProvider;
use opi_ai::bedrock::event_stream::build_test_frame;
use opi_ai::http::HttpClient;
use opi_ai::stream::AssistantStreamEvent;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn repeated_metadata_updates_pending_done_before_eof() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("data/bedrock-repeated-metadata.json"))
            .expect("valid Bedrock fixture");
    let mut bytes = Vec::new();
    for event in fixture["events"].as_array().expect("event array") {
        let event_type = event["type"].as_str().expect("event type");
        let payload = serde_json::to_vec(&event["payload"]).expect("event payload");
        bytes.extend_from_slice(&build_test_frame(event_type, "application/json", &payload));
    }

    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));
    let mut stream = provider.stream_from_fixture(&bytes, CancellationToken::new());
    let mut events = Vec::new();
    while let Some(item) = stream.next().await {
        events.push(item.expect("repeated metadata fixture maps"));
    }

    let done_messages: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            AssistantStreamEvent::Done { message, .. } => Some(message),
            _ => None,
        })
        .collect();
    let expected = &fixture["expected"];
    assert_eq!(
        done_messages.len(),
        expected["done_count"].as_u64().expect("numeric done count") as usize
    );
    let done = done_messages[0];
    assert_eq!(
        done.stop_reason.as_str(),
        expected["stop"].as_str().expect("string stop reason")
    );
    assert_eq!(
        done.raw_stop_reason.as_deref(),
        Some(
            expected["raw_stop"]
                .as_str()
                .expect("string raw stop reason")
        )
    );
    assert_eq!(
        done.usage.input_tokens,
        expected["input_tokens"]
            .as_u64()
            .expect("numeric input tokens") as u32
    );
    assert_eq!(
        done.usage.output_tokens,
        expected["output_tokens"]
            .as_u64()
            .expect("numeric output tokens") as u32
    );
    assert_eq!(
        done.usage.cache_read_tokens,
        expected["cache_read_tokens"]
            .as_u64()
            .expect("numeric cache-read tokens") as u32
    );
    assert_eq!(
        done.content.iter().find_map(|content| match content {
            opi_ai::message::AssistantContent::Text { text, .. } => Some(text.as_str()),
            _ => None,
        }),
        Some("latest metadata wins")
    );
    let event_types: Vec<_> = events
        .iter()
        .map(|event| {
            serde_json::to_value(event)
                .expect("serializable stream event")
                .get("type")
                .and_then(serde_json::Value::as_str)
                .expect("stream event type")
                .to_owned()
        })
        .collect();

    println!(
        "{}",
        serde_json::json!({
            "case": fixture["id"],
            "stop": done.stop_reason.as_str(),
            "raw_stop": done.raw_stop_reason,
            "usage": {
                "input": done.usage.input_tokens,
                "output": done.usage.output_tokens,
                "cache_read": done.usage.cache_read_tokens,
            },
            "done_count": done_messages.len(),
            "events": event_types,
        })
    );
}
