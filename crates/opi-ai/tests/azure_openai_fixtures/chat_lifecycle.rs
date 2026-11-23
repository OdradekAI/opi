//! Azure OpenAI Chat lifecycle and transport assertions.

use super::*;

#[tokio::test]
async fn stream_drains_text_lifecycle_through_http() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/openai/deployments/my-gpt4o/chat/completions"))
        .and(query_param("api-version", "2024-06-01"))
        .and(header("api-key", "test-api-key-12345"))
        .and(body_partial_json(serde_json::json!({
            "model": "my-gpt4o",
            "messages": [
                {"role": "system", "content": "You are helpful."},
                {"role": "user", "content": "Hello"}
            ],
            "max_completion_tokens": 256
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_sse_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = AzureOpenAIProvider::new(
        Some(server.uri()),
        "my-gpt4o".into(),
        Some("2024-06-01".into()),
    )
    .unwrap();

    // The api-key header is supplied via ResolvedAuth (Phase 17.5); the
    // production transport derives the `api-key` header from `auth.secret`
    // in stream_prepared.
    let auth = opi_ai::auth::ResolvedAuth {
        env: Default::default(),
        scheme: opi_ai::auth::AuthScheme::ApiKey,
        secret: secrecy::SecretString::from("test-api-key-12345"),
        base_url: None,
        account_id: None,
        provenance: opi_ai::AuthProvenance::default(),
    };
    let events = collect_events(provider.stream_prepared(text_request(), auth)).await;

    // Lifecycle: Start -> TextDelta(s) -> Done.
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::Start { .. })),
        "should emit Start through the HTTP adapter path"
    );
    let text: String = events
        .iter()
        .filter_map(|e| match e {
            AssistantStreamEvent::TextDelta { delta, .. } => Some(delta.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(text, "Hi there", "should stream the full text content");
    let done = events
        .last()
        .expect("stream should produce a terminal event");
    match done {
        AssistantStreamEvent::Done { reason, .. } => {
            // OpenAI `stop` finish reason must map to the shared StopReason::Stop.
            assert_eq!(*reason, opi_ai::stream::StopReason::Stop);
        }
        other => panic!("expected Done, got {other:?}"),
    }

    // The matchers above (path + api-version query + api-key header + body
    // shape) ARE the request assertion; verify() confirms the production
    // request carried all of them.
    server.verify().await;
}

#[tokio::test]
async fn auth_error_bodies_are_absent_from_public_errors() {
    let canaries = [
        "azure-access-canary-with-no-known-token-shape",
        "azure-secret-canary-with-no-known-token-shape",
        "azure-session-canary-with-no-known-token-shape",
        "azure-token-canary-with-no-known-token-shape",
    ];
    let body = canaries.join(" ");

    for status in [401, 403] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/openai/deployments/my-gpt4o/chat/completions"))
            .respond_with(ResponseTemplate::new(status).set_body_string(body.clone()))
            .mount(&server)
            .await;

        let provider = AzureOpenAIProvider::new(
            Some(server.uri()),
            "my-gpt4o".into(),
            Some("2024-06-01".into()),
        )
        .unwrap();

        let mut stream =
            provider.stream_prepared(text_request(), opi_ai::test_support::resolved_auth());
        let error = stream
            .next()
            .await
            .expect("auth failure should produce an event")
            .expect_err("auth failure should produce ProviderError");
        assert!(matches!(
            direct_error_cause(&error),
            opi_ai::provider::ProviderError::AuthFailed(_)
        ));
        let rendered = format!("{error} {error:?}");
        for canary in canaries {
            assert!(
                !rendered.contains(canary),
                "Azure HTTP {status} body leaked through ProviderError: {rendered}"
            );
        }
    }
}

#[tokio::test]
async fn request_enrichment_reaches_azure_http_boundary() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/openai/deployments/my-gpt4o/chat/completions"))
        .and(header("api-key", "test-api-key-12345"))
        .and(header("content-type", "application/json"))
        .and(header("x-opi-request", "azure"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_sse_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
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
    let mut request = text_request();
    request.extra_headers = vec![("X-Opi-Request".into(), "azure".into())];

    let events = collect_events(provider.stream_prepared(request, auth)).await;

    assert!(
        events
            .iter()
            .any(|event| matches!(event, AssistantStreamEvent::Done { .. }))
    );
    server.verify().await;
}

#[tokio::test]
async fn request_timeout_maps_to_typed_timeout_at_azure_boundary() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_millis(200)))
        .mount(&server)
        .await;
    let provider = AzureOpenAIProvider::new(
        Some(server.uri()),
        "my-gpt4o".into(),
        Some("2024-06-01".into()),
    )
    .unwrap();
    let mut request = text_request();
    request.timeout = Some(std::time::Duration::from_millis(20));
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());

    let result = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
        .await
        .expect("Azure request timeout must resolve promptly")
        .expect("Azure timeout must produce a stream item");

    assert!(matches!(
        result,
        Err(ref error) if matches!(direct_error_cause(error), opi_ai::provider::ProviderError::Timeout)
    ));
}

