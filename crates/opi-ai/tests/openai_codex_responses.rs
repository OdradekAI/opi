use std::sync::Arc;

use futures_util::StreamExt;
use opi_ai::auth::{AuthScheme, ResolvedAuth};
use opi_ai::http::HttpClient;
use opi_ai::message::{
    AssistantContent, AssistantMessage, InputContent, Message, ToolDef, UserMessage,
};
use opi_ai::openai_codex_responses::OpenAiCodexResponsesProvider;
use opi_ai::provider::{CacheRetention, Provider, ProviderError, Request, ThinkingConfig};
use opi_ai::stream::{AssistantStreamEvent, StopReason, Usage};
use opi_ai::{ModelCapabilities, ModelInfo, ThinkingLevel, ThinkingLevelMap, WireApi};
use secrecy::SecretString;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[path = "openai_codex_responses/replay.rs"]
mod replay;

#[path = "openai_codex_responses/deflate_audit.rs"]
mod deflate_audit;

fn auth(secret: &str, base_url: Option<String>, account_id: Option<String>) -> ResolvedAuth {
    ResolvedAuth {
        scheme: AuthScheme::Bearer,
        secret: SecretString::from(secret),
        base_url,
        account_id,
        provenance: opi_ai::AuthProvenance::default(),
    }
}

fn model(base_url: Option<String>) -> ModelInfo {
    let mut model = ModelInfo::new(
        "gpt-5.4",
        "GPT-5.4",
        WireApi::OpenAiCodexResponses,
        ModelCapabilities::new(272_000, 128_000)
            .with_images(true)
            .with_streaming(true)
            .with_thinking(true),
    );
    model.base_url = base_url;
    model
}

fn request() -> Request {
    Request {
        transport: opi_ai::provider::TransportPreference::Sse,
        model: "openai-codex:gpt-5.4".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: Some("system prompt".into()),
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "hello".into(),
                }],
                timestamp_ms: 0,
            })],
            tools: vec![ToolDef {
                name: "read".into(),
                description: "Read a file".into(),
                input_schema: serde_json::json!({"type":"object"}),
                constrained_sampling: None,
            }],
        })
        .expect("valid context declarations"),
        max_tokens: Some(1024),
        temperature: Some(0.2),
        sampling_params: Default::default(),
        thinking: ThinkingConfig {
            enabled: true,
            budget_tokens: None,
            level: ThinkingLevel::High,
        },
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::Short,
        session_id: Some("session-fixed".into()),
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

async fn drain(
    provider: &dyn Provider,
    request: Request,
    auth: ResolvedAuth,
) -> Option<ProviderError> {
    let mut stream = provider.stream_prepared(request, auth);
    while let Some(result) = stream.next().await {
        match result {
            Ok(event) if event.is_terminal() => return None,
            Ok(_) => {}
            Err(error) => return Some(error),
        }
    }
    None
}

async fn capture_stream(
    provider: &dyn Provider,
    request: Request,
    auth: ResolvedAuth,
) -> (Vec<String>, usize, usize) {
    let mut stream = provider.stream_prepared(request, auth);
    let mut captures = Vec::new();
    let mut events = 0;
    let mut errors = 0;
    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => {
                events += 1;
                captures.push(format!("{event:?}"));
                captures.push(serde_json::to_string(&event).expect("serialize stream event"));
                if event.is_terminal() {
                    break;
                }
            }
            Err(error) => {
                errors += 1;
                captures.push(format!("{error:?} {error}"));
                break;
            }
        }
    }
    (captures, events, errors)
}

async fn typed_terminal(
    provider: &dyn Provider,
    request: Request,
    auth: ResolvedAuth,
) -> AssistantMessage {
    let mut stream = provider.stream_prepared(request, auth);
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        let mut terminal = None;
        while let Some(result) = stream.next().await {
            let event = result.expect("scripted Codex response must stream successfully");
            assert!(terminal.is_none(), "no event may follow the terminal Done");
            match event {
                AssistantStreamEvent::Done { reason, message } => {
                    assert_eq!(message.stop_reason, reason);
                    terminal = Some(message);
                }
                AssistantStreamEvent::Error { .. } => {
                    panic!("Codex response must finish with Done")
                }
                _ => {}
            }
        }
        terminal.expect("Codex response must produce exactly one Done and EOF")
    })
    .await
    .expect("Codex response must terminate within the peer deadline")
}

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
    for (label, data, sentinel) in [
        (
            "error",
            r#"{"type":"error","message":"sentinel-valid-error-message"}"#,
            "sentinel-valid-error-message",
        ),
        (
            "response.failed",
            r#"{"type":"response.failed","error":{"message":"sentinel-failed-detail"}}"#,
            "sentinel-failed-detail",
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
    }
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
            text: "Hello".into()
        }]
    );
    assert_eq!(message.usage, Usage::reported(13, 9, 4, 0, None, Some(3)));
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
        [AssistantContent::Text { text: "Hel".into() }]
    );
    assert_eq!(message.usage, Usage::reported(17, 7, 5, 0, None, Some(2)));
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
        let error = drain(&provider, request(), auth)
            .await
            .expect("missing account id");
        assert!(
            matches!(
                error,
                ProviderError::AccountIdMissing { ref provider_id }
                    if provider_id == "openai-codex"
            ),
            "{error:?}"
        );
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

fn codex_completed_sse() -> String {
    concat!(
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"response-1\",\"model\":\"gpt-5.4\",\"output\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}}\n\n",
    )
    .into()
}

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
        if matches!(item, Err(ProviderError::CallbackPhase(_))) {
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

mod codex_constrained_tests {
    use super::*;
    use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};

    fn provider() -> OpenAiCodexResponsesProvider {
        OpenAiCodexResponsesProvider::new(None, vec![model(None)], Arc::new(HttpClient::new()))
    }

    fn request_with(tools: Vec<ToolDef>) -> Request {
        let mut request = request();
        request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: Some("system prompt".into()),
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "hello".into(),
                }],
                timestamp_ms: 0,
            })],
            tools,
        })
        .expect("valid context declarations");
        request
    }

    fn json_schema_tool(strict: StrictPreference) -> ToolDef {
        ToolDef {
            name: "structured".into(),
            description: "d".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {"path": {"type": "string"}},
                "required": ["path"]
            }),
            constrained_sampling: Some(ConstrainedSampling::JsonSchema { strict }),
        }
    }

    fn grammar_tool() -> ToolDef {
        ToolDef {
            name: "grammar-tool".into(),
            description: "d".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {"input": {"type": "string"}},
                "required": ["input"]
            }),
            constrained_sampling: Some(ConstrainedSampling::Grammar {
                openai_lark: Some("start: WORD".into()),
                openai_regex: None,
            }),
        }
    }

    #[test]
    fn plain_tool_keeps_the_null_strict_default() {
        let body = provider()
            .build_request_body(&request_with(vec![ToolDef {
                name: "read".into(),
                description: "Read a file".into(),
                input_schema: serde_json::json!({"type": "object"}),
                constrained_sampling: None,
            }]))
            .unwrap();
        assert_eq!(body["tools"][0]["strict"], serde_json::json!(null));
        assert_eq!(
            body["tools"][0]["parameters"],
            serde_json::json!({"type": "object"})
        );
    }

    #[test]
    fn json_schema_tool_resolves_strict_over_the_null_default() {
        for preference in [StrictPreference::Prefer, StrictPreference::Require] {
            let body = provider()
                .build_request_body(&request_with(vec![json_schema_tool(preference)]))
                .unwrap();
            assert_eq!(body["tools"][0]["strict"], serde_json::json!(true));
            assert_eq!(
                body["tools"][0]["parameters"]["additionalProperties"],
                serde_json::json!(false)
            );
        }
    }

    #[tokio::test]
    async fn grammar_tool_falls_back_to_plain_json_with_recorded_loss() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_1\",\"model\":\"gpt-5.4\",\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}}\n\n",
            ))
            .mount(&server)
            .await;
        let provider = OpenAiCodexResponsesProvider::new(
            Some(server.uri()),
            vec![model(None)],
            Arc::new(HttpClient::new()),
        );
        let mut events = Vec::new();
        let mut stream = provider.stream_prepared(
            request_with(vec![grammar_tool()]),
            auth("k", None, Some("acct".into())),
        );
        use futures_util::StreamExt;
        while let Some(item) = stream.next().await {
            match item {
                Ok(event) => events.push(event),
                Err(_) => break,
            }
        }
        let sent = server.received_requests().await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
        assert_eq!(body["tools"][0]["type"], serde_json::json!("function"));
        assert!(body["tools"][0].get("format").is_none());
        let done = events
            .iter()
            .find_map(|event| match event {
                AssistantStreamEvent::Done { message, .. } => Some(message.clone()),
                _ => None,
            })
            .expect("stream completes");
        assert!(
            done.diagnostics.iter().any(|diagnostic| diagnostic.kind
                == "constrained_sampling_fallback"
                && diagnostic.reason == "grammar_target_unsupported"
                && diagnostic.detail.as_deref() == Some("grammar-tool")),
            "the unsupported grammar target records its loss on the message"
        );
    }
}

// ===========================================================================
// Task 20.16: real Codex WS/Auto/continuation public path. Local scripted
// peers only; no real provider endpoints are contacted.
// ===========================================================================

mod ws_path {
    fn request() -> Request {
        let mut request = super::request();
        request.transport = opi_ai::provider::TransportPreference::Auto;
        request
    }

    use super::*;
    use opi_ai::http::{HttpClientBuilder, RedirectPolicy};
    use opi_ai::provider::TransportPreference;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::{Arc, Mutex};

    // -- scripted local peer ------------------------------------------------

    pub(crate) struct PeerCapture {
        pub heads: Vec<Vec<String>>,
        pub frames: Vec<serde_json::Value>,
        pub posts: Vec<serde_json::Value>,
    }

    pub(crate) struct CodexPeer {
        pub port: u16,
        pub capture: Arc<Mutex<PeerCapture>>,
        pub(crate) handle: Option<std::thread::JoinHandle<Result<(), String>>>,
    }

    pub(crate) fn accept_token_for(key: &str) -> String {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(key.as_bytes())
            .unwrap();
        let mut nonce = [0_u8; 16];
        nonce.copy_from_slice(&bytes);
        opi_ai::ws::accept_token(&nonce)
    }

    /// Byte-wise head reader: no internal buffering, so request bodies that
    /// arrive with the head are never consumed ahead of the caller.
    pub(crate) fn read_head(stream: &mut TcpStream) -> Vec<String> {
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut lines = Vec::new();
        let mut line = Vec::new();
        let mut byte = [0; 1];
        loop {
            match stream.read(&mut byte) {
                Ok(0) => break,
                Ok(_) => {
                    if byte[0] == b'\n' {
                        if line == b"\r" || line.is_empty() {
                            break;
                        }
                        lines.push(String::from_utf8_lossy(&line).trim_end().to_owned());
                        line.clear();
                    } else {
                        line.push(byte[0]);
                    }
                }
                Err(_error) => break,
            }
        }
        lines
    }

