use super::*;

#[tokio::test]
async fn dedicated_codex_request_uses_exact_base_path_body_and_headers() {
    let default_server = MockServer::start().await;
    let model_server = MockServer::start().await;
    let auth_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/codex/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(
                    "event: response.completed\ndata: {\"type\":\"response.completed\",\"response\":{\"id\":\"response-1\",\"model\":\"gpt-5.4\",\"output\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}}\n\n",
                )
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&model_server)
        .await;

    let provider = OpenAiCodexResponsesProvider::new(
        Some(default_server.uri()),
        vec![model(Some(model_server.uri()))],
        Arc::new(HttpClient::new()),
    );
    let auth = auth(
        "sentinel-access",
        Some(auth_server.uri()),
        Some("account-fixed".into()),
    );
    assert_eq!(provider.id(), "openai-codex");
    assert!(drain(&provider, request(), auth).await.is_none());
    assert!(default_server.received_requests().await.unwrap().is_empty());
    assert!(auth_server.received_requests().await.unwrap().is_empty());
    let requests = model_server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let captured = &requests[0];
    assert_eq!(captured.url.path(), "/codex/responses");
    assert_eq!(
        captured.headers["authorization"].to_str().unwrap(),
        "Bearer sentinel-access"
    );
    assert_eq!(
        captured.headers["chatgpt-account-id"].to_str().unwrap(),
        "account-fixed"
    );
    assert_eq!(captured.headers["originator"].to_str().unwrap(), "opi");
    assert_eq!(
        captured.headers["OpenAI-Beta"].to_str().unwrap(),
        "responses=experimental"
    );
    assert_eq!(
        captured.headers["accept"].to_str().unwrap(),
        "text/event-stream"
    );
    assert_eq!(
        captured.headers["session-id"].to_str().unwrap(),
        "session-fixed"
    );
    let client_request_id = captured.headers["x-client-request-id"].to_str().unwrap();
    assert_ne!(client_request_id, "session-fixed");
    assert_eq!(
        uuid::Uuid::parse_str(client_request_id)
            .unwrap()
            .get_version_num(),
        7
    );

    let body: serde_json::Value = serde_json::from_slice(&captured.body).unwrap();
    assert_eq!(body["model"], "gpt-5.4");
    assert_eq!(body["store"], false);
    assert_eq!(body["stream"], true);
    assert_eq!(body["instructions"], "system prompt");
    assert_eq!(body["input"][0]["role"], "user");
    assert_eq!(body["input"][0]["content"], "hello");
    assert_eq!(body["tools"][0]["strict"], serde_json::Value::Null);
    assert_eq!(body["tool_choice"], "auto");
    assert_eq!(body["parallel_tool_calls"], true);
    assert_eq!(
        body["reasoning"],
        serde_json::json!({"effort":"high","summary":"auto"})
    );
    assert_eq!(body["text"], serde_json::json!({"verbosity":"low"}));
    assert_eq!(
        body["include"],
        serde_json::json!(["reasoning.encrypted_content"])
    );
    assert_eq!(body["prompt_cache_key"], "session-fixed");
    assert!(body.get("max_output_tokens").is_none());
    assert_eq!(body["temperature"], 0.2);
}

#[test]
fn dedicated_codex_only_strips_its_own_model_prefix() {
    let provider =
        OpenAiCodexResponsesProvider::new(None, vec![model(None)], Arc::new(HttpClient::new()));
    let mut foreign = request();
    foreign.model = "foreign:gpt-5.4".into();

    assert_eq!(
        provider.build_request_body(&foreign).unwrap()["model"],
        "foreign:gpt-5.4"
    );
}

#[test]
fn dedicated_codex_does_not_fallback_for_unsupported_thinking_level() {
    let unsupported = model(None).with_thinking_level_map(ThinkingLevelMap::disabled());
    let provider =
        OpenAiCodexResponsesProvider::new(None, vec![unsupported], Arc::new(HttpClient::new()));

    assert!(
        provider
            .build_request_body(&request())
            .unwrap()
            .get("reasoning")
            .is_none()
    );
}

