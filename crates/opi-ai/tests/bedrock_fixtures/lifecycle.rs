//! Bedrock lifecycle, auth-source and transport contract tests.

use super::*;

#[path = "aws_credentials.rs"]
mod aws_credentials;
#[path = "aws_sso.rs"]
mod aws_sso;

#[derive(Clone, Copy)]
enum BedrockStallPoint {
    BeforeHeaders,
    ResponseBody,
}

async fn spawn_stalled_bedrock_server(stall_point: BedrockStallPoint) -> (String, Arc<Notify>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind stalled Bedrock server");
    let addr = listener.local_addr().expect("stalled Bedrock server addr");
    let stalled = Arc::new(Notify::new());
    let server_stalled = stalled.clone();

    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept Bedrock request");
        let mut request = Vec::new();
        let mut buf = [0_u8; 1024];
        loop {
            let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                .await
                .expect("read Bedrock request");
            if read == 0 {
                return;
            }
            request.extend_from_slice(&buf[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }

        if matches!(stall_point, BedrockStallPoint::ResponseBody) {
            tokio::io::AsyncWriteExt::write_all(
                &mut socket,
                b"HTTP/1.1 200 OK\r\nContent-Type: application/vnd.amazon.eventstream\r\nContent-Length: 1024\r\nConnection: close\r\n\r\n",
            )
            .await
            .expect("write Bedrock response headers");
            tokio::io::AsyncWriteExt::flush(&mut socket)
                .await
                .expect("flush Bedrock response headers");
        }
        server_stalled.notify_one();
        std::future::pending::<()>().await;
    });

    (format!("http://{addr}"), stalled)
}

async fn assert_bedrock_cancelled(stall_point: BedrockStallPoint) {
    let (server, stalled) = spawn_stalled_bedrock_server(stall_point).await;
    let cancel = CancellationToken::new();
    let provider = BedrockProvider::new(Some(server), Arc::new(HttpClient::new()));
    let mut request = lifecycle_text_request();
    request.cancel = cancel.clone();
    let mut stream = provider.stream_prepared(request, test_auth());

    tokio::time::timeout(std::time::Duration::from_secs(1), stalled.notified())
        .await
        .expect("Bedrock server must reach the selected stall point");
    cancel.cancel();

    let remaining = tokio::time::timeout(std::time::Duration::from_secs(1), async move {
        let mut remaining = Vec::new();
        while let Some(item) = stream.next().await {
            remaining.push(item);
        }
        remaining
    })
    .await
    .expect("Bedrock cancellation must terminate without waiting for HTTP");
    assert!(
        matches!(remaining.as_slice(), [Err(cause)] if cause.category() == opi_ai::provider::ProviderErrorCategory::Cancelled),
        "Bedrock cancellation must yield exactly one typed error, got {remaining:?}"
    );
}

#[tokio::test]
async fn cancellation_before_response_headers_is_typed_and_prompt() {
    assert_bedrock_cancelled(BedrockStallPoint::BeforeHeaders).await;
}

#[tokio::test]
async fn cancellation_during_response_body_is_typed_and_prompt() {
    assert_bedrock_cancelled(BedrockStallPoint::ResponseBody).await;
}

#[tokio::test]
async fn stream_drains_text_lifecycle_through_http() {
    let body_bytes = bedrock_text_lifecycle_bytes();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/model/anthropic.claude-sonnet-4/converse-stream"))
        .and(body_partial_json(serde_json::json!({
            "messages": [{"role": "user", "content": [{"text": "Hello"}]}],
            "system": [{"text": "You are helpful."}],
            "inferenceConfig": {"maxTokens": 1024}
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(body_bytes, "application/vnd.amazon.eventstream"),
        )
        .mount(&server)
        .await;

    let provider = BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()));

    let events =
        collect_events(provider.stream_prepared(lifecycle_text_request(), test_auth())).await;

    // Lifecycle: Start -> TextDelta("Hello!") -> Done.
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::Start { .. })),
        "should emit Start through the HTTP adapter path"
    );
    assert!(
        events.iter().any(|e| matches!(
            e,
            AssistantStreamEvent::TextDelta { delta, .. } if delta == "Hello!"
        )),
        "should emit TextDelta carrying the streamed text"
    );
    let done = events
        .last()
        .expect("stream should produce a terminal event");
    match done {
        AssistantStreamEvent::Done { reason, message } => {
            // Bedrock `end_turn` must map to the shared StopReason::Stop.
            assert_eq!(*reason, StopReason::Stop);
            assert_eq!(message.usage.input_tokens, 10);
            assert_eq!(message.usage.output_tokens, 5);
        }
        other => panic!("expected Done, got {other:?}"),
    }

    // The body+path matchers above are the request-shape assertion; verify()
    // confirms the production request actually carried that Converse body and
    // the signed SigV4 header set.
    server.verify().await;
    let received = server
        .received_requests()
        .await
        .expect("should have recorded the provider request");
    let recorded = &received[0];
    assert!(
        recorded.headers.contains_key("authorization"),
        "Bedrock request must carry a SigV4 authorization header"
    );
    assert!(
        recorded.headers.contains_key("x-amz-date"),
        "Bedrock request must carry an x-amz-date header"
    );
    assert!(
        recorded.headers.contains_key("x-amz-content-sha256"),
        "Bedrock request must carry an x-amz-content-sha256 header"
    );
}

