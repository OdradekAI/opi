//! Tests for the shared MockProvider test harness (task 1.17).
//!
//! Verifies MockProvider correctly simulates Provider behavior for
//! text, tool-call, and error responses, and tracks call history.

use futures_util::StreamExt;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{CacheRetention, Provider, Request};
use opi_ai::stream::{AssistantStreamEvent, StopReason, Usage};
use opi_ai::test_support::{self, MockProvider};
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Construction and basic trait methods
// ---------------------------------------------------------------------------

#[test]
fn mock_provider_returns_id() {
    let provider = MockProvider::new("test-provider", vec![]);
    assert_eq!(provider.id(), "test-provider");
}

#[test]
fn mock_provider_returns_models() {
    let provider = MockProvider::new("test", vec![]);
    let models = provider.models();
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].id, "mock-model");
    assert!(models[0].capabilities.supports_streaming);
}

#[test]
fn mock_provider_initial_call_count_is_zero() {
    let provider = MockProvider::new("test", vec![]);
    assert_eq!(provider.stream_call_count(), 0);
}

// ---------------------------------------------------------------------------
// stream() yields configured events
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_yields_text_response_events() {
    let response = test_support::text_response("Hello, world!");
    let provider = MockProvider::new("test", vec![response]);

    let request = Request {
        model: "mock-model".into(),
        system: None,
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text { text: "Hi".into() }],
            timestamp_ms: 0,
        })],
        tools: vec![],
        max_tokens: None,
        temperature: None,
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    };

    let stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    let events: Vec<Result<AssistantStreamEvent, _>> = stream.collect::<Vec<_>>().await;

    assert_eq!(events.len(), 3);
    assert!(matches!(
        events[0].as_ref().unwrap(),
        AssistantStreamEvent::Start { .. }
    ));
    assert!(matches!(
        events[1].as_ref().unwrap(),
        AssistantStreamEvent::TextDelta { .. }
    ));
    assert!(matches!(
        events[2].as_ref().unwrap(),
        AssistantStreamEvent::Done {
            reason: StopReason::Stop,
            ..
        }
    ));
    assert_eq!(provider.stream_call_count(), 1);
}

