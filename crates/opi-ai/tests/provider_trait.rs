//! Behavioral tests for task 1.2: replace placeholder provider trait.
//!
//! DoD: "stream(Request) replaces complete"

use futures_util::{StreamExt, stream};
use opi_ai::registry::ModelCapabilities;
use opi_ai::{
    message::{AssistantContent, AssistantMessage, Message, ToolDef},
    provider::{
        CacheRetention, EventStream, ModelInfo, Provider, ProviderError, ProviderKind, Request,
        ThinkingConfig,
    },
    stream::{AssistantStreamEvent, StopReason, Usage},
};
use tokio_util::sync::CancellationToken;

fn sample_assistant_message() -> AssistantMessage {
    AssistantMessage {
        content: vec![AssistantContent::Text {
            text: "hello".into(),
        }],
        api: opi_ai::ApiKind::Anthropic,
        provider: "anthropic".into(),
        model: "claude-sonnet-4-5-20250514".into(),
        response_model: None,
        response_id: None,
        usage: Usage::default(),
        stop_reason: StopReason::Stop,
        error_message: None,
        timestamp_ms: 1000,
    }
}

// --- ProviderError tests ---

#[test]
fn provider_error_has_rate_limited_variant() {
    let err = ProviderError::RateLimited {
        retry_after_ms: Some(5000),
    };
    let msg = err.to_string();
    assert!(msg.contains("rate") || msg.contains("Rate"), "got: {msg}");
}

#[test]
fn provider_error_has_timeout_variant() {
    let err = ProviderError::Timeout;
    let _msg = err.to_string();
}

#[test]
fn provider_error_has_request_failed_variant() {
    let err = ProviderError::RequestFailed("connection reset".into());
    let msg = err.to_string();
    assert!(msg.contains("connection reset"), "got: {msg}");
}

#[test]
fn provider_error_has_stream_error_variant() {
    let err = ProviderError::StreamError("unexpected EOF".into());
    let msg = err.to_string();
    assert!(msg.contains("unexpected EOF"), "got: {msg}");
}

#[test]
fn provider_error_has_auth_failed_variant() {
    let err = ProviderError::AuthFailed("invalid API key".into());
    let msg = err.to_string();
    assert!(msg.contains("invalid API key"), "got: {msg}");
}

// --- ThinkingConfig tests ---

#[test]
fn thinking_config_default_is_disabled() {
    let cfg = ThinkingConfig::default();
    assert!(!cfg.enabled);
    assert!(cfg.budget_tokens.is_none());
}

#[test]
fn thinking_config_enabled_with_budget() {
    let cfg = ThinkingConfig {
        enabled: true,
        budget_tokens: Some(10000),
    };
    assert!(cfg.enabled);
    assert_eq!(cfg.budget_tokens, Some(10000));
}

// --- ModelInfo tests ---

#[test]
fn model_info_fields() {
    let info = ModelInfo {
        id: "claude-sonnet-4-5-20250514".into(),
        display_name: "Claude Sonnet 4.5".into(),
        capabilities: ModelCapabilities::new(200000, 8192)
            .with_images(true)
            .with_streaming(true)
            .with_thinking(true),
    };
    assert_eq!(info.id, "claude-sonnet-4-5-20250514");
    assert_eq!(info.capabilities.context_window, 200000);
    assert!(info.capabilities.supports_thinking);
}

// --- Request construction tests ---

#[test]
fn request_builds_with_all_fields() {
    let cancel = CancellationToken::new();
    let req = Request {
        model: "claude-sonnet-4-5-20250514".into(),
        system: Some("You are helpful.".into()),
        messages: vec![Message::User(opi_ai::message::UserMessage {
            content: vec![opi_ai::message::InputContent::Text {
                text: "Hello".into(),
            }],
            timestamp_ms: 42,
        })],
        tools: vec![ToolDef {
            name: "read_file".into(),
            description: "Read a file".into(),
            input_schema: serde_json::json!({"type": "object"}),
        }],
        max_tokens: Some(4096),
        temperature: Some(0.7),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec!["STOP".into()],
        metadata: Some(serde_json::json!({"session": "abc"})),
        cancel: cancel.clone(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    };
    assert_eq!(req.model, "claude-sonnet-4-5-20250514");
    assert_eq!(req.messages.len(), 1);
    assert_eq!(req.tools.len(), 1);
    assert!(!cancel.is_cancelled());
}

#[test]
fn request_cancellation_propagates() {
    let cancel = CancellationToken::new();
    let req = Request {
        model: "test".into(),
        system: None,
        messages: vec![],
        tools: vec![],
        max_tokens: None,
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: cancel.clone(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    };
    cancel.cancel();
    assert!(req.cancel.is_cancelled());
}

// --- Provider trait contract tests ---

struct DummyProvider;

impl Provider for DummyProvider {
    fn stream(&self, _request: Request) -> EventStream {
        let msg = sample_assistant_message();
        let event = AssistantStreamEvent::Done {
            reason: StopReason::Stop,
            message: msg,
        };
        Box::pin(stream::once(async move { Ok(event) }))
    }

    fn id(&self) -> &str {
        "dummy"
    }

    fn models(&self) -> &[ModelInfo] {
        &[]
    }
}

#[tokio::test]
async fn provider_trait_yields_done_event() {
    let provider = DummyProvider;
    let cancel = CancellationToken::new();
    let request = Request {
        model: "test".into(),
        system: None,
        messages: vec![],
        tools: vec![],
        max_tokens: None,
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel,
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    };
    let mut stream = provider.stream(request);
    let event = stream.next().await.unwrap().unwrap();
    assert!(event.is_terminal());
}

#[tokio::test]
async fn provider_trait_id_and_models() {
    let provider = DummyProvider;
    assert_eq!(provider.id(), "dummy");
    assert!(provider.models().is_empty());
}

// --- ProviderKind tests ---

#[test]
fn provider_kind_variants() {
    let kinds = [
        ProviderKind::OpenAI,
        ProviderKind::Anthropic,
        ProviderKind::Google,
        ProviderKind::Mistral,
        ProviderKind::Bedrock,
        ProviderKind::Azure,
    ];
    assert_eq!(kinds.len(), 6);
}

// --- EventStream type alias compiles ---

#[test]
fn event_stream_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<EventStream>();
}

// --- No legacy complete method ---

#[test]
fn provider_trait_has_no_complete_method() {
    // This test exists to ensure the old `complete` method is gone.
    // If Provider still had `async fn complete`, DummyProvider above would
    // need to implement it, and compilation would fail.
    // Since DummyProvider only implements id/models/stream, this compiles.
    let _provider = DummyProvider;
}

// --- refresh_models default (task 14.6) ---

#[tokio::test]
async fn provider_refresh_models_default_returns_none() {
    let provider = DummyProvider;
    let result = provider.refresh_models().await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}
