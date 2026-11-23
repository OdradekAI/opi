//! OpenRouter Images callback and telemetry assertions.

use super::*;

#[tokio::test]
async fn images_payload_and_response_callbacks_fire_in_order() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-openrouter-id", "resp-1")
                .set_body_json(serde_json::json!({
                    "id": "gen-cb",
                    "choices": [
                        { "message": { "role": "assistant", "content": "ok" } }
                    ]
                })),
        )
        .mount(&server)
        .await;

    let record = image_record(
        server.uri(),
        vec![ImageOutputModality::Image, ImageOutputModality::Text],
    );
    let models = image_models(&record);

    let payload_first = std::sync::Arc::new(AtomicBool::new(false));
    let response_seen = std::sync::Arc::new(AtomicBool::new(false));
    let order_ok = std::sync::Arc::new(AtomicBool::new(false));
    let payload_set = std::sync::Arc::clone(&payload_first);
    let payload_check = std::sync::Arc::clone(&payload_first);
    let response_flag = std::sync::Arc::clone(&response_seen);
    let order_flag = std::sync::Arc::clone(&order_ok);

    let callbacks = opi_ai::callbacks::RequestCallbacks::new()
        .with_on_payload(std::sync::Arc::new(move |body: &serde_json::Value| {
            assert_eq!(body["modalities"], serde_json::json!(["image", "text"]));
            assert_eq!(body["stream"], false);
            payload_set.store(true, AtomicOrdering::SeqCst);
            Ok(None)
        }))
        .with_on_response(std::sync::Arc::new(move |status, headers| {
            assert_eq!(status, 200);
            assert!(
                headers.iter().any(|(name, _)| name == "x-openrouter-id"),
                "response headers observed: {headers:?}"
            );
            if response_flag
                .compare_exchange(false, true, AtomicOrdering::SeqCst, AtomicOrdering::SeqCst)
                .is_ok()
            {
                let observed = payload_check.load(AtomicOrdering::SeqCst);
                order_flag.store(observed, AtomicOrdering::SeqCst);
            }
            Ok(())
        }));

    let result = models
        .generate_images(
            record,
            ImagesContext {
                input: vec![ImagesInput::Text { text: "hi".into() }],
            },
            ImagesCall {
                callbacks,
                ..images_call()
            },
        )
        .await;

    assert_eq!(result.stop_reason, ImagesStopReason::Stop);
    assert!(
        payload_first.load(AtomicOrdering::SeqCst),
        "payload observed"
    );
    assert!(
        response_seen.load(AtomicOrdering::SeqCst),
        "response observed"
    );
    assert!(
        order_ok.load(AtomicOrdering::SeqCst),
        "payload before response"
    );
}

#[tokio::test]
async fn images_telemetry_context_passes_through() {
    let recorder = Recorder::default();
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "gen-tel",
            "choices": [
                { "message": { "role": "assistant", "content": "ok" } }
            ]
        })))
        .mount(&server)
        .await;

    let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
    let models = image_models(&record);
    let result = models
        .generate_images(
            record,
            ImagesContext {
                input: vec![ImagesInput::Text { text: "hi".into() }],
            },
            ImagesCall {
                telemetry_context: Some(recorder.context()),
                ..images_call()
            },
        )
        .await;

    assert_eq!(result.stop_reason, ImagesStopReason::Stop);
    assert!(
        recorder.spans().is_empty(),
        "image generation carries telemetry context but does not install a pi.ai span"
    );
}

#[tokio::test]
async fn images_reused_callbacks_keep_each_calls_payload() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"choices": []})))
        .expect(2)
        .mount(&server)
        .await;
    let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
    let models = image_models(&record);
    let observed = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = observed.clone();
    let callbacks = opi_ai::callbacks::RequestCallbacks::new().with_on_payload(
        std::sync::Arc::new(move |_| {
            counter.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(None)
        }),
    );
    for prompt in ["first image", "second image"] {
        let result = models
            .generate_images(
                record.clone(),
                ImagesContext {
                    input: vec![ImagesInput::Text {
                        text: prompt.into(),
                    }],
                },
                ImagesCall {
                    callbacks: callbacks.clone(),
                    ..images_call()
                },
            )
            .await;
        assert_eq!(result.stop_reason, ImagesStopReason::Stop);
    }
    let sent = server.received_requests().await.unwrap();
    let prompts: Vec<serde_json::Value> = sent
        .iter()
        .map(|request| {
            let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
            body["messages"][0]["content"][0]["text"].clone()
        })
        .collect();
    assert_eq!(
        prompts,
        [
            serde_json::json!("first image"),
            serde_json::json!("second image")
        ]
    );
    assert_eq!(observed.load(AtomicOrdering::SeqCst), 2);
}

mod image_response_observer_facts {
    use super::*;
    use std::sync::{Arc, atomic::AtomicUsize};

    use opi_ai::callbacks::{CallbackError, RequestCallbacks};
    use opi_ai::images::ImagesResult;

