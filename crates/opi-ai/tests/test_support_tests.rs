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
        transport: Default::default(),
        model: "mock-model".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text { text: "Hi".into() }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
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
        transport: Default::default(),
        model: "mock-model".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
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
        transport: Default::default(),
        model: "mock-model".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
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
        transport: Default::default(),
        model: "mock-model".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    };

    // First call
    let stream1 = provider.stream_prepared(dummy_request, opi_ai::test_support::resolved_auth());
    let events1: Vec<_> = stream1.collect::<Vec<_>>().await;
    assert_eq!(events1.len(), 3);

    // Second call
    let stream2 = provider.stream_prepared(
        Request {
            transport: Default::default(),
            model: "mock-model".into(),
            context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
                system_prompt: None,
                messages: vec![],
                tools: vec![],
            })
            .expect("valid context declarations"),
            max_tokens: None,
            temperature: None,
            sampling_params: Default::default(),
            thinking: Default::default(),
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

#[test]
fn resolved_auth_helpers_pin_their_prepared_schemes() {
    // `resolved_auth` prepares ApiKey (anthropic/azure/gemini wires) and
    // `resolved_bearer_auth` prepares Bearer (openai family, vertex); the
    // Bearer-only adapters reject the ApiKey helper before the wire.
    assert!(matches!(
        test_support::resolved_auth().scheme,
        opi_ai::auth::AuthScheme::ApiKey
    ));
    assert!(matches!(
        test_support::resolved_bearer_auth().scheme,
        opi_ai::auth::AuthScheme::Bearer
    ));
}

// ---------------------------------------------------------------------------
// Provider trait compliance
// ---------------------------------------------------------------------------

#[test]
fn mock_provider_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<MockProvider>();
}

// ---------------------------------------------------------------------------
// task 20.53.8: scripted delayed poll, cancellation, ordered multi-block
// ---------------------------------------------------------------------------

use opi_ai::message::{AssistantContent, ToolCall};
use opi_ai::provider::ProviderError;
use opi_ai::test_support::{MockResponse, blocks_response};

fn scripted_request(cancel: CancellationToken) -> Request {
    Request {
        transport: Default::default(),
        model: "mock-model".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
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

#[tokio::test]
async fn delayed_poll_yields_rate_limited_then_events() {
    let provider = MockProvider::new_with_errors(
        "test",
        vec![MockResponse::DelayedAfter {
            pending_attempts: 2,
            retry_after_ms: Some(37),
            events: test_support::text_response("ready"),
        }],
    );
    for attempt in 0..2 {
        let stream = provider.stream_prepared(
            scripted_request(CancellationToken::new()),
            opi_ai::test_support::resolved_auth(),
        );
        let events: Vec<_> = stream.collect::<Vec<_>>().await;
        assert_eq!(events.len(), 1, "pending attempt {attempt}");
        assert!(matches!(
            events[0].as_ref().unwrap_err(),
            ProviderError::RateLimited {
                retry_after_ms: Some(37)
            }
        ));
    }
    let stream = provider.stream_prepared(
        scripted_request(CancellationToken::new()),
        opi_ai::test_support::resolved_auth(),
    );
    let events: Vec<_> = stream.collect::<Vec<_>>().await;
    assert_eq!(events.len(), 3);
    assert_eq!(provider.stream_call_count(), 3);
}

#[tokio::test]
async fn delayed_poll_with_zero_pending_matches_events() {
    let plain = MockProvider::new("test", vec![test_support::text_response("same")]);
    let delayed = MockProvider::new_with_errors(
        "test",
        vec![MockResponse::DelayedAfter {
            pending_attempts: 0,
            retry_after_ms: None,
            events: test_support::text_response("same"),
        }],
    );
    let plain_events: Vec<_> = plain
        .stream_prepared(
            scripted_request(CancellationToken::new()),
            opi_ai::test_support::resolved_auth(),
        )
        .collect::<Vec<_>>()
        .await;
    let delayed_events: Vec<_> = delayed
        .stream_prepared(
            scripted_request(CancellationToken::new()),
            opi_ai::test_support::resolved_auth(),
        )
        .collect::<Vec<_>>()
        .await;
    assert_eq!(plain_events.len(), delayed_events.len());
    assert_eq!(plain_events.len(), 3);
    assert_eq!(delayed.stream_call_count(), 1);
}

#[tokio::test(start_paused = true)]
async fn hanging_response_fails_with_cancelled_when_token_fires() {
    let token = CancellationToken::new();
    let cancel_at = token.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        cancel_at.cancel();
    });
    let provider = MockProvider::new_with_errors("test", vec![MockResponse::HangingUntilCancelled]);
    let events: Vec<_> = provider
        .stream_prepared(
            scripted_request(token),
            opi_ai::test_support::resolved_auth(),
        )
        .collect::<Vec<_>>()
        .await;
    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0].as_ref().unwrap_err(),
        ProviderError::Cancelled
    ));
}

