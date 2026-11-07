use futures_util::stream;
use opi_ai::auth::{AuthProvenanceSource, AuthScheme, StaticAuthResolver};
use opi_ai::deferred::{DeferredCancelCall, DeferredFetchCall};
use opi_ai::message::ToolCall;
use opi_ai::models::{ChatModelRecord, SimpleOptions};
use opi_ai::provider::{EventStream, Provider, ProviderError, Request};
use opi_ai::retry::AcquisitionPolicy;
use opi_ai::stream::{AssistantStreamEvent, ChatStream, StopReason};
use opi_ai::telemetry::{
    AI_BLOCK, AI_FIRST_CHUNK, AiRequestAttributes, AiRequestTelemetry, ai_telemetry_schema,
};
use opi_ai::test_support::{
    MockProvider, MockResponse, ScriptedDeferredBackend, base_assistant, single_route_collection,
    text_response,
};
use opi_ai::transcript::normalize_context;
use opi_telemetry::memory::Recorder;
use opi_telemetry::{AttributeValue, SpanStatus};
use secrecy::SecretString;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[tokio::test]
async fn ordinary_models_chat_auto_records_ai_request() {
    let recorder = Recorder::default();
    let telemetry_context = recorder.context();
    let mut start = base_assistant();
    start.provider = "probe".into();
    start.model = "mock-model".into();
    let mut message = start.clone();
    message
        .content
        .push(opi_ai::message::AssistantContent::Text {
            text: "A😀".into(),
            signature: None,
        });
    let provider = MockProvider::new(
        "probe",
        vec![vec![
            AssistantStreamEvent::Start { partial: start },
            AssistantStreamEvent::TextEnd {
                content_index: 0,
                content: "A😀".into(),
                partial: message.clone(),
            },
            AssistantStreamEvent::Done {
                reason: StopReason::Stop,
                message,
            },
        ]],
    );
    let model = provider.models()[0].clone();
    let models = single_route_collection(Box::new(provider));
    let request = Request {
        model: "probe:mock-model".into(),
        context: normalize_context(Default::default()).expect("empty context normalizes"),
        projection: Default::default(),
        transport: Default::default(),
        max_tokens: None,
        temperature: None,
        sampling_params: None,
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: Default::default(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: Default::default(),
        session_id: None,
        telemetry_context: Some(telemetry_context),
        callbacks: Default::default(),
    };

    let result = models
        .complete_simple(
            ChatModelRecord::new("probe", model),
            request,
            SimpleOptions::default(),
        )
        .await
        .expect("ordinary chat completes");
    assert_eq!(result.stop_reason, StopReason::Stop);

    let spans = recorder.spans();
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].name, "pi.ai.request");
    assert_eq!(
        spans[0].attributes.get("provider"),
        Some(&AttributeValue::String("probe".into()))
    );
    assert_eq!(
        spans[0].attributes.get("model"),
        Some(&AttributeValue::String("mock-model".into()))
    );
    assert_eq!(spans[0].events.len(), 2);
    assert_eq!(spans[0].events[0].name, AI_FIRST_CHUNK);
    assert_eq!(spans[0].events[1].name, AI_BLOCK);
    assert_eq!(
        spans[0].events[1].attributes.get("chars"),
        Some(&AttributeValue::Number(3.0))
    );
    assert_eq!(
        spans[0].attributes.get("stop"),
        Some(&AttributeValue::String("stop".into()))
    );
}