    fn received_body(with_usage: bool) -> serde_json::Value {
        let mut body = serde_json::json!({
            "id": "gen-received",
            "choices": [{ "message": {
                "content": "received text",
                "images": [{ "image_url": "data:image/png;base64,QUJD" }]
            } }]
        });
        if with_usage {
            body["usage"] = serde_json::json!({
                "prompt_tokens": 100,
                "completion_tokens": 40,
                "prompt_tokens_details": {"cached_tokens": 30, "cache_write_tokens": 10}
            });
        }
        body
    }

    fn rejected_response(counter: Arc<AtomicUsize>) -> RequestCallbacks {
        RequestCallbacks::new().with_on_response(Arc::new(move |status, _| {
            assert_eq!(status, 200);
            counter.fetch_add(1, AtomicOrdering::SeqCst);
            Err(CallbackError::new("synthetic observer failure"))
        }))
    }

    async fn observed_images(
        response: ResponseTemplate,
        known_prices: bool,
        callbacks: RequestCallbacks,
        cancel: CancellationToken,
    ) -> (ImagesResult, MockServer) {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(response)
            .expect(1)
            .mount(&server)
            .await;
        let mut record = image_record(
            server.uri(),
            vec![ImageOutputModality::Image, ImageOutputModality::Text],
        );
        if !known_prices {
            record.cost = None;
        }
        let mut models = Models::new();
        let handle = models
            .register_image_api(
                Arc::new(OpenRouterImagesApi::new(HttpClient::new(), record.clone())),
                Arc::new(opi_ai::auth::StaticAuthResolver::new(
                    AuthScheme::Bearer,
                    secrecy::SecretString::from("synthetic-observer-key"),
                )),
                AuthProvenanceSource::Static,
            )
            .unwrap();
        let result = models
            .generate_images_native(
                record,
                ImagesContext::default(),
                ImagesCall {
                    callbacks,
                    cancel,
                    ..images_call()
                },
                handle.options(OpenRouterImagesOptions {
                    max_retries: 2,
                    max_retry_delay_ms: None,
                }),
            )
            .await;
        (result, server)
    }

    async fn assert_observer_failure(
        result: &ImagesResult,
        server: &MockServer,
        counter: &AtomicUsize,
        cancelled: bool,
    ) {
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
        assert_eq!(counter.load(AtomicOrdering::SeqCst), 1);
        if cancelled {
            assert_eq!(result.stop_reason, ImagesStopReason::Aborted);
            assert!(matches!(result.error, Some(ProviderError::Cancelled)));
        } else {
            assert_eq!(result.stop_reason, ImagesStopReason::Error);
            assert!(matches!(
                result.error,
                Some(ProviderError::CallbackPhase(_))
            ));
        }
        assert!(result.error_message.is_some());
    }

    fn assert_received_output(result: &ImagesResult) {
        assert_eq!(result.api, "openrouter-images");
        assert_eq!(result.provider, "openrouter");
        assert_eq!(result.model, "google/gemini-2.5-flash-image");
        assert_eq!(result.response_id.as_deref(), Some("gen-received"));
        assert_eq!(
            result.output,
            vec![
                ImagesOutput::Text {
                    text: "received text".into()
                },
                ImagesOutput::Image {
                    mime_type: "image/png".into(),
                    data: "QUJD".into()
                },
            ]
        );
    }

    fn assert_received_usage(result: &ImagesResult) {
        let usage = result.usage.as_ref().expect("received usage retained");
        assert!(usage.reported);
        assert_eq!(usage.input_tokens, 70);
        assert_eq!(usage.output_tokens, 40);
        assert_eq!(usage.cache_read_tokens, 20);
        assert_eq!(usage.cache_write_tokens, 10);
        assert_eq!(usage.total_tokens, Some(140));
    }

    fn assert_received_known_facts(result: &ImagesResult) {
        assert_received_output(result);
        assert_received_usage(result);
        let cost = result.cost.as_ref().expect("received cost retained");
        for (actual, expected) in [
            (cost.input, 0.000_21),
            (cost.output, 0.000_6),
            (cost.cache_read, 0.000_006),
            (cost.cache_write, 0.000_037_5),
            (cost.total, 0.000_853_5),
        ] {
            assert!((actual - expected).abs() < 1e-12);
        }
    }

    #[tokio::test]
    async fn failure_retains_received_result() {
        let counter = Arc::new(AtomicUsize::new(0));
        let (result, server) = observed_images(
            ResponseTemplate::new(200).set_body_json(received_body(true)),
            true,
            rejected_response(counter.clone()),
            CancellationToken::new(),
        )
        .await;
        assert_observer_failure(&result, &server, &counter, false).await;
        assert_received_known_facts(&result);
    }

