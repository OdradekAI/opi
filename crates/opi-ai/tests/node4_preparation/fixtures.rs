//! Requests, Chat SSE and captured bodies shared by limits and projection tests.

use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{CacheRetention, Request, ThinkingConfig};
use opi_ai::transcript::{Context, normalize_context};
use tokio_util::sync::CancellationToken;
use wiremock::MockServer;

pub(super) const CHAT_OK: &str = "data: {\"id\":\"chat-test\",\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"ok\"},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1}}\n\ndata: [DONE]\n\n";

pub(super) fn request(model: &str) -> Request {
    Request {
        projection: Default::default(),
        model: model.into(),
        context: normalize_context(Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "hello".into(),
                }],
                timestamp_ms: 1,
            })],
            tools: vec![],
        })
        .unwrap(),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::Disabled,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
        transport: Default::default(),
    }
}

pub(super) async fn bodies(server: &MockServer) -> Vec<serde_json::Value> {
    server
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .map(|request| serde_json::from_slice(&request.body).unwrap())
        .collect()
}