#[tokio::test]
async fn ordinary_models_chat_abort_finishes_request_telemetry() {
    let recorder = Recorder::default();
    let telemetry_context = recorder.context();
    let provider =
        MockProvider::new_with_errors("probe", vec![MockResponse::HangingUntilCancelled]);
    let calls = provider.call_log_handle();
    let model = provider.models()[0].clone();
    let models = single_route_collection(Box::new(provider));
    let cancel = tokio_util::sync::CancellationToken::new();
    let request = Request {
        model: "probe:mock-model".into(),
        context: normalize_context(Default::default()).expect("empty context normalizes"),
        projection: Default::default(),
        transport: Default::default(),
        max_tokens: None,
        temperature: None,
        sampling_params: None,
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel,
        timeout: None,
        extra_headers: vec![],
        cache_retention: Default::default(),
        session_id: None,
        telemetry_context: Some(telemetry_context),
        callbacks: Default::default(),
    };

    let mut call = models.stream_simple(
        ChatModelRecord::new("probe", model),
        request,
        SimpleOptions::default(),
    );
    for _ in 0..100 {
        if !calls.lock().expect("mock call log lock").is_empty() {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(
        !calls.lock().expect("mock call log lock").is_empty(),
        "ordinary call reached the provider before explicit abort"
    );

    let failure = call
        .abort()
        .await
        .expect_err("explicit abort fails the ordinary call");
    assert_eq!(failure.message().stop_reason, StopReason::Aborted);

    let spans = recorder.spans();
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].name, "pi.ai.request");
    assert_eq!(
        spans[0].attributes.get("stop"),
        Some(&AttributeValue::String("aborted".into()))
    );
    assert!(spans[0].end_sequence.is_some());
}

#[tokio::test]
async fn ordinary_models_request_token_cancel_finishes_request_telemetry() {
    let recorder = Recorder::default();
    let telemetry_context = recorder.context();
    let provider =
        MockProvider::new_with_errors("probe", vec![MockResponse::HangingUntilCancelled]);
    let calls = provider.call_log_handle();
    let model = provider.models()[0].clone();
    let models = single_route_collection(Box::new(provider));
    let cancel = tokio_util::sync::CancellationToken::new();
    let request = Request {
        model: "probe:mock-model".into(),
        context: normalize_context(Default::default()).expect("empty context normalizes"),
        projection: Default::default(),
        transport: Default::default(),
        max_tokens: None,
        temperature: None,
        sampling_params: None,
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: cancel.clone(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: Default::default(),
        session_id: None,
        telemetry_context: Some(telemetry_context),
        callbacks: Default::default(),
    };

    let call = models.stream_simple(
        ChatModelRecord::new("probe", model),
        request,
        SimpleOptions::default(),
    );
    for _ in 0..100 {
        if !calls.lock().expect("mock call log lock").is_empty() {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(
        !calls.lock().expect("mock call log lock").is_empty(),
        "ordinary call reached the provider before cooperative cancellation"
    );

    cancel.cancel();
    let failure = call
        .result()
        .await
        .expect_err("request-token cancellation fails the ordinary call");
    assert_eq!(failure.message().stop_reason, StopReason::Aborted);
    assert_eq!(failure.cause(), &ProviderError::Cancelled);

    let spans = recorder.spans();
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].name, "pi.ai.request");
    assert_eq!(
        spans[0].attributes.get("stop"),
        Some(&AttributeValue::String("aborted".into()))
    );
    assert!(spans[0].end_sequence.is_some());
}

#[tokio::test]
async fn public_chat_stream_telemetry_constructor_records_request() {
    let recorder = Recorder::default();
    let context = recorder.context();
    let mut start = base_assistant();
    start.provider = "probe".into();
    start.model = "mock-model".into();
    let mut message = start.clone();
    message.stop_reason = StopReason::Stop;

    let source: EventStream = Box::pin(stream::iter([
        Ok(AssistantStreamEvent::Start {
            partial: start.clone(),
        }),
        Ok(AssistantStreamEvent::Done {
            reason: StopReason::Stop,
            message,
        }),
    ]));
    let result = ChatStream::new_with_telemetry(
        source,
        Some(&context),
        AiRequestAttributes::new("probe", "mock-model", "openai-completions"),
    )
    .result()
    .await
    .expect("public observed ChatStream completes");
    assert_eq!(result.stop_reason, StopReason::Stop);

    let spans = recorder.spans();
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].name, "pi.ai.request");
    assert_eq!(
        spans[0].attributes.get("provider"),
        Some(&AttributeValue::String("probe".into()))
    );
    assert_eq!(
        spans[0].attributes.get("model"),
        Some(&AttributeValue::String("mock-model".into()))
    );
    assert_eq!(
        spans[0].attributes.get("stop"),
        Some(&AttributeValue::String("stop".into()))
    );
}