#[tokio::test]
async fn stream_yields_tool_call_response_events() {
    let response = test_support::tool_call_response("tc-1", "read", r#"{"path":"/tmp/f"}"#);
    let provider = MockProvider::new("test", vec![response]);

    let request = Request {
        model: "mock-model".into(),
        system: None,
        messages: vec![],
        tools: vec![],
        max_tokens: None,
        temperature: None,
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    };

    let stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    let events: Vec<Result<AssistantStreamEvent, _>> = stream.collect::<Vec<_>>().await;

    assert_eq!(events.len(), 3);
    assert!(matches!(
        events[0].as_ref().unwrap(),
        AssistantStreamEvent::Start { .. }
    ));
    assert!(matches!(
        events[1].as_ref().unwrap(),
        AssistantStreamEvent::ToolCallEnd { .. }
    ));
    if let AssistantStreamEvent::ToolCallEnd { tool_call, .. } = events[1].as_ref().unwrap() {
        assert_eq!(tool_call.id, "tc-1");
        assert_eq!(tool_call.name, "read");
    }
    assert!(matches!(
        events[2].as_ref().unwrap(),
        AssistantStreamEvent::Done {
            reason: StopReason::ToolUse,
            ..
        }
    ));
}

#[tokio::test]
async fn stream_yields_error_response_events() {
    let response = test_support::error_response("something went wrong");
    let provider = MockProvider::new("test", vec![response]);

    let request = Request {
        model: "mock-model".into(),
        system: None,
        messages: vec![],
        tools: vec![],
        max_tokens: None,
        temperature: None,
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    };

    let stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    let events: Vec<Result<AssistantStreamEvent, _>> = stream.collect::<Vec<_>>().await;

    assert_eq!(events.len(), 2);
    assert!(matches!(
        events[0].as_ref().unwrap(),
        AssistantStreamEvent::Start { .. }
    ));
    assert!(matches!(
        events[1].as_ref().unwrap(),
        AssistantStreamEvent::Error {
            reason: StopReason::Error,
            ..
        }
    ));
}

// ---------------------------------------------------------------------------
// Multiple responses consumed in order
// ---------------------------------------------------------------------------

#[tokio::test]
async fn multiple_stream_calls_consume_responses_in_order() {
    let first = test_support::text_response("first");
    let second = test_support::text_response("second");
    let provider = MockProvider::new("test", vec![first, second]);

    let dummy_request = Request {
        model: "mock-model".into(),
        system: None,
        messages: vec![],
        tools: vec![],
        max_tokens: None,
        temperature: None,
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    };

    // First call
    let stream1 = provider.stream_prepared(dummy_request, opi_ai::test_support::resolved_auth());
    let events1: Vec<_> = stream1.collect::<Vec<_>>().await;
    assert_eq!(events1.len(), 3);

    // Second call
    let stream2 = provider.stream_prepared(
        Request {
            model: "mock-model".into(),
            system: None,
            messages: vec![],
            tools: vec![],
            max_tokens: None,
            temperature: None,
            thinking: Default::default(),
            stop_sequences: vec![],
            metadata: None,
            cancel: CancellationToken::new(),
            timeout: None,
            extra_headers: vec![],
            cache_retention: CacheRetention::None,
            session_id: None,
        },
        opi_ai::test_support::resolved_auth(),
    );
    let events2: Vec<_> = stream2.collect::<Vec<_>>().await;
    assert_eq!(events2.len(), 3);

    assert_eq!(provider.stream_call_count(), 2);

    // Verify the text deltas are different
    let delta1 = match events1[1].as_ref().unwrap() {
        AssistantStreamEvent::TextDelta { delta, .. } => delta.clone(),
        _ => String::new(),
    };
    let delta2 = match events2[1].as_ref().unwrap() {
        AssistantStreamEvent::TextDelta { delta, .. } => delta.clone(),
        _ => String::new(),
    };
    assert_eq!(delta1, "first");
    assert_eq!(delta2, "second");
}

// ---------------------------------------------------------------------------
// Builder helpers produce correct structure
// ---------------------------------------------------------------------------

#[test]
fn text_response_produces_start_delta_done() {
    let events = test_support::text_response("hello");
    assert_eq!(events.len(), 3);
    assert!(matches!(&events[0], AssistantStreamEvent::Start { .. }));
    assert!(
        matches!(&events[1], AssistantStreamEvent::TextDelta { delta, .. } if delta == "hello")
    );
    assert!(matches!(
        &events[2],
        AssistantStreamEvent::Done {
            reason: StopReason::Stop,
            ..
        }
    ));
}

#[test]
fn tool_call_response_produces_correct_tool_call() {
    let events = test_support::tool_call_response("tc-42", "bash", r#"{"cmd":"ls"}"#);
    assert_eq!(events.len(), 3);
    assert!(matches!(&events[0], AssistantStreamEvent::Start { .. }));
    if let AssistantStreamEvent::ToolCallEnd { tool_call, .. } = &events[1] {
        assert_eq!(tool_call.id, "tc-42");
        assert_eq!(tool_call.name, "bash");
        assert_eq!(tool_call.arguments, r#"{"cmd":"ls"}"#);
    } else {
        panic!("expected ToolCallEnd at index 1");
    }
    assert!(matches!(
        &events[2],
        AssistantStreamEvent::Done {
            reason: StopReason::ToolUse,
            ..
        }
    ));
}

#[test]
fn error_response_produces_start_error() {
    let events = test_support::error_response("boom");
    assert_eq!(events.len(), 2);
    assert!(matches!(&events[0], AssistantStreamEvent::Start { .. }));
    assert!(matches!(
        &events[1],
        AssistantStreamEvent::Error {
            reason: StopReason::Error,
            ..
        }
    ));
}

#[test]
fn base_assistant_has_sensible_defaults() {
    let msg = test_support::base_assistant();
    assert!(msg.content.is_empty());
    assert_eq!(msg.provider, "mock");
    assert_eq!(msg.model, "mock-model");
    assert_eq!(msg.stop_reason, StopReason::Stop);
    assert_eq!(msg.usage, Usage::default());
}

// ---------------------------------------------------------------------------
// Provider trait compliance
// ---------------------------------------------------------------------------

#[test]
fn mock_provider_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<MockProvider>();
}
