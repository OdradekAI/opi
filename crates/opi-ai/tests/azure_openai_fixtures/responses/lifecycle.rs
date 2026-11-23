//! Azure OpenAI Responses transport and execution lifecycle assertions.

use super::*;

#[tokio::test]
async fn azure_responses_text_lifecycle_uses_deployment_url_and_api_key_auth() {
    let server = wiremock::MockServer::start().await;
    mount_azure_responses_mock(&server, azure_responses_sse_text()).await;

    let provider = azure_responses_provider(format!("{}/openai/v1", server.uri()));
    let mut stream = provider.stream_prepared(
        azure_responses_request("my-gpt4o", None),
        azure_api_key_auth(),
    );
    let events = collect_azure_events(&mut stream).await;

    let done = events
        .iter()
        .find_map(|event| match event {
            AssistantStreamEvent::Done { message, .. } => Some(message.clone()),
            _ => None,
        })
        .expect("terminal Done");
    assert_eq!(done.stop_reason, opi_ai::stream::StopReason::Stop);
    assert_eq!(done.usage.input_tokens, 7);
    assert_eq!(done.usage.output_tokens, 3);
    assert!(
        done.duration_ms.is_some(),
        "terminal Done carries the measured duration"
    );

    let sent = &server.received_requests().await.unwrap()[0];
    assert!(
        sent.headers.get("api-key").is_some(),
        "azure responses uses the api-key header"
    );
    assert!(sent.headers.get("authorization").is_none());
    let body = serde_json::from_slice::<serde_json::Value>(&sent.body).unwrap();
    assert_eq!(body["model"], "my-gpt4o", "deployment name rides the body");
    assert_eq!(body["store"], false, "source semantics: store false");
    assert_eq!(body["stream"], true);
    server.verify().await;
}

#[tokio::test]
async fn azure_responses_tail_raw_callback_failure_precedes_pending_done() {
    let server = wiremock::MockServer::start().await;
    let body = concat!(
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_tail\",\"status\":\"in_progress\",\"model\":\"my-gpt4o\",\"output\":[]}}\n\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_tail\",\"status\":\"completed\",\"model\":\"my-gpt4o\",\"output\":[],\"usage\":{\"input_tokens\":10,\"output_tokens\":5}}}\n\n",
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_tail\",\"status\":\"completed\",\"model\":\"my-gpt4o\",\"output\":[]}}\n\n",
    )
    .to_owned();
    mount_azure_responses_mock(&server, body).await;

    let seen = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let callback_seen = seen.clone();
    let mut request = azure_responses_request("my-gpt4o", None);
    request.callbacks = opi_ai::callbacks::RequestCallbacks::new().with_on_raw(
        std::sync::Arc::new(move |_event| {
            let count = callback_seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            if count == 3 {
                Err(opi_ai::callbacks::CallbackError::new(
                    "tail observer failed",
                ))
            } else {
                Ok(())
            }
        }),
    );

    let provider = azure_responses_provider(format!("{}/openai/v1", server.uri()));
    let results: Vec<_> = provider
        .stream_prepared(request, azure_api_key_auth())
        .collect()
        .await;
    assert_eq!(seen.load(std::sync::atomic::Ordering::SeqCst), 3);
    assert!(
        !results
            .iter()
            .any(|result| matches!(result, Ok(AssistantStreamEvent::Done { .. })))
    );
    assert!(results.iter().any(|result| matches!(
        result,
        Err(ProviderError::StreamFailure(failure))
            if matches!(failure.cause(), ProviderError::CallbackPhase(_))
    )));
    server.verify().await;
}

#[tokio::test]
async fn azure_responses_tool_call_lifecycle_reaches_the_wire() {
    let server = wiremock::MockServer::start().await;
    mount_azure_responses_mock(&server, azure_responses_sse_tool()).await;

    let provider = azure_responses_provider(format!("{}/openai/v1", server.uri()));
    let mut request = azure_responses_request("my-gpt4o", None);
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: vec![opi_ai::message::Message::User(
            opi_ai::message::UserMessage {
                content: vec![opi_ai::message::InputContent::Text {
                    text: "What is the weather?".into(),
                }],
                timestamp_ms: 0,
            },
        )],
        tools: vec![opi_ai::message::ToolDef {
            name: "get_weather".into(),
            description: "Get weather".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {"city": {"type": "string"}}
            }),
            constrained_sampling: None,
        }],
    })
    .expect("valid context declarations");
    let mut stream = provider.stream_prepared(request, azure_api_key_auth());
    let events = collect_azure_events(&mut stream).await;

    assert!(
        events
            .iter()
            .any(|event| matches!(event, AssistantStreamEvent::ToolCallStart { .. })),
        "tool call start observed: {events:?}"
    );
    let body = sent_body(&server).await;
    assert_eq!(
        body["tools"][0]["name"], "get_weather",
        "declared tools reach the azure responses body"
    );
    server.verify().await;
}

