//! OpenRouter Images lifecycle, admission and auth assertions.

use super::*;

#[test]
fn image_registration_preserves_arbitrary_exact_api_ids_without_auth() {
    for id in ["normal", "", " \t", "api/β\0id"] {
        let mut models = Models::new();
        let resolver = std::sync::Arc::new(pending_auth::PendingAuth::default());
        let mut record = image_record("http://127.0.0.1:1".into(), vec![]);
        record.api = ApiId::new(id);
        let handle = models
            .register_image_api(
                std::sync::Arc::new(OpenRouterImagesApi::new(HttpClient::new(), record.clone())),
                resolver.clone(),
                AuthProvenanceSource::Static,
            )
            .expect("ApiId has no nonempty or whitespace grammar");
        assert_eq!(handle.api_id().as_str(), id);
        assert_eq!(
            models
                .image_model("openrouter", &record.id)
                .unwrap()
                .api
                .as_str(),
            id
        );
        record.provider_id = " \t".into();
        assert!(matches!(
            models.register_image_api(
                std::sync::Arc::new(OpenRouterImagesApi::new(HttpClient::new(), record)),
                resolver.clone(),
                AuthProvenanceSource::Static,
            ),
            Err(opi_ai::registry::RegistrationError::EmptyProviderId)
        ));
        assert_eq!(resolver.calls.load(AtomicOrdering::SeqCst), 0);
    }
}

#[tokio::test]
async fn images_default_zero_extra_retries_fails_immediately() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
            "error": { "message": "Rate limited", "code": "rate_limited" }
        })))
        .expect(1)
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
            images_call(),
        )
        .await;

    assert_eq!(result.stop_reason, ImagesStopReason::Error);
    let message = result.error_message.expect("error message");
    assert!(message.contains("429"), "message: {message}");
    assert_eq!(message, "HTTP 429: [REDACTED]");
    assert!(matches!(
        result.error.as_ref().expect("typed error"),
        ProviderError::RateLimited { .. }
    ));
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

/// Manual server answering the first request with 429 + `retry-after-ms: 5`
/// and every later request with a successful generation, capturing the raw
#[tokio::test]
async fn images_explicit_retry_reuses_frozen_payload() {
    let (base, bodies) = spawn_retry_images_server().await;
    let record = image_record(base, vec![ImageOutputModality::Image]);
    let mut models = image_models(&record);
    let handle = {
        // Re-register to obtain a handle with typed native options.
        let api = OpenRouterImagesApi::new(HttpClient::new(), record.clone());
        let api_id = api.api_id().clone();
        let provider_id = api.provider_id().to_owned();
        let boxed: std::sync::Arc<dyn opi_ai::api::ImageApi<OpenRouterImagesOptions>> =
            std::sync::Arc::new(api);
        let handle = models
            .register_image_api(
                boxed.clone(),
                std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                    AuthScheme::Bearer,
                    secrecy::SecretString::from("test-key"),
                )),
                AuthProvenanceSource::Static,
            )
            .expect("register");
        assert_eq!(handle.api_id().as_str(), "openrouter-images");
        assert_eq!(api_id.as_str(), "openrouter-images");
        assert_eq!(provider_id, "openrouter");
        handle
    };
    let native = handle.options(OpenRouterImagesOptions {
        max_retries: 1,
        max_retry_delay_ms: None,
    });

    let result = models
        .generate_images_native(
            record,
            ImagesContext {
                input: vec![ImagesInput::Text { text: "hi".into() }],
            },
            images_call(),
            native,
        )
        .await;

    assert_eq!(result.stop_reason, ImagesStopReason::Stop);
    assert_eq!(
        result.output,
        vec![ImagesOutput::Image {
            mime_type: "image/png".into(),
            data: "QUJD".into()
        }]
    );

    let captured = bodies.lock().expect("bodies").clone();
    assert_eq!(captured.len(), 2);
    assert_eq!(captured[0], captured[1], "retry reuses frozen payload");
}

