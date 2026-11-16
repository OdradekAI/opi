use futures_util::StreamExt;
use opi_ai::callbacks::RequestCallbacks;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::mistral::{MistralConversationsProvider, MistralOptions};
use opi_ai::model_info::WireApi;
use opi_ai::provider::{CacheRetention, EventStream, Provider, ProviderError, Request};
use opi_ai::registry::ModelCapabilities;
use opi_ai::stream::{AssistantStreamEvent, StopReason};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const MODEL: &str = "mistral-small-latest";

fn fixture() -> (String, Value) {
    let fixture: Value = serde_json::from_str(include_str!("data/mistral-repeated-finish.json"))
        .expect("valid Mistral repeated-finish fixture");
    let body = fixture["chunks"]
        .as_array()
        .expect("fixture chunks")
        .iter()
        .map(|chunk| format!("data: {chunk}\n\n"))
        .collect::<String>()
        + "data: [DONE]\n\n";
    (body, fixture)
}

fn model() -> opi_ai::provider::ModelInfo {
    opi_ai::provider::ModelInfo::new(
        MODEL,
        MODEL,
        WireApi::MistralConversations,
        ModelCapabilities::new(32_000, 8_192).with_streaming(true),
    )
}

fn request() -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: format!("mistral:{MODEL}"),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "hello".into(),
                }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: Some(4096),
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
        callbacks: RequestCallbacks::default(),
    }
}

fn event_type(event: &AssistantStreamEvent) -> &'static str {
    match event {
        AssistantStreamEvent::Start { .. } => "start",
        AssistantStreamEvent::TextStart { .. } => "text_start",
        AssistantStreamEvent::TextDelta { .. } => "text_delta",
        AssistantStreamEvent::TextEnd { .. } => "text_end",
        AssistantStreamEvent::ThinkingStart { .. } => "thinking_start",
        AssistantStreamEvent::ThinkingDelta { .. } => "thinking_delta",
        AssistantStreamEvent::ThinkingEnd { .. } => "thinking_end",
        AssistantStreamEvent::ToolCallStart { .. } => "toolcall_start",
        AssistantStreamEvent::ToolCallDelta { .. } => "toolcall_delta",
        AssistantStreamEvent::ToolCallEnd { .. } => "toolcall_end",
        AssistantStreamEvent::Done { .. } => "done",
        _ => "unknown",
    }
}

fn provider_event(value: &Value) -> Value {
    let choice = value["choices"]
        .as_array()
        .and_then(|choices| choices.first());
    serde_json::json!({
        "finish": choice.and_then(|choice| choice["finish_reason"].as_str()),
        "output": value["usage"]["completion_tokens"].as_u64(),
    })
}

async fn run_route(
    route: &str,
    body: &str,
) -> (
    Vec<Result<AssistantStreamEvent, ProviderError>>,
    Vec<Value>,
    usize,
) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(body)
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let seen = Arc::new(Mutex::new(Vec::new()));
    let observed = seen.clone();
    let mut req = request();
    req.callbacks = RequestCallbacks::new().with_async_on_raw(move |value| {
        let observed = observed.clone();
        async move {
            observed.lock().unwrap().push(provider_event(&value));
            Ok(())
        }
    });
    let provider = MistralConversationsProvider::new(Some(server.uri()))
        .with_models(vec![model()])
        .with_native_options(MistralOptions::default());
    let stream: EventStream = if route == "simple" {
        provider.stream_simple(req, opi_ai::test_support::resolved_bearer_auth())
    } else {
        provider.stream_prepared(req, opi_ai::test_support::resolved_bearer_auth())
    };
    let events = stream.collect().await;
    let raw = seen.lock().unwrap().clone();
    let requests = server.received_requests().await.unwrap().len();
    server.verify().await;
    (events, raw, requests)
}