    pub(crate) fn header<'a>(lines: &'a [String], name: &str) -> Option<&'a str> {
        lines
            .iter()
            .filter_map(|l| l.split_once(':'))
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.trim())
    }

    pub(crate) fn response_101(lines: &[String], extra: &[(&str, &str)]) -> Vec<u8> {
        let accept = accept_token_for(header(lines, "sec-websocket-key").unwrap());
        let mut response = format!(
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n"
        );
        for (name, value) in extra {
            response.push_str(&format!("{name}: {value}\r\n"));
        }
        response.push_str("\r\n");
        response.into_bytes()
    }

    pub(crate) fn sse_response(events: &str, status: u16) -> Vec<u8> {
        let reason = if status == 200 { "OK" } else { "Bad Request" };
        format!("HTTP/1.1 {status} {reason}\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{events}", events.len()).into_bytes()
    }

    /// Read one masked client frame; returns (opcode, payload).
    pub(crate) fn read_frame(stream: &mut TcpStream) -> Result<(u8, Vec<u8>), String> {
        read_frame_with_compression(stream).map(|(opcode, payload, _)| (opcode, payload))
    }

    pub(crate) fn read_frame_with_compression(
        stream: &mut TcpStream,
    ) -> Result<(u8, Vec<u8>, bool), String> {
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut head = [0; 2];
        stream.read_exact(&mut head).map_err(|e| e.to_string())?;
        let opcode = head[0] & 0x0f;
        let masked = head[1] & 0x80 != 0;
        let len = match head[1] & 0x7f {
            126 => {
                let mut extended = [0; 2];
                stream
                    .read_exact(&mut extended)
                    .map_err(|e| e.to_string())?;
                u16::from_be_bytes(extended) as usize
            }
            127 => {
                let mut extended = [0; 8];
                stream
                    .read_exact(&mut extended)
                    .map_err(|e| e.to_string())?;
                u64::from_be_bytes(extended) as usize
            }
            value => value as usize,
        };
        let mut mask = [0; 4];
        if masked {
            stream.read_exact(&mut mask).map_err(|e| e.to_string())?;
        }
        let mut payload = vec![0; len];
        stream.read_exact(&mut payload).map_err(|e| e.to_string())?;
        if masked {
            for (i, byte) in payload.iter_mut().enumerate() {
                *byte ^= mask[i % 4];
            }
        }
        Ok((opcode, payload, head[0] & 0x40 != 0))
    }

    /// Write one unmasked server frame.
    pub(crate) fn write_frame(stream: &mut TcpStream, opcode: u8, payload: &[u8]) {
        let mut frame = vec![0x80 | opcode];
        match payload.len() {
            0..=125 => frame.push(payload.len() as u8),
            126..=65535 => {
                frame.push(126);
                frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
            }
            length => {
                frame.push(127);
                frame.extend_from_slice(&(length as u64).to_be_bytes());
            }
        }
        frame.extend_from_slice(payload);
        stream.write_all(&frame).unwrap();
        stream.flush().unwrap();
    }

    pub(crate) fn drain_to_eof(stream: &mut TcpStream) -> Result<usize, String> {
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut total = 0;
        let mut buffer = [0; 512];
        loop {
            match stream.read(&mut buffer) {
                Ok(0) => return Ok(total),
                Ok(n) => total += n,
                Err(error) => {
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        || error.kind() == std::io::ErrorKind::TimedOut
                    {
                        return Ok(total);
                    }
                    return Err(error.to_string());
                }
            }
        }
    }

    pub(crate) fn completed_event() -> String {
        "{\"type\":\"response.completed\",\"response\":{\"id\":\"response-1\",\"model\":\"gpt-5.4\",\"output\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}}".to_owned()
    }

    impl CodexPeer {
        /// Spawn a peer thread running `handler` once per accepted connection.
        pub(crate) fn spawn<H>(handler: H) -> Self
        where
            H: Fn(TcpStream, Vec<String>, &Arc<Mutex<PeerCapture>>) -> Result<(), String>
                + Send
                + 'static,
        {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            let capture = Arc::new(Mutex::new(PeerCapture {
                heads: Vec::new(),
                frames: Vec::new(),
                posts: Vec::new(),
            }));
            let shared = capture.clone();
            let handle = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().map_err(|e| e.to_string())?;
                stream.set_nodelay(true).unwrap();
                let lines = read_head(&mut stream);
                shared.lock().unwrap().heads.push(lines.clone());
                if header(&lines, "upgrade").is_some() {
                    handler(stream, lines, &shared)
                } else {
                    // POST: SSE-style response; record the JSON body.
                    let length = header(&lines, "content-length")
                        .and_then(|v| v.parse::<usize>().ok())
                        .unwrap_or(0);
                    let mut body = vec![0; length];
                    stream
                        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                        .unwrap();
                    stream.read_exact(&mut body).map_err(|e| e.to_string())?;
                    let value: serde_json::Value =
                        serde_json::from_slice(&body).unwrap_or_default();
                    shared.lock().unwrap().posts.push(value);
                    handler(stream, lines, &shared)
                }
            });
            CodexPeer {
                port,
                capture,
                handle: Some(handle),
            }
        }

        pub(crate) fn base_url(&self) -> String {
            format!("http://127.0.0.1:{}", self.port)
        }

        pub(crate) fn finish(mut self) -> Result<Arc<Mutex<PeerCapture>>, String> {
            self.handle
                .take()
                .expect("peer thread")
                .join()
                .map_err(|_| "server thread panicked".to_string())??;
            Ok(self.capture)
        }
    }

    fn none_redirect_client() -> Arc<opi_ai::http::HttpClient> {
        Arc::new(
            HttpClientBuilder::new()
                .redirects(RedirectPolicy::None)
                .build()
                .expect("client"),
        )
    }

    // -- transport resolution -------------------------------------------------

    #[tokio::test]
    async fn default_auto_uses_real_websocket_path() {
        let peer = CodexPeer::spawn(
            |mut stream: TcpStream,
             lines: Vec<String>,
             capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
                assert_eq!(
                    header(&lines, "openai-beta").unwrap(),
                    "responses_websockets=2026-02-06",
                    "WS handshake must carry the websocket beta header"
                );
                assert_eq!(header(&lines, "authorization").unwrap(), "Bearer ws-secret");
                assert_eq!(header(&lines, "chatgpt-account-id").unwrap(), "acct-1");
                assert_eq!(header(&lines, "originator").unwrap(), "opi");
                assert_eq!(header(&lines, "session-id").unwrap(), "session-fixed");
                assert_eq!(
                    header(&lines, "x-client-request-id").unwrap(),
                    "session-fixed"
                );
                assert!(
                    header(&lines, "content-type").is_none(),
                    "WS handshake drops the SSE content-type header"
                );
                stream.write_all(&response_101(&lines, &[])).unwrap();
                let (opcode, payload) = read_frame(&mut stream)?;
                assert_eq!(opcode, 1, "response.create travels as a text frame");
                let frame: serde_json::Value = serde_json::from_slice(&payload).unwrap();
                capture.lock().unwrap().frames.push(frame);
                write_frame(&mut stream, 1, completed_event().as_bytes());
                // Auto with an active session keeps the connection pooled: no
                // Close handshake; the socket closes silently at provider drop.
                drain_to_eof(&mut stream)?;
                Ok(())
            },
        );
        let provider = OpenAiCodexResponsesProvider::new(
            Some(peer.base_url()),
            vec![model(None)],
            none_redirect_client(),
        );
        let terminal = typed_terminal(
            &provider,
            request(),
            auth("ws-secret", None, Some("acct-1".into())),
        )
        .await;
        assert_eq!(terminal.response_id.as_deref(), Some("response-1"));
        let capture = peer.finish().expect("peer script");
        let frame = capture.lock().unwrap().frames[0].clone();
        assert_eq!(frame["type"], "response.create");
        assert_eq!(frame["model"], "gpt-5.4");
        assert_eq!(frame["store"], false);
        assert_eq!(frame["stream"], true);
        assert_eq!(frame["prompt_cache_key"], "session-fixed");
        assert!(!frame["input"].as_array().unwrap().is_empty());
        // ONB-03: the transport preference is a typed transport-layer hint
        // only; it never leaks into the wire body under any mode.
        assert!(
            frame.get("transport").is_none(),
            "transport preference must not appear in the request body"
        );
    }

    #[tokio::test]
    async fn explicit_sse_never_attempts_websocket() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/codex/responses"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(codex_completed_sse()),
            )
            .mount(&server)
            .await;
        // A listener that fails any WS upgrade attempt.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let guard = tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                let mut buffer = [0; 512];
                use tokio::io::AsyncReadExt;
                let _ = socket.read(&mut buffer).await;
                use tokio::io::AsyncWriteExt;
                let _ = socket
                    .write_all(b"HTTP/1.1 500 Internal Server Error\r\ncontent-length: 0\r\n\r\n")
                    .await;
            }
        });
        let provider = OpenAiCodexResponsesProvider::new(
            Some(server.uri()),
            vec![model(None)],
            Arc::new(HttpClient::new()),
        );
        let mut call = request();
        call.transport = TransportPreference::Sse;
        let terminal =
            typed_terminal(&provider, call, auth("sse-secret", None, Some("a".into()))).await;
        assert_eq!(terminal.response_id.as_deref(), Some("response-1"));
        guard.abort();
    }

    #[tokio::test]
    async fn explicit_websocket_reuses_connection_without_continuation() {
        let peer = CodexPeer::spawn(
            |mut stream: TcpStream, lines: Vec<String>, capture: &Arc<Mutex<PeerCapture>>| {
                stream.write_all(&response_101(&lines, &[])).unwrap();
                for _turn in 0..2 {
                    let (opcode, payload) = read_frame(&mut stream)?;
                    assert_eq!(opcode, 1);
                    capture
                        .lock()
                        .unwrap()
                        .frames
                        .push(serde_json::from_slice(&payload).unwrap());
                    write_frame(&mut stream, 1, completed_event().as_bytes());
                }
                // Pooled connection: no Close handshake; the socket closes
                // silently at provider drop.
                drain_to_eof(&mut stream)?;
                Ok(())
            },
        );
        let provider = OpenAiCodexResponsesProvider::new(
            Some(peer.base_url()),
            vec![model(None)],
            none_redirect_client(),
        );
        for call in 0..2 {
            let mut call_request = request();
            call_request.transport = TransportPreference::Websocket;
            let mut stream = provider
                .stream_prepared(call_request, auth("ws-secret", None, Some("acct-1".into())));
            let mut saw_done = false;
            while let Some(result) = stream.next().await {
                if let AssistantStreamEvent::Done { message, .. } =
                    result.expect("scripted Codex response must stream successfully")
                {
                    assert_eq!(message.response_id.as_deref(), Some("response-1"));
                    saw_done = true;
                }
            }
            assert!(saw_done, "call {call} must finish with Done");
        }
        let capture = peer.finish().expect("peer script");
        {
            let guard = capture.lock().unwrap();
            assert_eq!(
                guard.heads.len(),
                1,
                "non-cached websocket still pools the idle connection"
            );
            assert_eq!(guard.frames.len(), 2);
            for frame in &guard.frames {
                assert!(
                    frame.get("previous_response_id").is_none(),
                    "non-cached websocket never sends continuation deltas"
                );
                assert_eq!(frame["input"].as_array().unwrap().len(), 1);
            }
        }
    }
}

mod ws_continuation {
    fn request() -> Request {
        let mut request = super::request();
        request.transport = opi_ai::provider::TransportPreference::Auto;
        request
    }

    use super::ws_path::{
        CodexPeer, completed_event, drain_to_eof, read_frame, response_101, write_frame,
    };
    use super::*;
    use opi_ai::http::{HttpClientBuilder, RedirectPolicy};
    use opi_ai::provider::TransportPreference;
    use std::io::Write;
    use std::net::{TcpListener, TcpStream};
    use std::sync::{Arc, Mutex};

