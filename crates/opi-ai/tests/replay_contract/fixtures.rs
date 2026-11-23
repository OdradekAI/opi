//! Canonical histories and adapter request builders shared within the replay target.

use std::sync::Arc;

use opi_ai::message::{
    AssistantContent, AssistantMessage, InputContent, Message, OutputContent, ToolCall,
    ToolNotExecutedReason, ToolResultMessage, UserMessage,
};
use opi_ai::provider::{CacheRetention, Request, ThinkingConfig};
use opi_ai::providers::anthropic::AnthropicProvider;
use opi_ai::stream::StopReason;
use tokio_util::sync::CancellationToken;

pub(super) fn user(text: &str) -> Message {
    Message::User(UserMessage {
        content: vec![InputContent::Text { text: text.into() }],
        timestamp_ms: 0,
    })
}

pub(super) fn assistant(content: Vec<AssistantContent>, stop_reason: StopReason) -> Message {
    Message::Assistant(AssistantMessage {
        projection: None,
        content,
        api: opi_ai::ApiKind::Anthropic,
        provider: "anthropic".into(),
        model: "claude-sonnet-4-5-20250514".into(),
        response_model: None,
        wire: Some(opi_ai::model_info::WireApi::AnthropicMessages),
        response_id: None,
        usage: Default::default(),
        stop_reason,
        error_message: None,
        timestamp_ms: 0,
        diagnostics: Vec::new(),
        duration_ms: None,
        raw_stop_reason: None,
        provider_thinking_level: None,

        deferred: None,
    })
}

pub(super) fn call(id: &str, name: &str) -> AssistantContent {
    AssistantContent::ToolCall {
        tool_call: ToolCall {
            id: id.into(),
            name: name.into(),
            arguments: "{\"path\":\"a.rs\"}".into(),
            arguments_error: None,
            signature: None,
        },
    }
}

pub(super) fn result(id: &str, name: &str, text: &str) -> Message {
    Message::ToolResult(ToolResultMessage {
        tool_call_id: id.into(),
        tool_name: name.into(),
        content: vec![OutputContent::Text { text: text.into() }],
        details: None,
        is_error: false,
        not_executed: None,
        truncated: false,
        timestamp_ms: 0,
    })
}

pub(super) fn not_executed_result(id: &str, name: &str, reason: ToolNotExecutedReason) -> Message {
    let mut message = result(id, name, "private execution-boundary diagnostic");
    if let Message::ToolResult(result) = &mut message {
        result.is_error = true;
        result.not_executed = Some(reason);
    }
    message
}

pub(super) fn request(model: &str, messages: Vec<Message>) -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: model.into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages,
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: Some(64),
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

pub(super) fn anthropic() -> AnthropicProvider {
    AnthropicProvider::with_client(
        Some("http://127.0.0.1:1".into()),
        Arc::new(opi_ai::http::HttpClient::new()),
    )
}