#[tokio::test]
async fn stream_http_flushes_done_without_metadata() {
    let body_bytes = build_bedrock_stream(&[
        ("messageStart", r#"{"role":"assistant"}"#),
        (
            "contentBlockStart",
            r#"{"start":{"text":{}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"text":"Hello without metadata"},"contentBlockIndex":0}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":0}"#),
        ("messageStop", r#"{"stopReason":"end_turn"}"#),
    ]);
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/model/anthropic.claude-sonnet-4/converse-stream"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(body_bytes, "application/vnd.amazon.eventstream"),
        )
        .mount(&server)
        .await;

    let provider = BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()));

    let events =
        collect_events(provider.stream_prepared(lifecycle_text_request(), test_auth())).await;

    assert!(
        events
            .iter()
            .any(|event| matches!(event, AssistantStreamEvent::Done { .. })),
        "HTTP Bedrock stream must flush pending Done when metadata is absent"
    );
}

#[tokio::test]
async fn terminal_stream_with_truncated_trailer_fails_closed() {
    let mut body_bytes = build_bedrock_stream(&[
        ("messageStart", r#"{"role":"assistant"}"#),
        (
            "contentBlockStart",
            r#"{"start":{"text":{}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"text":"partial terminal"},"contentBlockIndex":0}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":0}"#),
        ("messageStop", r#"{"stopReason":"end_turn"}"#),
    ]);
    let trailer = event_stream::build_test_frame("metadata", "application/json", b"{}");
    body_bytes.extend_from_slice(&trailer[..trailer.len() - 1]);

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/model/anthropic.claude-sonnet-4/converse-stream"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(body_bytes, "application/vnd.amazon.eventstream"),
        )
        .mount(&server)
        .await;

    let provider = BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()));
    let mut stream = provider.stream_prepared(lifecycle_text_request(), test_auth());
    let mut events = Vec::new();
    while let Some(event) = stream.next().await {
        events.push(event);
    }

    assert_eq!(
        events
            .iter()
            .filter(|event| event.as_ref().is_err_and(
                |cause| cause.category() == opi_ai::provider::ProviderErrorCategory::Stream
            ))
            .count(),
        1,
        "truncated Bedrock trailer must produce exactly one StreamError: {events:?}"
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Ok(AssistantStreamEvent::Done { .. }))),
        "truncated Bedrock trailer must not flush Done: {events:?}"
    );
}

#[tokio::test]
async fn auth_error_bodies_are_absent_from_public_errors() {
    let canary = "AKIA_AUTH_ERROR_CANARY";
    for status in [401, 403] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/model/anthropic.claude-sonnet-4/converse-stream"))
            .respond_with(ResponseTemplate::new(status).set_body_string(canary))
            .mount(&server)
            .await;

        let provider = BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()));

        let mut stream = provider.stream_prepared(lifecycle_text_request(), test_auth());
        let error = stream
            .next()
            .await
            .expect("auth failure should produce an event")
            .expect_err("auth failure should produce ProviderError");
        assert!(matches!(
            direct_error_cause(&error),
            ProviderError::AuthFailed(_)
        ));
        let rendered = format!("{error} {error:?}");
        assert!(
            !rendered.contains(canary),
            "Bedrock HTTP {status} body leaked through ProviderError: {rendered}"
        );
    }
}

#[tokio::test]
async fn bedrock_http_errors_never_echo_aws_credential_canaries() {
    let canaries = [
        "AKIA_HTTP_ERROR_CANARY",
        "aws-secret-access-key-error-canary",
        "aws-session-token-error-canary",
    ];
    let body = canaries.join(" ");

    for status in [400, 401, 403, 408, 429, 500, 504] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/model/anthropic.claude-sonnet-4/converse-stream"))
            .respond_with(ResponseTemplate::new(status).set_body_string(body.clone()))
            .mount(&server)
            .await;
        let provider = BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()));
        let mut stream = provider.stream_prepared(lifecycle_text_request(), test_auth());
        let error = stream
            .next()
            .await
            .expect("HTTP failure should produce an event")
            .expect_err("HTTP failure should produce ProviderError");
        let rendered = format!("{error:?} {error}");
        for canary in canaries {
            assert!(
                !rendered.contains(canary),
                "HTTP {status} echoed AWS credential material: {rendered}"
            );
        }
    }
}

/// Bedrock 5xx responses retain the shared `provider` classification without
/// carrying any upstream response body into the public error.
#[tokio::test]
async fn stream_500_classifies_as_provider_with_bodyless_error() {
    let secret = "sk-proj-1234567890abcdefghijklmnopqrstuv";
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/model/anthropic.claude-sonnet-4/converse-stream"))
        .respond_with(
            ResponseTemplate::new(500).set_body_string(format!("rejected token {secret}")),
        )
        .mount(&server)
        .await;

    let provider = BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()));
    let stream = provider.stream_prepared(lifecycle_text_request(), test_auth());
    pin_mut!(stream);
    let first = stream.next().await.expect("should produce an event");
    match first {
        Err(err) => {
            assert_eq!(
                err.category(),
                ProviderErrorCategory::Provider,
                "5xx must classify as provider: {err:?}"
            );
            assert!(
                !err.to_string().contains(secret),
                "bedrock error must omit the upstream body: {err}"
            );
        }
        other => panic!("expected provider error from HTTP 500, got {other:?}"),
    }
}