#[tokio::test(start_paused = true)]
async fn hanging_response_pre_cancelled_fails_immediately() {
    let token = CancellationToken::new();
    token.cancel();
    let provider = MockProvider::new_with_errors("test", vec![MockResponse::HangingUntilCancelled]);
    let events: Vec<_> = provider
        .stream_prepared(
            scripted_request(token),
            opi_ai::test_support::resolved_auth(),
        )
        .collect::<Vec<_>>()
        .await;
    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0].as_ref().unwrap_err(),
        ProviderError::Cancelled
    ));
}

#[test]
fn blocks_response_text_prefix_snapshots() {
    assert_blocks_response_prefix_snapshots(
        vec![
            AssistantContent::Text {
                text: "first".into(),
            },
            AssistantContent::Text {
                text: "second".into(),
            },
        ],
        StopReason::Stop,
    );
}

#[test]
fn blocks_response_text_tool_text_prefix_snapshots() {
    assert_blocks_response_prefix_snapshots(
        vec![
            AssistantContent::Text {
                text: "before".into(),
            },
            AssistantContent::ToolCall {
                tool_call: ToolCall {
                    id: "t1".into(),
                    name: "lookup".into(),
                    arguments: "{}".into(),
                    arguments_error: None,
                },
            },
            AssistantContent::Text {
                text: "after".into(),
            },
        ],
        StopReason::ToolUse,
    );
}

fn assert_blocks_response_prefix_snapshots(
    blocks: Vec<AssistantContent>,
    expected_stop: StopReason,
) {
    let mut events = blocks_response(blocks.clone());
    assert_eq!(events.len(), blocks.len() + 2);
    match &events[0] {
        AssistantStreamEvent::Start { partial } => assert!(partial.content.is_empty()),
        other => panic!("expected Start, got {other:?}"),
    }
    match events.last_mut().unwrap() {
        AssistantStreamEvent::Done { reason, message } => {
            assert_eq!(*reason, expected_stop);
            assert_eq!(message.content, blocks);
            // A later owned message cannot alter previously emitted snapshots.
            message.content.clear();
        }
        other => panic!("expected Done, got {other:?}"),
    }
    for (index, block) in blocks.iter().enumerate() {
        let (content_index, partial) = match (&events[index + 1], block) {
            (
                AssistantStreamEvent::TextDelta {
                    content_index,
                    delta,
                    partial,
                },
                AssistantContent::Text { text },
            ) => {
                assert_eq!(delta, text);
                (*content_index, partial)
            }
            (
                AssistantStreamEvent::ToolCallEnd {
                    content_index,
                    tool_call,
                    partial,
                },
                AssistantContent::ToolCall {
                    tool_call: expected,
                },
            ) => {
                assert_eq!(tool_call, expected);
                (*content_index, partial)
            }
            (other, _) => panic!("unexpected event for block {index}: {other:?}"),
        };
        assert_eq!(content_index, index);
        assert_eq!(
            partial.content,
            blocks[..=index],
            "snapshot at block {index}"
        );
    }
}

