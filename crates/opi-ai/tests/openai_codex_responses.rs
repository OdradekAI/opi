use std::sync::Arc;

use futures_util::StreamExt;
use opi_ai::auth::{AuthResolver, AuthScheme, ResolvedAuth};
use opi_ai::credential::BoxAuthFuture;
use opi_ai::http::HttpClient;
use opi_ai::message::{InputContent, Message, ToolDef, UserMessage};
use opi_ai::openai_codex_responses::OpenAiCodexResponsesProvider;
use opi_ai::provider::{CacheRetention, Provider, ProviderError, Request, ThinkingConfig};
use opi_ai::{ModelCapabilities, ModelInfo, ThinkingLevel, WireApi};
use secrecy::SecretString;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

struct FixedAuth {
    secret: SecretString,
    base_url: Option<String>,
    account_id: Option<String>,
}

impl AuthResolver for FixedAuth {
    fn resolve<'a>(&'a self) -> BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        let secret = self.secret.clone();
        let base_url = self.base_url.clone();
        let account_id = self.account_id.clone();
        Box::pin(async move {
            Ok(ResolvedAuth {
                scheme: AuthScheme::Bearer,
                secret,
                base_url,
                account_id,
            })
        })
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
        model: "openai-codex:gpt-5.4".into(),
        system: Some("system prompt".into()),
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
        }],
        max_tokens: Some(1024),
        temperature: Some(0.2),
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
    }
}

async fn drain(provider: &dyn Provider, request: Request) -> Option<ProviderError> {
    let mut stream = provider.stream(request);
    while let Some(result) = stream.next().await {
        match result {
            Ok(event) if event.is_terminal() => return None,
            Ok(_) => {}
            Err(error) => return Some(error),
        }
    }
    None
}

async fn capture_stream(provider: &dyn Provider, request: Request) -> (Vec<String>, usize, usize) {
    let mut stream = provider.stream(request);
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
        .mount(&auth_server)
        .await;

    let provider = OpenAiCodexResponsesProvider::new(
        Arc::new(FixedAuth {
            secret: SecretString::from("sentinel-access"),
            base_url: Some(auth_server.uri()),
            account_id: Some("account-fixed".into()),
        }),
        Some(default_server.uri()),
        vec![model(Some(model_server.uri()))],
        Arc::new(HttpClient::new()),
    );
    assert_eq!(provider.id(), "openai-codex");
    assert!(drain(&provider, request()).await.is_none());
    assert!(default_server.received_requests().await.unwrap().is_empty());
    assert!(model_server.received_requests().await.unwrap().is_empty());
    let requests = auth_server.received_requests().await.unwrap();
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
        Arc::new(FixedAuth {
            secret: SecretString::from("sentinel-access"),
            base_url: Some(server.uri()),
            account_id: Some("account-fixed".into()),
        }),
        None,
        vec![model(None)],
        Arc::new(HttpClient::new()),
    );

    for _ in 0..2 {
        let mut generated = request();
        generated.session_id = None;
        assert!(drain(&provider, generated).await.is_none());
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
        Arc::new(FixedAuth {
            secret: SecretString::from("sentinel-access"),
            base_url: Some(server.uri()),
            account_id: Some("account-fixed".into()),
        }),
        None,
        vec![model(None)],
        Arc::new(HttpClient::new()),
    );
    let mut disabled = request();
    disabled.cache_retention = CacheRetention::Disabled;
    disabled.session_id = Some("user-session-must-not-leak".into());

    assert!(drain(&provider, disabled).await.is_none());

    let captured = server.received_requests().await.unwrap().remove(0);
    let body: serde_json::Value = serde_json::from_slice(&captured.body).unwrap();
    assert!(body.get("prompt_cache_key").is_none());
    let session_id = captured.headers["session-id"].to_str().unwrap();
    let request_id = captured.headers["x-client-request-id"].to_str().unwrap();
    for generated in [session_id, request_id] {
        assert_eq!(
            uuid::Uuid::parse_str(generated).unwrap().get_version_num(),
            7
        );
        assert_ne!(generated, "user-session-must-not-leak");
    }
    assert_ne!(session_id, request_id);
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
        Arc::new(FixedAuth {
            secret: SecretString::from("sentinel-access"),
            base_url: Some(server.uri()),
            account_id: Some("account-fixed".into()),
        }),
        None,
        vec![model(None)],
        Arc::new(HttpClient::new()),
    );

    let (captures, events, errors) = capture_stream(&provider, request()).await;

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
async fn dedicated_codex_valid_error_sse_never_surfaces_message_or_event_name() {
    for (event_name, data, sentinel) in [
        (
            "error",
            r#"{"message":"sentinel-valid-error-message"}"#,
            "sentinel-valid-error-message",
        ),
        (
            "sentinel-unknown-event-name",
            r#"{"message":"benign"}"#,
            "sentinel-unknown-event-name",
        ),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/codex/responses"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(format!("event: {event_name}\ndata: {data}\n\n"))
                    .insert_header("content-type", "text/event-stream"),
            )
            .mount(&server)
            .await;
        let provider = OpenAiCodexResponsesProvider::new(
            Arc::new(FixedAuth {
                secret: SecretString::from("sentinel-access"),
                base_url: Some(server.uri()),
                account_id: Some("account-fixed".into()),
            }),
            None,
            vec![model(None)],
            Arc::new(HttpClient::new()),
        );

        let (captures, events, errors) = capture_stream(&provider, request()).await;

        assert_eq!(events, 1);
        assert_eq!(errors, 0);
        let rendered = captures.join("\n");
        assert!(
            rendered.contains("OpenAI Codex returned a streaming error"),
            "{rendered}"
        );
        assert!(!rendered.contains(sentinel), "{rendered}");
    }
}