#[tokio::test]
async fn direct_auth_statuses_do_not_wait_for_stalled_bodies() {
    for status in [401, 403] {
        let (server, headers_flushed) = spawn_stalled_azure_error_body_server(status).await;
        let provider =
            AzureOpenAIProvider::new(Some(server), "my-gpt4o".into(), Some("2024-06-01".into()))
                .unwrap();
        let mut stream =
            provider.stream_prepared(text_request(), opi_ai::test_support::resolved_auth());

        tokio::time::timeout(
            std::time::Duration::from_secs(2),
            headers_flushed.notified(),
        )
        .await
        .expect("Azure auth-error headers must be flushed before the body stalls");

        let result = tokio::time::timeout(std::time::Duration::from_millis(500), stream.next())
            .await
            .expect("Azure direct auth status must not wait for its body")
            .expect("Azure direct auth status must produce a stream item");

        let error = result.expect_err("Azure direct auth status must fail");
        assert!(matches!(
            direct_error_cause(&error),
            opi_ai::provider::ProviderError::AuthFailed(_)
        ));
        assert_eq!(
            error.to_string(),
            "authentication failed: provider rejected credentials"
        );
    }
}

#[tokio::test]
async fn request_header_cannot_override_azure_auth_routing() {
    let server = MockServer::start().await;
    let provider = AzureOpenAIProvider::new(
        Some(server.uri()),
        "my-gpt4o".into(),
        Some("2024-06-01".into()),
    )
    .unwrap();
    let mut request = text_request();
    request.extra_headers = vec![("api-key".into(), "override".into())];
    let mut stream =
        provider.stream_prepared(request.clone(), opi_ai::test_support::resolved_auth());
    let error = stream
        .next()
        .await
        .expect("one refusal")
        .expect_err("managed header override");
    let ProviderError::StreamFailure(failure) = &error else {
        panic!("accepted projection must retain its failure carrier")
    };
    assert!(matches!(failure.cause(), ProviderError::RequestFailed(_)));
    assert_direct_projection_prefix(failure.message(), &request);
    assert!(stream.next().await.is_none());
    assert!(
        server
            .received_requests()
            .await
            .expect("recorded requests")
            .is_empty(),
        "reserved Azure request headers must fail before dispatch"
    );
}

// ---------------------------------------------------------------------------
// Phase 12 task 12.3 — Azure inherits the shared compat profile path
// ---------------------------------------------------------------------------

#[test]
fn azure_inherits_shared_compat_flags_via_with_compat() {
    // Azure is a first-class provider that routes request-body serialization
    // through the shared OpenAI Chat adapter. With a developer-role + strict-
    // tools compat applied via with_compat, its request body reflects both
    // flags through the shared serializer (DoD), not an Azure-specific one.
    use opi_ai::providers::openai::CompatConfig;

    let provider = make_provider().with_compat(CompatConfig {
        instruction_role: Some(opi_ai::model_info::ChatInstructionRole::Developer),
        strict_tool_schema: true,
        ..Default::default()
    });
    // text_request carries a system message -> developer role override applies.
    let body_text = wire_adapter(&provider)
        .build_request_body(&text_request())
        .unwrap();
    let messages = body_text["messages"].as_array().unwrap();
    assert_eq!(
        messages[0]["role"], "developer",
        "Azure inherits developer-role override from the shared compat path"
    );
    // tool_request carries tools -> strict flag applies.
    let body_tool = wire_adapter(&provider)
        .build_request_body(&tool_request())
        .unwrap();
    let tools = body_tool["tools"].as_array().expect("tools present");
    assert!(
        tools[0]["function"]["strict"] == false,
        "Azure inherits strict capability while an unconstrained tool explicitly stays non-strict"
    );
}

// ---------------------------------------------------------------------------
// Provider stream cancellation (Phase 12 task 12.7 DoD clause 6)
// ---------------------------------------------------------------------------

async fn assert_azure_cancelled(stall_point: AzureStallPoint) {
    let (server, stalled) = spawn_stalled_azure_server(stall_point).await;
    let cancel = CancellationToken::new();
    let provider =
        AzureOpenAIProvider::new(Some(server), "my-gpt4o".into(), Some("2024-06-01".into()))
            .unwrap();
    let mut request = text_request();
    request.cancel = cancel.clone();
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());

    tokio::time::timeout(std::time::Duration::from_secs(1), stalled.notified())
        .await
        .expect("Azure server must reach the selected stall point");
    cancel.cancel();

    let remaining = tokio::time::timeout(std::time::Duration::from_secs(1), async move {
        let mut remaining = Vec::new();
        while let Some(item) = stream.next().await {
            remaining.push(item);
        }
        remaining
    })
    .await
    .expect("Azure cancellation must terminate without waiting for HTTP");
    assert!(
        matches!(
            remaining.as_slice(),
            [Err(cause)] if cause.category() == opi_ai::provider::ProviderErrorCategory::Cancelled
        ),
        "Azure cancellation must yield exactly one typed error, got {remaining:?}"
    );
}

#[tokio::test]
async fn cancellation_before_response_headers_is_typed_and_prompt() {
    assert_azure_cancelled(AzureStallPoint::BeforeHeaders).await;
}

#[tokio::test]
async fn cancellation_during_response_body_is_typed_and_prompt() {
    assert_azure_cancelled(AzureStallPoint::ResponseBody).await;
}

// ---------------------------------------------------------------------------
// Task 20.51: callbacks flow through the shared Responses wire (P20-S51)
// ---------------------------------------------------------------------------
