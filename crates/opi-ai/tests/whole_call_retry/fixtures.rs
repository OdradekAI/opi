//! Canonical messages, retry policy, callback sink and request shared by this target.

use opi_ai::ApiKind;
use opi_ai::message::{AssistantContent, AssistantMessage, Message, UserMessage};
use opi_ai::provider::{CacheRetention, Request};
use opi_ai::retry::WholeCallRetryPolicy;
use opi_ai::stream::{StopReason, Usage};
use opi_ai::transcript;

pub(super) fn message(stop: StopReason, error: Option<&str>) -> AssistantMessage {
    AssistantMessage {
        projection: None,
        content: vec![AssistantContent::Text {
            text: "partial".into(),
            signature: None,
        }],
        api: ApiKind::Anthropic,
        provider: "anthropic".into(),
        model: "claude-sonnet-4-5-20250514".into(),
        response_model: None,
        wire: None,
        response_id: None,
        usage: Usage {
            input_tokens: 3,
            output_tokens: 5,
            ..Usage::default()
        },
        stop_reason: stop,
        error_message: error.map(str::to_string),
        timestamp_ms: 1_000,
        diagnostics: Vec::new(),
        duration_ms: None,
        raw_stop_reason: None,
        provider_thinking_level: None,
        deferred: None,
    }
}

pub(super) fn transient_policy(max_retries: u32, base_delay_ms: u64) -> WholeCallRetryPolicy {
    WholeCallRetryPolicy {
        enabled: true,
        max_retries,
        base_delay_ms,
        max_delay_ms: None,
    }
}

/// Shared sink for finished-callback assertions.
pub(super) type Finals = std::sync::Arc<std::sync::Mutex<Vec<(bool, u32, Option<String>)>>>;

pub(super) fn logical_request() -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "anthropic:claude-sonnet-4-5-20250514".into(),
        context: transcript::normalize_context(transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![opi_ai::message::InputContent::Text {
                    text: "Hello".into(),
                }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: Some(64),
        temperature: None,
        sampling_params: Default::default(),
        thinking: opi_ai::provider::ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: tokio_util::sync::CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}