#[tokio::test]
async fn request_enrichment_reaches_bedrock_http_boundary() {
    let body_bytes = bedrock_text_lifecycle_bytes();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/model/anthropic.claude-sonnet-4/converse-stream"))
        .and(wiremock::matchers::header(
            "content-type",
            "application/json",
        ))
        .and(wiremock::matchers::header("x-opi-request", "bedrock"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(body_bytes, "application/vnd.amazon.eventstream"),
        )
        .mount(&server)
        .await;
    let provider = BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()));
    let mut request = lifecycle_text_request();
    request.extra_headers = vec![("X-Opi-Request".into(), "bedrock".into())];

    let events = collect_events(provider.stream_prepared(request, test_auth())).await;

    assert!(
        events
            .iter()
            .any(|event| matches!(event, AssistantStreamEvent::Done { .. }))
    );
    server.verify().await;
    let received = server.received_requests().await.expect("recorded requests");
    let authorization = received[0]
        .headers
        .get("authorization")
        .expect("SigV4 authorization")
        .to_str()
        .expect("authorization is ASCII");
    assert!(
        authorization.contains("x-opi-request"),
        "Bedrock request enrichment must participate in the SigV4 signed-header set"
    );
    assert!(received[0].headers.contains_key("x-amz-date"));
    assert!(received[0].headers.contains_key("x-amz-content-sha256"));
}

#[tokio::test]
async fn request_timeout_maps_to_typed_timeout_at_bedrock_boundary() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_millis(200)))
        .mount(&server)
        .await;
    let provider = BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()));
    let mut request = lifecycle_text_request();
    request.timeout = Some(std::time::Duration::from_millis(20));
    let mut stream = provider.stream_prepared(request, test_auth());

    let result = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
        .await
        .expect("Bedrock request timeout must resolve promptly")
        .expect("Bedrock timeout must produce a stream item");

    assert!(
        matches!(result, Err(ref error) if matches!(direct_error_cause(error), ProviderError::Timeout))
    );
}

#[tokio::test]
async fn request_header_cannot_override_bedrock_signature_routing() {
    let server = MockServer::start().await;
    let provider = BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()));
    let mut request = lifecycle_text_request();
    request.extra_headers = vec![("x-amz-date".into(), "override".into())];
    let mut stream = provider.stream_prepared(request.clone(), test_auth());

    let error = stream
        .next()
        .await
        .expect("one refusal")
        .expect_err("managed header must fail closed");
    let ProviderError::StreamFailure(failure) = &error else {
        panic!("accepted projection must retain its failure carrier: {error:?}");
    };
    let ProviderError::RequestFailed(summary) = failure.cause() else {
        panic!("expected exact managed-header cause: {error:?}");
    };
    assert_eq!(
        summary.as_str(),
        "request header 'x-amz-date' cannot override a provider-managed header"
    );
    assert_direct_projection_prefix(failure.message(), &request);
    assert!(stream.next().await.is_none(), "refusal must be terminal");
    assert!(
        server
            .received_requests()
            .await
            .expect("recorded requests")
            .is_empty(),
        "reserved Bedrock request headers must fail before dispatch"
    );
}

#[tokio::test]
async fn request_header_cannot_duplicate_bedrock_session_token() {
    let server = MockServer::start().await;
    let provider = BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()));
    let mut request = lifecycle_text_request();
    request.extra_headers = vec![("x-amz-security-token".into(), "override".into())];
    let credentials = AwsCredentials {
        session_token: Some("real-session-token".into()),
        ..test_credentials()
    };
    let auth = ResolvedAuth::aws_sigv4(
        credentials,
        AuthProvenance {
            source: AuthProvenanceSource::AwsSigV4 {
                source: AwsCredentialSource::ExplicitConfig,
            },
            fallback: AuthFallback::NotAttempted,
        },
    );
    let mut stream = provider.stream_prepared(request.clone(), auth);

    let error = stream
        .next()
        .await
        .expect("one refusal")
        .expect_err("managed header must fail closed");
    let ProviderError::StreamFailure(failure) = &error else {
        panic!("accepted projection must retain its failure carrier: {error:?}");
    };
    let ProviderError::RequestFailed(summary) = failure.cause() else {
        panic!("expected exact managed-header cause: {error:?}");
    };
    assert_eq!(
        summary.as_str(),
        "request header 'x-amz-security-token' is reserved for Bedrock SigV4 authentication"
    );
    assert_direct_projection_prefix(failure.message(), &request);
    assert!(stream.next().await.is_none(), "refusal must be terminal");
    assert!(
        server
            .received_requests()
            .await
            .expect("recorded requests")
            .is_empty(),
        "Bedrock session-token collisions must fail before dispatch"
    );
}

