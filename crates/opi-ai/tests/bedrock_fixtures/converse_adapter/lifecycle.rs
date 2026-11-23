//! Bedrock Converse Models lifecycle and telemetry cases.

use super::*;

#[tokio::test]
async fn native_lifecycle_through_models_complete() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/model/anthropic.claude-sonnet-4-test42/converse-stream",
        ))
        .and(body_partial_json(serde_json::json!({
            "additionalModelRequestFields": {
                "thinking": {"type": "enabled", "budget_tokens": 16384, "display": "summarized"},
                "anthropic_beta": ["interleaved-thinking-2025-05-14"],
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(bedrock_text_lifecycle_bytes()))
        .expect(1)
        .mount(&server)
        .await;

    let model = model_42("anthropic.claude-sonnet-4-test42", "Claude (Bedrock)");
    let provider = Arc::new(
        BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()))
            .with_models(vec![model.clone()]),
    );
    let api =
        BedrockConverseStreamChatApi::new(provider, model.clone()).expect("valid bedrock record");
    let mut models = Models::new();
    let mut options = native();
    options.reasoning = Some(ThinkingLevel::High);
    options.tool_choice = Some(opi_ai::api::bedrock::BedrockToolChoice::Any);
    let handle = models
        .register_chat_api(
            Arc::new(api),
            Arc::new(StaticAwsResolver42 {
                credentials: test_credentials(),
            }),
            opi_ai::AuthProvenanceSource::AwsSigV4 {
                source: AwsCredentialSource::ExplicitConfig,
            },
        )
        .expect("register");

    let message = models
        .complete(
            ChatModelRecord::new("bedrock", model),
            req_42(None, "Hello"),
            handle.options(options),
        )
        .await
        .expect("native lifecycle completes");
    assert_eq!(message.stop_reason, opi_ai::stream::StopReason::Stop);
}

#[tokio::test]
async fn govcloud_region_omits_display_on_stream() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/model/anthropic.claude-sonnet-4-gov42/converse-stream",
        ))
        .and(body_partial_json(serde_json::json!({
            "additionalModelRequestFields": {
                "thinking": {"type": "enabled", "budget_tokens": 16384}
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(bedrock_text_lifecycle_bytes()))
        .expect(1)
        .mount(&server)
        .await;

    let mut credentials = test_credentials();
    credentials.region = "us-gov-west-1".into();
    let model = model_42("anthropic.claude-sonnet-4-gov42", "Claude (Bedrock)");
    let provider = Arc::new(
        BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()))
            .with_models(vec![model.clone()]),
    );
    let api =
        BedrockConverseStreamChatApi::new(provider, model.clone()).expect("valid bedrock record");
    let mut models = Models::new();
    let mut options = native();
    options.reasoning = Some(ThinkingLevel::High);
    let handle = models
        .register_chat_api(
            Arc::new(api),
            Arc::new(StaticAwsResolver42 { credentials }),
            opi_ai::AuthProvenanceSource::AwsSigV4 {
                source: AwsCredentialSource::ExplicitConfig,
            },
        )
        .expect("register");
    let message = models
        .complete(
            ChatModelRecord::new("bedrock", model),
            req_42(None, "Hello"),
            handle.options(options),
        )
        .await
        .expect("govcloud lifecycle completes");
    assert_eq!(message.stop_reason, opi_ai::stream::StopReason::Stop);
}

#[test]
fn wrong_wire_record_rejected() {
    let anthropic_model = ModelInfo::new(
        "claude-sonnet-4",
        "Claude",
        WireApi::AnthropicMessages,
        ModelCapabilities::new(200_000, 8_192).with_thinking(true),
    );
    let error = BedrockConverseStreamChatApi::new(
        Arc::new(BedrockProvider::new(None, Arc::new(HttpClient::new()))),
        anthropic_model,
    )
    .expect_err("wrong wire must fail typed");
    assert!(matches!(error, ProviderError::Config(_)));
}

#[test]
fn from_simple_maps_only_tool_choice() {
    let simple = opi_ai::api::contract::SimpleOptions {
        tool_choice: Some(opi_ai::api::contract::SimpleToolChoice::None),
        ..Default::default()
    };
    let options = BedrockOptions::from_simple(&simple);
    assert!(options.is_simple());
    assert!(matches!(
        options.tool_choice,
        Some(opi_ai::api::bedrock::BedrockToolChoice::None_)
    ));
    assert_eq!(options.reasoning, None);
    assert!(options.thinking_budgets.is_empty());
    assert!(options.request_metadata.is_empty());
}

#[cfg(feature = "telemetry-tracing")]
#[tokio::test]
async fn telemetry_carrier_instruments_bedrock_producer() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tracing_core::span::{Attributes, Id, Record};
    use tracing_core::{Dispatch, Event, Metadata, Subscriber, dispatcher};

    #[derive(Default)]
    struct Recording {
        next: u64,
        spans: Vec<(&'static str, Option<u64>)>,
    }
    struct Recorder(std::sync::Arc<std::sync::Mutex<Recording>>);

    impl Subscriber for Recorder {
        fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, attributes: &Attributes<'_>) -> Id {
            let mut state = self.0.lock().unwrap();
            state.next += 1;
            let id = Id::from_u64(state.next);
            let parent = attributes
                .parent()
                .cloned()
                .or_else(|| attributes.is_contextual().then(|| Id::from_u64(1)))
                .map(|id| id.into_u64());
            state.spans.push((attributes.metadata().name(), parent));
            id
        }
        fn record(&self, _id: &Id, _values: &Record<'_>) {}
        fn record_follows_from(&self, _sub: &Id, _follows_from: &Id) {}
        fn event(&self, _event: &Event<'_>) {}
        fn enter(&self, _id: &Id) {}
        fn exit(&self, _id: &Id) {}
        fn try_close(&self, _id: Id) -> bool {
            false
        }
    }

    let recorder = std::sync::Arc::new(std::sync::Mutex::new(Recording::default()));
    let source = Dispatch::new(Recorder(recorder.clone()));
    let parent = dispatcher::with_default(&source, || tracing::info_span!("source_parent_42"));
    let context = opi_ai::telemetry::TelemetryContext::from_span(parent);

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/model/anthropic.claude-sonnet-4-tel42/converse-stream",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(bedrock_text_lifecycle_bytes()))
        .expect(1)
        .mount(&server)
        .await;

    let seen = Arc::new(AtomicUsize::new(0));
    let counter = seen.clone();
    let mut request = req_42(None, "Hello");
    request.model = "bedrock:anthropic.claude-sonnet-4-tel42".into();
    request.telemetry_context = Some(context);
    request.callbacks =
        opi_ai::callbacks::RequestCallbacks::new().with_async_on_payload(move |_| {
            let counter = counter.clone();
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                tracing::info_span!("payload_probe_42").in_scope(|| ());
                Ok(None)
            }
        });

    let provider = BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()));
    let stream = provider.stream_prepared(request, test_auth());
    let mut stream = Box::pin(stream);
    let mut done = false;
    while let Some(item) = stream.next().await {
        if let AssistantStreamEvent::Done { .. } = item.expect("stream item") {
            done = true;
            break;
        }
    }
    assert!(done, "stream reaches Done");
    assert_eq!(seen.load(Ordering::SeqCst), 1);
    let state = recorder.lock().unwrap();
    assert!(
        state
            .spans
            .iter()
            .any(|(name, _)| *name == "payload_probe_42"),
        "bedrock producer telemetry flows to the frozen carrier: {:?}",
        state.spans
    );
}
