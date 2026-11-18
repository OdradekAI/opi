use futures_util::StreamExt;
use opi_ai::api::pi_messages::PiMessagesOptions;
use opi_ai::callbacks::RequestCallbacks;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::model_info::WireApi;
use opi_ai::provider::{CacheRetention, Provider, ProviderError, Request};
use opi_ai::providers::pi_messages::PiMessagesProvider;
use opi_ai::stream::{AssistantStreamEvent, StopReason};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const MODEL: &str = "claude-sonnet-4-5";

fn fixture() -> (String, Value) {
    let fixture: Value = serde_json::from_str(include_str!("data/pi-messages-terminal.json"))
        .expect("valid pi-messages terminal fixture");
    let body = fixture["events"]
        .as_array()
        .expect("fixture events")
        .iter()
        .map(|event| format!("data: {event}\n\n"))
        .collect::<String>();
    (body, fixture)
}

fn model() -> opi_ai::provider::ModelInfo {
    opi_ai::provider::ModelInfo::new(
        MODEL,
        MODEL,
        WireApi::PiMessages,
        opi_ai::registry::ModelCapabilities::new(200_000, 16_384).with_streaming(true),
    )
}

fn request(prompt: &str) -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: format!("radius:{MODEL}"),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: prompt.to_owned(),
                }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .expect("valid context"),
        max_tokens: Some(256),
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
        AssistantStreamEvent::Done { .. } => "done",
        _ => "other",
    }
}

fn raw_type(event: &Value) -> Value {
    event["type"].clone()
}

#[tokio::test]
async fn terminal_metadata_and_repeated_done_trace() {
    let (body, fixture) = fixture();
    let mut cases = Vec::new();
    for route in ["simple", "direct"] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(body.clone())
                    .insert_header("content-type", "text/event-stream"),
            )
            .expect(1)
            .mount(&server)
            .await;
        let seen = Arc::new(Mutex::new(Vec::new()));
        let observed = seen.clone();
        let mut req = request("hello");
        req.callbacks = RequestCallbacks::new().with_on_raw(Arc::new(move |event| {
            observed.lock().unwrap().push(raw_type(event));
            Ok(())
        }));
        let provider = PiMessagesProvider::new(Some(server.uri()))
            .with_models(vec![model()])
            .with_native_options(PiMessagesOptions::native(None, None, false));
        let stream = if route == "simple" {
            provider.stream_simple(req, opi_ai::test_support::resolved_bearer_auth())
        } else {
            provider.stream_prepared(req, opi_ai::test_support::resolved_bearer_auth())
        };
        let events: Vec<_> = stream.collect().await;
        let done = events
            .iter()
            .find_map(|event| match event {
                Ok(AssistantStreamEvent::Done { message, .. }) => Some(message),
                _ => None,
            })
            .expect("one Done");
        assert_eq!(done.stop_reason, StopReason::Stop);
        assert_eq!(done.response_model.as_deref(), Some("radius-final"));
        assert_eq!(done.usage.input_tokens, fixture["expected"]["input"]);
        assert_eq!(done.usage.output_tokens, fixture["expected"]["output"]);
        assert_eq!(done.usage.total_tokens, Some(15));
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, Ok(AssistantStreamEvent::Done { .. })))
                .count(),
            1
        );
        let raw_events = seen.lock().unwrap().clone();
        assert_eq!(
            raw_events,
            fixture["expected"]["raw_events"]
                .as_array()
                .cloned()
                .expect("expected raw events")
        );
        cases.push(serde_json::json!({
            "route": route,
            "stop": done.stop_reason,
            "response_model": done.response_model,
            "usage": {"input": done.usage.input_tokens, "output": done.usage.output_tokens, "total": done.usage.total_tokens},
            "done_count": events.iter().filter(|event| matches!(event, Ok(AssistantStreamEvent::Done { .. }))).count(),
            "event_types": events.iter().filter_map(|event| event.as_ref().ok().map(event_type)).collect::<Vec<_>>(),
            "raw_events": raw_events,
            "requests": server.received_requests().await.unwrap().len(),
        }));
        server.verify().await;
    }
    let mut simple = cases[0].clone();
    let mut direct = cases[1].clone();
    simple.as_object_mut().unwrap().remove("route");
    direct.as_object_mut().unwrap().remove("route");
    assert_eq!(simple, direct);
    println!(
        "{}",
        serde_json::json!({"case": fixture["id"], "cases": cases})
    );
}

#[tokio::test]
async fn terminal_raw_observer_failure_releases_for_next_call() {
    let (body, _) = fixture();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(body)
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(2)
        .mount(&server)
        .await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model()])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let mut failing = request("first");
    failing.callbacks = RequestCallbacks::new().with_on_raw(Arc::new(|event| {
        if event["type"] == "done" {
            Err(opi_ai::callbacks::CallbackError::new(
                "terminal raw observer failed",
            ))
        } else {
            Ok(())
        }
    }));
    let failed = provider
        .stream_prepared(failing, opi_ai::test_support::resolved_bearer_auth())
        .collect::<Vec<_>>()
        .await;
    assert!(failed.iter().any(|item| {
        matches!(item, Err(ProviderError::StreamFailure(failure)) if matches!(failure.cause(), ProviderError::CallbackPhase(_)))
    }));
    assert!(
        !failed
            .iter()
            .any(|item| matches!(item, Ok(AssistantStreamEvent::Done { .. })))
    );

    let success = provider
        .stream_prepared(
            request("second"),
            opi_ai::test_support::resolved_bearer_auth(),
        )
        .collect::<Vec<_>>()
        .await;
    assert!(
        success
            .iter()
            .any(|item| matches!(item, Ok(AssistantStreamEvent::Done { .. })))
    );
    server.verify().await;
}