#[tokio::test]
async fn request_header_cannot_supply_bedrock_session_token_when_auth_has_none() {
    let server = MockServer::start().await;
    let provider = BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()));
    let mut request = lifecycle_text_request();
    request.extra_headers = vec![("x-amz-security-token".into(), "request-token".into())];
    let mut stream = provider.stream_prepared(request.clone(), test_auth());

    let error = stream
        .next()
        .await
        .expect("one refusal")
        .expect_err("managed header must fail closed");
    let ProviderError::StreamFailure(failure) = &error else {
        panic!("accepted projection must retain its failure carrier: {error:?}");
    };
    let ProviderError::RequestFailed(summary) = failure.cause() else {
        panic!("expected exact managed-header cause: {error:?}");
    };
    assert_eq!(
        summary.as_str(),
        "request header 'x-amz-security-token' is reserved for Bedrock SigV4 authentication"
    );
    assert_direct_projection_prefix(failure.message(), &request);
    assert!(stream.next().await.is_none(), "refusal must be terminal");
    assert!(
        server
            .received_requests()
            .await
            .expect("recorded requests")
            .is_empty(),
        "Bedrock session-token headers must be provider-managed even when prepared auth has none"
    );
}

// ---------------------------------------------------------------------------
// Fixture-path cancellation: named negative (Phase 12 task 12.7 DoD clause 7)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn fixture_path_does_not_observe_cancel_documented_http_only_limitation() {
    // Adapter-level cancellation for bedrock is implemented on the signed HTTP
    // body-stream path (bedrock/mod.rs `cancel.cancelled()` select arm), which
    // consumes AWS binary event-stream frames. The fixture suite drives the
    // local `stream_from_fixture` decoder instead, and that path binds the
    // token as a no-op placeholder. So a pre-cancelled token must NOT interrupt
    // the fixture stream — this test pins that substrate limitation so
    // cancellation coverage is not silently claimed for bedrock from the
    // fixture path. Real cancellation for bedrock is exercised only through the
    // signed HTTP path; mounting a successful binary event-stream response
    // through wiremock is out of fixture scope.
    let events_data = build_bedrock_stream(&[
        ("messageStart", r#"{"role":"assistant"}"#),
        (
            "contentBlockStart",
            r#"{"start":{"text":{}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"text":"Hello!"},"contentBlockIndex":0}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":0}"#),
        ("messageStop", r#"{"stopReason":"end_turn"}"#),
        (
            "metadata",
            r#"{"usage":{"inputTokens":10,"outputTokens":5}}"#,
        ),
    ]);

    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));

    let cancel = CancellationToken::new();
    cancel.cancel(); // pre-cancelled before the stream starts
    let mut request = text_stream_request();
    request.cancel = cancel;

    let stream = provider.stream_from_fixture(&events_data, request.cancel);
    let events = collect_events(stream).await;

    // The fixture stream completes normally despite the cancelled token: the
    // fixture decoder path does not poll cancel (by design).
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::Done { .. })),
        "fixture-path stream must complete normally regardless of a cancelled token; \
         bedrock adapter-level cancel is exercised only on the signed HTTP path"
    );
}

// ---------------------------------------------------------------------------
// Per-call prepared AWS SigV4 authentication
// ---------------------------------------------------------------------------

fn prepared_bedrock_request(cancel: CancellationToken) -> Request {
    let mut request = text_stream_request();
    request.model = "bedrock:us.anthropic.claude-sonnet-4-20250514-v1:0".into();
    request.cancel = cancel;
    request
}

fn aws_provenance(source: AwsCredentialSource) -> AuthProvenance {
    AuthProvenance {
        source: AuthProvenanceSource::AwsSigV4 { source },
        fallback: AuthFallback::NotAttempted,
    }
}

fn prepared_aws_auth(call: usize) -> ResolvedAuth {
    ResolvedAuth::aws_sigv4(
        AwsCredentials {
            access_key_id: format!("AKIAPREPARED{call:04}").into(),
            secret_access_key: format!("prepared-secret-{call:04}").into(),
            session_token: Some(format!("prepared-session-{call:04}").into()),
            region: "us-east-1".into(),
            expires_at: None,
        },
        aws_provenance(AwsCredentialSource::Environment),
    )
}

struct CountingBedrockResolver {
    resolutions: Arc<AtomicUsize>,
}

impl AuthResolver for CountingBedrockResolver {
    fn resolve<'a>(&'a self) -> BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        let call = self.resolutions.fetch_add(1, Ordering::SeqCst) + 1;
        Box::pin(async move { Ok(prepared_aws_auth(call)) })
    }
}