#[tokio::test]
async fn images_retry_delay_above_cap_fails_typed() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after-ms", "120000")
                .set_body_json(serde_json::json!({
                    "error": { "message": "Rate limited" }
                })),
        )
        .expect(1)
        .mount(&server)
        .await;

    let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
    let mut models = image_models(&record);
    let handle = models
        .register_image_api(
            std::sync::Arc::new(OpenRouterImagesApi::new(HttpClient::new(), record.clone())),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                secrecy::SecretString::from("test-key"),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register");
    let native = handle.options(OpenRouterImagesOptions {
        max_retries: 2,
        max_retry_delay_ms: None,
    });

    let result = models
        .generate_images_native(
            record,
            ImagesContext {
                input: vec![ImagesInput::Text { text: "hi".into() }],
            },
            images_call(),
            native,
        )
        .await;

    assert_eq!(result.stop_reason, ImagesStopReason::Error);
    let message = result.error_message.expect("error message");
    assert!(
        message.contains("retry delay (max: 60s)"),
        "message: {message}"
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

/// Manual server that answers headers plus a partial body, then hangs: the
#[tokio::test]
async fn images_timeout_is_body_inclusive() {
    let base = spawn_hanging_body_images_server().await;
    let record = image_record(base, vec![ImageOutputModality::Image]);
    let models = image_models(&record);

    let started = Instant::now();
    let result = models
        .generate_images(
            record,
            ImagesContext {
                input: vec![ImagesInput::Text { text: "hi".into() }],
            },
            ImagesCall {
                timeout: Some(Duration::from_millis(300)),
                ..Default::default()
            },
        )
        .await;

    assert_eq!(result.stop_reason, ImagesStopReason::Error);
    assert!(matches!(
        result.error.as_ref().expect("typed"),
        ProviderError::Timeout
    ));
    assert!(started.elapsed() >= Duration::from_millis(300));
}

#[tokio::test]
async fn images_zero_timeout_rejects_before_request() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200))
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
                timeout: Some(Duration::ZERO),
                ..Default::default()
            },
        )
        .await;

    assert_eq!(result.stop_reason, ImagesStopReason::Error);
    assert!(matches!(
        result.error.as_ref().expect("typed"),
        ProviderError::Config(_)
    ));
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .to_lowercase()
            .contains("positive"),
        "message: {:?}",
        result.error_message
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn images_cancellation_maps_to_aborted() {
    let base = spawn_hanging_body_images_server().await;
    let record = image_record(base, vec![ImageOutputModality::Image]);
    let models = image_models(&record);

    let cancel = tokio_util::sync::CancellationToken::new();
    let call = ImagesCall {
        timeout: Some(Duration::from_secs(30)),
        cancel: cancel.clone(),
        ..Default::default()
    };
    let models = std::sync::Arc::new(models);
    let spawned = std::sync::Arc::clone(&models);
    let task = tokio::spawn(async move {
        spawned
            .generate_images(
                record,
                ImagesContext {
                    input: vec![ImagesInput::Text { text: "hi".into() }],
                },
                call,
            )
            .await
    });
    tokio::time::sleep(Duration::from_millis(100)).await;
    cancel.cancel();
    let result = task.await.expect("task joins");

    assert_eq!(result.stop_reason, ImagesStopReason::Aborted);
}

/// Resolver that always fails: pins the missing-key pre-dispatch rejection.
struct FailingAuthResolver;

impl AuthResolver for FailingAuthResolver {
    fn resolve<'a>(
        &'a self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<ResolvedAuth, ProviderError>> + Send + 'a>,
    > {
        Box::pin(async {
            Err(ProviderError::CredentialNeeded {
                provider_id: "openrouter".into(),
            })
        })
    }
}

