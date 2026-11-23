//! Gemini HTTP, cancellation and stream lifecycle assertions.

use super::*;

#[tokio::test]
async fn auth_error_bodies_are_absent_from_public_errors() {
    let canaries = [
        "gemini-access-canary-with-no-known-token-shape",
        "gemini-secret-canary-with-no-known-token-shape",
        "gemini-session-canary-with-no-known-token-shape",
        "gemini-token-canary-with-no-known-token-shape",
    ];
    let body = canaries.join(" ");

    for status in [401, 403] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(status).set_body_string(body.clone()))
            .mount(&server)
            .await;
        let provider = GeminiProvider::new(Some(server.uri()));
        let mut stream =
            provider.stream_prepared(gemini_http_request(), opi_ai::test_support::resolved_auth());
        let error = stream
            .next()
            .await
            .expect("auth failure should produce an event")
            .expect_err("auth failure should produce ProviderError");
        assert!(matches!(
            direct_cause(&error),
            opi_ai::provider::ProviderError::AuthFailed(_)
        ));
        let rendered = format!("{error} {error:?}");
        for canary in canaries {
            assert!(
                !rendered.contains(canary),
                "Gemini HTTP {status} body leaked through ProviderError: {rendered}"
            );
        }
    }
}

#[tokio::test]
async fn direct_auth_statuses_do_not_wait_for_stalled_bodies() {
    for status in [401, 403] {
        let (server, headers_flushed) = spawn_stalled_gemini_error_body_server(status).await;
        let provider = GeminiProvider::new(Some(server));
        let mut stream =
            provider.stream_prepared(gemini_http_request(), opi_ai::test_support::resolved_auth());

        assert!(futures_util::poll!(stream.next()).is_pending());
        tokio::time::timeout(
            std::time::Duration::from_secs(2),
            headers_flushed.notified(),
        )
        .await
        .expect("Gemini auth-error headers must be flushed before the body stalls");

        let result = tokio::time::timeout(std::time::Duration::from_millis(500), stream.next())
            .await
            .expect("Gemini direct auth status must not wait for its body")
            .expect("Gemini direct auth status must produce a stream item");

        let error = result.expect_err("Gemini direct auth status must fail");
        assert!(matches!(
            direct_cause(&error),
            opi_ai::provider::ProviderError::AuthFailed(_)
        ));
        assert_eq!(
            error.to_string(),
            "authentication failed: provider rejected credentials"
        );
    }
}

#[tokio::test]
async fn oversized_embedded_auth_body_is_not_read_past_the_classification_cap() {
    let server = MockServer::start().await;
    let body = format!(
        r#"{{"error":{{"code":401}},"padding":"{}"}}"#,
        "x".repeat(64 * 1024)
    );
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(400).set_body_string(body))
        .mount(&server)
        .await;
    let provider = GeminiProvider::new(Some(server.uri()));
    let mut stream =
        provider.stream_prepared(gemini_http_request(), opi_ai::test_support::resolved_auth());

    let error = stream
        .next()
        .await
        .expect("oversized Gemini error body must produce a stream item")
        .expect_err("oversized Gemini error body must fail");

    assert!(matches!(
        direct_cause(&error),
        opi_ai::provider::ProviderError::ProviderSide(_)
    ));
}