#[tokio::test]
async fn started_observed_chat_stream_finishes_request_telemetry() {
    let recorder = Recorder::default();
    let context = recorder.context();
    let mut start = base_assistant();
    start.provider = "probe".into();
    start.model = "mock-model".into();
    let mut message = start.clone();
    message.stop_reason = StopReason::Stop;

    let source: EventStream = Box::pin(stream::iter([
        Ok(AssistantStreamEvent::Start { partial: start }),
        Ok(AssistantStreamEvent::Done {
            reason: StopReason::Stop,
            message,
        }),
    ]));
    let result = ChatStream::new_with_telemetry(
        source,
        Some(&context),
        AiRequestAttributes::new("probe", "mock-model", "openai-completions"),
    )
    .start()
    .result()
    .await
    .expect("started observed ChatStream completes");
    assert_eq!(result.stop_reason, StopReason::Stop);

    let spans = recorder.spans();
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].name, "pi.ai.request");
    assert_eq!(
        spans[0].attributes.get("stop"),
        Some(&AttributeValue::String("stop".into()))
    );
    assert!(spans[0].end_sequence.is_some());
}

#[tokio::test]
async fn started_observed_chat_stream_abort_finishes_request_telemetry() {
    use futures_util::StreamExt as _;

    let recorder = Recorder::default();
    let context = recorder.context();
    let mut start = base_assistant();
    start.provider = "probe".into();
    start.model = "mock-model".into();
    let source: EventStream = Box::pin(
        futures_util::stream::iter([Ok(AssistantStreamEvent::Start { partial: start })]).chain(
            futures_util::stream::pending::<
                Result<AssistantStreamEvent, opi_ai::provider::ProviderError>,
            >(),
        ),
    );
    let mut started = ChatStream::new_with_telemetry(
        source,
        Some(&context),
        AiRequestAttributes::new("probe", "mock-model", "openai-completions"),
    )
    .start();
    assert!(started.next().await.expect("partial event").is_ok());

    let failure = started
        .abort()
        .await
        .expect_err("explicit abort fails the call");
    assert_eq!(failure.message().stop_reason, StopReason::Aborted);

    let spans = recorder.spans();
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].name, "pi.ai.request");
    assert_eq!(
        spans[0].attributes.get("stop"),
        Some(&AttributeValue::String("aborted".into()))
    );
    assert!(spans[0].end_sequence.is_some());
}

#[tokio::test]
async fn already_terminal_observed_chat_stream_finishes_before_start() {
    use futures_util::StreamExt as _;

    let recorder = Recorder::default();
    let context = recorder.context();
    let mut start = base_assistant();
    start.provider = "probe".into();
    start.model = "mock-model".into();
    let mut message = start.clone();
    message.stop_reason = StopReason::Stop;

    let source: EventStream = Box::pin(stream::iter([
        Ok(AssistantStreamEvent::Start { partial: start }),
        Ok(AssistantStreamEvent::Done {
            reason: StopReason::Stop,
            message,
        }),
    ]));
    let mut observed = ChatStream::new_with_telemetry(
        source,
        Some(&context),
        AiRequestAttributes::new("probe", "mock-model", "openai-completions"),
    );
    assert!(observed.next().await.expect("start event").is_ok());
    assert!(observed.next().await.expect("terminal event").is_ok());

    let result = observed
        .start()
        .result()
        .await
        .expect("already-terminal observed ChatStream completes");
    assert_eq!(result.stop_reason, StopReason::Stop);
    let spans = recorder.spans();
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].name, "pi.ai.request");
    assert_eq!(
        spans[0].attributes.get("stop"),
        Some(&AttributeValue::String("stop".into()))
    );
    assert!(spans[0].end_sequence.is_some());
}