#[tokio::test]
async fn dedicated_codex_generates_fresh_uuid_v7_headers_without_session_id() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/codex/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(
                    "event: response.completed\ndata: {\"type\":\"response.completed\",\"response\":{\"id\":\"response-1\",\"model\":\"gpt-5.4\",\"output\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}}\n\n",
                )
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;
    let provider = OpenAiCodexResponsesProvider::new(
        Some(server.uri()),
        vec![model(None)],
        Arc::new(HttpClient::new()),
    );

    for _ in 0..2 {
        let mut generated = request();
        generated.session_id = None;
        let auth = auth(
            "sentinel-access",
            Some(server.uri()),
            Some("account-fixed".into()),
        );
        assert!(drain(&provider, generated, auth).await.is_none());
    }

    let requests = server.received_requests().await.expect("captured requests");
    assert_eq!(requests.len(), 2);
    let header_values = |name: &str| {
        requests
            .iter()
            .map(|request| {
                request.headers[name]
                    .to_str()
                    .expect("UUID header is text")
                    .to_owned()
            })
            .collect::<Vec<_>>()
    };
    let session_ids = header_values("session-id");
    let request_ids = header_values("x-client-request-id");
    for value in session_ids.iter().chain(&request_ids) {
        assert_eq!(
            uuid::Uuid::parse_str(value)
                .expect("generated header is a UUID")
                .get_version_num(),
            7,
            "{value}"
        );
    }
    assert_ne!(session_ids[0], session_ids[1]);
    assert_ne!(request_ids[0], request_ids[1]);
    assert_ne!(session_ids[0], request_ids[0]);
    assert_ne!(session_ids[1], request_ids[1]);
}

#[tokio::test]
async fn dedicated_codex_disabled_affinity_omits_user_session_everywhere() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/codex/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(
                    "event: response.completed\ndata: {\"type\":\"response.completed\",\"response\":{\"id\":\"response-1\",\"model\":\"gpt-5.4\",\"output\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}}\n\n",
                )
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;
    let provider = OpenAiCodexResponsesProvider::new(
        Some(server.uri()),
        vec![model(None)],
        Arc::new(HttpClient::new()),
    );
    let auth = auth(
        "sentinel-access",
        Some(server.uri()),
        Some("account-fixed".into()),
    );
    let mut disabled = request();
    disabled.cache_retention = CacheRetention::Disabled;
    disabled.session_id = Some("user-session-must-not-leak".into());

    assert!(drain(&provider, disabled, auth).await.is_none());

    let captured = server.received_requests().await.unwrap().remove(0);
    let body: serde_json::Value = serde_json::from_slice(&captured.body).unwrap();
    assert!(body.get("prompt_cache_key").is_none());
    // C7: CacheRetention::Disabled suppresses BOTH session-affinity headers,
    // not just the user-provided session id.
    assert!(
        !captured.headers.contains_key("session-id"),
        "session-id header must be omitted when cache retention is Disabled"
    );
    assert!(
        !captured.headers.contains_key("x-client-request-id"),
        "x-client-request-id header must be omitted when cache retention is Disabled"
    );
    let rendered_headers = format!("{:?}", captured.headers);
    assert!(!rendered_headers.contains("user-session-must-not-leak"));
}

#[tokio::test]
async fn dedicated_codex_malformed_sse_never_surfaces_upstream_data() {
    const SENTINEL: &str = "sentinel-malformed-sse-secret";
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/codex/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(format!(
                    "event: response.output_text.delta\ndata: {{{SENTINEL}\n\n"
                ))
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;
    let provider = OpenAiCodexResponsesProvider::new(
        Some(server.uri()),
        vec![model(None)],
        Arc::new(HttpClient::new()),
    );
    let auth = auth(
        "sentinel-access",
        Some(server.uri()),
        Some("account-fixed".into()),
    );

    let (captures, events, errors) = capture_stream(&provider, request(), auth).await;

    assert_eq!(events, 0);
    assert_eq!(errors, 1);
    let rendered = captures.join("\n");
    assert!(
        rendered.contains("OpenAI Codex returned malformed streaming data"),
        "{rendered}"
    );
    assert!(!rendered.contains(SENTINEL), "{rendered}");
}

