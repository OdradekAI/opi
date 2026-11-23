//! Vertex callback and transform assertions.

use super::fixtures::*;
use super::*;

// ---------------------------------------------------------------------------
// Functional Vertex callbacks: payload/raw supported, response unsupported.
// ---------------------------------------------------------------------------

use opi_ai::callbacks::RequestCallbacks;

#[tokio::test]
async fn vertex_response_observer_unsupported_but_payload_callback_flows() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wiremock::matchers::body_partial_json(
            serde_json::json!({"vertex_marker": "replaced"}),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_sse_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let observer_server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&observer_server)
        .await;

    let provider = VertexProvider::new(
        "my-project".into(),
        "us-central1".into(),
        Some(server.uri()),
    );
    let auth = opi_ai::auth::ResolvedAuth {
        env: Default::default(),
        scheme: opi_ai::auth::AuthScheme::Bearer,
        secret: secrecy::SecretString::from("test-access-token"),
        base_url: None,
        account_id: None,
        provenance: opi_ai::AuthProvenance::default(),
    };

    // Payload-only callbacks flow through the shared Google wire projection.
    let mut request = lifecycle_text_request();
    request.callbacks = RequestCallbacks::new().with_on_payload(std::sync::Arc::new(|body| {
        let mut replacement = body.clone();
        replacement["vertex_marker"] = serde_json::json!("replaced");
        Ok(Some(replacement))
    }));
    let events = collect_stream(provider.stream_prepared(request, auth.clone())).await;
    assert!(
        events
            .iter()
            .any(|event| matches!(event, AssistantStreamEvent::Done { .. }))
    );
    server.verify().await;

    // HTTP response observers are explicitly unsupported on Google wires.
    let observer_provider = VertexProvider::new(
        "my-project".into(),
        "us-central1".into(),
        Some(observer_server.uri()),
    );
    let mut observer_request = lifecycle_text_request();
    observer_request.callbacks =
        RequestCallbacks::new().with_on_response(std::sync::Arc::new(|_status, _headers| Ok(())));
    let mut stream = observer_provider.stream_prepared(observer_request.clone(), auth);
    let error = stream
        .next()
        .await
        .expect("one refusal")
        .expect_err("vertex rejects response observers before dispatch");
    let ProviderError::UnsupportedCapability(summary) =
        accepted_failure_cause(&error, &observer_request)
    else {
        panic!("expected exact response-observer cause: {error:?}");
    };
    assert_eq!(
        summary.as_str(),
        "Google wires do not support response callbacks"
    );
    assert!(stream.next().await.is_none(), "refusal must be terminal");
    observer_server.verify().await;
}

#[tokio::test]
async fn vertex_async_raw_observer_sees_wire_fields_before_normalization() {
    let raw = serde_json::json!({"wire_only":"retained","candidates":[{"content":{"parts":[{"text":"Hi"}]},"finishReason":"STOP"}]});
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(format!("data: {raw}\n\n")),
        )
        .mount(&server)
        .await;
    let seen = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let count = seen.clone();
    let mut request = lifecycle_text_request();
    request.callbacks = RequestCallbacks::new().with_async_on_raw(move |value| {
        let count = count.clone();
        async move {
            tokio::task::yield_now().await;
            assert_eq!(value["wire_only"], "retained");
            count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
    });
    let provider = VertexProvider::new(
        "my-project".into(),
        "us-central1".into(),
        Some(server.uri()),
    );
    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
    let mut done = false;
    while let Some(item) = stream.next().await {
        let event = item.expect("Vertex raw observer is supported");
        assert_eq!(seen.load(std::sync::atomic::Ordering::SeqCst), 1);
        done |= matches!(event, AssistantStreamEvent::Done { .. });
    }
    assert!(done);
}

#[tokio::test]
async fn vertex_transform_cannot_introduce_google_or_host_identity() {
    for name in ["Host", "X-Goog-Api-Key", "X-Goog-User-Project"] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;
        let mut request = lifecycle_text_request();
        request.callbacks =
            RequestCallbacks::new().with_header_transform(std::sync::Arc::new(move |_| {
                Ok(vec![(name.into(), "changed".into())])
            }));
        let provider = VertexProvider::new(
            "my-project".into(),
            "us-central1".into(),
            Some(server.uri()),
        );
        let failure = opi_ai::stream::ChatStream::new(
            provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth()),
        )
        .result()
        .await
        .unwrap_err();
        assert!(matches!(failure.cause(), ProviderError::CallbackPhase(_)));
        server.verify().await;
    }
}