#[test]
fn blocks_response_orders_blocks_and_uses_tool_use_stop() {
    let blocks = vec![
        AssistantContent::Text {
            text: "Let me check.".into(),
        },
        AssistantContent::ToolCall {
            tool_call: ToolCall {
                id: "t1".into(),
                name: "lookup".into(),
                arguments: "{}".into(),
                arguments_error: None,
            },
        },
        AssistantContent::ToolCall {
            tool_call: ToolCall {
                id: "t2".into(),
                name: "store".into(),
                arguments: "[]".into(),
                arguments_error: None,
            },
        },
    ];
    let events = blocks_response(blocks);
    assert_eq!(events.len(), 5);
    assert!(matches!(events[0], AssistantStreamEvent::Start { .. }));
    let delta = match &events[1] {
        AssistantStreamEvent::TextDelta {
            content_index,
            delta,
            ..
        } => (*content_index, delta.clone()),
        other => panic!("expected TextDelta, got {other:?}"),
    };
    assert_eq!(delta, (0, "Let me check.".to_owned()));
    let tool_one = match &events[2] {
        AssistantStreamEvent::ToolCallEnd {
            content_index,
            tool_call,
            ..
        } => (*content_index, tool_call.id.clone()),
        other => panic!("expected ToolCallEnd, got {other:?}"),
    };
    assert_eq!(tool_one, (1, "t1".to_owned()));
    let tool_two = match &events[3] {
        AssistantStreamEvent::ToolCallEnd {
            content_index,
            tool_call,
            ..
        } => (*content_index, tool_call.id.clone()),
        other => panic!("expected ToolCallEnd, got {other:?}"),
    };
    assert_eq!(tool_two, (2, "t2".to_owned()));
    match &events[4] {
        AssistantStreamEvent::Done { reason, message } => {
            assert_eq!(*reason, StopReason::ToolUse);
            assert_eq!(message.content.len(), 3);
        }
        other => panic!("expected Done, got {other:?}"),
    }
}

#[test]
fn blocks_response_text_only_matches_text_response_shape() {
    let blocks = blocks_response(vec![AssistantContent::Text { text: "hi".into() }]);
    let plain = test_support::text_response("hi");
    assert_eq!(blocks.len(), plain.len());
    let delta = match &blocks[1] {
        AssistantStreamEvent::TextDelta { delta, .. } => delta.clone(),
        other => panic!("expected TextDelta, got {other:?}"),
    };
    assert_eq!(delta, "hi");
    assert!(matches!(
        &blocks[2],
        AssistantStreamEvent::Done {
            reason: StopReason::Stop,
            ..
        }
    ));
}

#[tokio::test]
async fn mixed_script_queue_consumes_in_order() {
    let partial = {
        let mut partial = test_support::base_assistant();
        partial.content.push(AssistantContent::Text {
            text: "half".into(),
        });
        vec![AssistantStreamEvent::Start {
            partial: test_support::base_assistant(),
        }]
    };
    let provider = MockProvider::new_with_errors(
        "test",
        vec![
            MockResponse::DelayedAfter {
                pending_attempts: 1,
                retry_after_ms: None,
                events: test_support::text_response("delayed"),
            },
            MockResponse::Events(test_support::text_response("plain")),
            MockResponse::EventsThenError(
                partial,
                ProviderError::RateLimited {
                    retry_after_ms: None,
                },
            ),
        ],
    );
    let collect = |p: &MockProvider| {
        let stream = p.stream_prepared(
            scripted_request(CancellationToken::new()),
            opi_ai::test_support::resolved_auth(),
        );
        futures_util::stream::StreamExt::collect::<Vec<_>>(stream)
    };
    let first: Vec<_> = collect(&provider).await;
    assert!(matches!(
        first[0].as_ref().unwrap_err(),
        ProviderError::RateLimited {
            retry_after_ms: None
        }
    ));
    let second: Vec<_> = collect(&provider).await;
    assert_eq!(second.len(), 3);
    let third: Vec<_> = collect(&provider).await;
    assert_eq!(third.len(), 3);
    let fourth: Vec<_> = collect(&provider).await;
    assert!(fourth[0].is_ok());
    assert!(matches!(
        fourth[1].as_ref().unwrap_err(),
        ProviderError::RateLimited {
            retry_after_ms: None
        }
    ));
    assert_eq!(provider.stream_call_count(), 4);
}