#[tokio::test]
async fn dedicated_codex_valid_error_sse_never_surfaces_message() {
    // Canonical Codex SSE is data-only: the type lives in the JSON payload.
    // Genuine `error` and `response.failed` types must terminate the stream
    // with a redacted, provider-neutral error and never echo the upstream
    // message. Unknown event types are ignored (not errors) -- covered by
    // `dedicated_codex_data_only_frames_stream_to_completion`.
    for (label, data, sentinel, transient) in [
        (
            "error",
            r#"{"type":"error","message":"sentinel-valid-error-message"}"#,
            "sentinel-valid-error-message",
            false,
        ),
        (
            "response.failed",
            r#"{"type":"response.failed","error":{"message":"sentinel-failed-detail"}}"#,
            "sentinel-failed-detail",
            false,
        ),
        (
            "response.failed code alone",
            r#"{"type":"response.failed","response":{"status":"failed","error":{"code":"upstream_stream_read_error","message":"PRIVATE_CANARY"}}}"#,
            "PRIVATE_CANARY",
            false,
        ),
        (
            "response.failed transient message",
            r#"{"type":"response.failed","response":{"status":"failed","error":{"message":"Upstream HTTP/2 stream failed. PRIVATE_CANARY"}}}"#,
            "PRIVATE_CANARY",
            true,
        ),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/codex/responses"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(format!("data: {data}\n\n"))
                    .insert_header("content-type", "text/event-stream"),
            )
            .mount(&server)
            .await;
        let provider = OpenAiCodexResponsesProvider::new(
            Some(server.uri()),
            vec![model(None)],
            Arc::new(HttpClient::new()),
        );
        let auth = auth(
            "sentinel-access",
            Some(server.uri()),
            Some("account-fixed".into()),
        );

        let (captures, events, errors) = capture_stream(&provider, request(), auth).await;

        // Current producers stop the dual terminal format: the error frame
        // terminates the attempt with a typed stream failure whose bounded
        // cause text never echoes the upstream message.
        assert_eq!(events, 0, "{label}");
        assert_eq!(errors, 1, "{label}");
        let rendered = captures.join("\n");
        assert!(
            rendered.contains("openai responses stream error"),
            "{label}: {rendered}"
        );
        assert!(!rendered.contains(sentinel), "{label}: {rendered}");
        assert_eq!(
            rendered.contains("upstream request failed"),
            transient,
            "{label}"
        );
    }
}

#[tokio::test]
async fn dedicated_codex_sse_error_uses_the_selected_code_and_message() {
    use opi_ai::retry::{WholeCallRetryClassification, classify_whole_call_retry};
    use serde_json::{Value, json};
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("../data/responses-error.json")).unwrap();
    let mut mismatches = Vec::new();
    for case in cases.iter().filter(|case| case["route"] == "codex") {
        let name = case["name"].as_str().unwrap();
        let decision = case["decision"].as_str().unwrap();
        let mut body = String::new();
        if let Some(event) = case["event"].as_str() {
            body.push_str(&format!("event: {event}\n"));
        }
        body.push_str(&format!("data: {}\n\n", case["data"]));
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/codex/responses"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(body)
                    .insert_header("content-type", "text/event-stream"),
            )
            .expect(1)
            .mount(&server)
            .await;
        let provider = OpenAiCodexResponsesProvider::new(
            Some(server.uri()),
            vec![model(None)],
            Arc::new(HttpClient::new()),
        );
        let failure = opi_ai::stream::ChatStream::new(provider.stream_prepared(
            request(),
            auth(
                "fixture-key",
                Some(server.uri()),
                Some("fixture-account".into()),
            ),
        ))
        .result()
        .await
        .unwrap_err();
        assert!(
            matches!(failure.cause(), ProviderError::StreamError(_)),
            "{name}"
        );
        assert_eq!(failure.message().raw_stop_reason, None, "{name}");
        let message = failure.message();
        let diagnostic = message.error_message.as_deref().unwrap();
        let retryable =
            classify_whole_call_retry(message) == WholeCallRetryClassification::Transient;
        let overflow = opi_ai::message::is_context_overflow(message, None);
        let quota = diagnostic.contains("quota exceeded");
        let sent = server.received_requests().await.unwrap();
        assert_eq!(sent.len(), 1, "{name}");
        assert!(!format!("{failure:?}").contains("PRIVATE_CANARY"), "{name}");
        println!(
            "codex sse error observation {}",
            json!({
                "name":name,"decision":decision,"failed":true,"diagnostic":diagnostic,
                "retryable":retryable,"overflow":overflow,"quota":quota,"sends":sent.len(),
            })
        );
        if (retryable, overflow, quota) != (decision == "transient", false, decision == "quota") {
            mismatches.push(name);
        }
    }
    assert!(
        mismatches.is_empty(),
        "SSE error decision mismatches: {mismatches:?}"
    );
}

