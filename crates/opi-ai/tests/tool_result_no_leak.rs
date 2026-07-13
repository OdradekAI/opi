//! Phase 11.1 guard: a `ToolResultMessage` carrying `truncated: true` must NOT
//! leak the `truncated` field into any provider's HTTP request body.
//!
//! Providers build bodies by explicit field selection (`serde_json::json!`
//! reading only the fields each API needs); this test locks that invariant so a
//! future refactor that serializes the whole struct is caught. Azure /
//! OpenRouter / Mistral inherit `OpenAiChatProvider`, and Vertex inherits
//! `GeminiProvider`, so the four builders below cover every chat-style path.
//! Bedrock's converse body is exercised by `bedrock_fixtures.rs`.

use opi_ai::anthropic::AnthropicProvider;
use opi_ai::gemini::GeminiProvider;
use opi_ai::message::{InputContent, Message, OutputContent, ToolResultMessage, UserMessage};
use opi_ai::openai_chat::OpenAiChatProvider;
use opi_ai::openai_responses::OpenAiResponsesProvider;
use opi_ai::provider::{CacheRetention, Request, ThinkingConfig};
use tokio_util::sync::CancellationToken;

fn request_with_truncated_tool_result() -> Request {
    Request {
        model: "m".into(),
        system: None,
        messages: vec![
            Message::User(UserMessage {
                content: vec![InputContent::Text { text: "hi".into() }],
                timestamp_ms: 0,
            }),
            Message::ToolResult(ToolResultMessage {
                tool_call_id: "tc-1".into(),
                tool_name: "tool".into(),
                content: vec![OutputContent::Text {
                    text: "result".into(),
                }],
                details: None,
                is_error: false,
                truncated: true,
                timestamp_ms: 0,
            }),
        ],
        tools: vec![],
        max_tokens: Some(128),
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    }
}

fn assert_no_truncated(body: &serde_json::Value, label: &str) {
    let text = serde_json::to_string(body).expect("body serializes");
    assert!(
        !text.contains("truncated"),
        "{label} request body must not leak `truncated` field: {text}"
    );
}

#[test]
fn anthropic_body_does_not_leak_truncated() {
    let provider = AnthropicProvider::new("key".into(), None);
    let body = provider.build_request_body(&request_with_truncated_tool_result());
    assert_no_truncated(&body, "anthropic");
}

#[test]
fn openai_chat_body_does_not_leak_truncated() {
    let provider = OpenAiChatProvider::new("key".into(), None);
    let body = provider.build_request_body(&request_with_truncated_tool_result());
    assert_no_truncated(&body, "openai_chat");
}

#[test]
fn openai_responses_body_does_not_leak_truncated() {
    let provider = OpenAiResponsesProvider::new("key".into(), None);
    let body = provider.build_request_body(&request_with_truncated_tool_result());
    assert_no_truncated(&body, "openai_responses");
}

#[test]
fn gemini_body_does_not_leak_truncated() {
    let provider = GeminiProvider::new("key".into(), None);
    let body = provider.build_request_body(&request_with_truncated_tool_result());
    assert_no_truncated(&body, "gemini");
}
