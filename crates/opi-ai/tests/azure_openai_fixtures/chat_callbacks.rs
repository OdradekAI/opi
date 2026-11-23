//! Azure OpenAI Chat callback assertions.

use super::*;
use opi_ai::callbacks::RequestCallbacks;

// ---------------------------------------------------------------------------
// Task 20.51: callbacks flow through the shared Responses wire (P20-S51)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn azure_async_callbacks_flow_through_shared_responses_wire() {
    let order: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let raw_recorder = order.clone();
    let response_recorder = order.clone();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/openai/deployments/my-gpt4o/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_sse_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = AzureOpenAIProvider::new(
        Some(server.uri()),
        "my-gpt4o".into(),
        Some("2024-06-01".into()),
    )
    .unwrap();
    let auth = opi_ai::auth::ResolvedAuth {
        env: Default::default(),
        scheme: opi_ai::auth::AuthScheme::ApiKey,
        secret: secrecy::SecretString::from("test-api-key-12345"),
        base_url: None,
        account_id: None,
        provenance: opi_ai::AuthProvenance::default(),
    };
    let payload_calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let payload_counter = payload_calls.clone();
    let mut request = text_request();
    request.callbacks = RequestCallbacks::new()
        .with_async_header_transform(|headers| async move {
            tokio::task::yield_now().await;
            Ok(headers)
        })
        .with_async_on_payload(move |_body| {
            let payload_counter = payload_counter.clone();
            async move {
                tokio::task::yield_now().await;
                payload_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(None)
            }
        })
        .with_async_on_response(move |status, _headers| {
            let response_recorder = response_recorder.clone();
            async move {
                tokio::task::yield_now().await;
                response_recorder
                    .lock()
                    .unwrap()
                    .push(format!("response:{status}"));
                Ok(())
            }
        })
        .with_async_on_raw(move |event| {
            let raw_recorder = raw_recorder.clone();
            async move {
                tokio::task::yield_now().await;
                raw_recorder
                    .lock()
                    .unwrap()
                    .push(format!("raw:{}", event["object"].as_str().unwrap_or("?")));
                Ok(())
            }
        });

    let events = collect_events(provider.stream_prepared(request, auth)).await;
    assert!(
        events
            .iter()
            .any(|event| matches!(event, AssistantStreamEvent::Done { .. }))
    );
    assert_eq!(payload_calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    let order = order.lock().unwrap().clone();
    assert_eq!(order.first().map(String::as_str), Some("response:200"));
    assert!(
        order.len() > 1,
        "raw wire events follow the response callback"
    );
    server.verify().await;
}
