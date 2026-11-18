use futures_util::StreamExt;
use opi_ai::callbacks::RequestCallbacks;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{CacheRetention, Provider, ProviderError, Request, ThinkingConfig};
use opi_ai::providers::gemini::GeminiProvider;
use opi_ai::stream::AssistantStreamEvent;
use serde_json::Value;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

fn fixture_body() -> (String, Value) {
    let fixture: Value = serde_json::from_str(include_str!("data/gemini-repeated-finish.json"))
        .expect("valid repeated Gemini finish fixture");
    let body = fixture["chunks"]
        .as_array()
        .expect("fixture chunks")
        .iter()
        .map(|chunk| format!("data: {chunk}\n\n"))
        .collect();
    (body, fixture)
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

#[tokio::test]
async fn repeated_finish_usage_is_deferred_until_eof() {
    let (body, fixture) = fixture_body();
    let provider = GeminiProvider::new(None);
    let mut cases = Vec::new();
    for route in ["simple", "direct"] {
        let events: Vec<_> = provider
            .stream_from_sse(&body, CancellationToken::new())
            .collect()
            .await;
        let done = events
            .iter()
            .find_map(|event| match event {
                Ok(AssistantStreamEvent::Done { message, .. }) => Some(message),
                _ => None,
            })
            .expect("one Done after EOF");
        assert_eq!(done.stop_reason, opi_ai::stream::StopReason::Stop);
        assert_eq!(done.raw_stop_reason.as_deref(), Some("STOP"));
        assert_eq!(done.usage.input_tokens, fixture["expected"]["input"]);
        assert_eq!(done.usage.output_tokens, fixture["expected"]["output"]);
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
            "usage": {"input": done.usage.input_tokens, "output": done.usage.output_tokens},
            "done_count": events.iter().filter(|event| matches!(event, Ok(AssistantStreamEvent::Done { .. }))).count(),
            "event_types": events.iter().filter_map(|event| event.as_ref().ok().map(event_type)).collect::<Vec<_>>(),
            "requests": 1,
        }));
    }
    let trace = serde_json::json!({"case": fixture["id"], "cases": cases});
    println!("{trace}");
}

#[tokio::test]
async fn tail_raw_observer_failure_suppresses_deferred_done() {
    let fixture: Value = serde_json::from_str(include_str!("data/gemini-repeated-finish.json"))
        .expect("valid repeated Gemini finish fixture");
    let body: String = fixture["chunks"]
        .as_array()
        .expect("fixture chunks")
        .iter()
        .map(|chunk| format!("data: {chunk}\n\n"))
        .collect();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(body),
        )
        .mount(&server)
        .await;
    let seen = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observer_seen = seen.clone();
    let request = Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "gemini:gemini-2.5-flash".into(),
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
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: RequestCallbacks::new().with_on_raw(Arc::new(move |_event| {
            let index = observer_seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            if index == 3 {
                Err(opi_ai::callbacks::CallbackError::new(
                    "fixture Gemini tail observer failure",
                ))
            } else {
                Ok(())
            }
        })),
    };
    let provider = GeminiProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    let mut outcomes = Vec::new();
    while let Some(item) = stream.next().await {
        outcomes.push(item);
    }
    assert_eq!(seen.load(std::sync::atomic::Ordering::SeqCst), 3);
    assert!(
        !outcomes
            .iter()
            .any(|item| matches!(item, Ok(AssistantStreamEvent::Done { .. })))
    );
    let error = outcomes
        .into_iter()
        .find_map(Result::err)
        .expect("tail observer failure must be surfaced");
    let cause = match &error {
        ProviderError::StreamFailure(failure) => failure.cause(),
        other => other,
    };
    assert!(
        matches!(cause, ProviderError::CallbackPhase(_)),
        "{error:?}"
    );
    server.verify().await;
}

#[tokio::test]
async fn malformed_tail_after_finish_suppresses_deferred_done() {
    let sse = concat!(
        "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"hello\"}]}}]}\n\n",
        "data: {\"candidates\":[{\"finishReason\":\"STOP\"}],\"usageMetadata\":{\"promptTokenCount\":10,\"candidatesTokenCount\":7}}\n\n",
        "data: {not-json}\n\n",
    );
    let events: Vec<_> = GeminiProvider::new(None)
        .stream_from_sse(sse, CancellationToken::new())
        .collect()
        .await;
    assert!(events.iter().any(Result::is_err));
    assert!(
        !events
            .iter()
            .any(|item| matches!(item, Ok(AssistantStreamEvent::Done { .. })))
    );
}
