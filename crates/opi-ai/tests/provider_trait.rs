//! Behavioral tests for task 1.2: replace placeholder provider trait.
//!
//! DoD: "stream(Request) replaces complete"

use futures_util::{StreamExt, stream};
use opi_ai::registry::ModelCapabilities;
use opi_ai::{ThinkingLevel, WireApi};
use opi_ai::{
    message::{AssistantContent, AssistantMessage, Message, ToolDef},
    provider::{
        CacheRetention, EventStream, ModelInfo, Provider, ProviderError, ProviderErrorSummary,
        ProviderKind, Request, ThinkingConfig,
    },
    stream::{AssistantStreamEvent, StopReason, Usage},
};
use tokio_util::sync::CancellationToken;

fn sample_assistant_message() -> AssistantMessage {
    AssistantMessage {
        content: vec![AssistantContent::Text {
            text: "hello".into(),
            signature: None,
        }],
        api: opi_ai::ApiKind::Anthropic,
        provider: "anthropic".into(),
        model: "claude-sonnet-4-5-20250514".into(),
        response_model: None,
        wire: None,
        response_id: None,
        usage: Usage::default(),
        stop_reason: StopReason::Stop,
        error_message: None,
        timestamp_ms: 1000,
        diagnostics: Vec::new(),
        duration_ms: None,
        provider_thinking_level: None,

        deferred: None,
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
    let err = ProviderError::RequestFailed(ProviderErrorSummary::redacted());
    let msg = err.to_string();
    assert_eq!(msg, "request failed: [REDACTED]");
}

#[test]
fn provider_error_has_stream_error_variant() {
    let err = ProviderError::StreamError(ProviderErrorSummary::redacted());
    let msg = err.to_string();
    assert_eq!(msg, "stream error: [REDACTED]");
}

#[test]
fn provider_error_has_auth_failed_variant() {
    let err = ProviderError::AuthFailed(ProviderErrorSummary::authentication_rejected());
    let msg = err.to_string();
    assert_eq!(msg, "authentication failed: provider rejected credentials");
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
        level: ThinkingLevel::Medium,
    };
    assert!(cfg.enabled);
    assert_eq!(cfg.budget_tokens, Some(10000));
}

// --- ModelInfo tests ---

#[test]
fn model_info_fields() {
    let info = ModelInfo::new(
        "claude-sonnet-4-5-20250514",
        "Claude Sonnet 4.5",
        WireApi::AnthropicMessages,
        ModelCapabilities::new(200000, 8192)
            .with_images(true)
            .with_streaming(true)
            .with_thinking(true),
    );
    assert_eq!(info.id, "claude-sonnet-4-5-20250514");
    assert_eq!(info.capabilities.context_window, 200000);
    assert!(info.capabilities.supports_thinking);
}

// --- Request construction tests ---

#[test]
fn request_builds_with_all_fields() {
    let cancel = CancellationToken::new();
    let req = Request {
        transport: Default::default(),
        model: "claude-sonnet-4-5-20250514".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: Some("You are helpful.".into()),
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
                constrained_sampling: None,
            }],
        })
        .expect("valid context declarations"),
        max_tokens: Some(4096),
        temperature: Some(0.7),
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec!["STOP".into()],
        metadata: Some(serde_json::json!({"session": "abc"})),
        cancel: cancel.clone(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    };
    assert_eq!(req.model, "claude-sonnet-4-5-20250514");
    // One leading system entry folded from the shorthand plus one user
    // message (AIR-020).
    assert_eq!(req.context.messages().len(), 2);
    assert!(matches!(
        req.context.messages()[0],
        opi_ai::message::Message::System(_)
    ));
    assert_eq!(
        opi_ai::transcript::get_current_tools(req.context.messages()).len(),
        1
    );
    assert!(!cancel.is_cancelled());
}

#[test]
fn request_cancellation_propagates() {
    let cancel = CancellationToken::new();
    let req = Request {
        transport: Default::default(),
        model: "test".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: cancel.clone(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    };
    cancel.cancel();
    assert!(req.cancel.is_cancelled());
}

// --- Provider trait contract tests ---

struct DummyProvider;

impl Provider for DummyProvider {
    fn stream_prepared(&self, _request: Request, _auth: opi_ai::auth::ResolvedAuth) -> EventStream {
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

struct CanaryErrorProvider {
    upstream: String,
}

impl Provider for CanaryErrorProvider {
    fn stream_prepared(&self, _request: Request, _auth: opi_ai::auth::ResolvedAuth) -> EventStream {
        let summary = ProviderErrorSummary::from_untrusted(&self.upstream);
        let errors = vec![
            ProviderError::RequestFailed(summary.clone()),
            ProviderError::StreamError(summary.clone()),
            ProviderError::AuthFailed(summary.clone()),
            ProviderError::Network(summary.clone()),
            ProviderError::Config(summary.clone()),
            ProviderError::ProviderSide(summary.clone()),
            ProviderError::UnsupportedCapability(summary),
        ];
        Box::pin(stream::iter(errors.into_iter().map(Err)))
    }

    fn id(&self) -> &str {
        "canary-errors"
    }

    fn models(&self) -> &[ModelInfo] {
        &[]
    }
}

#[tokio::test]
async fn custom_provider_untrusted_constructor_discards_error_payloads() {
    const STATIC_PROVIDER_CANARY: &str = "custom-static-canary-with-no-known-token-shape";
    let canaries = [
        STATIC_PROVIDER_CANARY,
        "custom-access-canary-with-no-known-token-shape",
        "custom-secret-canary-with-no-known-token-shape",
        "custom-session-canary-with-no-known-token-shape",
        "custom-token-canary-with-no-known-token-shape",
    ];
    let provider = CanaryErrorProvider {
        upstream: canaries.join(" "),
    };
    let mut stream = provider.stream_prepared(
        Request {
            transport: Default::default(),
            model: "canary-errors:model".into(),
            context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
                system_prompt: None,
                messages: vec![],
                tools: vec![],
            })
            .expect("valid context declarations"),
            max_tokens: None,
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
        },
        opi_ai::test_support::resolved_auth(),
    );

    let mut count = 0;
    while let Some(Err(error)) = stream.next().await {
        count += 1;
        let rendered = format!("{error} {error:?}");
        for canary in canaries {
            assert!(
                !rendered.contains(canary),
                "custom provider leaked untrusted payload: {rendered}"
            );
        }
    }
    assert_eq!(
        count, 7,
        "every free-form error class must use the safe type"
    );
}

#[test]
fn closed_provider_error_summaries_accept_no_external_text() {
    assert_eq!(ProviderErrorSummary::redacted(), "[REDACTED]");
    assert_eq!(
        ProviderErrorSummary::authentication_rejected(),
        "provider rejected credentials"
    );
}

#[tokio::test]
async fn provider_trait_yields_done_event() {
    let provider = DummyProvider;
    let cancel = CancellationToken::new();
    let request = Request {
        transport: Default::default(),
        model: "test".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel,
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    };
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
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
    // Since DummyProvider only implements id/models/stream_prepared, this compiles.
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