    #[tokio::test]
    async fn async_cancellation_retains_received_result() {
        let counter = Arc::new(AtomicUsize::new(0));
        let cancel = CancellationToken::new();
        let callback_counter = counter.clone();
        let callback_cancel = cancel.clone();
        let callbacks = RequestCallbacks::new().with_async_on_response(move |status, _| {
            let counter = callback_counter.clone();
            let cancel = callback_cancel.clone();
            async move {
                assert_eq!(status, 200);
                counter.fetch_add(1, AtomicOrdering::SeqCst);
                cancel.cancel();
                std::future::pending::<Result<(), CallbackError>>().await
            }
        });
        let (result, server) = observed_images(
            ResponseTemplate::new(200).set_body_json(received_body(true)),
            true,
            callbacks,
            cancel,
        )
        .await;
        assert_observer_failure(&result, &server, &counter, true).await;
        assert_received_known_facts(&result);
    }

    #[tokio::test]
    async fn synchronous_cancellation_retains_received_result() {
        let counter = Arc::new(AtomicUsize::new(0));
        let cancel = CancellationToken::new();
        let callback_counter = counter.clone();
        let callback_cancel = cancel.clone();
        let callbacks = RequestCallbacks::new().with_on_response(Arc::new(move |status, _| {
            assert_eq!(status, 200);
            callback_counter.fetch_add(1, AtomicOrdering::SeqCst);
            callback_cancel.cancel();
            Ok(())
        }));
        let (result, server) = observed_images(
            ResponseTemplate::new(200).set_body_json(received_body(true)),
            true,
            callbacks,
            cancel,
        )
        .await;
        assert_observer_failure(&result, &server, &counter, true).await;
        assert_received_known_facts(&result);
    }

    #[tokio::test]
    async fn unknown_usage_stays_unknown_after_failure() {
        let counter = Arc::new(AtomicUsize::new(0));
        let (result, server) = observed_images(
            ResponseTemplate::new(200).set_body_json(received_body(false)),
            true,
            rejected_response(counter.clone()),
            CancellationToken::new(),
        )
        .await;
        assert_observer_failure(&result, &server, &counter, false).await;
        assert_received_output(&result);
        assert!(result.usage.is_none());
        assert!(result.cost.is_none());
    }

    #[tokio::test]
    async fn unknown_prices_stay_unknown_after_failure() {
        let counter = Arc::new(AtomicUsize::new(0));
        let (result, server) = observed_images(
            ResponseTemplate::new(200).set_body_json(received_body(true)),
            false,
            rejected_response(counter.clone()),
            CancellationToken::new(),
        )
        .await;
        assert_observer_failure(&result, &server, &counter, false).await;
        assert_received_output(&result);
        assert_received_usage(&result);
        assert!(result.cost.is_none());
    }

    #[tokio::test]
    async fn invalid_json_does_not_notify_or_retry() {
        let counter = Arc::new(AtomicUsize::new(0));
        let callback_counter = counter.clone();
        let callbacks = RequestCallbacks::new().with_on_response(Arc::new(move |_, _| {
            callback_counter.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(())
        }));
        let (result, server) = observed_images(
            ResponseTemplate::new(200).set_body_string("{incomplete"),
            true,
            callbacks,
            CancellationToken::new(),
        )
        .await;
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
        assert_eq!(counter.load(AtomicOrdering::SeqCst), 0);
        assert_eq!(result.stop_reason, ImagesStopReason::Error);
        assert!(matches!(result.error, Some(ProviderError::StreamError(_))));
        assert!(result.output.is_empty());
        assert!(result.response_id.is_none());
        assert!(result.usage.is_none());
        assert!(result.cost.is_none());
    }
}

#[tokio::test]
async fn images_response_callback_is_after_json_and_outside_attempt_timeout() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"choices": []})))
        .expect(1)
        .mount(&server)
        .await;
    let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
    let models = image_models(&record);
    let result = models
        .generate_images(
            record,
            ImagesContext::default(),
            ImagesCall {
                timeout: Some(Duration::from_millis(300)),
                callbacks: opi_ai::callbacks::RequestCallbacks::new().with_async_on_response(
                    |_, _| async {
                        tokio::time::sleep(Duration::from_millis(600)).await;
                        Ok(())
                    },
                ),
                ..Default::default()
            },
        )
        .await;
    assert_eq!(
        result.stop_reason,
        ImagesStopReason::Stop,
        "{:?}",
        result.error
    );

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{incomplete"))
        .expect(1)
        .mount(&server)
        .await;
    let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
    let models = image_models(&record);
    let observed = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = observed.clone();
    let result = models
        .generate_images(
            record,
            ImagesContext::default(),
            ImagesCall {
                callbacks: opi_ai::callbacks::RequestCallbacks::new().with_on_response(
                    std::sync::Arc::new(move |_, _| {
                        counter.fetch_add(1, AtomicOrdering::SeqCst);
                        Ok(())
                    }),
                ),
                ..images_call()
            },
        )
        .await;
    assert_eq!(result.stop_reason, ImagesStopReason::Error);
    assert_eq!(observed.load(AtomicOrdering::SeqCst), 0);
}