fn prepared_bedrock_collection(base_url: String, resolutions: Arc<AtomicUsize>) -> Models {
    let mut collection = Models::new();
    collection
        .register_route(
            Box::new(BedrockProvider::new(
                Some(base_url),
                Arc::new(HttpClient::new()),
            )),
            Arc::new(CountingBedrockResolver { resolutions }),
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .expect("register Bedrock route");
    collection
}

async fn finish_timeout_attempt(prepared: &opi_ai::PreparedProviderCall) {
    let mut attempt = prepared.start_attempt().expect("start attempt");
    // 504 rides the structured service fault (a transient status), not the
    // transport Timeout slot.
    assert!(
        matches!(
            attempt.next().await,
            Some(Err(ref error)) if matches!(direct_error_cause(error), ProviderError::ServiceFault { status: 504, .. })
        ),
        "fixture should terminate the attempt with the configured 504"
    );
}

async fn finish_provider_side_attempt(prepared: &opi_ai::PreparedProviderCall) {
    let mut attempt = prepared.start_attempt().expect("start attempt");
    assert!(
        matches!(
            attempt.next().await,
            Some(Err(ref error)) if matches!(direct_error_cause(error), ProviderError::ServiceFault { status: 500, .. })
        ),
        "fixture should terminate the attempt with the configured 500"
    );
}

#[tokio::test]
async fn bedrock_resolves_once_per_logical_call_and_reuses_frozen_auth_for_retry() {
    let server = MockServer::start().await;
    // 504 is one of the transient statuses, so the acquisition owner admits
    // the second send of the episode; the frozen SigV4 credential must be
    // reused, not re-resolved.
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(504).set_body_string("retryable fixture failure"))
        .mount(&server)
        .await;
    let resolutions = Arc::new(AtomicUsize::new(0));
    let collection = prepared_bedrock_collection(server.uri(), resolutions.clone());
    let spec = "bedrock:us.anthropic.claude-sonnet-4-20250514-v1:0";
    let prepared = collection
        .prepare_call_with_policy(
            spec,
            prepared_bedrock_request(CancellationToken::new()),
            opi_ai::retry::AcquisitionPolicy::with_max_retries(1),
        )
        .await
        .expect("prepare Bedrock call");

    finish_timeout_attempt(&prepared).await;
    // Wait out the SDK-computed admission delay before the admitted retry.
    let delay_ms = prepared
        .retry_admission()
        .expect("the timeout with budget admits the retry")
        .delay_ms;
    tokio::time::sleep(std::time::Duration::from_millis(delay_ms + 5)).await;
    finish_timeout_attempt(&prepared).await;

    assert_eq!(
        resolutions.load(Ordering::SeqCst),
        1,
        "retry attempts must reuse one frozen Bedrock credential"
    );
    let received = server.received_requests().await.expect("recorded requests");
    assert_eq!(received.len(), 2);
    for request in received {
        let authorization = request
            .headers
            .get("authorization")
            .expect("SigV4 authorization")
            .to_str()
            .expect("authorization is ASCII");
        assert!(authorization.contains("AKIAPREPARED0001"));
    }
}

#[tokio::test]
async fn separate_bedrock_logical_calls_resolve_fresh_credentials() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500).set_body_string("fixture failure"))
        .mount(&server)
        .await;
    let resolutions = Arc::new(AtomicUsize::new(0));
    let collection = prepared_bedrock_collection(server.uri(), resolutions.clone());
    let spec = "bedrock:us.anthropic.claude-sonnet-4-20250514-v1:0";

    for _ in 0..2 {
        let prepared = collection
            .prepare_call(spec, prepared_bedrock_request(CancellationToken::new()))
            .await
            .expect("prepare Bedrock call");
        finish_provider_side_attempt(&prepared).await;
    }

    assert_eq!(resolutions.load(Ordering::SeqCst), 2);
    let received = server.received_requests().await.expect("recorded requests");
    let authorizations: Vec<_> = received
        .iter()
        .map(|request| {
            request
                .headers
                .get("authorization")
                .expect("SigV4 authorization")
                .to_str()
                .expect("authorization is ASCII")
                .to_owned()
        })
        .collect();
    assert!(authorizations[0].contains("AKIAPREPARED0001"));
    assert!(authorizations[1].contains("AKIAPREPARED0002"));
}

// ---------------------------------------------------------------------------
// Bearer / ProxyPlaceholderSigning adapter wiring and Models-level
// selected-local-source acceptance
// ---------------------------------------------------------------------------

fn bearer_auth(token: &str, base_url: Option<String>) -> ResolvedAuth {
    ResolvedAuth {
        env: Default::default(),
        scheme: AuthScheme::Bearer,
        secret: SecretString::from(token),
        base_url,
        account_id: None,
        provenance: AuthProvenance {
            source: AuthProvenanceSource::Explicit,
            fallback: AuthFallback::NotAttempted,
        },
    }
}

#[tokio::test]
async fn bearer_prepared_auth_streams_without_sigv4_attachment() {
    let body_bytes = bedrock_text_lifecycle_bytes();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(body_bytes, "application/vnd.amazon.eventstream"),
        )
        .mount(&server)
        .await;

    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));
    let auth = bearer_auth("bedrock-api-key-fixture", Some(server.uri()));
    let events = collect_events(provider.stream_prepared(lifecycle_text_request(), auth)).await;
    assert!(
        events.iter().any(
            |e| matches!(e, AssistantStreamEvent::TextDelta { delta, .. } if delta == "Hello!")
        ),
        "bearer authentication must still stream the event lifecycle"
    );

    let received = server.received_requests().await.expect("recorded requests");
    let recorded = &received[0];
    let authorization = recorded
        .headers
        .get("authorization")
        .expect("bearer authorization header")
        .to_str()
        .expect("authorization is ASCII");
    assert_eq!(authorization, "Bearer bedrock-api-key-fixture");
    for sigv4_header in ["x-amz-date", "x-amz-content-sha256", "x-amz-security-token"] {
        assert!(
            !recorded.headers.contains_key(sigv4_header),
            "bearer attachment must not stack SigV4 header '{sigv4_header}'"
        );
    }
    assert!(!authorization.starts_with("AWS4-HMAC-SHA256"));
}

