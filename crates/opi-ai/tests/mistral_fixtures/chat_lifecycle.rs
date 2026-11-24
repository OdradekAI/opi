//! Mistral HTTP lifecycle assertions.

use super::*;

// Provider stream cancellation (Phase 12 task 12.7 DoD clause 6)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_cancellation_aborts_before_completion() {
    // The CancellationToken is threaded into the Mistral adapter's HTTP
    // body-stream loop (inherited from the shared OpenAI-compat path; see
    // api/openai_completions.rs `cancel.cancelled()` select arm). Cancelling while the
    // stream is open must terminate it before the delayed terminal chunk
    // arrives, proving the inherited adapter path observes cancellation.
    let server = spawn_stalled_mistral_server().await;

    let cancel = CancellationToken::new();
    let provider = opi_ai::providers::mistral::mistral_provider(Some(server));
    let request = Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "mistral:mistral-small-latest".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "Hello".into(),
                }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: Some(1024),
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
    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());

    let first = stream
        .next()
        .await
        .expect("stream should produce at least one event")
        .expect("first event should be valid");
    assert!(matches!(first, AssistantStreamEvent::Start { .. }));
    cancel.cancel();

    let next = tokio::time::timeout(std::time::Duration::from_millis(200), stream.next())
        .await
        .expect("stream must surface cancellation before the delayed terminal fixture completes");
    match next {
        Some(Err(cause))
            if cause.category() == opi_ai::provider::ProviderErrorCategory::Cancelled =>
        { /* typed cancellation, as contracted */ }
        other => panic!("expected Err(ProviderError::Cancelled) on cancellation, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