#[tokio::test]
async fn dedicated_codex_data_only_frames_stream_to_completion() {
    // Reproduces the canonical pi-0.80.6 Codex capture: every frame is
    // data-only (no `event:` line, type in the JSON payload) and the stream
    // terminates with the `data: [DONE]` sentinel. Before the JSON-type
    // dispatch fix this terminated on the first frame; it must now stream to a
    // typed Done.
    let sse = [
        r#"data: {"type":"response.output_item.added","item":{"type":"message","id":"msg_1","role":"assistant","status":"in_progress","content":[]}}"#,
        r#"data: {"type":"response.content_part.added","part":{"type":"output_text","text":""}}"#,
        r#"data: {"type":"response.output_text.delta","delta":"Hello"}"#,
        r#"data: {"type":"response.output_item.done","item":{"type":"message","id":"msg_1","role":"assistant","status":"completed","content":[{"type":"output_text","text":"Hello"}]}}"#,
        r#"data: {"type":"response.completed","response":{"status":"completed","usage":{"input_tokens":13,"output_tokens":9,"input_tokens_details":{"cached_tokens":4},"output_tokens_details":{"reasoning_tokens":3}}}}"#,
        "data: [DONE]",
    ]
    .join("\n\n");
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/codex/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(format!("{sse}\n\n"))
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let provider = OpenAiCodexResponsesProvider::new(
        Some(server.uri()),
        vec![model(None)],
        Arc::new(HttpClient::new()),
    );
    let auth = auth(
        "sentinel-access",
        Some(server.uri()),
        Some("account-fixed".into()),
    );

    let request = request();
    let canonical = serde_json::to_vec(&request.context).unwrap();
    let message = typed_terminal(&provider, request.clone(), auth).await;

    assert_eq!(message.stop_reason, StopReason::Stop);
    assert_eq!(message.provider, "openai-codex");
    assert_eq!(message.wire, Some(WireApi::OpenAiCodexResponses));
    assert_eq!(message.model, "gpt-5.4");
    assert_eq!(
        message.content,
        [AssistantContent::Text {
            text: "Hello".into(),
            signature: None,
        }]
    );
    assert_eq!(message.usage, Usage::reported(9, 9, 4, 0, None, Some(3)));
    assert_eq!(serde_json::to_vec(&request.context).unwrap(), canonical);
    server.verify().await;
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn dedicated_codex_incomplete_terminal_is_length_not_error() {
    // `response.incomplete` (max_output_tokens) must terminate as a typed Done
    // with a length stop reason, not a stream error.
    let sse = [
        r#"data: {"type":"response.output_item.added","item":{"type":"message","id":"msg_1","role":"assistant","status":"in_progress","content":[]}}"#,
        r#"data: {"type":"response.content_part.added","part":{"type":"output_text","text":""}}"#,
        r#"data: {"type":"response.output_text.delta","delta":"Hel"}"#,
        r#"data: {"type":"response.incomplete","response":{"status":"incomplete","incomplete_details":{"reason":"max_output_tokens"},"usage":{"input_tokens":17,"output_tokens":7,"input_tokens_details":{"cached_tokens":5},"output_tokens_details":{"reasoning_tokens":2}}}}"#,
    ]
    .join("\n\n");
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/codex/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(format!("{sse}\n\n"))
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let provider = OpenAiCodexResponsesProvider::new(
        Some(server.uri()),
        vec![model(None)],
        Arc::new(HttpClient::new()),
    );
    let auth = auth(
        "sentinel-access",
        Some(server.uri()),
        Some("account-fixed".into()),
    );

    let request = request();
    let canonical = serde_json::to_vec(&request.context).unwrap();
    let message = typed_terminal(&provider, request.clone(), auth).await;

    assert_eq!(message.stop_reason, StopReason::Length);
    assert_eq!(message.provider, "openai-codex");
    assert_eq!(message.wire, Some(WireApi::OpenAiCodexResponses));
    assert_eq!(
        message.content,
        [AssistantContent::Text {
            text: "Hel".into(),
            signature: None,
        }]
    );
    assert_eq!(message.usage, Usage::reported(12, 7, 5, 0, None, Some(2)));
    assert_eq!(serde_json::to_vec(&request.context).unwrap(), canonical);
    server.verify().await;
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn dedicated_codex_completed_distinguishes_unreported_usage_from_reported_zero() {
    for (usage, expected) in [
        (None, Usage::unknown()),
        (
            Some(serde_json::json!({
                "input_tokens": 0,
                "output_tokens": 0,
                "input_tokens_details": {"cached_tokens": 0},
                "output_tokens_details": {"reasoning_tokens": 0}
            })),
            Usage::reported(0, 0, 0, 0, None, Some(0)),
        ),
    ] {
        let mut response = serde_json::json!({"status": "completed", "output": []});
        if let Some(usage) = usage {
            response["usage"] = usage;
        }
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/codex/responses"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(format!(
                        "data: {}\n\n",
                        serde_json::json!({"type": "response.completed", "response": response})
                    ))
                    .insert_header("content-type", "text/event-stream"),
            )
            .expect(1)
            .mount(&server)
            .await;
        let provider = OpenAiCodexResponsesProvider::new(
            Some(server.uri()),
            vec![model(None)],
            Arc::new(HttpClient::new()),
        );
        let request = request();
        let canonical = serde_json::to_vec(&request.context).unwrap();
        let message = typed_terminal(
            &provider,
            request.clone(),
            auth("sentinel-access", None, Some("account-fixed".into())),
        )
        .await;

        assert_eq!(message.stop_reason, StopReason::Stop);
        assert_eq!(message.usage, expected);
        assert_eq!(serde_json::to_vec(&request.context).unwrap(), canonical);
        server.verify().await;
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}