#[tokio::test]
async fn deferred_fetch_auto_records_ai_poll() {
    let recorder = Recorder::default();
    let telemetry_context = recorder.context();
    let backend = Arc::new(ScriptedDeferredBackend::new("probe", "mock-model"));
    let provider = MockProvider::new("probe", vec![text_response("unused")]);
    let mut models = single_route_collection(Box::new(provider));
    models
        .register_deferred_api(
            backend.clone(),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                SecretString::from("test-key"),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register deferred backend");

    let handle = backend.submit(text_response("deferred result"), 0, None);
    let message = models
        .fetch_deferred(
            &handle,
            DeferredFetchCall {
                telemetry_context: Some(telemetry_context),
                ..Default::default()
            },
        )
        .await
        .expect("deferred fetch succeeds");
    assert_eq!(message.stop_reason, StopReason::Stop);

    let streamed = models
        .stream_deferred(
            &handle,
            DeferredFetchCall {
                telemetry_context: Some(recorder.context()),
                ..Default::default()
            },
        )
        .result()
        .await
        .expect("deferred ChatStream succeeds");
    assert_eq!(streamed.stop_reason, StopReason::Stop);

    let spans = recorder.spans();
    assert_eq!(spans.len(), 2);
    assert!(spans.iter().all(|span| span.name == "pi.ai.poll"));
    assert_eq!(
        spans[0].attributes.get("provider"),
        Some(&AttributeValue::String("probe".into()))
    );
    assert_eq!(
        spans[0].attributes.get("model"),
        Some(&AttributeValue::String("mock-model".into()))
    );
    assert_eq!(
        spans[0].attributes.get("api"),
        Some(&AttributeValue::String("scripted-deferred".into()))
    );
    assert_eq!(
        spans[0].attributes.get("stop"),
        Some(&AttributeValue::String("stop".into()))
    );
}

#[tokio::test]
async fn deferred_cancel_does_not_create_ai_poll() {
    let recorder = Recorder::default();
    let backend = Arc::new(ScriptedDeferredBackend::new("probe", "mock-model"));
    let provider = MockProvider::new("probe", vec![text_response("unused")]);
    let mut models = single_route_collection(Box::new(provider));
    models
        .register_deferred_api(
            backend.clone(),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                SecretString::from("test-key"),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register deferred backend");

    let handle = backend.submit(text_response("deferred result"), 0, None);
    models
        .cancel_deferred(
            &handle,
            DeferredCancelCall {
                telemetry_context: Some(recorder.context()),
                ..Default::default()
            },
        )
        .await
        .expect("deferred cancel succeeds");

    assert!(
        recorder.spans().is_empty(),
        "remote deferred cancellation carries telemetry context but does not install a pi.ai span"
    );
}

#[tokio::test]
async fn prepared_attempt_observed_helper_records_ai_request() {
    let recorder = Recorder::default();
    let telemetry_context = recorder.context();
    let mut start = base_assistant();
    start.provider = "probe".into();
    start.model = "mock-model".into();
    let mut message = start.clone();
    message
        .content
        .push(opi_ai::message::AssistantContent::Text {
            text: "prepared".into(),
            signature: None,
        });
    let provider = MockProvider::new(
        "probe",
        vec![vec![
            AssistantStreamEvent::Start { partial: start },
            AssistantStreamEvent::Done {
                reason: StopReason::Stop,
                message,
            },
        ]],
    );
    let models = single_route_collection(Box::new(provider));
    let request = Request {
        model: "probe:mock-model".into(),
        context: normalize_context(Default::default()).expect("empty context normalizes"),
        projection: Default::default(),
        transport: Default::default(),
        max_tokens: None,
        temperature: None,
        sampling_params: None,
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: Default::default(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: Default::default(),
        session_id: None,
        telemetry_context: Some(telemetry_context),
        callbacks: Default::default(),
    };

    let prepared = models
        .prepare_call("probe:mock-model", request)
        .await
        .expect("prepared call resolves");
    let result = prepared
        .start_attempt_with_telemetry()
        .expect("observed attempt starts")
        .result()
        .await
        .expect("prepared attempt completes");
    assert_eq!(result.stop_reason, StopReason::Stop);

    let spans = recorder.spans();
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].name, "pi.ai.request");
    assert_eq!(
        spans[0].attributes.get("provider"),
        Some(&AttributeValue::String("probe".into()))
    );
    assert_eq!(
        spans[0].attributes.get("model"),
        Some(&AttributeValue::String("mock-model".into()))
    );
    assert_eq!(
        spans[0].attributes.get("api"),
        Some(&AttributeValue::String("openai-completions".into()))
    );
    assert_eq!(
        spans[0].attributes.get("stop"),
        Some(&AttributeValue::String("stop".into()))
    );
}

#[tokio::test]
async fn prepared_observed_attempt_request_token_cancel_finishes_request_telemetry() {
    let recorder = Recorder::default();
    let telemetry_context = recorder.context();
    let provider =
        MockProvider::new_with_errors("probe", vec![MockResponse::HangingUntilCancelled]);
    let calls = provider.call_log_handle();
    let models = single_route_collection(Box::new(provider));
    let cancel = tokio_util::sync::CancellationToken::new();
    let request = Request {
        model: "probe:mock-model".into(),
        context: normalize_context(Default::default()).expect("empty context normalizes"),
        projection: Default::default(),
        transport: Default::default(),
        max_tokens: None,
        temperature: None,
        sampling_params: None,
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: cancel.clone(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: Default::default(),
        session_id: None,
        telemetry_context: Some(telemetry_context),
        callbacks: Default::default(),
    };

    let prepared = models
        .prepare_call("probe:mock-model", request)
        .await
        .expect("prepared call resolves");
    let observed = prepared
        .start_attempt_with_telemetry()
        .expect("observed attempt starts");
    let result_task = tokio::spawn(observed.result());
    for _ in 0..100 {
        if !calls.lock().expect("mock call log lock").is_empty() {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(
        !calls.lock().expect("mock call log lock").is_empty(),
        "prepared observed attempt reached the provider before cooperative cancellation"
    );

    cancel.cancel();
    let failure = result_task
        .await
        .expect("prepared observed result task joins")
        .expect_err("request-token cancellation fails the prepared attempt");
    assert_eq!(failure.message().stop_reason, StopReason::Aborted);
    assert_eq!(failure.cause(), &ProviderError::Cancelled);

    let spans = recorder.spans();
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].name, "pi.ai.request");
    assert_eq!(
        spans[0].attributes.get("stop"),
        Some(&AttributeValue::String("aborted".into()))
    );
    assert!(spans[0].end_sequence.is_some());
}

#[tokio::test]
async fn prepared_attempt_observation_is_per_attempt_without_cross_attempt_aggregation() {
    let recorder = Recorder::default();
    let telemetry_context = recorder.context();
    let mut start = base_assistant();
    start.provider = "probe".into();
    start.model = "mock-model".into();
    let mut message = start.clone();
    message.stop_reason = StopReason::Stop;
    message
        .content
        .push(opi_ai::message::AssistantContent::Text {
            text: "retry-success".into(),
            signature: None,
        });

    let provider = MockProvider::new_with_errors(
        "probe",
        vec![MockResponse::DelayedAfter {
            pending_attempts: 1,
            retry_after_ms: Some(0),
            events: vec![
                AssistantStreamEvent::Start {
                    partial: start.clone(),
                },
                AssistantStreamEvent::Done {
                    reason: StopReason::Stop,
                    message: message.clone(),
                },
            ],
        }],
    );
    let models = single_route_collection(Box::new(provider));
    let request = Request {
        model: "probe:mock-model".into(),
        context: normalize_context(Default::default()).expect("empty context normalizes"),
        projection: Default::default(),
        transport: Default::default(),
        max_tokens: None,
        temperature: None,
        sampling_params: None,
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: Default::default(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: Default::default(),
        session_id: None,
        telemetry_context: Some(telemetry_context),
        callbacks: Default::default(),
    };

    let prepared = models
        .prepare_call_with_policy(
            "probe:mock-model",
            request,
            AcquisitionPolicy::with_max_retries(1),
        )
        .await
        .expect("prepared call resolves");
    let first = prepared
        .start_attempt_with_telemetry()
        .expect("first observed attempt starts")
        .result()
        .await
        .expect_err("first attempt is the scripted retryable failure");
    assert_eq!(first.message().stop_reason, StopReason::Error);
    assert_eq!(
        prepared
            .retry_admission()
            .expect("retry is admitted")
            .retry_index,
        1
    );

    let result = prepared
        .start_attempt_with_telemetry()
        .expect("second observed attempt starts")
        .result()
        .await
        .expect("retry succeeds");
    assert_eq!(result.stop_reason, StopReason::Stop);

    let spans = recorder.spans();
    assert_eq!(
        spans.len(),
        2,
        "each prepared attempt owns one request span"
    );
    assert!(spans.iter().all(|span| span.name == "pi.ai.request"));
    assert_eq!(
        spans[0].attributes.get("stop"),
        Some(&AttributeValue::String("error".into()))
    );
    assert_eq!(
        spans[1].attributes.get("stop"),
        Some(&AttributeValue::String("stop".into()))
    );
    assert!(spans.iter().all(|span| span.end_sequence.is_some()));
}

#[test]
fn records_request_events_and_result_after_the_terminal_payload() {
    let recorder = Recorder::default();
    let context = recorder.context();
    let mut telemetry = AiRequestTelemetry::start(
        Some(&context),
        AiRequestAttributes::new("probe", "model", "api").with_pricing(opi_ai::stream::Pricing {
            input_cost_per_mtok: 1_000_000.0,
            output_cost_per_mtok: 2_000_000.0,
            cache_read_cost_per_mtok: 3_000_000.0,
            cache_write_cost_per_mtok: 4_000_000.0,
        }),
    )
    .expect("telemetry starts")
    .expect("explicit context enables observation");

    let mut partial = base_assistant();
    partial.provider = "probe".into();
    partial.model = "model".into();
    let events = [
        AssistantStreamEvent::Start {
            partial: partial.clone(),
        },
        AssistantStreamEvent::TextEnd {
            content_index: 0,
            content: "A😀".into(),
            partial: partial.clone(),
        },
        AssistantStreamEvent::ToolCallDelta {
            content_index: 1,
            delta: "😀".into(),
            partial: partial.clone(),
        },
        AssistantStreamEvent::ToolCallEnd {
            content_index: 1,
            tool_call: ToolCall {
                id: "call-1".into(),
                name: "sample".into(),
                arguments: "{}".into(),
                arguments_error: None,
                signature: None,
            },
            partial: partial.clone(),
        },
    ];
    for event in &events {
        telemetry.on_event(event).expect("event observation");
    }

    partial.stop_reason = StopReason::Stop;
    partial.response_id = Some("response-1".into());
    partial.usage = opi_ai::stream::Usage::reported(1, 2, 3, 4, None, None);
    telemetry
        .finish(Some(&partial))
        .expect("result observation");

    let spans = recorder.spans();
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].name, "pi.ai.request");
    assert_eq!(spans[0].status, SpanStatus::Ok);
    assert!(spans[0].end_sequence.is_some());
    assert_eq!(spans[0].events.len(), 3);
    assert_eq!(spans[0].events[0].name, AI_FIRST_CHUNK);
    assert_eq!(spans[0].events[1].name, AI_BLOCK);
    assert_eq!(
        spans[0].events[1].attributes.get("chars"),
        Some(&AttributeValue::Number(3.0))
    );
    assert_eq!(spans[0].events[2].name, AI_BLOCK);
    assert_eq!(
        spans[0].events[2].attributes.get("chars"),
        Some(&AttributeValue::Number(2.0))
    );
    assert_eq!(
        spans[0].attributes.get("stop"),
        Some(&AttributeValue::String("stop".into()))
    );
    assert_eq!(
        spans[0].attributes.get("cost.total"),
        Some(&AttributeValue::Number(30.0))
    );

    let measurements = recorder.measurements();
    assert_eq!(measurements.len(), 7);
    assert_eq!(
        measurements[0].definition.name,
        "pi.ai.first_chunk.duration"
    );
    assert_eq!(measurements[1].definition.name, "pi.ai.request.duration");
    assert_eq!(
        measurements
            .iter()
            .filter(|measurement| measurement.definition.name == "pi.ai.tokens")
            .count(),
        4
    );
    assert_eq!(
        measurements
            .iter()
            .find(|measurement| measurement.definition.name == "pi.ai.cost")
            .map(|measurement| measurement.value),
        Some(30.0)
    );
}

#[test]
fn pending_poll_does_not_record_token_or_cost_counters() {
    let recorder = Recorder::default();
    let context = recorder.context();
    let mut telemetry = AiRequestTelemetry::start(
        Some(&context),
        AiRequestAttributes::new("probe", "model", "api").as_poll(true),
    )
    .expect("telemetry starts")
    .expect("explicit context enables observation");
    let mut message = base_assistant();
    message.stop_reason = StopReason::Deferred;
    message.usage = opi_ai::stream::Usage::reported(1, 2, 3, 4, None, None);
    telemetry
        .finish(Some(&message))
        .expect("result observation");

    let measurements = recorder.measurements();
    assert_eq!(measurements.len(), 1);
    assert_eq!(measurements[0].definition.name, "pi.ai.request.duration");
}

#[test]
fn schema_keeps_ai_names_and_aborted_success_status_policy() {
    let schema = ai_telemetry_schema();
    assert_eq!(schema.version, 1);
    assert!(schema.spans.contains_key("pi.ai.request"));
    assert!(schema.spans.contains_key("pi.ai.poll"));
    assert!(
        schema
            .metrics
            .as_ref()
            .unwrap()
            .contains_key("pi.ai.tokens")
    );
    assert_eq!(
        schema.spans["pi.ai.request"].status.default,
        opi_telemetry::schema::DefaultStatus::Ok
    );
    assert!(
        schema.spans["pi.ai.request"]
            .status
            .error_when
            .contains("aborted stays ok")
    );
}

#[tokio::test]
async fn chat_result_drains_source_after_terminal_delivery() {
    let tail_polled = Arc::new(AtomicBool::new(false));
    let tail_polled_by_source = tail_polled.clone();
    let mut partial = base_assistant();
    partial.stop_reason = StopReason::Stop;
    let terminal = AssistantStreamEvent::Done {
        reason: StopReason::Stop,
        message: partial.clone(),
    };
    let mut step = 0;
    let source: EventStream = Box::pin(futures_util::stream::poll_fn(move |_cx| {
        let item = match step {
            0 => Some(Ok(AssistantStreamEvent::Start {
                partial: partial.clone(),
            })),
            1 => Some(Ok(terminal.clone())),
            2 => {
                tail_polled_by_source.store(true, Ordering::SeqCst);
                step = 3;
                return std::task::Poll::Ready(None);
            }
            _ => return std::task::Poll::Pending,
        };
        step += 1;
        std::task::Poll::Ready(item)
    }));

    let result = opi_ai::stream::ChatStream::new(source)
        .result()
        .await
        .expect("terminal result");
    assert_eq!(result.stop_reason, StopReason::Stop);
    assert!(tail_polled.load(Ordering::SeqCst));
}
