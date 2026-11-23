//! Inputs and loopback responses shared by the Chat matrix and Models contract tests.

use std::collections::BTreeMap;
use std::sync::Arc;

use opi_ai::auth::{AuthContext, AuthContextError};
use opi_ai::message::{AssistantContent, InputContent, Message, UserMessage};
use opi_ai::provider::Request;
use opi_ai::transcript::{Context as ShorthandContext, normalize_context};
use wiremock::ResponseTemplate;

/// Injected auth context that records every lookup for zero-I/O assertions.
#[derive(Default)]
pub(super) struct OwnedRecordingContext {
    values: BTreeMap<String, String>,
    reads: std::sync::Mutex<Vec<String>>,
}

impl OwnedRecordingContext {
    pub(super) fn new(values: &[(&str, &str)]) -> Arc<Self> {
        Arc::new(Self {
            values: values
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            reads: std::sync::Mutex::new(Vec::new()),
        })
    }

    pub(super) fn read_count(&self) -> usize {
        self.reads.lock().unwrap().len()
    }
}

impl AuthContext for OwnedRecordingContext {
    fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
        self.reads.lock().unwrap().push(name.to_string());
        Ok(self.values.get(name).cloned())
    }

    fn file_exists(&self, _path: &str) -> Result<bool, AuthContextError> {
        Ok(false)
    }
}

pub(super) fn base_request(model: &str) -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: model.to_owned(),
        context: normalize_context(ShorthandContext::default()).unwrap(),
        max_tokens: Some(256),
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: Default::default(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: Default::default(),
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

pub(super) fn user_request(model: &str, text: &str) -> Request {
    let mut request = base_request(model);
    request.context = normalize_context(ShorthandContext {
        system_prompt: None,
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: text.to_owned(),
            }],
            timestamp_ms: 0,
        })],
        tools: vec![],
    })
    .unwrap();
    request
}

pub(super) fn message_text(message: &opi_ai::message::AssistantMessage) -> String {
    message
        .content
        .iter()
        .filter_map(|block| match block {
            AssistantContent::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

pub(super) fn anthropic_text_sse() -> String {
    concat!(
        "event: message_start\n",
        r#"data: {"type":"message_start","message":{"id":"msg_abc","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-5-20250514","stop_reason":null,"usage":{"input_tokens":25,"output_tokens":0}}}"#, "\n\n",
        "event: content_block_start\n",
        r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#, "\n\n",
        "event: content_block_delta\n",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#, "\n\n",
        "event: content_block_delta\n",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":" world"}}"#, "\n\n",
        "event: content_block_stop\n",
        r#"data: {"type":"content_block_stop","index":0}"#, "\n\n",
        "event: message_delta\n",
        r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},"usage":{"output_tokens":11}}"#, "\n\n",
        "event: message_stop\n",
        r#"data: {"type":"message_stop"}"#, "\n\n",
    )
    .into()
}

pub(super) fn openai_completions_sse() -> String {
    concat!(
        r#"data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}"#, "\n\n",
        r#"data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}"#, "\n\n",
        r#"data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":" world"},"finish_reason":null}]}"#, "\n\n",
        r#"data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":25,"completion_tokens":8,"total_tokens":33}}"#, "\n\n",
        "data: [DONE]\n\n",
    )
    .into()
}

pub(super) fn sse_template(body: String) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_string(body)
}