    impl CodexPeer {
        /// Spawn a peer accepting connections until `total` arrive or 10s of
        /// idle elapse. WebSocket connections consume the next handler;
        /// plain POST requests are answered with the shared SSE corpus.
        pub(crate) fn spawn_many<H>(handlers: Vec<H>, total: usize) -> Self
        where
            H: Fn(
                    TcpStream,
                    Vec<String>,
                    &Arc<Mutex<super::ws_path::PeerCapture>>,
                ) -> Result<(), String>
                + Send
                + 'static,
        {
            use std::collections::VecDeque;
            use std::io::{Read, Write};
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener
                .set_nonblocking(true)
                .expect("nonblocking listener");
            let port = listener.local_addr().unwrap().port();
            let capture = Arc::new(Mutex::new(super::ws_path::PeerCapture {
                heads: Vec::new(),
                frames: Vec::new(),
                posts: Vec::new(),
            }));
            let shared = capture.clone();
            let mut handlers: VecDeque<H> = handlers.into();
            let handle = std::thread::spawn(move || {
                let mut accepted = 0;
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
                while accepted < total {
                    if std::time::Instant::now() > deadline {
                        break;
                    }
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            accepted += 1;
                            // Windows accepted sockets inherit nonblocking mode.
                            // Every scripted read below expects a blocking socket.
                            stream.set_nonblocking(false).unwrap();
                            stream.set_nodelay(true).unwrap();
                            let lines = super::ws_path::read_head(&mut stream);
                            shared.lock().unwrap().heads.push(lines.clone());
                            if super::ws_path::header(&lines, "upgrade").is_some() {
                                let handler = handlers
                                    .pop_front()
                                    .ok_or("websocket connection without handler")?;
                                handler(stream, lines, &shared)?;
                            } else {
                                let length = super::ws_path::header(&lines, "content-length")
                                    .and_then(|v| v.parse::<usize>().ok())
                                    .unwrap_or(0);
                                let mut body = vec![0; length];
                                stream
                                    .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                                    .unwrap();
                                stream.read_exact(&mut body).map_err(|e| e.to_string())?;
                                let value: serde_json::Value =
                                    serde_json::from_slice(&body).unwrap_or_default();
                                shared.lock().unwrap().posts.push(value);
                                let response =
                                    super::ws_path::sse_response(&codex_completed_sse(), 200);
                                stream.write_all(&response).map_err(|e| e.to_string())?;
                                let _ = stream.shutdown(std::net::Shutdown::Both);
                            }
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(std::time::Duration::from_millis(5));
                        }
                        Err(error) => return Err(error.to_string()),
                    }
                }
                Ok(())
            });
            CodexPeer {
                port,
                capture,
                handle: Some(handle),
            }
        }
    }

    fn ws_provider(base_url: String) -> OpenAiCodexResponsesProvider {
        OpenAiCodexResponsesProvider::new(
            Some(base_url),
            vec![model(None)],
            Arc::new(
                HttpClientBuilder::new()
                    .redirects(RedirectPolicy::None)
                    .build()
                    .unwrap(),
            ),
        )
    }

    /// A second-turn request: same system prompt and tools, one more user
    /// message than the shared `request()` helper.
    pub(crate) fn second_turn_request() -> Request {
        let mut request = request();
        request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: Some("system prompt".into()),
            messages: vec![
                Message::User(UserMessage {
                    content: vec![InputContent::Text {
                        text: "hello".into(),
                    }],
                    timestamp_ms: 0,
                }),
                Message::User(UserMessage {
                    content: vec![InputContent::Text {
                        text: "second turn".into(),
                    }],
                    timestamp_ms: 0,
                }),
            ],
            tools: vec![ToolDef {
                name: "read".into(),
                description: "Read a file".into(),
                input_schema: serde_json::json!({"type":"object"}),
                constrained_sampling: None,
            }],
        })
        .expect("valid context declarations");
        request
    }

    pub(crate) async fn frame_call(provider: &OpenAiCodexResponsesProvider, request: Request) {
        // Drain to stream end: the producer finishing guarantees pooled
        // connections are released before the next call acquires them.
        let mut stream =
            provider.stream_prepared(request, auth("ws-secret", None, Some("acct-1".into())));
        let mut saw_done = false;
        while let Some(result) = stream.next().await {
            if let AssistantStreamEvent::Done { .. } =
                result.expect("scripted Codex response must stream successfully")
            {
                saw_done = true;
            }
        }
        assert!(saw_done, "stream must finish with Done");
    }

    #[tokio::test]
    async fn cached_continuation_sends_delta_over_one_pooled_connection() {
        let peer = CodexPeer::spawn_many(
            vec![|mut stream: TcpStream,
                  lines: Vec<String>,
                  capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
                stream.write_all(&response_101(&lines, &[])).unwrap();
                let (opcode, payload) = read_frame(&mut stream)?;
                assert_eq!(opcode, 1);
                let first: serde_json::Value = serde_json::from_slice(&payload).unwrap();
                capture.lock().unwrap().frames.push(first);
                write_frame(&mut stream, 1, completed_event().as_bytes());
                // The pooled connection stays open; the second request reuses it.
                let (opcode, payload) = read_frame(&mut stream)?;
                assert_eq!(opcode, 1);
                let second: serde_json::Value = serde_json::from_slice(&payload).unwrap();
                capture.lock().unwrap().frames.push(second);
                write_frame(&mut stream, 1, completed_event().as_bytes());
                // No Close handshake: the connection returns to the pool and
                // closes silently when the provider is dropped.
                drain_to_eof(&mut stream)?;
                Ok(())
            }],
            1,
        );
        let provider = ws_provider(peer.base_url());
        let mut first = request();
        first.transport = TransportPreference::WebsocketCached;
        frame_call(&provider, first).await;
        let mut second = second_turn_request();
        second.transport = TransportPreference::WebsocketCached;
        frame_call(&provider, second).await;
        let capture = peer.finish().expect("peer script");
        {
            let guard = capture.lock().unwrap();
            assert_eq!(guard.heads.len(), 1, "both calls share one connection");
            let first = &guard.frames[0];
            let second = &guard.frames[1];
            assert!(
                first.get("previous_response_id").is_none(),
                "first request is full input"
            );
            assert_eq!(first["input"].as_array().unwrap().len(), 1);
            assert_eq!(
                second["previous_response_id"], "response-1",
                "second request continues from the published response id"
            );
            let delta = second["input"].as_array().unwrap();
            assert_eq!(delta.len(), 1, "delta input carries only the new turn");
            assert_eq!(
                delta[0]["content"], "second turn",
                "the delta item is exactly the new turn's input item"
            );
        }
    }

    #[tokio::test]
    async fn cached_continuation_falls_back_to_full_input_on_noninput_change() {
        let peer = CodexPeer::spawn_many(
            vec![|mut stream: TcpStream,
                  lines: Vec<String>,
                  capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
                stream.write_all(&response_101(&lines, &[])).unwrap();
                for _turn in 0..2 {
                    let (opcode, payload) = read_frame(&mut stream)?;
                    assert_eq!(opcode, 1);
                    capture
                        .lock()
                        .unwrap()
                        .frames
                        .push(serde_json::from_slice(&payload).unwrap());
                    write_frame(&mut stream, 1, completed_event().as_bytes());
                }
                // No Close handshake: the pooled connection is reused.
                drain_to_eof(&mut stream)?;
                Ok(())
            }],
            1,
        );
        let provider = ws_provider(peer.base_url());
        let mut first = request();
        first.transport = TransportPreference::WebsocketCached;
        frame_call(&provider, first).await;
        let mut second = second_turn_request();
        second.transport = TransportPreference::WebsocketCached;
        second.temperature = Some(0.7);
        frame_call(&provider, second).await;
        let capture = peer.finish().expect("peer script");
        {
            let guard = capture.lock().unwrap();
            let second = &guard.frames[1];
            assert!(
                second.get("previous_response_id").is_none(),
                "changed non-input fields force full input"
            );
            assert_eq!(second["input"].as_array().unwrap().len(), 2);
        }
    }

    #[tokio::test]
    async fn continuation_not_published_without_response_id() {
        let idless =
            "{\"type\":\"response.completed\",\"response\":{\"model\":\"gpt-5.4\",\"output\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}}".to_owned();
        let peer = CodexPeer::spawn_many(
            vec![move |mut stream: TcpStream,
                       lines: Vec<String>,
                       capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
                stream.write_all(&response_101(&lines, &[])).unwrap();
                for turn in 0..2 {
                    let (opcode, payload) = read_frame(&mut stream)?;
                    assert_eq!(opcode, 1);
                    capture
                        .lock()
                        .unwrap()
                        .frames
                        .push(serde_json::from_slice(&payload).unwrap());
                    let event = if turn == 0 {
                        idless.as_bytes().to_vec()
                    } else {
                        completed_event().into_bytes()
                    };
                    write_frame(&mut stream, 1, &event);
                }
                // No Close handshake: the pooled connection is reused.
                drain_to_eof(&mut stream)?;
                Ok(())
            }],
            1,
        );
        let provider = ws_provider(peer.base_url());
        let mut first = request();
        first.transport = TransportPreference::WebsocketCached;
        // The id-less completion still finishes the call...
        let mut stream =
            provider.stream_prepared(first, auth("ws-secret", None, Some("acct-1".into())));
        while let Some(result) = stream.next().await {
            assert!(result.is_ok(), "id-less completion must still stream");
        }
        // ...but publishes no continuation: the next request is full input.
        let mut second = second_turn_request();
        second.transport = TransportPreference::WebsocketCached;
        frame_call(&provider, second).await;
        let capture = peer.finish().expect("peer script");
        {
            let guard = capture.lock().unwrap();
            assert!(guard.frames[1].get("previous_response_id").is_none());
            assert_eq!(guard.frames[1]["input"].as_array().unwrap().len(), 2);
        }
    }

    #[tokio::test]
    async fn sessionless_requests_use_temporary_connections() {
        let peer = CodexPeer::spawn_many(
            vec![
                |mut stream: TcpStream,
                 lines: Vec<String>,
                 capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
                    stream.write_all(&response_101(&lines, &[])).unwrap();
                    let (opcode, payload) = read_frame(&mut stream)?;
                    assert_eq!(opcode, 1);
                    capture
                        .lock()
                        .unwrap()
                        .frames
                        .push(serde_json::from_slice(&payload).unwrap());
                    write_frame(&mut stream, 1, completed_event().as_bytes());
                    assert_eq!(read_frame(&mut stream)?.0, 8);
                    write_frame(&mut stream, 8, &[]);
                    drain_to_eof(&mut stream)?;
                    Ok(())
                },
                |mut stream: TcpStream,
                 lines: Vec<String>,
                 capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
                    stream.write_all(&response_101(&lines, &[])).unwrap();
                    let (opcode, payload) = read_frame(&mut stream)?;
                    assert_eq!(opcode, 1);
                    capture
                        .lock()
                        .unwrap()
                        .frames
                        .push(serde_json::from_slice(&payload).unwrap());
                    write_frame(&mut stream, 1, completed_event().as_bytes());
                    assert_eq!(read_frame(&mut stream)?.0, 8);
                    write_frame(&mut stream, 8, &[]);
                    drain_to_eof(&mut stream)?;
                    Ok(())
                },
            ],
            2,
        );
        let provider = ws_provider(peer.base_url());
        for turn in 0..2 {
            let mut call = if turn == 0 {
                request()
            } else {
                second_turn_request()
            };
            call.cache_retention = CacheRetention::Disabled;
            call.transport = TransportPreference::WebsocketCached;
            frame_call(&provider, call).await;
        }
        let capture = peer.finish().expect("peer script");
        {
            let guard = capture.lock().unwrap();
            assert_eq!(guard.heads.len(), 2, "sessionless calls never pool");
            for frame in &guard.frames {
                assert!(frame.get("previous_response_id").is_none());
            }
        }
    }
}

mod ws_branches {
    fn request() -> Request {
        let mut request = super::request();
        request.transport = opi_ai::provider::TransportPreference::Auto;
        request
    }

    use super::ws_continuation::frame_call;
    use super::ws_continuation::second_turn_request;
    use super::ws_path::{
        CodexPeer, completed_event, drain_to_eof, read_frame, response_101, write_frame,
    };
    use super::*;
    use opi_ai::http::{HttpClientBuilder, RedirectPolicy};
    use opi_ai::provider::TransportPreference;
    use std::io::Write;
    use std::net::TcpStream;
    use std::sync::{Arc, Mutex};

    fn ws_provider(base_url: String) -> OpenAiCodexResponsesProvider {
        OpenAiCodexResponsesProvider::new(
            Some(base_url),
            vec![model(None)],
            Arc::new(
                HttpClientBuilder::new()
                    .redirects(RedirectPolicy::None)
                    .build()
                    .unwrap(),
            ),
        )
    }

    fn auth_ws() -> ResolvedAuth {
        auth("ws-secret", None, Some("acct-1".into()))
    }