#[tokio::test]
async fn bearer_without_any_endpoint_is_a_typed_config_error() {
    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));
    let auth = bearer_auth("bedrock-api-key-fixture", None);
    let outcome = collect_outcome(provider.stream_prepared(lifecycle_text_request(), auth)).await;
    assert!(
        matches!(outcome, Err(ref error) if matches!(direct_error_cause(error), ProviderError::Config(_))),
        "bearer without a provider or resolver endpoint must fail typed, got {outcome:?}"
    );
}

#[tokio::test]
async fn standard_aws_endpoints_reject_proxy_placeholder_signing() {
    let error = opi_ai::providers::bedrock::proxy_placeholder_auth(
        "https://bedrock-runtime.us-east-1.amazonaws.com",
        "us-east-1",
    )
    .expect_err("standard AWS endpoints reject the placeholder mode");
    assert!(matches!(error, ProviderError::Config(_)));

    // The provider side rejects it too, even with a placeholder client.
    let provider = BedrockProvider::new(
        Some("https://bedrock-runtime.eu-west-1.amazonaws.com".to_owned()),
        Arc::new(HttpClient::new()),
    )
    .with_placeholder_client(Arc::new(HttpClient::new()));
    let auth =
        opi_ai::providers::bedrock::proxy_placeholder_auth("http://127.0.0.1:9", "us-east-1")
            .expect("nonstandard endpoint is accepted");
    let outcome = collect_outcome(provider.stream_prepared(lifecycle_text_request(), auth)).await;
    assert!(
        matches!(outcome, Err(ref error) if matches!(direct_error_cause(error), ProviderError::Config(_))),
        "provider with a standard endpoint must reject placeholder signing, got {outcome:?}"
    );
}

#[tokio::test]
async fn proxy_placeholder_signing_without_placeholder_client_is_a_typed_error() {
    let provider = BedrockProvider::new(
        Some("http://127.0.0.1:9".to_owned()),
        Arc::new(HttpClient::new()),
    );
    let auth =
        opi_ai::providers::bedrock::proxy_placeholder_auth("http://127.0.0.1:9", "us-east-1")
            .expect("nonstandard endpoint is accepted");
    let outcome = collect_outcome(provider.stream_prepared(lifecycle_text_request(), auth)).await;
    assert!(
        matches!(outcome, Err(ref error) if matches!(direct_error_cause(error), ProviderError::Config(_))),
        "placeholder signing requires the explicit HTTP/1.1 placeholder client, got {outcome:?}"
    );
}

#[tokio::test]
async fn proxy_placeholder_signing_uses_h1_and_fixed_identity_at_a_local_proxy() {
    let body_bytes = bedrock_text_lifecycle_bytes();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind local proxy");
    let addr = listener.local_addr().expect("local proxy addr");
    let endpoint = format!("http://{addr}");

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept placeholder request");
        let mut request = Vec::new();
        let mut buf = [0_u8; 4096];
        loop {
            let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                .await
                .expect("read placeholder request");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buf[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        let mut response = Vec::new();
        response.extend_from_slice(
            b"HTTP/1.1 200 OK\r\nContent-Type: application/vnd.amazon.eventstream\r\nConnection: close\r\n\r\n",
        );
        response.extend_from_slice(&body_bytes);
        tokio::io::AsyncWriteExt::write_all(&mut socket, &response)
            .await
            .expect("write placeholder response");
        request
    });

    let http1_client = Arc::new(
        opi_ai::http::HttpClientBuilder::new()
            .tls(opi_ai::http::TlsOptions::default().http1_only())
            .build()
            .expect("HTTP/1.1-only placeholder client"),
    );
    let provider = BedrockProvider::new(Some(endpoint.clone()), http1_client.clone())
        .with_placeholder_client(http1_client);
    let auth = opi_ai::providers::bedrock::proxy_placeholder_auth(&endpoint, "us-east-1")
        .expect("local proxy endpoint is nonstandard");

    // Ordinary Models assembly: a static placeholder route reads zero real
    // credential sources even while ambient-looking values exist.
    struct CountingLookupContext {
        env_reads: AtomicUsize,
    }
    impl AuthContext for CountingLookupContext {
        fn env(&self, _name: &str) -> Result<Option<String>, AuthContextError> {
            self.env_reads.fetch_add(1, Ordering::SeqCst);
            Ok(Some("AMBIENT-CANARY-LEAK-CHECK".to_owned()))
        }
        fn file_exists(&self, _path: &str) -> Result<bool, AuthContextError> {
            Ok(true)
        }
    }
    let context = Arc::new(CountingLookupContext {
        env_reads: AtomicUsize::new(0),
    });
    struct FixedAuthResolver(ResolvedAuth);
    impl AuthResolver for FixedAuthResolver {
        fn resolve<'a>(&'a self) -> BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
            Box::pin(async { Ok(self.0.clone()) })
        }
    }
    let mut collection = Models::new();
    collection.set_auth_context(context.clone());
    collection
        .register_route(
            Box::new(provider),
            Arc::new(FixedAuthResolver(auth)),
            AuthProvenanceSource::AwsSigV4 {
                source: AwsCredentialSource::ProxyPlaceholder,
            },
            CompatMetadata::default(),
        )
        .expect("register placeholder route");
    let resolved = collection
        .get_auth("bedrock", Default::default())
        .await
        .expect("placeholder route resolves")
        .expect("placeholder route is configured");
    assert_eq!(
        resolved.provenance.source,
        AuthProvenanceSource::AwsSigV4 {
            source: AwsCredentialSource::ProxyPlaceholder,
        }
    );
    assert_eq!(
        context.env_reads.load(Ordering::SeqCst),
        0,
        "the placeholder route must read zero environment sources"
    );

    let mut request = lifecycle_text_request();
    request.model = "bedrock:us.anthropic.claude-sonnet-4-20250514-v1:0".into();
    let prepared = collection
        .prepare_call(
            "bedrock:us.anthropic.claude-sonnet-4-20250514-v1:0",
            request,
        )
        .await
        .expect("prepare placeholder call");
    let mut attempt = prepared.start_attempt().expect("start placeholder attempt");
    let mut saw_done = false;
    while let Some(item) = attempt.next().await {
        if matches!(item, Ok(AssistantStreamEvent::Done { .. })) {
            saw_done = true;
        }
    }
    assert!(saw_done, "placeholder route must stream the lifecycle");
    drop(attempt);

    let raw_request = server.await.expect("local proxy handled one request");
    let text = String::from_utf8_lossy(&raw_request);
    let request_line = text.lines().next().expect("request line");
    assert!(
        request_line.ends_with("HTTP/1.1"),
        "placeholder signing must use HTTP/1.1, got '{request_line}'"
    );
    let authorization = text
        .lines()
        .find(|line| line.to_ascii_lowercase().starts_with("authorization:"))
        .expect("placeholder authorization header");
    assert!(
        authorization.contains("AWS4-HMAC-SHA256"),
        "placeholder mode keeps fixed placeholder SigV4, got '{authorization}'"
    );
    assert!(
        authorization.contains("dummy-access-key"),
        "placeholder identity is the fixed nonsecret value"
    );
    assert!(
        authorization.contains("us-east-1/bedrock/aws4_request"),
        "frozen region participates in the placeholder credential scope"
    );
    assert!(!text.contains("AMBIENT-CANARY-LEAK-CHECK"));
}