#[tokio::test]
async fn dedicated_codex_requires_non_empty_account_id_before_http() {
    let server = MockServer::start().await;
    for account_id in [None, Some("".into()), Some("   ".into())] {
        let provider = OpenAiCodexResponsesProvider::new(
            Some(server.uri()),
            vec![model(None)],
            Arc::new(HttpClient::new()),
        );
        let auth = auth("sentinel-access", Some(server.uri()), account_id);
        let request = request();
        let mut stream = provider.stream_prepared(request.clone(), auth);
        let error = stream
            .next()
            .await
            .expect("one refusal")
            .expect_err("missing account id");
        let ProviderError::StreamFailure(failure) = &error else {
            panic!("accepted projection must retain its failure carrier")
        };
        assert_eq!(
            failure.cause(),
            &ProviderError::AccountIdMissing {
                provider_id: "openai-codex".into()
            }
        );
        assert_direct_projection_prefix(failure.message(), &request);
        assert!(stream.next().await.is_none());
        assert!(!error.is_retryable());
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn dedicated_codex_rejects_managed_header_overrides_before_http() {
    let server = MockServer::start().await;
    for name in [
        "Authorization",
        "chatgpt-account-id",
        "originator",
        "OpenAI-Beta",
        "accept",
        "session-id",
        "x-client-request-id",
    ] {
        let provider = OpenAiCodexResponsesProvider::new(
            Some(server.uri()),
            vec![model(None)],
            Arc::new(HttpClient::new()),
        );
        let auth = auth(
            "sentinel-access",
            Some(server.uri()),
            Some("account-fixed".into()),
        );
        let mut request = request();
        request.extra_headers.push((name.into(), "override".into()));
        assert!(
            matches!(
                drain(&provider, request, auth).await,
                Some(ProviderError::RequestFailed(_))
            ),
            "{name}"
        );
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn dedicated_codex_rejects_invalid_header_names_and_values_before_http() {
    let server = MockServer::start().await;
    for (name, value) in [
        ("bad header", "value"),
        ("x-extra", "bad\nvalue"),
        ("x-extra", "bad\rvalue"),
        ("x-extra", "bad\0value"),
    ] {
        let provider = OpenAiCodexResponsesProvider::new(
            Some(server.uri()),
            vec![model(None)],
            Arc::new(HttpClient::new()),
        );
        let auth = auth(
            "sentinel-access",
            Some(server.uri()),
            Some("account-fixed".into()),
        );
        let mut request = request();
        request.extra_headers.push((name.into(), value.into()));
        let error = drain(&provider, request, auth)
            .await
            .expect("invalid header must fail");
        assert!(matches!(error, ProviderError::RequestFailed(_)));
        assert!(!error.is_retryable());
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn dedicated_codex_401_and_403_are_revoked_and_redacted() {
    for status in [401, 403] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/codex/responses"))
            .respond_with(
                ResponseTemplate::new(status).set_body_string("sentinel-access sentinel-envelope"),
            )
            .mount(&server)
            .await;
        let provider = OpenAiCodexResponsesProvider::new(
            Some(server.uri()),
            vec![model(None)],
            Arc::new(HttpClient::new()),
        );
        let auth = auth(
            "sentinel-access",
            Some(server.uri()),
            Some("account-fixed".into()),
        );
        let error = drain(&provider, request(), auth).await.expect("revoked");
        assert!(matches!(
            error,
            ProviderError::CredentialRevoked { ref provider_id }
                if provider_id == "openai-codex"
        ));
        let rendered = format!("{error:?} {error}");
        assert!(!rendered.contains("sentinel-access"));
        assert!(!rendered.contains("sentinel-envelope"));
        assert!(!error.is_retryable());
        // C18: revocation ends the turn with exactly one request — no
        // follow-up call, no auto-refresh attempt.
        assert_eq!(
            server.received_requests().await.unwrap().len(),
            1,
            "revocation must issue exactly one HTTP request"
        );
    }
}

#[tokio::test]
async fn dedicated_codex_provider_failures_are_typed_and_redacted() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/codex/responses"))
        .respond_with(
            ResponseTemplate::new(500).set_body_string(
                "sentinel-access sentinel-refresh sentinel-envelope sentinel-account",
            ),
        )
        .mount(&server)
        .await;
    let provider = OpenAiCodexResponsesProvider::new(
        Some(server.uri()),
        vec![model(None)],
        Arc::new(HttpClient::new()),
    );
    let auth = auth(
        "sentinel-access",
        Some(server.uri()),
        Some("sentinel-account".into()),
    );

    let error = drain(&provider, request(), auth)
        .await
        .expect("provider failure");
    assert!(matches!(error, ProviderError::ProviderSide(_)));
    let rendered = format!("{error:?} {error}");
    for sentinel in [
        "sentinel-access",
        "sentinel-refresh",
        "sentinel-envelope",
        "sentinel-account",
    ] {
        assert!(!rendered.contains(sentinel), "{sentinel}: {rendered}");
    }
}

// ---------------------------------------------------------------------------
// Task 20.51: functional callbacks at the Codex wire (P20-S51)
// ---------------------------------------------------------------------------

use opi_ai::callbacks::RequestCallbacks;

#[tokio::test]
async fn codex_payload_response_and_raw_callbacks_flow_through_mapper_events() {
    let order: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let raw_recorder = order.clone();
    let response_recorder = order.clone();
    let payload_bodies: std::sync::Arc<std::sync::Mutex<Vec<serde_json::Value>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let payload_recorder = payload_bodies.clone();

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/codex/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(codex_completed_sse())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = OpenAiCodexResponsesProvider::new(
        Some(server.uri()),
        vec![model(None)],
        Arc::new(HttpClient::new()),
    );
    let auth = auth("sentinel-access", None, Some("account-fixed".into()));
    let mut request = request();
    request.callbacks = RequestCallbacks::new()
        .with_on_payload(std::sync::Arc::new(move |body| {
            payload_recorder.lock().unwrap().push(body.clone());
            Ok(None)
        }))
        .with_on_response(std::sync::Arc::new(move |status, _headers| {
            response_recorder
                .lock()
                .unwrap()
                .push(format!("response:{status}"));
            Ok(())
        }))
        .with_on_raw(std::sync::Arc::new(move |event| {
            raw_recorder
                .lock()
                .unwrap()
                .push(format!("raw:{}", event["type"].as_str().unwrap_or("?")));
            Ok(())
        }));

    assert!(drain(&provider, request, auth).await.is_none());

    let bodies = payload_bodies.lock().unwrap().clone();
    assert_eq!(bodies.len(), 1);
    assert_eq!(bodies[0]["model"], serde_json::json!("gpt-5.4"));
    assert!(
        bodies[0]["input"].is_array(),
        "codex payload sees the mapper body"
    );
    let order = order.lock().unwrap().clone();
    assert_eq!(
        order.iter().map(String::as_str).collect::<Vec<_>>(),
        vec!["response:200", "raw:response.completed"],
        "codex HTTP wire hands mapper events to the raw observer in order"
    );
    server.verify().await;
}

#[tokio::test]
async fn codex_transform_touching_managed_headers_fails_typed_before_http() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let provider = OpenAiCodexResponsesProvider::new(
        Some(server.uri()),
        vec![model(None)],
        Arc::new(HttpClient::new()),
    );
    let auth = auth("sentinel-access", None, Some("account-fixed".into()));
    let mut request = request();
    request.callbacks =
        RequestCallbacks::new().with_header_transform(std::sync::Arc::new(|merged| {
            let mut out = merged.to_vec();
            out.push(("ChatGPT-Account-Id".to_string(), "attacker".to_string()));
            Ok(out)
        }));

    let mut stream = provider.stream_prepared(request, auth);
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
        "transform touching a codex managed header must fail typed"
    );
    server.verify().await;
}

// --- task 20.11: Codex constrained sampling (strict resolution + grammar
// fallback loss) ---

#[tokio::test]
async fn codex_source_developer_default_and_ordinary_only_flags_reach_actual_wire() {
    use opi_ai::model_info::{
        OpenAiCodexResponsesCompat, OpenAiResponsesSourceCompat, SessionAffinityFormat, WireCompat,
    };
    for flag in [None, Some(false), Some(true)] {
        let server = MockServer::start().await;
        Mock::given(method("POST")).respond_with(ResponseTemplate::new(200)
            .insert_header("content-type", "text/event-stream")
            .set_body_string("data: {\"type\":\"response.completed\",\"response\":{\"id\":\"codex-source\",\"model\":\"gpt-5.4\",\"output\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}}\n\n"))
            .expect(1).mount(&server).await;
        let source = OpenAiResponsesSourceCompat {
            supports_developer_role: flag,
            supports_strict_mode: flag,
            supports_mid_convo_system_messages: Some(true),
            supports_max_output_tokens: Some(true),
            supports_long_cache_retention: Some(true),
            supports_explicit_prompt_cache_mode: Some(true),
            supports_mid_convo_effort: Some(true),
            session_affinity_format: Some(SessionAffinityFormat::OpenRouter),
            ..Default::default()
        };
        let record = model(Some(server.uri()))
            .with_compat(WireCompat::OpenAiCodexResponses(
                OpenAiCodexResponsesCompat {
                    source,
                    ..Default::default()
                },
            ))
            .unwrap();
        let provider =
            OpenAiCodexResponsesProvider::new(None, vec![record], Arc::new(HttpClient::new()));
        let mut request = request();
        let mut context = opi_ai::transcript::Context {
            system_prompt: None,
            messages: request.context.messages().to_vec(),
            tools: vec![],
        };
        context
            .messages
            .push(Message::System(opi_ai::transcript::SystemMessage {
                content: "later instruction".into(),
                sections: vec![],
                tools_added: vec![],
                tools_removed: vec![],
                timestamp_ms: 1,
            }));
        request.context = opi_ai::transcript::normalize_context(context).unwrap();
        request.cache_retention = CacheRetention::Long;
        let message = typed_terminal(
            &provider,
            request,
            auth("offline-key", None, Some("account".into())),
        )
        .await;
        assert!(message.projection.is_some());
        let sent = server.received_requests().await.unwrap();
        assert_eq!(sent.len(), 1);
        let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
        let later = body["input"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["content"] == "later instruction")
            .unwrap();
        assert_eq!(
            later["role"],
            if flag == Some(false) {
                "system"
            } else {
                "developer"
            }
        );
        assert_eq!(
            body["tools"][0].get("strict"),
            (flag != Some(false)).then_some(&serde_json::Value::Null),
            "Codex keeps its unconstrained strict:null default only when capability is enabled"
        );
        assert_eq!(body["instructions"], "system prompt");
        assert!(
            !body["input"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["content"] == "system prompt")
        );
        assert!(body.get("max_output_tokens").is_none());
        assert!(body.get("prompt_cache_options").is_none());
        assert!(body.get("prompt_cache_retention").is_none());
        assert!(
            !body["input"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["type"] == "configuration_update")
        );
        assert!(
            !sent[0].headers.contains_key("x-session-id"),
            "ordinary affinity format stays inert on Codex"
        );
        assert_eq!(body["tool_choice"], "auto");
        assert_eq!(body["text"], serde_json::json!({"verbosity":"low"}));
        assert_eq!(body["parallel_tool_calls"], true);
        server.verify().await;
    }
}