    /// Read one frame and reply with a JSON error event.
    fn reply_error(stream: &mut TcpStream, code: &str) -> Result<(), String> {
        let (opcode, _) = read_frame(stream)?;
        assert_eq!(opcode, 1);
        let error = format!(r#"{{"type":"error","code":"{code}"}}"#);
        write_frame(stream, 1, error.as_bytes());
        drain_to_eof(stream)?;
        Ok(())
    }

    #[tokio::test]
    async fn missing_previous_response_allows_one_full_input_retry() {
        let peer = CodexPeer::spawn_many(
            vec![
                // One pooled connection: call 1's full input, then call 2's
                // delta, which the server rejects.
                |mut stream: TcpStream,
                 lines: Vec<String>,
                 capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
                    stream.write_all(&response_101(&lines, &[])).unwrap();
                    for turn in 0..2 {
                        let (opcode, payload) = read_frame(&mut stream)?;
                        assert_eq!(opcode, 1);
                        capture
                            .lock()
                            .unwrap()
                            .frames
                            .push(serde_json::from_slice(&payload).unwrap());
                        if turn == 0 {
                            write_frame(&mut stream, 1, completed_event().as_bytes());
                        } else {
                            let error = r#"{"type":"error","code":"previous_response_not_found"}"#;
                            write_frame(&mut stream, 1, error.as_bytes());
                        }
                    }
                    drain_to_eof(&mut stream)?;
                    Ok(())
                },
                // Call 2 attempt 2: full input on a fresh connection.
                |mut stream: TcpStream,
                 lines: Vec<String>,
                 capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
                    stream.write_all(&response_101(&lines, &[])).unwrap();
                    let (opcode, payload) = read_frame(&mut stream)?;
                    assert_eq!(opcode, 1);
                    capture
                        .lock()
                        .unwrap()
                        .frames
                        .push(serde_json::from_slice(&payload).unwrap());
                    write_frame(&mut stream, 1, completed_event().as_bytes());
                    drain_to_eof(&mut stream)?;
                    Ok(())
                },
            ],
            2,
        );
        let provider = ws_provider(peer.base_url());
        let mut first = request();
        first.transport = TransportPreference::WebsocketCached;
        frame_call(&provider, first).await;
        let mut second = second_turn_request();
        second.transport = TransportPreference::WebsocketCached;
        // The retry happens before public output: the stream still finishes
        // with exactly one Done.
        frame_call(&provider, second).await;
        let capture = peer.finish().expect("peer script");
        {
            let guard = capture.lock().unwrap();
            assert_eq!(guard.frames.len(), 3);
            assert_eq!(guard.frames[1]["previous_response_id"], "response-1");
            assert!(
                guard.frames[2].get("previous_response_id").is_none(),
                "the retry sends full input"
            );
            assert_eq!(guard.frames[2]["input"].as_array().unwrap().len(), 2);
        }
    }

    #[tokio::test]
    async fn missing_previous_response_retry_is_bounded_to_one() {
        let peer = CodexPeer::spawn_many(
            vec![
                |mut stream: TcpStream,
                 lines: Vec<String>,
                 _capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
                    stream.write_all(&response_101(&lines, &[])).unwrap();
                    let (opcode, _) = read_frame(&mut stream)?;
                    assert_eq!(opcode, 1);
                    write_frame(&mut stream, 1, completed_event().as_bytes());
                    reply_error(&mut stream, "previous_response_not_found")
                },
                reply_missing,
            ],
            2,
        );
        let provider = ws_provider(peer.base_url());
        let mut first = request();
        first.transport = TransportPreference::WebsocketCached;
        frame_call(&provider, first).await;
        let mut second = second_turn_request();
        second.transport = TransportPreference::WebsocketCached;
        let error = drain(&provider, second, auth_ws()).await;
        assert!(
            error.is_some(),
            "the second missing-id rejection is terminal"
        );
        peer.finish().expect("peer script");
    }

    fn reply_missing(
        mut stream: TcpStream,
        lines: Vec<String>,
        _capture: &Arc<Mutex<super::ws_path::PeerCapture>>,
    ) -> Result<(), String> {
        stream.write_all(&response_101(&lines, &[])).unwrap();
        reply_error(&mut stream, "previous_response_not_found")
    }

    #[tokio::test]
    async fn connection_limit_before_start_allows_one_reconnect() {
        let peer = CodexPeer::spawn_many(
            vec![
                |mut stream: TcpStream,
                 lines: Vec<String>,
                 _capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
                    stream.write_all(&response_101(&lines, &[])).unwrap();
                    reply_error(&mut stream, "websocket_connection_limit_reached")
                },
                |mut stream: TcpStream,
                 lines: Vec<String>,
                 capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
                    stream.write_all(&response_101(&lines, &[])).unwrap();
                    let (opcode, payload) = read_frame(&mut stream)?;
                    assert_eq!(opcode, 1);
                    capture
                        .lock()
                        .unwrap()
                        .frames
                        .push(serde_json::from_slice(&payload).unwrap());
                    write_frame(&mut stream, 1, completed_event().as_bytes());
                    drain_to_eof(&mut stream)?;
                    Ok(())
                },
            ],
            2,
        );
        let provider = ws_provider(peer.base_url());
        let mut call = request();
        call.transport = TransportPreference::WebsocketCached;
        frame_call(&provider, call).await;
        let capture = peer.finish().expect("peer script");
        {
            let guard = capture.lock().unwrap();
            assert_eq!(guard.heads.len(), 2, "one reconnect, then success");
            assert!(
                guard.frames[0].get("previous_response_id").is_none(),
                "the successful reconnect sends full input"
            );
        }
    }

    #[tokio::test]
    async fn transport_failure_before_events_falls_back_to_sse() {
        let peer = CodexPeer::spawn_many(
            vec![|mut stream: TcpStream,
                  lines: Vec<String>,
                  _capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
                stream.write_all(&response_101(&lines, &[])).unwrap();
                // Abrupt transport loss before any provider event.
                let _ = read_frame(&mut stream);
                drop(stream);
                Ok(())
            }],
            2,
        );
        let provider = ws_provider(peer.base_url());
        let mut call = request();
        call.transport = TransportPreference::Auto;
        let terminal = typed_terminal(&provider, call, auth_ws()).await;
        assert_eq!(terminal.response_id.as_deref(), Some("response-1"));
        let capture = peer.finish().expect("peer script");
        {
            let guard = capture.lock().unwrap();
            assert_eq!(guard.heads.len(), 2, "one WS attempt then one SSE POST");
            assert!(
                guard.heads[1].first().unwrap().starts_with("POST"),
                "the fallback request is the SSE transport"
            );
            assert_eq!(guard.posts.len(), 1);
        }
    }

    #[tokio::test]
    async fn upgrade_auth_status_is_terminal_without_fallback() {
        let peer = CodexPeer::spawn_many(
            vec![|mut stream: TcpStream,
                  _lines: Vec<String>,
                  _capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
                let _ = read_frame(&mut stream);
                stream
                    .write_all(b"HTTP/1.1 401 Unauthorized\r\ncontent-length: 0\r\n\r\n")
                    .unwrap();
                Ok(())
            }],
            1,
        );
        let provider = ws_provider(peer.base_url());
        let mut call = request();
        call.transport = TransportPreference::Websocket;
        let error = drain(&provider, call, auth_ws()).await;
        assert!(
            matches!(error, Some(ProviderError::CredentialRevoked { .. })),
            "401 on upgrade stays a revoked-credential failure, got {error:?}"
        );
        let capture = peer.finish().expect("peer script");
        {
            let guard = capture.lock().unwrap();
            assert!(
                guard.posts.is_empty(),
                "auth failure never masquerades as transport loss"
            );
        }
    }

    #[tokio::test]
    async fn started_failure_is_terminal_and_ends_the_stream() {
        let events = [
            r#"{"type":"response.output_item.added","item":{"type":"message","id":"msg_1","role":"assistant","status":"in_progress","content":[]}}"#,
            r#"{"type":"response.content_part.added","part":{"type":"output_text","text":""}}"#,
            r#"{"type":"response.output_text.delta","delta":"Hello"}"#,
        ];
        let peer = CodexPeer::spawn_many(
            vec![move |mut stream: TcpStream,
                       lines: Vec<String>,
                       _capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
                stream.write_all(&response_101(&lines, &[])).unwrap();
                let (opcode, _) = read_frame(&mut stream)?;
                assert_eq!(opcode, 1);
                // Effective provider events, then abrupt transport loss.
                for event in events {
                    write_frame(&mut stream, 1, event.as_bytes());
                }
                let _ = read_frame(&mut stream);
                drop(stream);
                Ok(())
            }],
            1,
        );
        let provider = ws_provider(peer.base_url());
        let mut call = request();
        call.transport = TransportPreference::WebsocketCached;
        let error = drain(&provider, call, auth_ws()).await;
        assert!(
            matches!(
                error,
                Some(ProviderError::StreamError(_)) | Some(ProviderError::StreamFailure(_))
            ),
            "an output-delivered error ends the stream, got {error:?}"
        );
        let capture = peer.finish().expect("peer script");
        assert!(
            capture.lock().unwrap().posts.is_empty(),
            "no SSE splice after output"
        );
    }

    #[tokio::test]
    async fn fallback_is_sticky_for_the_session() {
        let peer = CodexPeer::spawn_many(
            vec![|mut stream: TcpStream,
                  lines: Vec<String>,
                  _capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
                stream.write_all(&response_101(&lines, &[])).unwrap();
                let _ = read_frame(&mut stream);
                drop(stream);
                Ok(())
            }],
            3,
        );
        let provider = ws_provider(peer.base_url());
        for call_index in 0..2 {
            let mut call = if call_index == 0 {
                request()
            } else {
                second_turn_request()
            };
            call.transport = TransportPreference::Auto;
            let terminal = typed_terminal(&provider, call, auth_ws()).await;
            assert_eq!(terminal.response_id.as_deref(), Some("response-1"));
        }
        let capture = peer.finish().expect("peer script");
        {
            let guard = capture.lock().unwrap();
            assert_eq!(
                guard.heads.len(),
                3,
                "first call: WS + SSE; second call: SSE only"
            );
            assert!(
                guard.heads[1].first().unwrap().starts_with("POST")
                    && guard.heads[2].first().unwrap().starts_with("POST"),
                "the sticky marker keeps later calls on SSE"
            );
        }
    }

    #[tokio::test]
    async fn idle_timeout_before_events_falls_back_to_sse() {
        let peer = CodexPeer::spawn_many(
            vec![|mut stream: TcpStream,
                  lines: Vec<String>,
                  _capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
                stream.write_all(&response_101(&lines, &[])).unwrap();
                // Hold the connection open without sending any event; the
                // hold runs off-thread so the peer can serve the SSE fallback.
                std::thread::spawn(move || {
                    let mut held = stream;
                    let (opcode, _) = read_frame(&mut held).unwrap_or((0, Vec::new()));
                    assert_eq!(opcode, 1);
                    std::thread::sleep(std::time::Duration::from_millis(600));
                });
                Ok(())
            }],
            2,
        );
        let provider = ws_provider(peer.base_url());
        let mut call = request();
        call.transport = TransportPreference::WebsocketCached;
        call.timeout = Some(std::time::Duration::from_millis(150));
        let terminal = typed_terminal(&provider, call, auth_ws()).await;
        assert_eq!(terminal.response_id.as_deref(), Some("response-1"));
        let capture = peer.finish().expect("peer script");
        assert_eq!(
            capture.lock().unwrap().posts.len(),
            1,
            "idle timeout falls back once"
        );
    }
}

mod ws_cancellation {
    fn request() -> Request {
        let mut request = super::request();
        request.transport = opi_ai::provider::TransportPreference::Auto;
        request
    }

    use super::ws_path::{CodexPeer, read_frame, response_101, write_frame};
    use super::*;
    use opi_ai::http::{HttpClientBuilder, RedirectPolicy};
    use opi_ai::provider::TransportPreference;
    use std::io::Write;
    use std::net::TcpStream;
    use std::sync::{Arc, Mutex};

    fn ws_provider(base_url: String) -> OpenAiCodexResponsesProvider {
        OpenAiCodexResponsesProvider::new(
            Some(base_url),
            vec![model(None)],
            Arc::new(
                HttpClientBuilder::new()
                    .redirects(RedirectPolicy::None)
                    .build()
                    .unwrap(),
            ),
        )
    }

    const DELTA: &str = r#"{"type":"response.output_text.delta","delta":"Hello"}"#;