// ---------------------------------------------------------------------------
// Production request contract through Provider::stream (Phase 12.1)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_sends_text_request_body_and_auth_through_http() {
    let sse = "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Hi\"}]},\"index\":0}]}\n\n\
               data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"\"}]},\"finishReason\":\"STOP\",\"index\":0}],\"usageMetadata\":{\"promptTokenCount\":5,\"candidatesTokenCount\":2,\"totalTokenCount\":7}}\n\n";
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(query_param("alt", "sse"))
        .and(header("x-goog-api-key", "test-key"))
        .and(body_partial_json(serde_json::json!({
            "contents": [{"role": "user", "parts": [{"text": "Hello"}]}],
            "systemInstruction": {"parts": [{"text": "You are helpful."}]},
            "generationConfig": {"maxOutputTokens": 1024}
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = GeminiProvider::new(Some(server.uri()));
    let request = Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "gemini:gemini-2.5-flash".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: Some("You are helpful.".into()),
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "Hello".into(),
                }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: Some(1024),
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    };

    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    let mut saw_done = false;
    while let Some(result) = stream.next().await {
        match result.expect("matched text request must stream successfully") {
            AssistantStreamEvent::Done { reason, message } => {
                assert!(!saw_done, "text request must have one terminal event");
                assert_eq!(reason, opi_ai::stream::StopReason::Stop);
                // Independent totals and absent child counts retain their wire facts.
                assert_eq!(
                    message.usage,
                    opi_ai::stream::Usage::reported(5, 2, 0, 0, None, None)
                        .with_total_tokens(Some(7)),
                    "HTTP usage must retain the fixture's exact token values"
                );
                saw_done = true;
            }
            AssistantStreamEvent::Error { .. } => {
                panic!("text fixture must complete with Done");
            }
            _ => {}
        }
    }
    assert!(saw_done, "text fixture must produce a terminal Done event");

    // verify() confirms the production request carried the Gemini body
    // (contents + systemInstruction + generationConfig.maxOutputTokens), the
    // x-goog-api-key auth header, and the alt=sse Vertex-style query. The
    // structural path (model + :streamGenerateContent) is asserted by the
    // vertex lifecycle suite and the URL-construction unit tests above.
    server.verify().await;
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn request_enrichment_reaches_gemini_http_boundary() {
    let sse = "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Hi\"}]},\"finishReason\":\"STOP\",\"index\":0}]}\n\n";
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(header("x-goog-api-key", "test-key"))
        .and(header("content-type", "application/json"))
        .and(header("x-opi-request", "gemini"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;
    let provider = GeminiProvider::new(Some(server.uri()));
    let mut request = gemini_http_request();
    request.extra_headers = vec![("X-Opi-Request".into(), "gemini".into())];

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
async fn request_timeout_maps_to_typed_timeout_at_gemini_boundary() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_millis(200)))
        .mount(&server)
        .await;
    let provider = GeminiProvider::new(Some(server.uri()));
    let mut request = gemini_http_request();
    request.timeout = Some(std::time::Duration::from_millis(20));
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());

    let result = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
        .await
        .expect("Gemini request timeout must resolve promptly")
        .expect("Gemini timeout must produce a stream item");

    assert!(matches!(
        result.as_ref().err().map(direct_cause),
        Some(opi_ai::provider::ProviderError::Timeout)
    ));
}

#[tokio::test]
async fn stalled_embedded_auth_body_uses_a_fixed_default_deadline() {
    let (server, headers_flushed) = spawn_stalled_gemini_error_body_server(400).await;
    let provider = GeminiProvider::new(Some(server));
    let mut stream =
        provider.stream_prepared(gemini_http_request(), opi_ai::test_support::resolved_auth());

    assert!(futures_util::poll!(stream.next()).is_pending());
    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        headers_flushed.notified(),
    )
    .await
    .expect("Gemini error headers must be flushed before the body stalls");

    let result = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
        .await
        .expect("Gemini error-body timeout must resolve promptly")
        .expect("Gemini error-body timeout must produce a stream item");

    assert!(matches!(
        result.as_ref().err().map(direct_cause),
        Some(opi_ai::provider::ProviderError::Timeout)
    ));
}

