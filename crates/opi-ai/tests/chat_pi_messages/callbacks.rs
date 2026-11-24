//! Pi Messages callback ordering assertions.

use super::*;

// ---------------------------------------------------------------------------
// callbacks, cancel, auth, models
// ---------------------------------------------------------------------------

#[tokio::test]
async fn callbacks_observe_payload_response_and_raw_events() {
    let server = MockServer::start().await;
    mount_44(&server, sse_ok_44()).await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));

    let order: std::sync::Arc<std::sync::Mutex<Vec<String>>> = Default::default();
    let payload_order = order.clone();
    let response_order = order.clone();
    let raw_order = order.clone();
    let mut request = req_44("hi");
    request.callbacks = RequestCallbacks::new()
        .with_on_payload(std::sync::Arc::new(move |body| {
            payload_order.lock().unwrap().push(format!(
                "payload:{}",
                body.get("model").and_then(|m| m.as_str()).unwrap_or("?")
            ));
            Ok(None)
        }))
        .with_on_response(std::sync::Arc::new(move |status, _headers| {
            response_order
                .lock()
                .unwrap()
                .push(format!("response:{status}"));
            Ok(())
        }))
        .with_on_raw(std::sync::Arc::new(move |event| {
            if let Some(kind) = event.get("type").and_then(|t| t.as_str()) {
                raw_order.lock().unwrap().push(format!("raw:{kind}"));
            }
            Ok(())
        }));

    let events = run_44(&provider, request).await;
    assert!(
        events
            .iter()
            .any(|item| matches!(item, Ok(AssistantStreamEvent::Done { .. })))
    );
    let order = order.lock().unwrap().clone();
    assert_eq!(
        order.first().map(String::as_str),
        Some("payload:claude-sonnet-4-5")
    );
    let response_at = order
        .iter()
        .position(|m| m == "response:200")
        .expect("response observer fires");
    let raw_at = order
        .iter()
        .position(|m| m == "raw:start")
        .expect("raw observer sees parsed events");
    assert!(response_at < raw_at, "response precedes raw events");
    assert!(
        order.iter().any(|m| m == "raw:done"),
        "terminal events reach the raw observer"
    );
    server.verify().await;
}