    #[tokio::test]
    async fn active_cancellation_mid_stream_ends_with_cancelled() {
        let peer = CodexPeer::spawn(
            |mut stream: TcpStream,
             lines: Vec<String>,
             _capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
                stream.write_all(&response_101(&lines, &[])).unwrap();
                let (opcode, _) = read_frame(&mut stream).unwrap();
                assert_eq!(opcode, 1);
                write_frame(&mut stream, 1, DELTA.as_bytes());
                // Hold the connection open; the client cancels mid-stream.
                std::thread::sleep(std::time::Duration::from_millis(800));
                Ok(())
            },
        );
        let provider = ws_provider(peer.base_url());
        let mut call = request();
        call.transport = TransportPreference::WebsocketCached;
        let cancel = call.cancel.clone();
        let mut stream = provider.stream_prepared(call, auth("s", None, Some("a".into())));
        let mut error = None;
        while let Some(result) = stream.next().await {
            match result {
                Ok(_) => cancel.cancel(),
                Err(found) => {
                    error = Some(found);
                    break;
                }
            }
        }
        let cancelled = matches!(&error, Some(ProviderError::Cancelled))
            || matches!(&error, Some(ProviderError::StreamFailure(failure))
                if matches!(failure.cause(), ProviderError::Cancelled));
        assert!(
            cancelled,
            "active cancellation ends the stream as Cancelled, got {error:?}"
        );
        peer.finish().expect("peer script");
    }

    #[tokio::test]
    async fn idle_cancellation_before_events_ends_with_cancelled() {
        let peer = CodexPeer::spawn(
            |mut stream: TcpStream,
             lines: Vec<String>,
             _capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
                stream.write_all(&response_101(&lines, &[])).unwrap();
                let (opcode, _) = read_frame(&mut stream).unwrap();
                assert_eq!(opcode, 1);
                // No events: the client cancels while idle.
                std::thread::sleep(std::time::Duration::from_millis(800));
                Ok(())
            },
        );
        let provider = ws_provider(peer.base_url());
        let mut call = request();
        call.transport = TransportPreference::WebsocketCached;
        let cancel = call.cancel.clone();
        let mut stream = provider.stream_prepared(call, auth("s", None, Some("a".into())));
        // First poll starts the lazy direct call. Dropping only this borrowed
        // wait leaves the call and its idle connection alive for cancellation.
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(100), stream.next())
                .await
                .is_err()
        );
        cancel.cancel();
        let mut error = None;
        while let Some(result) = stream.next().await {
            if let Err(found) = result {
                error = Some(found);
                break;
            }
        }
        let cancelled = matches!(&error, Some(ProviderError::Cancelled))
            || matches!(&error, Some(ProviderError::StreamFailure(failure))
                if matches!(failure.cause(), ProviderError::Cancelled));
        assert!(
            cancelled,
            "idle cancellation ends the stream as Cancelled, got {error:?}"
        );
        peer.finish().expect("peer script");
    }
}

mod ws_resources {
    //! Task 20.17: instance resource generation, pool cleanup and awaited
    //! local release through the public `Models::cleanup_session_resources`
    //! entry (P20-RESOURCES owner consumed by the Codex provider).

    fn request() -> Request {
        let mut request = super::request();
        request.transport = opi_ai::provider::TransportPreference::Auto;
        request
    }

    use super::ws_path::{header, read_frame, read_head, response_101, write_frame};
    use super::*;
    use opi_ai::codex_resources::CleanupError;
    use opi_ai::http::{HttpClientBuilder, RedirectPolicy};
    use opi_ai::provider::TransportPreference;
    use opi_ai::{ChatModelRecord, Models, SimpleOptions};
    use std::io::Write;
    use std::net::{TcpListener, TcpStream};
    use std::sync::{Arc, Mutex};

    const DELTA: &str = r#"{"type":"response.output_text.delta","delta":"Hello"}"#;

    struct StaticBearer {
        secret: &'static str,
        account: &'static str,
    }