// ---------------------------------------------------------------------------
// Local AWS source chain (env -> INI static overlay -> credential_process)
// ---------------------------------------------------------------------------
use aws_source_transport::FakeSourceTransport;
use opi_ai::auth::aws::local::{
    AwsAuthSelection, AwsLocalChainInput, CredentialResolutionError, CredentialSource,
    ScopedAwsEnv, WebIdentityRoute, resolve_local_aws_auth,
};
use opi_ai::auth::aws::process::{
    CredentialProcessOutput, CredentialProcessRunner, CredentialProcessSpec,
};
use opi_ai::auth::{AuthContext, AuthContextError};
use secrecy::ExposeSecret;

// ===========================================================================
// Task 20.19.2: STS AssumeRole / web identity / ECS / IMDS source families.
// ===========================================================================
use opi_ai::auth::aws::endpoint::sts_endpoint_url;
use opi_ai::auth::aws::local::TerminalReason;
use opi_ai::auth::aws::role::MfaTokenPresenter;
use opi_ai::auth::aws::source_http::{
    AwsSourceTransport, ReqwestSourceTransport, SourceHttpError, SourceHttpMethod,
    SourceHttpRequest, SourceHttpResponse,
};

// ---------------------------------------------------------------------------
// Task 20.19.3: SSO / login_session source families and the full chain.
// ---------------------------------------------------------------------------
use opi_ai::auth::aws::cache::{
    AwsSourceCache, CacheIdentity, DpopJtiSource, PassiveRefreshDeps, SourceCacheValue,
    SourceClock, SsoRefreshMaterial, SsoTokenSnapshot,
};
use opi_ai::auth::aws::login::{
    login_cache_file_name, read_login_cache_file, write_login_cache_file,
};
use opi_ai::auth::aws::sso::{read_sso_token_file, write_sso_token_file};
// ---------------------------------------------------------------------------
// Task 20.51: functional callbacks at the Bedrock wire (P20-S51)
// ---------------------------------------------------------------------------
use opi_ai::callbacks::RequestCallbacks;

