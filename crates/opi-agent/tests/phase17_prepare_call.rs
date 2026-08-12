//! Phase 17.2 — collection-owned prepare_call dispatch (P17-PRV-003 slice).
//!
//! Proves the Agent resolves authentication ONCE per logical call (at
//! `prepare_call`) and opens every retry attempt from the same opaque prepared
//! call: a counting resolver increments exactly once even though the provider
//! retries, and the mock provider's `stream` is invoked once per attempt.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use opi_agent::agent::Agent;
use opi_agent::hooks::{
    AgentHooks, BeforeToolCallContext, BeforeToolCallResult, ShouldStopAfterTurnContext,
};
use opi_agent::loop_types::{AgentError, AgentLoopConfig, InferenceConfig};
use opi_agent::message::AgentMessage;
use opi_ai::provider::ProviderError;
use opi_ai::test_support::{CountingAuthResolver, MockProvider, MockResponse, text_response};
use opi_ai::{AuthProvenanceSource, CompatMetadata, ProviderCollection};

struct StopImmediatelyHooks;
impl AgentHooks for StopImmediatelyHooks {
    fn convert_to_llm(
        &self,
        messages: &[AgentMessage],
    ) -> Result<Vec<opi_ai::message::Message>, AgentError> {
        Ok(messages
            .iter()
            .filter_map(|m| match m {
                AgentMessage::Llm(m) => Some(m.clone()),
                _ => None,
            })
            .collect())
    }
    fn should_stop_after_turn(
        &self,
        _ctx: ShouldStopAfterTurnContext,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send>> {
        Box::pin(async { true })
    }
    fn before_tool_call(
        &self,
        _ctx: BeforeToolCallContext,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = BeforeToolCallResult> + Send>> {
        Box::pin(async { BeforeToolCallResult::Allow })
    }
}

#[tokio::test]
async fn prepare_call_resolves_auth_once_across_retries() {
    // First attempt: retriable rate-limit (no content delivered). Second
    // attempt: a successful text response. The retry reuses the same prepared
    // call, so the resolver must be invoked exactly once.
    let provider = MockProvider::new_with_errors(
        "mock",
        vec![
            MockResponse::Error(ProviderError::RateLimited {
                retry_after_ms: None,
            }),
            MockResponse::Events(text_response("recovered")),
        ],
    );
    let call_log = provider.call_log_handle();

    let resolve_count = Arc::new(AtomicU32::new(0));
    let resolver: Arc<dyn opi_ai::auth::AuthResolver> =
        Arc::new(CountingAuthResolver::new(resolve_count.clone()));

    let mut collection = ProviderCollection::new();
    collection
        .register_route(
            Box::new(provider),
            resolver,
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .expect("register route");

    let mut agent = Agent::new(
        Arc::new(collection),
        Vec::new(),
        "mock:mock-model".to_string(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig {
            max_turns: 5,
            retry: Some(opi_ai::retry::RetryConfig {
                max_attempts: 3,
                ..Default::default()
            }),
        },
        Box::new(StopImmediatelyHooks),
    )
    .expect("agent");

    let result = agent.prompt("hi").await;
    assert!(
        result.is_ok(),
        "prompt should succeed after retry: {:?}",
        result.err()
    );

    // Auth resolved exactly once across the two attempts (P17-PRV-003).
    assert_eq!(
        resolve_count.load(Ordering::SeqCst),
        1,
        "resolver must be invoked once per logical call, not per attempt"
    );
    // Two provider attempts: the failed attempt plus the recovered one.
    assert_eq!(
        call_log.lock().unwrap().len(),
        2,
        "provider stream must be invoked once per attempt"
    );
}