    impl opi_ai::auth::AuthResolver for StaticBearer {
        fn resolve(
            &self,
        ) -> opi_ai::credential::BoxAuthFuture<'_, Result<ResolvedAuth, ProviderError>> {
            Box::pin(async move {
                Ok(ResolvedAuth {
                    scheme: AuthScheme::Bearer,
                    secret: SecretString::from(self.secret),
                    base_url: None,
                    account_id: Some(self.account.to_owned()),
                    provenance: opi_ai::AuthProvenance::default(),
                })
            })
        }
    }

    fn provider(base_url: String) -> OpenAiCodexResponsesProvider {
        OpenAiCodexResponsesProvider::new(
            Some(base_url),
            vec![model(None)],
            Arc::new(
                HttpClientBuilder::new()
                    .redirects(RedirectPolicy::None)
                    .build()
                    .unwrap(),
            ),
        )
    }

    fn models_with(base_url: String) -> Models {
        let mut models = Models::new();
        models
            .register_route(
                Box::new(provider(base_url)),
                Arc::new(StaticBearer {
                    secret: "ws-secret",
                    account: "acct-1",
                }),
                opi_ai::AuthProvenanceSource::Static,
                Default::default(),
            )
            .expect("register codex route");
        models
    }

    fn session_request(session: &str) -> Request {
        let mut request = request();
        request.session_id = Some(session.into());
        request
    }

    fn sessionless_request() -> Request {
        let mut request = request();
        request.cache_retention = CacheRetention::Disabled;
        request.session_id = None;
        request
    }

    fn is_cancelled(failure: &opi_ai::stream::CallFailure) -> bool {
        matches!(failure.cause(), ProviderError::Cancelled)
    }

    /// ChatCall is lazy: one poll starts the producer without waiting for
    /// its first event, so cleanup-relative timing is deterministic.
    async fn start_call(
        models: &Models,
        request: Request,
    ) -> std::pin::Pin<Box<opi_ai::models::ChatCall>> {
        let record = ChatModelRecord::new("openai-codex", model(None));
        let call = models.stream_simple(
            record,
            request,
            SimpleOptions {
                tool_choice: None,
                temperature: None,
                max_tokens: None,
                timeout: None,
                stop_sequences: vec![],
            },
        );
        let mut call = Box::pin(call);
        // Drive the lazy prepare to a live producer: the first event arrives
        // when the peer speaks; otherwise the bounded window still covers
        // prepare + connect + first request frame.
        let _ = tokio::time::timeout(std::time::Duration::from_millis(400), call.next()).await;
        call
    }

    /// Drain a started call to its single terminal outcome.
    async fn finish_call(
        mut call: std::pin::Pin<Box<opi_ai::models::ChatCall>>,
    ) -> Result<opi_ai::message::AssistantMessage, opi_ai::stream::CallFailure> {
        use futures_util::StreamExt;
        let mut terminal = None;
        while let Some(item) = call.next().await {
            match item {
                Ok(AssistantStreamEvent::Done { message, .. }) => terminal = Some(Ok(message)),
                Ok(_) => {}
                Err(failure) => {
                    terminal = Some(Err(failure));
                    break;
                }
            }
        }
        terminal.expect("call produced a terminal")
    }

    /// A scripted multi-connection peer: each accepted connection consumes
    /// the next script entry; `slow_post` replaces the fixed SSE answer with
    /// a partial write that holds the SSE leg open.
    struct ScriptPeer {
        port: u16,
        heads: Arc<Mutex<Vec<Vec<String>>>>,
        join: Option<std::thread::JoinHandle<()>>,
    }

    type ConnScript = Box<dyn Fn(&mut TcpStream, &[String]) -> Result<(), String> + Send>;

    impl ScriptPeer {
        fn spawn(scripts: Vec<ConnScript>, slow_post: bool) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let port = listener.local_addr().unwrap().port();
            let heads = Arc::new(Mutex::new(Vec::new()));
            let mut scripts: std::collections::VecDeque<ConnScript> = scripts.into();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(6);
            let head_store = heads.clone();
            let join = std::thread::spawn(move || {
                let mut accepted = 0;
                while std::time::Instant::now() < deadline {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            accepted += 1;
                            stream.set_nodelay(true).unwrap();
                            stream.set_nonblocking(false).unwrap();
                            stream
                                .set_read_timeout(Some(std::time::Duration::from_secs(15)))
                                .unwrap();
                            let lines = read_head(&mut stream);
                            if header(&lines, "upgrade").is_some() {
                                head_store.lock().unwrap().push(lines.clone());
                                let script = scripts
                                    .pop_front()
                                    .expect("websocket connection without script");
                                script(&mut stream, &lines).expect("ws script");
                            } else {
                                let length = header(&lines, "content-length")
                                    .and_then(|value| value.parse::<usize>().ok())
                                    .expect("SDK sends a complete JSON body");
                                let mut body = vec![0; length];
                                use std::io::Read;
                                stream
                                    .read_exact(&mut body)
                                    .expect("consume POST body before replying");
                                head_store.lock().unwrap().push(lines);
                                if slow_post {
                                    let response = concat!(
                                        "HTTP/1.1 200 OK\r\n",
                                        "content-type: text/event-stream\r\n\r\n",
                                        "event: response.output_text.delta\n",
                                        "data: {\"type\":\"response.output_text.delta\",\"delta\":\"Hi\"}\n\n",
                                    );
                                    stream.write_all(response.as_bytes()).unwrap();
                                    stream.flush().unwrap();
                                    std::thread::sleep(std::time::Duration::from_millis(1500));
                                    let rest = concat!(
                                        "event: response.completed\n",
                                        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"response-sse\",\"model\":\"gpt-5.4\",\"output\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}}\n\n",
                                    );
                                    let _ = stream.write_all(rest.as_bytes());
                                } else {
                                    let response = format!(
                                        "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                                        codex_completed_sse().len(),
                                        codex_completed_sse()
                                    );
                                    stream.write_all(response.as_bytes()).unwrap();
                                }
                                let _ = stream.shutdown(std::net::Shutdown::Both);
                            }
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(std::time::Duration::from_millis(5));
                        }
                        Err(_) => break,
                    }
                    if scripts.is_empty() && accepted >= 1 && false {
                        break;
                    }
                }
            });
            ScriptPeer {
                port,
                heads,
                join: Some(join),
            }
        }

        fn base_url(&self) -> String {
            format!("http://127.0.0.1:{}", self.port)
        }

        fn finish(mut self) {
            self.join.take().unwrap().join().expect("peer thread");
        }
    }

    /// Upgrade + serve one completed response; optionally wait for the
    /// client Close and echo it (well-behaved pooled close).
    fn ws_serve_completed(
        creates: &Arc<Mutex<Vec<String>>>,
        closes: &Arc<Mutex<usize>>,
    ) -> ConnScript {
        let creates = creates.clone();
        let closes = closes.clone();
        Box::new(move |stream: &mut TcpStream, lines: &[String]| {
            stream.write_all(&response_101(lines, &[])).unwrap();
            loop {
                // A read timeout or EOF means the client kept the pooled
                // connection open: the script ends without asserting a Close.
                let (opcode, payload) = match read_frame(stream) {
                    Ok(frame) => frame,
                    Err(_) => return Ok(()),
                };
                match opcode {
                    1 => {
                        creates
                            .lock()
                            .unwrap()
                            .push(String::from_utf8_lossy(&payload).into_owned());
                        write_frame(stream, 1, super::ws_path::completed_event().as_bytes());
                    }
                    8 => {
                        *closes.lock().unwrap() += 1;
                        write_frame(stream, 8, &[]);
                        return Ok(());
                    }
                    other => return Err(format!("unexpected opcode {other}")),
                }
            }
        })
    }

    /// Upgrade, send one delta, then hold: the call is busy. Returns when
    /// the client Close arrives (asserting no retry create was sent).
    fn ws_hold_busy(creates: &Arc<Mutex<Vec<String>>>) -> ConnScript {
        let creates = creates.clone();
        Box::new(move |stream: &mut TcpStream, lines: &[String]| {
            stream.write_all(&response_101(lines, &[])).unwrap();
            let (opcode, payload) = read_frame(stream)?;
            assert_eq!(opcode, 1);
            creates
                .lock()
                .unwrap()
                .push(String::from_utf8_lossy(&payload).into_owned());
            write_frame(stream, 1, DELTA.as_bytes());
            if let Ok((close_opcode, _)) = read_frame(stream) {
                assert_eq!(close_opcode, 8, "cancelled busy call must close, not retry");
                write_frame(stream, 8, &[]);
            }
            Ok(())
        })
    }

    /// Respond 503 to the upgrade: transport-class pre-start failure that
    /// routes this call to the SSE fallback leg.
    fn ws_reject_503() -> ConnScript {
        Box::new(move |stream: &mut TcpStream, _lines: &[String]| {
            stream
                .write_all(b"HTTP/1.1 503 Service Unavailable\r\ncontent-length: 0\r\nconnection: close\r\n\r\n")
                .unwrap();
            Ok(())
        })
    }

    #[allow(clippy::type_complexity)]
    async fn run_call(
        models: &Models,
        request: Request,
    ) -> Result<opi_ai::message::AssistantMessage, opi_ai::stream::CallFailure> {
        let record = ChatModelRecord::new("openai-codex", model(None));
        let call = models.stream_simple(
            record,
            request,
            SimpleOptions {
                tool_choice: None,
                temperature: None,
                max_tokens: None,
                timeout: None,
                stop_sequences: vec![],
            },
        );
        tokio::time::timeout(std::time::Duration::from_secs(20), call.result())
            .await
            .expect("call completes within the deadline")
    }

    // -- distinct cleanup cases ------------------------------------------------

    #[tokio::test]
    async fn cleanup_session_closes_idle_pooled_connection() {
        let creates = Arc::new(Mutex::new(Vec::new()));
        let closes = Arc::new(Mutex::new(0));
        let peer = ScriptPeer::spawn(vec![ws_serve_completed(&creates, &closes)], false);
        let models = models_with(peer.base_url());
        run_call(&models, session_request("s1"))
            .await
            .expect("call");
        let report = models
            .cleanup_session_resources(Some("s1"))
            .await
            .expect("cleanup");
        assert_eq!(report.closed_connections, 1);
        assert_eq!(report.cancelled_generations, 0);
        assert_eq!(report.cleared_fallback_sessions, 0);
        assert!(report.unconfirmed_effects.is_empty());
        peer.finish();
        assert_eq!(*closes.lock().unwrap(), 1, "peer observed the Close");
    }

    #[tokio::test]
    async fn cleanup_none_cancels_inflight_sessionless_and_closes_pool() {
        let creates = Arc::new(Mutex::new(Vec::new()));
        let closes = Arc::new(Mutex::new(0));
        let peer = ScriptPeer::spawn(
            vec![
                ws_serve_completed(&creates, &closes),
                ws_hold_busy(&creates),
            ],
            false,
        );
        let models = models_with(peer.base_url());
        run_call(&models, session_request("s2"))
            .await
            .expect("pooled call");
        let parent = sessionless_request();
        let parent_token = parent.cancel.clone();
        let call = run_call(&models, parent);
        let cleanup = models.cleanup_session_resources(None);
        let (outcome, report) = tokio::join!(call, cleanup);
        let failure = outcome.expect_err("sessionless call is cancelled");
        assert!(
            is_cancelled(&failure),
            "sessionless in-flight work is cancelled by cleanup(None), got {failure:?}"
        );
        assert!(report.is_ok());
        let report = report.unwrap();
        assert!(report.cancelled_generations >= 1);
        assert!(report.closed_connections >= 1);
        assert!(!parent_token.is_cancelled(), "caller parent token intact");
        peer.finish();
    }

    #[tokio::test]
    async fn cleanup_empty_session_is_typed_failure_and_keeps_pool() {
        let creates = Arc::new(Mutex::new(Vec::new()));
        let closes = Arc::new(Mutex::new(0));
        let peer = ScriptPeer::spawn(vec![ws_serve_completed(&creates, &closes)], false);
        let models = models_with(peer.base_url());
        run_call(&models, session_request("s1"))
            .await
            .expect("first call");
        let error = models
            .cleanup_session_resources(Some(""))
            .await
            .expect_err("empty session id is a typed failure");
        assert!(matches!(error, CleanupError::EmptySessionId), "{error:?}");
        // The pool survived: the second call reuses the same connection.
        run_call(&models, session_request("s1"))
            .await
            .expect("reuse call");
        assert_eq!(
            creates.lock().unwrap().len(),
            2,
            "second create reuses the pooled connection"
        );
        peer.finish();
    }

    #[tokio::test]
    async fn cleanup_unmatched_session_is_noop_and_retains_reuse() {
        let creates = Arc::new(Mutex::new(Vec::new()));
        let closes = Arc::new(Mutex::new(0));
        let peer = ScriptPeer::spawn(vec![ws_serve_completed(&creates, &closes)], false);
        let models = models_with(peer.base_url());
        run_call(&models, session_request("s1"))
            .await
            .expect("call");
        let report = models
            .cleanup_session_resources(Some("other"))
            .await
            .expect("unmatched cleanup is a no-op");
        assert_eq!(report.closed_connections, 0);
        assert_eq!(report.cancelled_generations, 0);
        assert_eq!(report.cleared_fallback_sessions, 0);
        run_call(&models, session_request("s1"))
            .await
            .expect("reuse");
        assert_eq!(creates.lock().unwrap().len(), 2, "idle pool still reused");
        peer.finish();
    }

    // -- cancellation coverage -------------------------------------------------

    #[tokio::test]
    async fn busy_call_cancelled_forbids_late_publish_retry_and_sse() {
        let creates = Arc::new(Mutex::new(Vec::new()));
        let closes = Arc::new(Mutex::new(0));
        let peer = ScriptPeer::spawn(vec![ws_hold_busy(&creates)], false);
        let models = models_with(peer.base_url());
        let call = start_call(&models, session_request("busy")).await;
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        let cleanup = models.cleanup_session_resources(Some("busy"));
        let (outcome, report) = tokio::join!(finish_call(call), cleanup);
        let failure = outcome.expect_err("busy call cancelled");
        assert!(is_cancelled(&failure), "got {failure:?}");
        let report = report.expect("cleanup");
        assert!(report.cancelled_generations >= 1);
        peer.finish();
        assert_eq!(
            creates.lock().unwrap().len(),
            1,
            "exactly one create, no retry"
        );
        let _ = closes;
    }

    #[tokio::test]
    async fn connecting_attempt_cancelled_before_request_frame() {
        let creates: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let script_creates = creates.clone();
        let peer = ScriptPeer::spawn(
            vec![Box::new(move |stream: &mut TcpStream, lines: &[String]| {
                stream.write_all(&response_101(lines, &[])).unwrap();
                // Never serve events; the client cancels while waiting. Hold
                // the connection open until its Close (or an idle timeout).
                loop {
                    let (opcode, payload) = match read_frame(stream) {
                        Ok(frame) => frame,
                        Err(_) => return Ok(()),
                    };
                    match opcode {
                        1 => script_creates
                            .lock()
                            .unwrap()
                            .push(String::from_utf8_lossy(&payload).into_owned()),
                        8 => return Ok(()),
                        other => return Err(format!("unexpected opcode {other}")),
                    }
                }
            })],
            false,
        );
        let models = models_with(peer.base_url());
        let call = start_call(&models, session_request("early")).await;
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        let report = models
            .cleanup_session_resources(Some("early"))
            .await
            .expect("cleanup");
        let outcome = finish_call(call).await;
        let failure = outcome.expect_err("early call cancelled");
        assert!(is_cancelled(&failure), "got {failure:?}");
        assert!(report.cancelled_generations >= 1);
        assert_eq!(
            creates.lock().unwrap().len(),
            1,
            "exactly one create reached the wire, no retry"
        );
        peer.finish();
    }

    #[tokio::test]
    async fn failed_call_generation_released_without_leak() {
        let peer = ScriptPeer::spawn(vec![], false);
        let models = models_with(peer.base_url());
        let mut request = session_request("broken");
        request.extra_headers = vec![("authorization".into(), "x".into())];
        let error = run_call(&models, request)
            .await
            .expect_err("invalid header");
        assert!(
            !is_cancelled(&error),
            "pre-wire failure is typed: {error:?}"
        );
        let report = models
            .cleanup_session_resources(Some("broken"))
            .await
            .expect("cleanup after failed call");
        assert_eq!(report.cancelled_generations, 0, "generation released");
        assert_eq!(report.closed_connections, 0);
        peer.finish();
    }

    #[tokio::test]
    async fn active_fallback_sse_leg_cancelled_and_marker_cleared() {
        let creates = Arc::new(Mutex::new(Vec::new()));
        let closes = Arc::new(Mutex::new(0));
        let peer = ScriptPeer::spawn(
            vec![ws_reject_503(), {
                let creates = creates.clone();
                ws_serve_completed(&creates, &closes)
            }],
            true,
        );
        let models = models_with(peer.base_url());
        let request = session_request("fallback");
        let call = start_call(&models, request).await;
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        let report = models
            .cleanup_session_resources(Some("fallback"))
            .await
            .expect("cleanup");
        let outcome = finish_call(call).await;
        let failure = outcome.expect_err("sse leg cancelled");
        assert!(is_cancelled(&failure), "got {failure:?}");
        assert!(report.cancelled_generations >= 1);
        assert_eq!(report.cleared_fallback_sessions, 1);
        // Marker cleared: the next call attempts WebSocket again.
        run_call(&models, session_request("fallback"))
            .await
            .expect("ws again");
        let heads = peer.heads.lock().unwrap().clone();
        peer.finish();
        let kinds: Vec<bool> = heads
            .iter()
            .map(|lines| header(lines, "upgrade").is_some())
            .collect();
        assert_eq!(
            kinds,
            vec![true, false, true],
            "upgrade, sse post, upgrade again — got {kinds:?}"
        );
        assert_eq!(creates.lock().unwrap().len(), 1);
    }

    // -- drop / barrier semantics ----------------------------------------------

    #[tokio::test]
    async fn borrowed_future_drop_releases_and_barrier_still_completes() {
        let creates = Arc::new(Mutex::new(Vec::new()));
        let peer = ScriptPeer::spawn(vec![ws_hold_busy(&creates)], false);
        let models = models_with(peer.base_url());
        let call = start_call(&models, session_request("dropped")).await;
        // The producer is live; drop the borrowed future without awaiting
        // its exit.
        drop(call);
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let report = models
            .cleanup_session_resources(Some("dropped"))
            .await
            .expect("barrier completes after drop");
        // The drop already started the release: the generation exited and
        // its in-flight connection was released with it, so the later
        // cleanup barrier confirms the exit with no remaining work.
        assert_eq!(report.cancelled_generations, 0);
        assert_eq!(report.closed_connections, 0);
        peer.finish();
    }

    #[tokio::test]
    async fn dropped_cleanup_retains_pending_idle_close_for_the_next_barrier() {
        let (close_seen_tx, close_seen_rx) = tokio::sync::oneshot::channel();
        let close_seen_tx = Mutex::new(Some(close_seen_tx));
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let peer = ScriptPeer::spawn(
            vec![Box::new(move |stream, lines| {
                stream.write_all(&response_101(lines, &[])).unwrap();
                let (opcode, _) = read_frame(stream)?;
                assert_eq!(opcode, 1);
                write_frame(stream, 1, super::ws_path::completed_event().as_bytes());
                let (opcode, payload) = read_frame(stream)?;
                assert_eq!(opcode, 8);
                close_seen_tx
                    .lock()
                    .unwrap()
                    .take()
                    .unwrap()
                    .send(())
                    .unwrap();
                release_rx
                    .recv_timeout(std::time::Duration::from_secs(2))
                    .unwrap();
                write_frame(stream, 8, &payload);
                stream.shutdown(std::net::Shutdown::Write).unwrap();
                Ok(())
            })],
            false,
        );
        let models = models_with(peer.base_url());
        run_call(&models, session_request("drop-cleanup"))
            .await
            .expect("pooled call");
        let mut first = Box::pin(models.cleanup_session_resources(Some("drop-cleanup")));
        tokio::select! {
            result = &mut first => panic!("Close completed before peer acknowledgment: {result:?}"),
            result = close_seen_rx => result.expect("peer saw Close"),
        }
        drop(first);
        let unrelated = tokio::time::timeout(
            std::time::Duration::from_millis(40),
            models.cleanup_session_resources(Some("unrelated-cleanup")),
        )
        .await
        .expect("another session must not wait for this Close")
        .expect("unmatched selector");
        assert_eq!(unrelated.closed_connections, 0);
        let mut repeated = Box::pin(models.cleanup_session_resources(Some("drop-cleanup")));
        let premature =
            tokio::time::timeout(std::time::Duration::from_millis(40), &mut repeated).await;
        release_tx.send(()).unwrap();
        assert!(
            premature.is_err(),
            "repeat cleanup must retain and await the pending Close: {premature:?}"
        );
        let report = repeated.await.expect("peer confirmed retained Close");
        assert_eq!(report.closed_connections, 1);
        peer.finish();
    }

    #[tokio::test]
    async fn cleanup_close_failure_continues_and_reports_unconfirmed() {
        let creates: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let closes = Arc::new(Mutex::new(0));
        let broken = creates.clone();
        let peer = ScriptPeer::spawn(
            vec![
                Box::new(move |stream: &mut TcpStream, lines: &[String]| {
                    stream.write_all(&response_101(lines, &[])).unwrap();
                    let (opcode, payload) = read_frame(stream)?;
                    assert_eq!(opcode, 1);
                    broken
                        .lock()
                        .unwrap()
                        .push(String::from_utf8_lossy(&payload).into_owned());
                    write_frame(stream, 1, super::ws_path::completed_event().as_bytes());
                    // Vanish before the client Close: the close write cannot
                    // be confirmed.
                    let _ = stream.shutdown(std::net::Shutdown::Both);
                    std::thread::sleep(std::time::Duration::from_millis(300));
                    Ok(())
                }),
                ws_serve_completed(&creates, &closes),
            ],
            false,
        );
        let models = models_with(peer.base_url());
        run_call(&models, session_request("bad"))
            .await
            .expect("call 1");
        run_call(&models, session_request("good"))
            .await
            .expect("call 2");
        let error = models
            .cleanup_session_resources(None)
            .await
            .expect_err("unconfirmed release is a typed cleanup failure");
        let CleanupError::Incomplete { report } = error else {
            panic!("expected confirmed effects and unconfirmed releases");
        };
        assert!(
            !report.unconfirmed_effects.is_empty(),
            "unconfirmed close is reported"
        );
        assert!(report.closed_connections >= 1, "other closes still ran");
        peer.finish();
    }

    #[tokio::test]
    async fn two_models_providers_isolated_on_shared_session_ids() {
        let creates_a = Arc::new(Mutex::new(Vec::new()));
        let creates_b = Arc::new(Mutex::new(Vec::new()));
        let closes_b = Arc::new(Mutex::new(0));
        let peer_a = ScriptPeer::spawn(vec![ws_hold_busy(&creates_a)], false);
        let peer_b = ScriptPeer::spawn(vec![ws_serve_completed(&creates_b, &closes_b)], false);
        let models_a = models_with(peer_a.base_url());
        let models_b = models_with(peer_b.base_url());
        let parent_token = {
            let request = session_request("shared");
            let token = request.cancel.clone();
            run_call(&models_b, request).await.expect("pooled on B");
            token
        };
        let busy = start_call(&models_a, session_request("shared")).await;
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        let cleanup = models_a.cleanup_session_resources(Some("shared"));
        let (outcome, report) = tokio::join!(finish_call(busy), cleanup);
        let failure = outcome.expect_err("A cancelled");
        assert!(is_cancelled(&failure), "got {failure:?}");
        let report = report.expect("cleanup");
        assert!(report.cancelled_generations >= 1);
        // B untouched: its pool still serves a reuse on the same session id.
        run_call(&models_b, session_request("shared"))
            .await
            .expect("B reuse");
        assert_eq!(creates_b.lock().unwrap().len(), 2, "B pool retained");
        assert!(!parent_token.is_cancelled(), "parent token intact");
        peer_a.finish();
        peer_b.finish();
    }

    #[tokio::test]
    async fn pure_sse_preference_call_is_not_registered() {
        let peer = ScriptPeer::spawn(vec![], true);
        let models = models_with(peer.base_url());
        let mut request = session_request("sse-only");
        request.transport = TransportPreference::Sse;
        let call = run_call(&models, request);
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        let report = models
            .cleanup_session_resources(None)
            .await
            .expect("cleanup");
        assert_eq!(report.cancelled_generations, 0, "sse-only not registered");
        let message = call
            .await
            .expect("pure sse call survives cleanup and completes");
        assert_eq!(message.response_id.as_deref(), Some("response-sse"));
        peer.finish();
    }
}

