//! Pi Messages HTTP, auth, cancellation and Models lifecycle assertions.

use super::*;

// ---------------------------------------------------------------------------
// HTTP failures
// ---------------------------------------------------------------------------

#[tokio::test]
async fn http_errors_format_status_message_code() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(503).set_body_string(
            "{\"error\":{\"message\":\"upstream down\",\"code\":\"rate_limited\"}}",
        ))
        .expect(1)
        .mount(&server)
        .await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let events = run_44(&provider, req_44("hi")).await;
    let error = first_error_44(&events).await;
    match error {
        ProviderError::ServiceFault { detail, .. } => {
            let text = detail.to_string();
            assert!(
                text.contains("503 Service Unavailable: upstream down (rate_limited)"),
                "got {text}"
            );
        }
        other => panic!("expected ServiceFault, got {other}"),
    }
    server.verify().await;
}

#[tokio::test]
async fn http_error_falls_back_to_raw_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(500).set_body_string("plain boom"))
        .expect(1)
        .mount(&server)
        .await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let events = run_44(&provider, req_44("hi")).await;
    let error = first_error_44(&events).await;
    match error {
        ProviderError::ServiceFault { detail, .. } => {
            let text = detail.to_string();
            assert!(
                text.contains("500 Internal Server Error: plain boom"),
                "got {text}"
            );
        }
        other => panic!("expected ServiceFault, got {other}"),
    }
    server.verify().await;
}

#[tokio::test]
async fn rate_limit_keeps_retryable_identity() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(429).set_body_string("{\"error\":{\"message\":\"slow\"}}"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let events = run_44(&provider, req_44("hi")).await;
    let error = first_error_44(&events).await;
    assert!(
        matches!(error, ProviderError::RateLimited { .. }),
        "got {error}"
    );
    server.verify().await;
}

#[tokio::test]
async fn empty_body_response_fails_typed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let events = run_44(&provider, req_44("hi")).await;
    let error = first_error_44(&events).await;
    match error {
        ProviderError::StreamError(summary) => {
            let text = summary.to_string();
            assert!(text.contains("no body"), "got {text}");
        }
        other => panic!("expected StreamError, got {other}"),
    }
    server.verify().await;
}

#[tokio::test]
async fn cancel_mid_stream_fails_cancelled() {
    let server = MockServer::start().await;
    mount_44(
        &server,
        ResponseTemplate::new(200)
            .set_body_string(concat!(
                "data: {\"type\":\"start\"}\n\n",
                "data: {\"type\":\"text_start\",\"contentIndex\":0}\n\n",
                "data: {\"type\":\"text_delta\",\"contentIndex\":0,\"delta\":\"chunk\"}\n\n",
            ))
            .insert_header("content-type", "text/event-stream")
            .set_delay(std::time::Duration::from_secs(30)),
    )
    .await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let request = req_44("hi");
    let cancel = request.cancel.clone();
    let stream = provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
    let handle = tokio::spawn(async move {
        let events: Vec<_> = stream.collect().await;
        events
    });
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    cancel.cancel();
    let events = handle.await.expect("join");
    assert_eq!(first_error_44(&events).await, ProviderError::Cancelled);
}

