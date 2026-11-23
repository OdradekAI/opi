//! Azure OpenAI Chat wire assertions.

use super::*;

#[test]
fn provider_id_is_azure() {
    let provider = make_provider();
    assert_eq!(provider.id(), "azure");
}

#[test]
fn azure_url_construction() {
    let provider = make_provider();
    let url = provider.build_azure_url("my-gpt4o");
    assert_eq!(
        url,
        "https://myresource.openai.azure.com/openai/deployments/my-gpt4o/chat/completions?api-version=2024-06-01"
    );
}

#[test]
fn missing_endpoint_returns_error() {
    let result = AzureOpenAIProvider::new(None, "deploy1".into(), None);
    assert!(result.is_err(), "missing endpoint should return error");
    let err = result.unwrap_err();
    match err {
        opi_ai::provider::ProviderError::Config(msg) => {
            // Task 20.38: the message follows the pinned source resolver
            // wording (base URL required, config/env positions named).
            assert!(
                msg.contains("base URL is required"),
                "unexpected error: {msg}"
            );
        }
        other => panic!("expected Config, got {other:?}"),
    }
}

#[test]
fn models_from_config_deployments() {
    let provider = make_provider_with_deployments(vec!["my-gpt4o", "my-gpt4o-mini"]);
    let models = provider.models();
    assert_eq!(models.len(), 2);
    assert_eq!(models[0].id, "my-gpt4o");
    assert_eq!(models[1].id, "my-gpt4o-mini");
}

#[test]
fn selected_deployment_is_advertised_when_catalog_not_configured() {
    let provider = make_provider();
    assert_eq!(provider.models().len(), 1);
    assert_eq!(provider.models()[0].id, "my-gpt4o");
    assert_eq!(
        provider.models()[0].wire_api,
        opi_ai::WireApi::AzureOpenAiCompletions
    );
}

#[tokio::test]
async fn text_streaming_from_fixture() {
    let provider = make_provider();
    let request = text_request();
    let stream = wire_adapter(&provider).stream_from_sse(text_sse_fixture(), request.cancel);
    let events = collect_events(stream).await;

    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::Start { .. }))
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::TextDelta { .. }))
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::Done { .. }))
    );

    // Check text content
    let text_deltas: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            AssistantStreamEvent::TextDelta { delta, .. } => Some(delta.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(text_deltas.join(""), "Hi there");
}

#[tokio::test]
async fn tool_call_from_fixture() {
    let provider = make_provider();
    let request = tool_request();
    let stream = wire_adapter(&provider).stream_from_sse(&tool_call_sse_fixture(), request.cancel);
    let events = collect_events(stream).await;

    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::ToolCallStart { .. }))
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::ToolCallDelta { .. }))
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::ToolCallEnd { .. }))
    );

    let done = events
        .iter()
        .find(|e| matches!(e, AssistantStreamEvent::Done { .. }));
    assert!(done.is_some());
    if let Some(AssistantStreamEvent::Done { reason, .. }) = done {
        assert_eq!(*reason, opi_ai::stream::StopReason::ToolUse);
    }
}

#[tokio::test]
async fn usage_from_fixture() {
    let provider = make_provider();
    let request = text_request();
    let stream = wire_adapter(&provider).stream_from_sse(text_sse_fixture(), request.cancel);
    let events = collect_events(stream).await;

    let done = events
        .iter()
        .find(|e| matches!(e, AssistantStreamEvent::Done { .. }));
    assert!(done.is_some());
    if let Some(AssistantStreamEvent::Done { message, .. }) = done {
        assert_eq!(message.usage.input_tokens, 10);
        assert_eq!(message.usage.output_tokens, 5);
    }
}

#[tokio::test]
async fn error_from_fixture() {
    let provider = make_provider();
    let request = text_request();
    let stream = wire_adapter(&provider).stream_from_sse(error_sse_fixture(), request.cancel);
    let events: Vec<Result<AssistantStreamEvent, ProviderError>> = collect_items(stream).await;

    // Current producers stop the dual terminal format: the in-band error
    // event is a stream failure with a bounded typed cause.
    let failure = events.iter().find(|e| {
        e.as_ref()
            .is_err_and(|cause| cause.category() == opi_ai::provider::ProviderErrorCategory::Stream)
    });
    let Some(Err(cause)) = failure else {
        panic!("expected a typed stream failure, got {events:?}");
    };
    let summary = cause.to_string();
    assert!(
        summary.contains("openai chat stream error"),
        "cause must be the neutral literal, got: {summary}"
    );
    assert!(
        !summary.contains("Deployment not found"),
        "raw upstream error text must not leak into the cause: {summary}"
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Ok(AssistantStreamEvent::Error { .. })))
    );
}

#[tokio::test]
async fn malformed_frame_does_not_expose_upstream_content() {
    let canary = "azure-malformed-secret-canary";
    let sse = format!("data: {{not-json-{canary}}}\n\n");
    let provider = make_provider();
    let events = wire_adapter(&provider)
        .stream_from_sse(&sse, CancellationToken::new())
        .collect::<Vec<_>>()
        .await;
    let rendered = format!("{events:?}");
    assert!(events.iter().any(Result::is_err));
    assert!(
        !rendered.contains(canary),
        "malformed Azure frame leaked upstream content: {rendered}"
    );
}

#[test]
fn secret_redaction_in_debug() {
    // Phase 17.5: the api key moved out of AzureOpenAIProvider construction
    // into ResolvedAuth (passed via stream_prepared). The provider no longer
    // stores the credential, so it cannot leak through its Debug output.
    // ResolvedAuth's own Debug redaction is pinned in auth_contracts.rs.
    let provider = make_provider();
    let debug_str = format!("{provider:?}");
    assert!(!debug_str.contains("test-api-key-12345"));
}

#[test]
fn request_body_uses_deployment_name() {
    let provider = make_provider();
    let request = text_request();
    let body = wire_adapter(&provider)
        .build_request_body(&request)
        .unwrap();
    assert_eq!(body["model"], "my-gpt4o");
}

// ---------------------------------------------------------------------------
// Production Provider::stream lifecycle through a real HTTP exchange
// ---------------------------------------------------------------------------
//
// The offline `stream_from_sse` tests above exercise the SSE parser/mapper but
// bypass the production HTTP transport (`Provider::stream` -> `stream_azure_http`),
// so they cannot prove the Azure deployment URL, `api-key` auth header, request
// body, or end-to-end draining through the adapter path. These wiremock tests
// cover that contract using local mock HTTP only.
