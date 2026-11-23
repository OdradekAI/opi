//! Request and successful text response shared by provider lifecycle topics.

use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{CacheRetention, Request};
use tokio_util::sync::CancellationToken;

pub(super) fn make_request(cancel: CancellationToken) -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "anthropic:claude-sonnet-4".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "hello".into(),
                }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: Some(1024),
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel,
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

pub(super) fn text_sse_fixture() -> &'static str {
    r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_lc","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4","stop_reason":null,"usage":{"input_tokens":10,"output_tokens":0}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":" world"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":5}}

event: message_stop
data: {"type":"message_stop"}

"#
}
