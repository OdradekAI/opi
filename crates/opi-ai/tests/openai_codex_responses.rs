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
        transport: Default::default(),
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
        Ok((opcode, payload))
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
        // Let the producer reach the idle wait, then cancel.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
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