#[tokio::test]
async fn images_missing_key_rejects_typed_before_dispatch() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
    let mut models = Models::new();
    models
        .register_image_api(
            std::sync::Arc::new(OpenRouterImagesApi::new(HttpClient::new(), record.clone())),
            std::sync::Arc::new(FailingAuthResolver),
            AuthProvenanceSource::Static,
        )
        .expect("register");

    let result = models
        .generate_images(
            record,
            ImagesContext {
                input: vec![ImagesInput::Text { text: "hi".into() }],
            },
            images_call(),
        )
        .await;

    assert_eq!(result.stop_reason, ImagesStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("No API key for provider: openrouter"),
        "message: {:?}",
        result.error_message
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn images_wrong_operation_rejects_before_auth() {
    let resolver_count = std::sync::Arc::new(AtomicU32::new(0));
    let mut models = Models::new();
    let record = default_openrouter_image_record();
    models
        .register_image_api(
            std::sync::Arc::new(OpenRouterImagesApi::new(HttpClient::new(), record.clone())),
            std::sync::Arc::new(opi_ai::test_support::CountingAuthResolver::new(
                resolver_count.clone(),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register");

    // A chat model from the registered OpenRouter chat profile is not an
    // image model: operation-qualified selection rejects typed.
    let mut chat_models =
        opi_ai::test_support::single_route_collection(Box::new(openrouter_provider()));
    chat_models
        .register_image_api(
            std::sync::Arc::new(OpenRouterImagesApi::new(HttpClient::new(), record.clone())),
            std::sync::Arc::new(opi_ai::test_support::CountingAuthResolver::new(
                resolver_count.clone(),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register");
    let chat_err = chat_models
        .image_model("openrouter", "openai/gpt-4o")
        .expect_err("chat model is not an image model");
    assert!(
        chat_err.to_string().contains("not an image model"),
        "error: {chat_err}"
    );

    // Unknown model ids also fail selection.
    let unknown = models
        .image_model("openrouter", "no-such-model")
        .expect_err("unknown model id");
    assert!(unknown.to_string().contains("no-such-model"));

    // The curated image record resolves and dispatch stays image-only.
    let selected = models
        .image_model("openrouter", &record.id)
        .expect("image record resolves");
    assert_eq!(selected.id, record.id);
    assert_eq!(
        resolver_count.load(AtomicOrdering::SeqCst),
        0,
        "selection must not resolve auth"
    );
}

#[tokio::test]
async fn images_unsupported_callbacks_reject_typed_before_dispatch() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
    let models = image_models(&record);

    let raw_callbacks =
        opi_ai::callbacks::RequestCallbacks::new().with_on_raw(std::sync::Arc::new(
            |_event: &serde_json::Value| Ok::<(), opi_ai::callbacks::CallbackError>(()),
        ));
    let raw_result = models
        .generate_images(
            record.clone(),
            ImagesContext {
                input: vec![ImagesInput::Text { text: "hi".into() }],
            },
            ImagesCall {
                callbacks: raw_callbacks,
                ..images_call()
            },
        )
        .await;
    assert_eq!(raw_result.stop_reason, ImagesStopReason::Error);
    assert!(matches!(
        raw_result.error.as_ref().expect("typed"),
        ProviderError::UnsupportedCapability { .. }
    ));

    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn image_preflight_rejects_headers_callbacks_and_timeout_before_auth() {
    for invalid in ["headers", "callbacks", "timeout"] {
        let server = wiremock::MockServer::start().await;
        let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
        let auth_calls = std::sync::Arc::new(AtomicU32::new(0));
        let mut models = Models::new();
        models
            .register_image_api(
                std::sync::Arc::new(OpenRouterImagesApi::new(HttpClient::new(), record.clone())),
                std::sync::Arc::new(opi_ai::test_support::CountingAuthResolver::new(
                    auth_calls.clone(),
                )),
                AuthProvenanceSource::Static,
            )
            .unwrap();
        let mut call = images_call();
        match invalid {
            "headers" => {
                call.extra_headers = vec![("Authorization".into(), "fixture-override".into())]
            }
            "callbacks" => {
                call.callbacks = opi_ai::callbacks::RequestCallbacks::new()
                    .with_on_raw(std::sync::Arc::new(|_| Ok(())))
            }
            "timeout" => call.timeout = Some(Duration::ZERO),
            _ => unreachable!(),
        }
        let result = models
            .generate_images(
                record,
                ImagesContext {
                    input: vec![ImagesInput::Text {
                        text: "fixture".into(),
                    }],
                },
                call,
            )
            .await;
        assert!(result.error.is_some());
        assert_eq!(
            auth_calls.load(AtomicOrdering::SeqCst),
            0,
            "{invalid} rejects before auth"
        );
        assert!(server.received_requests().await.unwrap().is_empty());
    }
}

#[tokio::test]
async fn image_custom_native_option_validation_runs_before_auth() {
    struct CountBearer(std::sync::Arc<AtomicU32>);
    impl AuthResolver for CountBearer {
        fn resolve<'a>(&'a self) -> opi_ai::BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
            Box::pin(async {
                self.0.fetch_add(1, AtomicOrdering::SeqCst);
                Ok(opi_ai::test_support::resolved_bearer_auth())
            })
        }
    }
    struct CheckedImages(OpenRouterImagesApi);
    impl ImageApi<bool> for CheckedImages {
        fn api_id(&self) -> &opi_ai::ApiId {
            self.0.api_id()
        }
        fn provider_id(&self) -> &str {
            self.0.provider_id()
        }
        fn complete_record(&self) -> &ImageModelRecord {
            self.0.complete_record()
        }
        fn default_options(&self) -> bool {
            true
        }
        fn validate_options(&self, allowed: &bool) -> Result<(), ProviderError> {
            if *allowed {
                Ok(())
            } else {
                Err(ProviderError::Config(
                    opi_ai::provider::ProviderErrorSummary::sanitized(
                        "fixture native options rejected",
                    ),
                ))
            }
        }
        fn generate_images_prepared<'a>(
            &'a self,
            record: ImageModelRecord,
            context: ImagesContext,
            _options: bool,
            auth: ResolvedAuth,
            call: ImagesCall,
        ) -> opi_ai::api::ImageFuture<'a> {
            self.0.generate_images_prepared(
                record,
                context,
                OpenRouterImagesOptions::default(),
                auth,
                call,
            )
        }
    }
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"choices":[]})))
        .mount(&server)
        .await;
    let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
    let auth_calls = std::sync::Arc::new(AtomicU32::new(0));
    let mut models = Models::new();
    let api = models
        .register_image_api(
            std::sync::Arc::new(CheckedImages(OpenRouterImagesApi::new(
                HttpClient::new(),
                record.clone(),
            ))),
            std::sync::Arc::new(CountBearer(auth_calls.clone())),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let context = ImagesContext {
        input: vec![ImagesInput::Text {
            text: "fixture".into(),
        }],
    };
    let rejected = models
        .generate_images_native(
            record.clone(),
            context.clone(),
            images_call(),
            api.options(false),
        )
        .await;
    assert!(matches!(rejected.error, Some(ProviderError::Config(_))));
    assert_eq!(auth_calls.load(AtomicOrdering::SeqCst), 0);
    assert!(server.received_requests().await.unwrap().is_empty());
    let accepted = models
        .generate_images_native(record, context, images_call(), api.options(true))
        .await;
    assert_eq!(accepted.stop_reason, ImagesStopReason::Stop);
    assert_eq!(auth_calls.load(AtomicOrdering::SeqCst), 1);
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn image_call_owns_binding_before_first_poll() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"choices": []})))
        .expect(2)
        .mount(&server)
        .await;
    let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
    let mut models = image_models(&record);
    let ordinary = models.generate_images(record.clone(), ImagesContext::default(), images_call());
    let handle = models
        .register_image_api(
            std::sync::Arc::new(OpenRouterImagesApi::new(HttpClient::new(), record.clone())),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                secrecy::SecretString::from("native-key"),
            )),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let native = models.generate_images_native(
        record,
        ImagesContext::default(),
        images_call(),
        handle.options(Default::default()),
    );
    assert!(server.received_requests().await.unwrap().is_empty());
    drop(models);
    assert_eq!(ordinary.await.stop_reason, ImagesStopReason::Stop);
    assert_eq!(native.await.stop_reason, ImagesStopReason::Stop);
    let requests = server.received_requests().await.unwrap();
    assert_eq!(
        requests[0].headers.get("authorization").unwrap(),
        "Bearer test-key"
    );
    assert_eq!(
        requests[1].headers.get("authorization").unwrap(),
        "Bearer native-key"
    );
}