#[tokio::test]
async fn repeated_finish_usage_is_deferred_until_eof() {
    let (body, fixture) = fixture();
    let mut cases = Vec::new();
    for route in ["simple", "direct"] {
        let (events, raw, requests) = run_route(route, &body).await;
        let done = events
            .iter()
            .find_map(|event| match event {
                Ok(AssistantStreamEvent::Done { message, .. }) => Some(message),
                _ => None,
            })
            .expect("one final Done");
        assert_eq!(done.stop_reason, StopReason::Stop);
        assert_eq!(done.raw_stop_reason.as_deref(), Some("stop"));
        assert_eq!(done.usage.input_tokens, fixture["expected"]["input"]);
        assert_eq!(done.usage.output_tokens, fixture["expected"]["output"]);
        assert_eq!(done.usage.total_tokens, Some(19));
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, Ok(AssistantStreamEvent::Done { .. })))
                .count(),
            1
        );
        cases.push(serde_json::json!({
            "route": route,
            "stop": done.stop_reason,
            "raw_stop": done.raw_stop_reason,
            "usage": {"input": done.usage.input_tokens, "output": done.usage.output_tokens, "total": done.usage.total_tokens},
            "done_count": events.iter().filter(|event| matches!(event, Ok(AssistantStreamEvent::Done { .. }))).count(),
            "event_types": events.iter().filter_map(|event| event.as_ref().ok().map(event_type)).collect::<Vec<_>>(),
            "provider_events": raw,
            "requests": requests,
        }));
    }
    let expected_events = fixture["chunks"]
        .as_array()
        .unwrap()
        .iter()
        .map(provider_event)
        .collect::<Vec<_>>();
    let expected_events = serde_json::Value::Array(expected_events);
    assert!(
        cases
            .iter()
            .all(|case| case["provider_events"] == expected_events)
    );
    println!(
        "{}",
        serde_json::json!({"case": fixture["id"], "cases": cases})
    );
}

#[tokio::test]
async fn tail_observer_failure_suppresses_done_and_releases_for_next_call() {
    let (body, _) = fixture();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(body.clone())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(2)
        .mount(&server)
        .await;
    let seen = Arc::new(Mutex::new(Vec::new()));
    let observed = seen.clone();
    let mut failing = request();
    failing.callbacks = RequestCallbacks::new().with_async_on_raw(move |value| {
        let observed = observed.clone();
        async move {
            let event = provider_event(&value);
            let fail = event["output"] == 9;
            observed.lock().unwrap().push(event);
            if fail {
                Err(opi_ai::callbacks::CallbackError::new(
                    "Mistral tail observer failed",
                ))
            } else {
                Ok(())
            }
        }
    });
    let provider = MistralConversationsProvider::new(Some(server.uri()))
        .with_models(vec![model()])
        .with_native_options(MistralOptions::default());
    let failed: Vec<_> = provider
        .stream_prepared(failing, opi_ai::test_support::resolved_bearer_auth())
        .collect()
        .await;
    let failure = failed
        .iter()
        .find_map(|item| match item {
            Err(ProviderError::StreamFailure(failure)) => Some(failure),
            _ => None,
        })
        .expect("observer failure is carried by the stream failure");
    assert!(matches!(failure.cause(), ProviderError::CallbackPhase(_)));
    assert!(
        !failed
            .iter()
            .any(|item| matches!(item, Ok(AssistantStreamEvent::Done { .. })))
    );
    assert_eq!(
        failure.message().usage.output_tokens,
        7,
        "the usage-only tail is consumed before the failing final terminal observer"
    );
    assert_eq!(
        seen.lock().unwrap().len(),
        fixture().1["chunks"].as_array().unwrap().len()
    );

    let success: Vec<_> = provider
        .stream_prepared(request(), opi_ai::test_support::resolved_bearer_auth())
        .collect()
        .await;
    assert!(
        success
            .iter()
            .any(|item| matches!(item, Ok(AssistantStreamEvent::Done { .. })))
    );
    server.verify().await;
}

#[tokio::test]
async fn malformed_tail_after_finish_suppresses_done() {
    let server = MockServer::start().await;
    let body = concat!(
        "data: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5}}\n\n",
        "data: {broken\n\n",
    );
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(body)
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let provider = MistralConversationsProvider::new(Some(server.uri()))
        .with_models(vec![model()])
        .with_native_options(MistralOptions::default());
    let events: Vec<_> = provider
        .stream_prepared(request(), opi_ai::test_support::resolved_bearer_auth())
        .collect()
        .await;
    assert!(events.iter().any(|item| item.is_err()));
    assert!(
        !events
            .iter()
            .any(|item| matches!(item, Ok(AssistantStreamEvent::Done { .. })))
    );
    server.verify().await;
}

#[tokio::test]
async fn pending_finish_without_reason_fails_without_done() {
    let server = MockServer::start().await;
    let body = concat!(
        "data: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":7}}\n\n",
        "data: [DONE]\n\n",
    );
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(body)
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let provider = MistralConversationsProvider::new(Some(server.uri()))
        .with_models(vec![model()])
        .with_native_options(MistralOptions::default());
    let events: Vec<_> = provider
        .stream_prepared(request(), opi_ai::test_support::resolved_bearer_auth())
        .collect()
        .await;
    assert!(events.iter().any(|item| item.is_err()));
    assert!(
        !events
            .iter()
            .any(|item| matches!(item, Ok(AssistantStreamEvent::Done { .. })))
    );
    server.verify().await;
}