#[tokio::test]
async fn request_header_cannot_override_gemini_auth_routing() {
    let server = MockServer::start().await;
    let provider = GeminiProvider::new(Some(server.uri()));
    let mut request = gemini_http_request();
    request.extra_headers = vec![("x-goog-api-key".into(), "override".into())];
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());

    assert!(matches!(
        stream
            .next()
            .await
            .as_ref()
            .and_then(|item| item.as_ref().err())
            .map(direct_cause),
        Some(opi_ai::provider::ProviderError::RequestFailed(_))
    ));
    assert!(
        server
            .received_requests()
            .await
            .expect("recorded requests")
            .is_empty(),
        "reserved Gemini request headers must fail before dispatch"
    );
}

#[tokio::test]
async fn cancellation_before_response_headers_is_typed_and_prompt() {
    assert_gemini_cancelled(GeminiStallPoint::BeforeHeaders).await;
}

#[tokio::test]
async fn cancellation_during_response_body_is_typed_and_prompt() {
    assert_gemini_cancelled(GeminiStallPoint::ResponseBody).await;
}

#[tokio::test]
async fn cancellation_during_error_body_is_typed_and_prompt() {
    assert_gemini_cancelled(GeminiStallPoint::ErrorBody).await;
}

// ---------------------------------------------------------------------------
#[tokio::test]
async fn multiline_sse_line_endings_survive_http_chunk_splits() {
    async fn run(body: String, split: Option<usize>) -> Vec<AssistantStreamEvent> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let peer = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = vec![0; 65536];
            let received = socket.read(&mut request).await.unwrap();
            assert!(
                received > 0,
                "client sent request bytes before the response"
            );
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).as_bytes()).await.unwrap();
            if let Some(split) = split {
                socket.write_all(&body.as_bytes()[..split]).await.unwrap();
                socket.flush().await.unwrap();
                tokio::time::sleep(std::time::Duration::from_millis(40)).await;
                socket.write_all(&body.as_bytes()[split..]).await.unwrap();
            } else {
                socket.write_all(body.as_bytes()).await.unwrap();
            }
            socket.shutdown().await.unwrap();
        });
        let provider = GeminiProvider::new(Some(format!("http://{address}")));
        let results: Vec<_> = provider
            .stream_prepared(gemini_http_request(), opi_ai::test_support::resolved_auth())
            .collect()
            .await;
        peer.await.unwrap();
        results
            .into_iter()
            .map(|event| event.expect("legal complete multiline event"))
            .collect()
    }

    let body = [
        serde_json::json!({"candidates":[{"content":{"role":"model","parts":[{"text":"hello"}]},"index":0}]}),
        serde_json::json!({"candidates":[{"content":{"role":"model","parts":[{"text":""}]},"finishReason":"STOP","index":0}],"usageMetadata":{"promptTokenCount":10,"candidatesTokenCount":5,"totalTokenCount":15}}),
    ].iter().map(|value| format!("data: {value}\n\n")).collect::<String>();
    for newline in ["\n", "\r\n", "\r"] {
        let multiline = body
            .lines()
            .map(|line| {
                if line.starts_with("data: {") {
                    line.replacen(',', &format!(",{newline}data: "), 1)
                } else {
                    line.to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join(newline)
            + newline;
        let split = multiline.find(&format!("{newline}data: ")).unwrap() + 1;
        let whole = run(multiline.clone(), None).await;
        let split = run(multiline, Some(split)).await;
        let kinds = |events: &[AssistantStreamEvent]| {
            events
                .iter()
                .map(|event| serde_json::to_value(event).unwrap()["type"].clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(kinds(&split), kinds(&whole), "newline={newline:?}");
        let final_message = |events: Vec<AssistantStreamEvent>| {
            events
                .into_iter()
                .find_map(|event| match event {
                    AssistantStreamEvent::Done { message, .. } => Some(message),
                    _ => None,
                })
                .expect("terminal outcome")
        };
        let whole = final_message(whole);
        let split = final_message(split);
        assert_eq!(split.content, whole.content);
        assert_eq!(split.stop_reason, whole.stop_reason);
        assert_eq!(split.usage, whole.usage);
        assert!(!split.content.is_empty());
    }
}