#[tokio::test]
async fn image_auth_lifecycle_preaborted_never_starts_resolution() {
    let resolver = std::sync::Arc::new(pending_auth::PendingAuth::default());
    let record = image_record(
        "http://127.0.0.1:1".into(),
        vec![ImageOutputModality::Image],
    );
    let mut models = Models::new();
    models
        .register_image_api(
            std::sync::Arc::new(OpenRouterImagesApi::new(HttpClient::new(), record.clone())),
            resolver.clone(),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let call = images_call();
    call.cancel.cancel();
    let result = models.generate_images(record, ImagesContext::default(), call);
    tokio::pin!(result);
    let outcome = futures_util::poll!(&mut result);
    assert_eq!(
        resolver.calls.load(AtomicOrdering::SeqCst),
        0,
        "pre-aborted generation must not start auth"
    );
    let std::task::Poll::Ready(result) = outcome else {
        panic!("pre-aborted image call must settle")
    };
    assert_eq!(result.stop_reason, ImagesStopReason::Aborted);
    assert!(matches!(result.error, Some(ProviderError::Cancelled)));
}

#[tokio::test]
async fn image_auth_lifecycle_cancel_releases_pending_resolution() {
    let resolver = std::sync::Arc::new(pending_auth::PendingAuth::default());
    let record = image_record(
        "http://127.0.0.1:1".into(),
        vec![ImageOutputModality::Image],
    );
    let mut models = Models::new();
    models
        .register_image_api(
            std::sync::Arc::new(OpenRouterImagesApi::new(HttpClient::new(), record.clone())),
            resolver.clone(),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let call = images_call();
    let cancel = call.cancel.clone();
    let result = models.generate_images(record, ImagesContext::default(), call);
    tokio::pin!(result);
    assert!(futures_util::poll!(&mut result).is_pending());
    assert_eq!(resolver.calls.load(AtomicOrdering::SeqCst), 1);
    cancel.cancel();
    let std::task::Poll::Ready(result) = futures_util::poll!(&mut result) else {
        panic!("cancellation must interrupt pending image auth")
    };
    assert_eq!(result.stop_reason, ImagesStopReason::Aborted);
    assert!(matches!(result.error, Some(ProviderError::Cancelled)));
    assert!(resolver.released.load(AtomicOrdering::SeqCst));
}