mod ws_protocol_regressions {
    fn request() -> Request {
        let mut request = super::request();
        request.transport = opi_ai::provider::TransportPreference::Auto;
        request
    }

    use super::ws_path::{header, read_frame, read_head, response_101, write_frame};
    use super::*;
    use opi_ai::http::{HttpClientBuilder, RedirectPolicy};
    use opi_ai::{ChatModelRecord, Models, SimpleOptions};
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Bearer;
    impl opi_ai::auth::AuthResolver for Bearer {
        fn resolve(
            &self,
        ) -> opi_ai::credential::BoxAuthFuture<'_, Result<ResolvedAuth, ProviderError>> {
            Box::pin(async { Ok(auth("fake", None, Some("account".into()))) })
        }
    }

    fn window8_request_text() -> String {
        let mut state = 0x35a9_17cdu32;
        let block: String = (0..512)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                char::from(b'a' + (state >> 28) as u8)
            })
            .collect();
        block.repeat(16)
    }

    #[tokio::test]
    async fn default_http_client_auto_uses_the_real_websocket_path() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let peer = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let head = read_head(&mut socket);
            if head[0].starts_with("GET ") {
                socket.write_all(&response_101(&head, &[])).unwrap();
                read_frame(&mut socket).unwrap();
                write_frame(&mut socket, 1, super::ws_path::completed_event().as_bytes());
                assert_eq!(read_small_frame(&mut socket).unwrap().0, 8);
                write_frame(&mut socket, 8, &[]);
            } else {
                let length = header(&head, "content-length").unwrap().parse().unwrap();
                let mut body = vec![0; length];
                socket.read_exact(&mut body).unwrap();
                socket
                    .write_all(&super::ws_path::sse_response(&codex_completed_sse(), 200))
                    .unwrap();
            }
            head[0].clone()
        });
        let mut models = Models::new();
        models
            .register_route(
                Box::new(OpenAiCodexResponsesProvider::new(
                    Some(format!("http://127.0.0.1:{port}")),
                    vec![model(None)],
                    Arc::new(HttpClient::new()),
                )),
                Arc::new(Bearer),
                opi_ai::AuthProvenanceSource::Static,
                Default::default(),
            )
            .unwrap();
        let mut request = request();
        request.cache_retention = CacheRetention::Disabled;
        request.session_id = None;
        models
            .complete_simple(
                ChatModelRecord::new("openai-codex", model(None)),
                request,
                Default::default(),
            )
            .await
            .unwrap();
        assert!(
            peer.join().unwrap().starts_with("GET "),
            "Auto must upgrade with the default HttpClient"
        );
    }

    #[tokio::test]
    async fn default_http_client_upgrade_rejects_redirect_without_a_destination_send() {
        let destination = TcpListener::bind("127.0.0.1:0").unwrap();
        destination.set_nonblocking(true).unwrap();
        let target = destination.local_addr().unwrap();
        let source = TcpListener::bind("127.0.0.1:0").unwrap();
        source.set_nonblocking(true).unwrap();
        let port = source.local_addr().unwrap().port();
        let peer = std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
            while std::time::Instant::now() < deadline {
                if let Ok((mut socket, _)) = source.accept() {
                    socket.set_nonblocking(false).unwrap();
                    let head = read_head(&mut socket);
                    assert!(head[0].starts_with("GET "));
                    write!(socket, "HTTP/1.1 302 Found\r\nLocation: http://{target}/redirected\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
                    return true;
                }
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            false
        });
        let result = opi_ai::ws::upgrade(
            &HttpClient::new(),
            opi_ai::ws::WsUpgradeRequest::new(
                url::Url::parse(&format!("ws://127.0.0.1:{port}/socket")).unwrap(),
            ),
        )
        .await;
        let sent = peer.join().unwrap();
        assert!(
            matches!(destination.accept(), Err(ref error) if error.kind() == std::io::ErrorKind::WouldBlock),
            "redirect target must receive no send"
        );
        assert!(
            sent,
            "default client must reach the original Upgrade endpoint"
        );
        assert!(
            matches!(result, Err(opi_ai::ws::WsUpgradeError::Status(302))),
            "3xx must be the typed terminal Upgrade status"
        );
    }

    #[tokio::test]
    async fn pooled_connection_does_not_cross_the_selected_model_endpoint() {
        fn endpoint(id: &'static str) -> (u16, std::thread::JoinHandle<usize>) {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            listener.set_nonblocking(true).unwrap();
            let thread = std::thread::spawn(move || {
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
                let mut accepted = None;
                while std::time::Instant::now() < deadline {
                    if let Ok((socket, _)) = listener.accept() {
                        accepted = Some(socket);
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(2));
                }
                let Some(mut socket) = accepted else {
                    return 0;
                };
                socket.set_nonblocking(false).unwrap();
                socket
                    .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                    .unwrap();
                let head = read_head(&mut socket);
                socket.write_all(&response_101(&head, &[])).unwrap();
                let mut creates = 0;
                while let Ok((opcode, _)) = read_frame(&mut socket) {
                    if opcode == 8 {
                        write_frame(&mut socket, 8, &[]);
                        break;
                    }
                    assert_eq!(opcode, 1);
                    creates += 1;
                    let completed = serde_json::json!({"type":"response.completed", "response":{"id":id,"model":"gpt-5.4","output":[]}}).to_string();
                    write_frame(&mut socket, 1, completed.as_bytes());
                }
                creates
            });
            (port, thread)
        }
        let (port_a, peer_a) = endpoint("endpoint-a");
        let (port_b, peer_b) = endpoint("endpoint-b");
        let mut a = model(None);
        a.base_url = Some(format!("http://127.0.0.1:{port_a}"));
        let mut b = model(None);
        b.id = "gpt-5.4-other".into();
        b.base_url = Some(format!("http://127.0.0.1:{port_b}"));
        let mut models = Models::new();
        models
            .register_route(
                Box::new(OpenAiCodexResponsesProvider::new(
                    None,
                    vec![a.clone(), b.clone()],
                    Arc::new(
                        HttpClientBuilder::new()
                            .redirects(RedirectPolicy::None)
                            .build()
                            .unwrap(),
                    ),
                )),
                Arc::new(Bearer),
                opi_ai::AuthProvenanceSource::Static,
                Default::default(),
            )
            .unwrap();
        let mut request = request();
        request.session_id = Some("same-session-two-endpoints".into());
        let first = models
            .complete_simple(
                ChatModelRecord::new("openai-codex", a),
                request.clone(),
                Default::default(),
            )
            .await
            .unwrap();
        let second = models
            .complete_simple(
                ChatModelRecord::new("openai-codex", b),
                request,
                Default::default(),
            )
            .await
            .unwrap();
        models.cleanup_session_resources(None).await.unwrap();
        let count_a = peer_a.join().unwrap();
        let count_b = peer_b.join().unwrap();
        assert_eq!(first.response_id.as_deref(), Some("endpoint-a"));
        assert_eq!(second.response_id.as_deref(), Some("endpoint-b"));
        assert_eq!(
            (count_a, count_b),
            (1, 1),
            "each frozen endpoint must receive its own call"
        );
    }

    #[tokio::test]
    async fn low_level_sse_attempt_drop_cancels_only_its_internal_token() {
        let mut request = request();
        request.transport = opi_ai::provider::TransportPreference::Sse;
        let started = Arc::new(tokio::sync::Notify::new());
        let callback_started = started.clone();
        request.callbacks =
            opi_ai::callbacks::RequestCallbacks::new().with_async_on_payload(move |_| {
                let started = callback_started.clone();
                async move {
                    started.notify_one();
                    std::future::pending::<
                        Result<Option<serde_json::Value>, opi_ai::callbacks::CallbackError>,
                    >()
                    .await
                }
            });
        let parent = request.cancel.clone();
        let sibling = parent.child_token();
        let provider = OpenAiCodexResponsesProvider::new(
            Some("http://127.0.0.1:9".into()),
            vec![model(None)],
            Arc::new(HttpClient::new()),
        );
        let stream = provider.stream_attempt(
            request,
            auth("fake", None, Some("account".into())),
            opi_ai::provider::AttemptContext {
                invocation_id: "direct-attempt".into(),
                attempt: 1,
                max_attempts: 1,
                payload_variant: opi_ai::provider::PayloadVariant::Ordinary,
            },
        );
        started.notified().await;
        drop(stream);
        let _ =
            tokio::time::timeout(std::time::Duration::from_millis(40), parent.cancelled()).await;
        assert!(
            !parent.is_cancelled(),
            "receiver drop must not cancel the caller parent"
        );
        assert!(
            !sibling.is_cancelled(),
            "a sibling sharing that parent must survive"
        );
    }

    enum Script {
        MaskedFrame,
        InvalidPmd,
        SilentProviderEvent,
        OutboundPmd,
        AsyncRaw(Arc<AtomicUsize>),
        Ping,
        Binary,
    }

    fn read_small_frame(stream: &mut TcpStream) -> Result<(u8, Vec<u8>), String> {
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();
        let mut head = [0; 2];
        stream.read_exact(&mut head).map_err(|e| e.to_string())?;
        if head[1] & 0x80 == 0 || head[1] & 0x7f > 125 {
            return Err("expected a small masked control frame".into());
        }
        let mut mask = [0; 4];
        stream.read_exact(&mut mask).map_err(|e| e.to_string())?;
        let mut payload = vec![0; (head[1] & 0x7f) as usize];
        stream.read_exact(&mut payload).map_err(|e| e.to_string())?;
        for (index, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[index % 4];
        }
        Ok((head[0] & 0x0f, payload))
    }

    async fn run(script: Script) -> (Result<AssistantMessage, opi_ai::stream::CallFailure>, usize) {
        let outbound_pmd = matches!(script, Script::OutboundPmd);
        let raw_counter = match &script {
            Script::AsyncRaw(count) => Some(count.clone()),
            _ => None,
        };
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let posts = Arc::new(AtomicUsize::new(0));
        let recorded = posts.clone();
        let thread = std::thread::spawn(move || -> Result<(), String> {
            let (mut socket, _) = listener.accept().map_err(|e| e.to_string())?;
            socket.set_nodelay(true).unwrap();
            let head = read_head(&mut socket);
            let extra = if matches!(script, Script::InvalidPmd) {
                vec![("Sec-WebSocket-Extensions", "permessage-deflate")]
            } else if matches!(script, Script::OutboundPmd) {
                vec![(
                    "Sec-WebSocket-Extensions",
                    "permessage-deflate; client_max_window_bits=8; server_max_window_bits=8; client_no_context_takeover; server_no_context_takeover",
                )]
            } else {
                vec![]
            };
            socket
                .write_all(&response_101(&head, &extra))
                .map_err(|e| e.to_string())?;
            let (opcode, payload, compressed) =
                super::ws_path::read_frame_with_compression(&mut socket)?;
            assert_eq!(opcode, 1);
            if matches!(script, Script::OutboundPmd) {
                assert!(
                    compressed,
                    "negotiated PMD must encode an eligible response.create"
                );
                let (plain, maximum_distance) =
                    super::deflate_audit::decode_pmd(&payload, &mut Vec::new(), 8).unwrap();
                assert!(maximum_distance <= 256);
                let body: serde_json::Value = serde_json::from_slice(&plain).unwrap();
                assert_eq!(body["input"][0]["content"], window8_request_text());
            }
            match script {
                Script::MaskedFrame => socket.write_all(&[0x81, 0x80, 0, 0, 0, 0]).unwrap(),
                Script::InvalidPmd => socket.write_all(&[0xc1, 1, 0xff]).unwrap(),
                Script::SilentProviderEvent => {
                    write_frame(&mut socket, 1, br#"{"type":"response.output_item.added","output_index":0,"item":{"type":"message","id":"message-1","content":[]}}"#);
                }
                Script::OutboundPmd => {
                    write_frame(&mut socket, 1, super::ws_path::completed_event().as_bytes());
                    assert_eq!(read_small_frame(&mut socket)?.0, 8);
                    write_frame(&mut socket, 8, &[]);
                }
                Script::Ping => {
                    write_frame(&mut socket, 9, b"probe");
                    let (opcode, payload) = read_small_frame(&mut socket)?;
                    assert_eq!((opcode, payload.as_slice()), (10, b"probe".as_slice()));
                    write_frame(&mut socket, 1, super::ws_path::completed_event().as_bytes());
                    let (opcode, _) = read_small_frame(&mut socket)?;
                    assert_eq!(opcode, 8);
                    write_frame(&mut socket, 8, &[]);
                }
                Script::Binary | Script::AsyncRaw(_) => {
                    write_frame(&mut socket, 2, super::ws_path::completed_event().as_bytes());
                    if let Ok((8, _)) = read_small_frame(&mut socket) {
                        write_frame(&mut socket, 8, &[]);
                    }
                }
            }
            let _ = socket.shutdown(std::net::Shutdown::Both);
            listener.set_nonblocking(true).unwrap();
            let until = std::time::Instant::now() + std::time::Duration::from_millis(200);
            while std::time::Instant::now() < until {
                match listener.accept() {
                    Ok((mut socket, _)) => {
                        socket.set_nonblocking(false).unwrap();
                        let head = read_head(&mut socket);
                        assert!(head[0].starts_with("POST "));
                        let length = header(&head, "content-length").unwrap().parse().unwrap();
                        let mut bytes = vec![0; length];
                        socket.read_exact(&mut bytes).unwrap();
                        recorded.fetch_add(1, Ordering::SeqCst);
                        socket
                            .write_all(&super::ws_path::sse_response(&codex_completed_sse(), 200))
                            .unwrap();
                        break;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(2))
                    }
                    Err(e) => return Err(e.to_string()),
                }
            }
            Ok(())
        });
        let mut models = Models::new();
        models
            .register_route(
                Box::new(OpenAiCodexResponsesProvider::new(
                    Some(format!("http://127.0.0.1:{port}")),
                    vec![model(None)],
                    Arc::new(
                        HttpClientBuilder::new()
                            .redirects(RedirectPolicy::None)
                            .build()
                            .unwrap(),
                    ),
                )),
                Arc::new(Bearer),
                opi_ai::AuthProvenanceSource::Static,
                Default::default(),
            )
            .unwrap();
        let mut request = request();
        request.session_id = None;
        request.cache_retention = CacheRetention::Disabled;
        request.timeout = Some(std::time::Duration::from_millis(100));
        if let Some(counter) = raw_counter {
            request.callbacks =
                opi_ai::callbacks::RequestCallbacks::new().with_async_on_raw(move |_| {
                    let counter = counter.clone();
                    async move {
                        tokio::task::yield_now().await;
                        counter.fetch_add(1, Ordering::SeqCst);
                        Ok(())
                    }
                });
        }
        if outbound_pmd {
            request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
                system_prompt: None,
                messages: vec![Message::User(UserMessage {
                    content: vec![InputContent::Text {
                        text: window8_request_text(),
                    }],
                    timestamp_ms: 0,
                })],
                tools: vec![],
            })
            .unwrap();
        }
        let result = models
            .complete_simple(
                ChatModelRecord::new("openai-codex", model(None)),
                request,
                SimpleOptions::default(),
            )
            .await;
        thread
            .join()
            .expect("script thread")
            .expect("script completes");
        (result, posts.load(Ordering::SeqCst))
    }

    #[tokio::test]
    async fn masked_server_frame_is_terminal_without_sse_replay() {
        let (result, posts) = run(Script::MaskedFrame).await;
        assert_eq!(
            posts, 0,
            "a framing protocol failure must never resend over SSE"
        );
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn invalid_pmd_is_terminal_without_sse_replay() {
        let (result, posts) = run(Script::InvalidPmd).await;
        assert_eq!(
            posts, 0,
            "a compression protocol failure must never resend over SSE"
        );
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn ping_receives_a_matching_masked_pong_before_generation_continues() {
        let (result, posts) = run(Script::Ping).await;
        assert_eq!(posts, 0);
        assert_eq!(
            result
                .expect("valid control flow succeeds")
                .response_id
                .as_deref(),
            Some("response-1")
        );
    }

    #[tokio::test]
    async fn binary_provider_json_is_projected_like_text() {
        let (result, posts) = run(Script::Binary).await;
        assert_eq!(posts, 0);
        assert_eq!(
            result
                .expect("valid binary provider event succeeds")
                .response_id
                .as_deref(),
            Some("response-1")
        );
    }

    #[tokio::test]
    async fn asynchronous_raw_observer_is_awaited_once_before_the_terminal() {
        let observed = Arc::new(AtomicUsize::new(0));
        let (result, posts) = run(Script::AsyncRaw(observed.clone())).await;
        assert_eq!(posts, 0);
        assert!(
            result.is_ok(),
            "an asynchronous functional raw hook is supported on WS"
        );
        assert_eq!(observed.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn valid_provider_event_seals_fallback_before_normalized_output() {
        let (result, posts) = run(Script::SilentProviderEvent).await;
        assert_eq!(
            posts, 0,
            "a valid provider event closes fallback even when it emits no public content"
        );
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn negotiated_pmd_encodes_large_request_with_actual_window8() {
        let (result, posts) = run(Script::OutboundPmd).await;
        assert_eq!(posts, 0);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn successful_upgrade_seals_advanced_acquisition_even_if_fallback_fails() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let peer = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let head = read_head(&mut socket);
            socket.write_all(&response_101(&head, &[])).unwrap();
            assert_eq!(read_frame(&mut socket).unwrap().0, 1);
            drop(socket);
            let (mut socket, _) = listener.accept().unwrap();
            let head = read_head(&mut socket);
            assert!(head[0].starts_with("POST "));
            let length: usize = header(&head, "content-length").unwrap().parse().unwrap();
            let mut body = vec![0; length];
            socket.read_exact(&mut body).unwrap();
            // Fail acquisition of the fallback response after its request.
            // This network failure is ordinarily retryable before handoff.
            drop(socket);
        });
        let mut models = Models::new();
        models
            .register_route(
                Box::new(OpenAiCodexResponsesProvider::new(
                    Some(format!("http://127.0.0.1:{port}")),
                    vec![model(None)],
                    Arc::new(
                        HttpClientBuilder::new()
                            .redirects(RedirectPolicy::None)
                            .build()
                            .unwrap(),
                    ),
                )),
                Arc::new(Bearer),
                opi_ai::AuthProvenanceSource::Static,
                Default::default(),
            )
            .unwrap();
        let prepared = models
            .prepare_call_with_policy(
                "openai-codex:gpt-5.4",
                request(),
                opi_ai::retry::AcquisitionPolicy {
                    max_retries: 1,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        let mut attempt = prepared.start_attempt().unwrap();
        assert!(attempt.next().await.unwrap().is_err());
        assert!(
            prepared.retry_admission().is_none(),
            "a successful WS acquisition cannot reset into another generic attempt"
        );
        assert_eq!(
            prepared.acquisition_seal(),
            Some(opi_ai::retry::AcquisitionSeal::PostHandoff)
        );
        peer.join().unwrap();
    }

    #[tokio::test]
    async fn overlapping_session_call_uses_a_temporary_connection() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let temporary_closed = Arc::new(AtomicUsize::new(0));
        let observed = temporary_closed.clone();
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let (second_tx, second_rx) = std::sync::mpsc::channel();
        let peer = std::thread::spawn(move || -> Result<(), String> {
            let (mut first, _) = listener.accept().map_err(|e| e.to_string())?;
            let first_peer = std::thread::spawn(move || -> Result<(), String> {
                let head = read_head(&mut first);
                first.write_all(&response_101(&head, &[])).unwrap();
                assert_eq!(read_frame(&mut first)?.0, 1);
                ready_tx.send(()).unwrap();
                second_rx.recv().unwrap();
                write_frame(&mut first, 1, super::ws_path::completed_event().as_bytes());
                assert_eq!(read_small_frame(&mut first)?.0, 8);
                write_frame(&mut first, 8, &[]);
                let _ = first.shutdown(std::net::Shutdown::Both);
                Ok(())
            });
            let (mut second, _) = listener.accept().map_err(|e| e.to_string())?;
            let head = read_head(&mut second);
            second.write_all(&response_101(&head, &[])).unwrap();
            assert_eq!(read_frame(&mut second)?.0, 1);
            write_frame(&mut second, 1, super::ws_path::completed_event().as_bytes());
            if let Ok((8, _)) = read_small_frame(&mut second) {
                observed.fetch_add(1, Ordering::SeqCst);
                write_frame(&mut second, 8, &[]);
            }
            let _ = second.shutdown(std::net::Shutdown::Both);
            second_tx.send(()).unwrap();
            first_peer.join().unwrap()
        });
        let mut models = Models::new();
        models
            .register_route(
                Box::new(OpenAiCodexResponsesProvider::new(
                    Some(format!("http://127.0.0.1:{port}")),
                    vec![model(None)],
                    Arc::new(
                        HttpClientBuilder::new()
                            .redirects(RedirectPolicy::None)
                            .build()
                            .unwrap(),
                    ),
                )),
                Arc::new(Bearer),
                opi_ai::AuthProvenanceSource::Static,
                Default::default(),
            )
            .unwrap();
        async fn drain(mut call: opi_ai::models::ChatCall) {
            let mut terminal = false;
            while let Some(item) = call.next().await {
                terminal |= matches!(
                    item.expect("valid generation"),
                    AssistantStreamEvent::Done { .. }
                );
            }
            assert!(terminal);
        }
        let record = ChatModelRecord::new("openai-codex", model(None));
        let first = models.stream_simple(record.clone(), request(), SimpleOptions::default());
        let second = async {
            ready_rx.await.unwrap();
            drain(models.stream_simple(record, request(), SimpleOptions::default())).await;
        };
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            tokio::join!(drain(first), second);
        })
        .await
        .unwrap();
        models.cleanup_session_resources(None).await.unwrap();
        peer.join().expect("peer thread").expect("peer script");
        assert_eq!(
            temporary_closed.load(Ordering::SeqCst),
            1,
            "a busy-overflow connection is temporary and must complete Close"
        );
    }
}