#[tokio::test]
async fn bedrock_async_payload_and_response_callbacks_flow_through_converse_stream() {
    let body_bytes = bedrock_text_lifecycle_bytes();
    let payload_bodies: std::sync::Arc<std::sync::Mutex<Vec<serde_json::Value>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let payload_recorder = payload_bodies.clone();
    let response_status = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let response_recorder = response_status.clone();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/model/anthropic.claude-sonnet-4/converse-stream"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(body_bytes, "application/vnd.amazon.eventstream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()));
    let mut request = lifecycle_text_request();
    request.callbacks = RequestCallbacks::new()
        .with_async_header_transform(|headers| async move {
            tokio::task::yield_now().await;
            Ok(headers)
        })
        .with_async_on_payload(move |body| {
            let payload_recorder = payload_recorder.clone();
            async move {
                tokio::task::yield_now().await;
                payload_recorder.lock().unwrap().push(body.clone());
                Ok(None)
            }
        })
        .with_async_on_response(move |status, headers| {
            let response_recorder = response_recorder.clone();
            async move {
                tokio::task::yield_now().await;
                response_recorder.lock().unwrap().push(format!("{status}"));
                assert!(
                    headers
                        .iter()
                        .any(|(name, _)| name.eq_ignore_ascii_case("content-type")),
                    "bedrock response callback sees the event-stream content type"
                );
                Ok(())
            }
        });

    let events = collect_events(provider.stream_prepared(request, test_auth())).await;
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::Done { .. }))
    );

    let bodies = payload_bodies.lock().unwrap().clone();
    assert_eq!(bodies.len(), 1);
    assert!(
        bodies[0]["messages"].is_array(),
        "bedrock payload callback sees the converse-stream body"
    );
    let sent = server.received_requests().await.unwrap()[0].clone();
    let wire_body: serde_json::Value =
        serde_json::from_slice(&sent.body).expect("captured body is JSON");
    assert_eq!(bodies[0], wire_body);
    assert_eq!(*response_status.lock().unwrap(), vec!["200"]);
    server.verify().await;
}

#[tokio::test]
async fn bedrock_raw_callback_sees_frame_events_in_order() {
    let body_bytes = bedrock_text_lifecycle_bytes();
    let raw_events: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorder = raw_events.clone();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(body_bytes, "application/vnd.amazon.eventstream"),
        )
        .mount(&server)
        .await;

    let provider = BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()));
    let mut request = lifecycle_text_request();
    request.callbacks = RequestCallbacks::new().with_on_raw(std::sync::Arc::new(move |event| {
        recorder
            .lock()
            .unwrap()
            .push(event["type"].as_str().unwrap_or("?").to_string());
        Ok(())
    }));

    let events = collect_events(provider.stream_prepared(request, test_auth())).await;
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::TextDelta { .. }))
    );
    let raw = raw_events.lock().unwrap();
    assert!(
        raw.first().map(String::as_str) == Some("messageStart"),
        "bedrock raw observer sees smithy frames in wire order: {raw:?}"
    );
    assert!(raw.contains(&"contentBlockDelta".to_string()));
    assert!(
        raw.last().map(String::as_str) == Some("metadata"),
        "every parsed smithy frame reaches the raw observer, in wire order: {raw:?}"
    );
}

#[tokio::test]
async fn bedrock_tail_raw_callback_failure_precedes_pending_done() {
    let body_bytes = build_bedrock_stream(&[
        ("messageStart", r#"{"role":"assistant"}"#),
        (
            "contentBlockStart",
            r#"{"start":{"text":{}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"text":"partial"},"contentBlockIndex":0}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":0}"#),
        ("messageStop", r#"{"stopReason":"end_turn"}"#),
        (
            "metadata",
            r#"{"usage":{"inputTokens":10,"outputTokens":2}}"#,
        ),
        (
            "metadata",
            r#"{"usage":{"inputTokens":12,"outputTokens":7}}"#,
        ),
    ]);
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/model/anthropic.claude-sonnet-4/converse-stream"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(body_bytes, "application/vnd.amazon.eventstream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let metadata_seen = Arc::new(AtomicUsize::new(0));
    let observer_count = metadata_seen.clone();
    let provider = BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()));
    let mut request = lifecycle_text_request();
    request.callbacks = RequestCallbacks::new().with_on_raw(Arc::new(move |event| {
        if event["type"] == "metadata" && observer_count.fetch_add(1, Ordering::SeqCst) == 1 {
            return Err(opi_ai::callbacks::CallbackError::new(
                "tail metadata observer failed",
            ));
        }
        Ok(())
    }));

    let mut stream = provider.stream_prepared(request, test_auth());
    let mut outcomes = Vec::new();
    while let Some(item) = stream.next().await {
        outcomes.push(item);
    }

    assert_eq!(metadata_seen.load(Ordering::SeqCst), 2);
    assert!(
        outcomes
            .iter()
            .any(|item| matches!(item, Err(error) if matches!(direct_error_cause(error), ProviderError::CallbackPhase(_)))),
        "the second metadata observer failure is surfaced as a typed callback error"
    );
    assert!(
        !outcomes
            .iter()
            .any(|item| matches!(item, Ok(AssistantStreamEvent::Done { .. }))),
        "pending Done must not be published before the tail observer succeeds"
    );
    server.verify().await;
}

#[tokio::test]
async fn bedrock_transform_touching_signing_headers_fails_typed_before_http() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let provider = BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()));
    let mut request = lifecycle_text_request();
    request.callbacks =
        RequestCallbacks::new().with_header_transform(std::sync::Arc::new(|merged| {
            let mut out = merged.to_vec();
            out.push(("amz-sdk-invocation-id".to_string(), "attacker".to_string()));
            Ok(out)
        }));

    let mut stream = provider.stream_prepared(request, test_auth());
    let mut typed = false;
    while let Some(item) = stream.next().await {
        if matches!(item, Err(ref error) if matches!(direct_error_cause(error), ProviderError::CallbackPhase(_)))
        {
            typed = true;
            break;
        }
    }
    assert!(
        typed,
        "transform touching the Smithy signing headers must fail typed"
    );
    server.verify().await;
}