#[tokio::test]
async fn azure_responses_setup_failure_before_activation_is_typed() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .respond_with(wiremock::ResponseTemplate::new(401).set_body_string("denied"))
        .expect(1)
        .mount(&server)
        .await;

    let provider = azure_responses_provider(format!("{}/openai/v1", server.uri()));
    let mut stream = provider.stream_prepared(
        azure_responses_request("my-gpt4o", None),
        azure_api_key_auth(),
    );
    let items = collect_azure_items(&mut stream).await;
    assert!(
        items
            .iter()
            .any(|item| matches!(item, Err(error) if matches!(direct_error_cause(error), ProviderError::AuthFailed(_)))),
        "401 maps to typed AuthFailed: {items:?}"
    );
    server.verify().await;
}

#[tokio::test]
async fn azure_responses_cancellation_during_body_is_typed_and_prompt() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let stalled = std::sync::Arc::new(tokio::sync::Notify::new());
    let server_stalled = stalled.clone();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept");
        let mut request = Vec::new();
        let mut buf = [0_u8; 1024];
        loop {
            let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                .await
                .expect("read");
            if read == 0 {
                return;
            }
            request.extend_from_slice(&buf[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        let text = String::from_utf8_lossy(&request);
        assert!(
            text.starts_with("POST /openai/v1/responses?api-version=v1 "),
            "unexpected request line: {text}"
        );
        tokio::io::AsyncWriteExt::write_all(
            &mut socket,
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: 1024\r\nConnection: close\r\n\r\n",
        )
        .await
        .expect("write headers");
        tokio::io::AsyncWriteExt::flush(&mut socket)
            .await
            .expect("flush");
        server_stalled.notify_one();
        std::future::pending::<()>().await;
    });

    let provider = azure_responses_provider(format!("http://{addr}/openai/v1"));
    let request = azure_responses_request("my-gpt4o", None);
    let cancel = request.cancel.clone();
    let mut stream = provider.stream_prepared(request, azure_api_key_auth());
    stalled.notified().await;
    cancel.cancel();
    let items = collect_azure_items(&mut stream).await;
    assert!(
        items
            .iter()
            .any(|item| matches!(item, Err(error) if matches!(direct_error_cause(error), ProviderError::Cancelled))),
        "cancellation surfaces typed Cancelled: {items:?}"
    );
}

#[tokio::test]
async fn azure_duration_includes_pending_first_body_poll() {
    use std::time::Duration;
    let (control, _) = azure_duration_on_peer(Duration::ZERO, Duration::ZERO).await;
    let (delayed, elapsed) =
        azure_duration_on_peer(Duration::from_millis(350), Duration::ZERO).await;
    assert_eq!(delayed.content, control.content);
    assert!(!delayed.content.is_empty());
    assert_eq!(delayed.usage, control.usage);
    assert!(elapsed >= Duration::from_millis(300));
    assert!(
        delayed.duration_ms.unwrap() >= 300,
        "first Pending body poll belongs to response duration: {:?}",
        delayed.duration_ms
    );
}

#[tokio::test]
async fn azure_duration_excludes_auth_setup_callback_and_unpolled_time() {
    use std::time::Duration;
    // Four 150ms intervals: resolver, response headers, response callback,
    // then delayed consumption after the raw terminal. None belongs to body polling.
    let (message, elapsed) =
        azure_duration_on_peer(Duration::ZERO, Duration::from_millis(150)).await;
    let measured = Duration::from_millis(message.duration_ms.unwrap());
    assert!(
        elapsed.saturating_sub(measured) >= Duration::from_millis(500),
        "duration included setup: measured={measured:?}, elapsed={elapsed:?}"
    );
}