#[tokio::test]
async fn dedicated_codex_requires_non_empty_account_id_before_http() {
    let server = MockServer::start().await;
    for account_id in [None, Some("".into()), Some("   ".into())] {
        let provider = OpenAiCodexResponsesProvider::new(
            Arc::new(FixedAuth {
                secret: SecretString::from("sentinel-access"),
                base_url: Some(server.uri()),
                account_id,
            }),
            None,
            vec![model(None)],
            Arc::new(HttpClient::new()),
        );
        let error = drain(&provider, request())
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
            Arc::new(FixedAuth {
                secret: SecretString::from("sentinel-access"),
                base_url: Some(server.uri()),
                account_id: Some("account-fixed".into()),
            }),
            None,
            vec![model(None)],
            Arc::new(HttpClient::new()),
        );
        let mut request = request();
        request.extra_headers.push((name.into(), "override".into()));
        assert!(
            matches!(
                drain(&provider, request).await,
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
            Arc::new(FixedAuth {
                secret: SecretString::from("sentinel-access"),
                base_url: Some(server.uri()),
                account_id: Some("account-fixed".into()),
            }),
            None,
            vec![model(None)],
            Arc::new(HttpClient::new()),
        );
        let mut request = request();
        request.extra_headers.push((name.into(), value.into()));
        let error = drain(&provider, request)
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
            Arc::new(FixedAuth {
                secret: SecretString::from("sentinel-access"),
                base_url: Some(server.uri()),
                account_id: Some("account-fixed".into()),
            }),
            None,
            vec![model(None)],
            Arc::new(HttpClient::new()),
        );
        let error = drain(&provider, request()).await.expect("revoked");
        assert!(matches!(
            error,
            ProviderError::CredentialRevoked { ref provider_id }
                if provider_id == "openai-codex"
        ));
        let rendered = format!("{error:?} {error}");
        assert!(!rendered.contains("sentinel-access"));
        assert!(!rendered.contains("sentinel-envelope"));
        assert!(!error.is_retryable());
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
        Arc::new(FixedAuth {
            secret: SecretString::from("sentinel-access"),
            base_url: Some(server.uri()),
            account_id: Some("sentinel-account".into()),
        }),
        None,
        vec![model(None)],
        Arc::new(HttpClient::new()),
    );

    let error = drain(&provider, request()).await.expect("provider failure");
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