#[tokio::test]
async fn non_bearer_auth_is_rejected_typed() {
    let server = MockServer::start().await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let auth = opi_ai::auth::ResolvedAuth {
        env: Default::default(),
        scheme: opi_ai::auth::AuthScheme::ApiKey,
        secret: SecretString::from("key"),
        base_url: None,
        account_id: None,
        provenance: Default::default(),
    };
    let request = req_44("hi");
    let mut stream = provider.stream_prepared(request.clone(), auth);
    let error = stream
        .next()
        .await
        .expect("one refusal")
        .expect_err("wrong auth scheme");
    let ProviderError::StreamFailure(failure) = error else {
        panic!("accepted projection must retain its failure carrier")
    };
    let ProviderError::Config(summary) = failure.cause() else {
        panic!("wrong underlying auth refusal: {:?}", failure.cause())
    };
    assert_eq!(
        summary.as_str(),
        "pi-messages accepts only Bearer authentication"
    );
    assert_direct_projection_prefix(failure.message(), &request);
    assert!(stream.next().await.is_none());
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn unknown_model_fails_typed() {
    let provider = PiMessagesProvider::new(None)
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let mut request = req_44("hi");
    request.model = "radius:nonexistent".into();
    let events = run_44(&provider, request).await;
    let error = first_error_44(&events).await;
    match error {
        ProviderError::Config(summary) => {
            let text = summary.to_string();
            assert!(text.contains("nonexistent"), "got {text}");
        }
        other => panic!("expected Config, got {other}"),
    }
}

// ---------------------------------------------------------------------------
// simple bypass and lifecycle
// ---------------------------------------------------------------------------

#[tokio::test]
async fn simple_passthrough_is_unclamped_and_omission_preserving() {
    let _guard = env_lock_44().lock().await;
    let server = MockServer::start().await;
    mount_44(&server, sse_ok_44()).await;
    let provider =
        PiMessagesProvider::new(Some(server.uri())).with_models(vec![clamped_model_44()]);
    let mut request = req_44("hi");
    request.thinking = ThinkingConfig {
        enabled: true,
        level: ThinkingLevel::High,
        budget_tokens: None,
    };
    let stream = provider.stream_simple(request, opi_ai::test_support::resolved_bearer_auth());
    let events: Vec<_> = stream.collect().await;
    assert!(
        events
            .iter()
            .any(|item| matches!(item, Ok(AssistantStreamEvent::Done { .. })))
    );

    let body = body_44(&server).await;
    assert_eq!(
        body["options"]["reasoning"],
        serde_json::json!("high"),
        "the simple bypass forwards the requested level verbatim; the record's map never clamps"
    );
    assert!(
        body["options"].get("maxTokens").is_none(),
        "omitted maxTokens stays omitted — no default injection"
    );
    server.verify().await;
}

#[test]
fn wrong_wire_record_is_rejected_typed() {
    let provider = PiMessagesProvider::new(None);
    let openai_model = opi_ai::provider::ModelInfo::new(
        MODEL_44,
        MODEL_44,
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(32_000, 8_192),
    );
    let error =
        PiMessagesChatApi::new(provider, openai_model).expect_err("wrong wire must be rejected");
    assert!(matches!(error, ProviderError::Config(_)), "got {error}");
}

#[tokio::test]
async fn native_lifecycle_through_models_complete() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .and(header("authorization", "Bearer radius-key"))
        .and(body_partial_json(serde_json::json!({
            "model": MODEL_44,
            "options": {"reasoning": "high"}
        })))
        .respond_with(sse_ok_44())
        .expect(1)
        .mount(&server)
        .await;

    let model = thinking_model_44();
    let api = PiMessagesChatApi::new(
        PiMessagesProvider::new(Some(server.uri())).with_models(vec![model.clone()]),
        model.clone(),
    )
    .expect("valid pi-messages record");
    let mut models = opi_ai::models::Models::new();
    let handle = models
        .register_chat_api(
            std::sync::Arc::new(api),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::Bearer,
                SecretString::from("radius-key"),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .expect("register");

    let mut request = req_44("go");
    request.model = format!("radius:{MODEL_44}");
    request.thinking = ThinkingConfig {
        enabled: true,
        level: ThinkingLevel::High,
        budget_tokens: None,
    };
    let options = PiMessagesOptions::native(Some(ThinkingLevel::High), None, false);
    let message = models
        .complete(
            opi_ai::models::ChatModelRecord::new("radius", model.clone()),
            request,
            handle.options(options),
        )
        .await
        .expect("complete succeeds");
    assert_eq!(message.stop_reason, StopReason::Stop);
    assert_eq!(message.response_id.as_deref(), Some("resp-1"));
    server.verify().await;
}
