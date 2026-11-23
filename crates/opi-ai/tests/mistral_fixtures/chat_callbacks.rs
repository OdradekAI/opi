//! Mistral callback assertions on the shared Chat adapter.

use super::*;

// Task 20.51: the Mistral profile carries callbacks through the Chat adapter
// ---------------------------------------------------------------------------

use opi_ai::callbacks::RequestCallbacks;

#[tokio::test]
async fn mistral_profile_carries_callbacks_through_chat_adapter() {
    let order: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let raw_recorder = order.clone();
    let response_recorder = order.clone();
    let sse = "data: {\"choices\":[{\"delta\":{\"content\":\"Hi\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n";
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(wiremock::matchers::body_partial_json(
            serde_json::json!({"mistral_marker": "replaced"}),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = opi_ai::providers::mistral::mistral_provider(Some(server.uri()));
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
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: RequestCallbacks::new()
            .with_on_payload(std::sync::Arc::new(|body| {
                let mut replacement = body.clone();
                replacement["mistral_marker"] = serde_json::json!("replaced");
                Ok(Some(replacement))
            }))
            .with_on_response(std::sync::Arc::new(move |status, _headers| {
                response_recorder
                    .lock()
                    .unwrap()
                    .push(format!("response:{status}"));
                Ok(())
            }))
            .with_on_raw(std::sync::Arc::new(move |event| {
                raw_recorder
                    .lock()
                    .unwrap()
                    .push(format!("raw:{}", event["object"].as_str().unwrap_or("?")));
                Ok(())
            })),
    };

    let events = collect_stream(
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth()),
    )
    .await;
    assert!(
        events
            .iter()
            .any(|event| matches!(event, AssistantStreamEvent::Done { .. }))
    );
    let order = order.lock().unwrap().clone();
    assert_eq!(order.first().map(String::as_str), Some("response:200"));
    assert!(
        order.iter().any(|m| m.starts_with("raw:")),
        "profile providers share the Chat adapter's callback seams"
    );
    server.verify().await;
}

// ---------------------------------------------------------------------------
