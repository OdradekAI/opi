//! Gemini callback and observer lifecycle assertions.

use super::*;

// Task 20.51: callbacks at the Google wire — payload flows, observers typed
// unsupported (P20-S51)
// ---------------------------------------------------------------------------

use opi_ai::callbacks::RequestCallbacks;

#[tokio::test]
async fn gemini_payload_callback_applies_to_wire_body() {
    let sse = "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Hi\"}]},\"finishReason\":\"STOP\",\"index\":0}]}\n\n";
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wiremock::matchers::body_partial_json(
            serde_json::json!({"gemini_marker": "replaced"}),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = GeminiProvider::new(Some(server.uri()));
    let mut request = gemini_http_request();
    request.callbacks = RequestCallbacks::new().with_on_payload(std::sync::Arc::new(|body| {
        let mut replacement = body.clone();
        replacement["gemini_marker"] = serde_json::json!("replaced");
        Ok(Some(replacement))
    }));

    let events =
        collect_stream(provider.stream_prepared(request, opi_ai::test_support::resolved_auth()))
            .await;
    assert!(
        events
            .iter()
            .any(|event| matches!(event, AssistantStreamEvent::Done { .. }))
    );
    server.verify().await;
}

#[tokio::test]
async fn gemini_response_callback_is_typed_unsupported_before_http() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let provider = GeminiProvider::new(Some(server.uri()));
    let mut request = gemini_http_request();
    let callbacks = RequestCallbacks::new();
    request.callbacks = callbacks.with_on_response(std::sync::Arc::new(|_status, _headers| Ok(())));

    let expected_prefix = request.context.messages().len();
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    let error = stream.next().await.expect("typed refusal").unwrap_err();
    let ProviderError::StreamFailure(failure) = error else {
        panic!("accepted projection retains failure facts")
    };
    assert!(matches!(
        failure.cause(),
        ProviderError::UnsupportedCapability(_)
    ));
    assert_eq!(
        failure
            .message()
            .projection
            .as_ref()
            .unwrap()
            .source_prefix
            .message_count,
        expected_prefix
    );
    assert!(stream.next().await.is_none());
    server.verify().await;
}

#[tokio::test]
async fn gemini_raw_callbacks_observe_unmodified_chunk_before_normalized_events() {
    let raw = serde_json::json!({"unknown_wire_field":"retained", "candidates":[{"content":{"parts":[{"text":"Hi"}]},"finishReason":"STOP"}]});
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(format!("data: {raw}\n\n")),
        )
        .mount(&server)
        .await;
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorder = seen.clone();
    let mut request = gemini_http_request();
    request.callbacks = RequestCallbacks::new().with_on_raw(std::sync::Arc::new(move |value| {
        recorder.lock().unwrap().push(value.clone());
        Ok(())
    }));
    let provider = GeminiProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    let mut done = false;
    while let Some(item) = stream.next().await {
        let event = item.expect("raw observer is supported");
        assert_eq!(seen.lock().unwrap().as_slice(), std::slice::from_ref(&raw));
        done |= matches!(event, AssistantStreamEvent::Done { .. });
    }
    assert!(done);
}

#[tokio::test]
async fn separate_gemini_calls_reusing_callbacks_freeze_independently() {
    let server = MockServer::start().await;
    Mock::given(method("POST")).respond_with(ResponseTemplate::new(200).insert_header("content-type", "text/event-stream").set_body_string("data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"Hi\"}]},\"finishReason\":\"STOP\"}]}\n\n")).expect(2).mount(&server).await;
    let payload_calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let count = payload_calls.clone();
    let callbacks = RequestCallbacks::new().with_on_payload(std::sync::Arc::new(move |_| {
        count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(None)
    }));
    let provider = GeminiProvider::new(Some(server.uri()));
    for text in ["first", "second"] {
        let mut request = gemini_http_request();
        request.callbacks = callbacks.clone();
        request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text { text: text.into() }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .unwrap();
        opi_ai::stream::ChatStream::new(
            provider.stream_prepared(request, opi_ai::test_support::resolved_auth()),
        )
        .result()
        .await
        .expect("separate call succeeds");
    }
    let requests = server.received_requests().await.unwrap();
    assert_eq!(payload_calls.load(std::sync::atomic::Ordering::SeqCst), 2);
    assert_ne!(
        requests[0].body, requests[1].body,
        "second call must retain its own input"
    );
}

#[tokio::test]
async fn async_google_raw_observation_blocks_normalized_delivery_until_ready() {
    let raw = serde_json::json!({"wire_only":"retained", "candidates":[{"content":{"parts":[{"text":"Hi"}]},"finishReason":"STOP"}]});
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(format!("data: {raw}\n\n")),
        )
        .mount(&server)
        .await;
    let entered = std::sync::Arc::new(tokio::sync::Notify::new());
    let release = std::sync::Arc::new(tokio::sync::Notify::new());
    let marker = entered.clone();
    let gate = release.clone();
    let mut request = gemini_http_request();
    request.callbacks = RequestCallbacks::new().with_async_on_raw(move |value| {
        let marker = marker.clone();
        let gate = gate.clone();
        async move {
            assert_eq!(value["wire_only"], "retained");
            marker.notify_one();
            gate.notified().await;
            Ok(())
        }
    });
    let provider = GeminiProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    assert!(futures_util::poll!(stream.next()).is_pending());
    tokio::time::timeout(std::time::Duration::from_secs(1), entered.notified())
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(10), stream.next())
            .await
            .is_err()
    );
    release.notify_one();
    opi_ai::stream::ChatStream::new(stream)
        .result()
        .await
        .expect("released observer allows final output");
}

#[tokio::test]
async fn cancelled_async_payload_is_dropped_without_http_dispatch() {
    struct NotifyDrop(std::sync::Arc<tokio::sync::Notify>);
    impl Drop for NotifyDrop {
        fn drop(&mut self) {
            self.0.notify_one();
        }
    }
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;
    let entered = std::sync::Arc::new(tokio::sync::Notify::new());
    let dropped = std::sync::Arc::new(tokio::sync::Notify::new());
    let marker = entered.clone();
    let release = dropped.clone();
    let mut request = gemini_http_request();
    let cancel = request.cancel.clone();
    request.callbacks = RequestCallbacks::new().with_async_on_payload(move |_| {
        let marker = marker.clone();
        let release = release.clone();
        async move {
            let _drop = NotifyDrop(release);
            marker.notify_one();
            std::future::pending::<()>().await;
            Ok(None)
        }
    });
    let provider = GeminiProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    assert!(futures_util::poll!(stream.next()).is_pending());
    tokio::time::timeout(std::time::Duration::from_secs(1), entered.notified())
        .await
        .unwrap();
    cancel.cancel();
    let failure = opi_ai::stream::ChatStream::new(stream)
        .result()
        .await
        .unwrap_err();
    assert_eq!(
        failure.cause().category(),
        opi_ai::provider::ProviderErrorCategory::Cancelled
    );
    tokio::time::timeout(std::time::Duration::from_secs(1), dropped.notified())
        .await
        .unwrap();
    server.verify().await;
}
