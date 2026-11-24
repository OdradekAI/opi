//! Native Mistral Conversations callback and telemetry assertions.

use super::*;

#[cfg(feature = "telemetry-tracing")]
#[tokio::test]
async fn producer_runs_under_the_frozen_telemetry_context() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tracing_core::span::{Attributes, Id, Record};
    use tracing_core::{Dispatch, Event, Metadata, Subscriber, dispatcher};

    #[derive(Default)]
    struct Recording {
        next: u64,
        spans: Vec<(&'static str, Option<u64>)>,
    }
    struct Recorder(std::sync::Arc<std::sync::Mutex<Recording>>);

    impl Subscriber for Recorder {
        fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, attributes: &Attributes<'_>) -> Id {
            let mut state = self.0.lock().unwrap();
            state.next += 1;
            let id = Id::from_u64(state.next);
            let parent = attributes
                .parent()
                .cloned()
                .or_else(|| attributes.is_contextual().then(|| Id::from_u64(1)))
                .map(|id| id.into_u64());
            state.spans.push((attributes.metadata().name(), parent));
            id
        }
        fn record(&self, _id: &Id, _values: &Record<'_>) {}
        fn record_follows_from(&self, _sub: &Id, _follows: &Id) {}
        fn event(&self, _event: &Event<'_>) {}
        fn enter(&self, _id: &Id) {}
        fn exit(&self, _id: &Id) {}
        fn try_close(&self, _id: Id) -> bool {
            false
        }
    }

    let recorder = std::sync::Arc::new(std::sync::Mutex::new(Recording::default()));
    let source = Dispatch::new(Recorder(recorder.clone()));
    let parent = dispatcher::with_default(&source, || tracing::info_span!("source_parent_native"));
    let context = opi_ai::telemetry::TelemetryContext::from_span(parent);

    let server = MockServer::start().await;
    mount_native(&server, sse_ok_native()).await;
    let provider = provider_native(
        Some(&server.uri()),
        model_native(SMALL_NATIVE, false, false),
        MistralOptions::default(),
    );

    let seen = std::sync::Arc::new(AtomicUsize::new(0));
    let counter = seen.clone();
    let mut request = req_native("go");
    request.telemetry_context = Some(context);
    request.callbacks =
        opi_ai::callbacks::RequestCallbacks::new().with_async_on_payload(move |_| {
            let counter = counter.clone();
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                tracing::info_span!("payload_probe_native").in_scope(|| ());
                Ok(None)
            }
        });

    let events = run_native(&provider, request).await;
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Ok(AssistantStreamEvent::Done { .. })))
    );
    assert_eq!(seen.load(Ordering::SeqCst), 1);
    let state = recorder.lock().unwrap();
    assert!(
        state
            .spans
            .iter()
            .any(|(name, _)| *name == "payload_probe_native"),
        "mistral producer telemetry flows to the frozen carrier: {:?}",
        state.spans
    );
}

#[tokio::test]
async fn mistral_raw_and_header_callbacks_reach_real_transport() {
    for callback_failure in [false, true] {
        let server = MockServer::start().await;
        let first =
            serde_json::json!({"wire_only":"first","choices":[{"delta":{"content":"visible"}}]});
        let last =
            serde_json::json!({"wire_only":"last","choices":[{"delta":{},"finish_reason":"stop"}]});
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(format!("data: {first}\n\ndata: {last}\n\ndata: [DONE]\n\n"))
                    .insert_header("content-type", "text/event-stream"),
            )
            .expect(1)
            .mount(&server)
            .await;
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let observed = seen.clone();
        let mut request = req_native("probe");
        request.callbacks = opi_ai::callbacks::RequestCallbacks::new()
            .with_async_on_raw(move |value| {
                let observed = observed.clone();
                async move {
                    observed.lock().unwrap().push(value.clone());
                    if callback_failure && value["wire_only"] == "last" {
                        Err(opi_ai::callbacks::CallbackError::new("fixture raw failure"))
                    } else {
                        Ok(())
                    }
                }
            })
            .with_async_header_transform(|mut headers| async move {
                headers.push(("x-fixture-transform".into(), "applied".into()));
                Ok(headers)
            });
        let result = mistral_complete(&server.uri(), request).await;
        assert_eq!(seen.lock().unwrap().as_slice(), &[first, last]);
        let sent = server.received_requests().await.unwrap();
        assert_eq!(
            sent[0].headers.get("x-fixture-transform").unwrap(),
            "applied"
        );
        if callback_failure {
            let failure = result.expect_err("callback failure");
            assert!(
                matches!(&failure.message().content[0],AssistantContent::Text{text,..} if text=="visible")
            );
        } else {
            assert!(result.is_ok());
        }
        server.verify().await;
    }
}
