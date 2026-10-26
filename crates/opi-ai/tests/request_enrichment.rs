//! Tests for Phase 14.3 Request scalar enrichment: timeout, extra_headers,
//! cache_retention, and session_id fields on `opi_ai::provider::Request`, plus
//! CacheRetention wire semantics and invalid-header rejection (no live calls).

use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use opi_ai::http::HttpClient;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{
    CacheRetention, ModelInfo, Provider, ProviderError, Request, ThinkingConfig,
    validate_extra_headers,
};
use opi_ai::registry::ModelCapabilities;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ---------------------------------------------------------------------------
// CacheRetention type
// ---------------------------------------------------------------------------

#[test]
fn cache_retention_default_is_none() {
    let cr = CacheRetention::default();
    assert_eq!(cr, CacheRetention::None);
}

#[test]
fn cache_retention_variants_exist() {
    let none = CacheRetention::None;
    let disabled = CacheRetention::Disabled;
    let short = CacheRetention::Short;
    let long = CacheRetention::Long;

    assert_ne!(none, disabled);
    assert_ne!(none, short);
    assert_ne!(none, long);
    assert_ne!(disabled, short);
    assert_ne!(disabled, long);
    assert_ne!(short, long);
}

// ---------------------------------------------------------------------------
// Request new fields
// ---------------------------------------------------------------------------

#[test]
fn request_new_scalars_default_to_none_empty() {
    let cancel = CancellationToken::new();
    let req = Request {
        transport: Default::default(),
        model: "test".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel,
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::default(),
        session_id: None,
        callbacks: Default::default(),
    };
    assert!(req.timeout.is_none());
    assert!(req.extra_headers.is_empty());
    assert_eq!(req.cache_retention, CacheRetention::None);
    assert!(req.session_id.is_none());
}

#[test]
fn request_scalars_carry_explicit_values() {
    let cancel = CancellationToken::new();
    let timeout = Duration::from_secs(30);
    let headers = vec![("X-Test".to_string(), "value".to_string())];
    let req = Request {
        transport: Default::default(),
        model: "test".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel,
        timeout: Some(timeout),
        extra_headers: headers.clone(),
        cache_retention: CacheRetention::Long,
        session_id: Some("sess-abc".into()),
        callbacks: Default::default(),
    };
    assert_eq!(req.timeout, Some(Duration::from_secs(30)));
    assert_eq!(req.extra_headers, vec![("X-Test".into(), "value".into())]);
    assert_eq!(req.cache_retention, CacheRetention::Long);
    assert_eq!(req.session_id.as_deref(), Some("sess-abc"));
}

// ---------------------------------------------------------------------------
// Invalid extra-header rejection (no network)
// ---------------------------------------------------------------------------

#[test]
fn extra_headers_reject_auth_header_names() {
    let auth_names = [
        "authorization",
        "Authorization",
        "AUTHORIZATION",
        "x-api-key",
        "X-Api-Key",
        "api-key",
        "Api-Key",
        "anthropic-version",
        "Anthropic-Version",
        "content-type",
        "Content-Type",
    ];
    for name in &auth_names {
        let headers = vec![(name.to_string(), "secret".into())];
        let err = validate_extra_headers(&headers).unwrap_err();
        let msg = err.to_string().to_lowercase();
        assert!(
            msg.contains("auth") || msg.contains("reserved") || msg.contains("forbidden"),
            "expected auth rejection for header '{name}', got: {err}"
        );
    }
}

#[test]
fn extra_headers_reject_control_characters_in_name() {
    for bad in &["X-\nHeader", "X-\rHeader", "X:\tHeader"] {
        let headers = vec![(bad.to_string(), "value".into())];
        let err = validate_extra_headers(&headers).unwrap_err();
        assert!(
            matches!(err, ProviderError::RequestFailed(_)),
            "expected RequestFailed for header {bad:?}, got: {err}"
        );
    }
}

#[test]
fn extra_headers_reject_empty_name() {
    let headers = vec![("".to_string(), "value".into())];
    let err = validate_extra_headers(&headers).unwrap_err();
    assert!(matches!(err, ProviderError::RequestFailed(_)));
}

#[test]
fn extra_headers_accept_valid_names() {
    let headers = vec![
        ("X-Custom".to_string(), "val".into()),
        ("x-request-id".to_string(), "abc123".into()),
        ("X-Trace-Id".to_string(), "t-1".into()),
    ];
    validate_extra_headers(&headers).unwrap();
}

// ---------------------------------------------------------------------------
// Timeout wire
// ---------------------------------------------------------------------------

#[tokio::test]
async fn provider_timeout_produces_timeout_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        .mount(&server)
        .await;

    let cancel = CancellationToken::new();
    let provider = opi_ai::anthropic::AnthropicProvider::new(Some(server.uri()));
    let request = make_anthropic_request(cancel, Some(Duration::from_millis(100)));

    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    let item = stream.next().await.unwrap();
    match item {
        Err(ProviderError::Timeout) => {}
        other => panic!("expected Timeout, got {other:?}"),
    }
}

#[tokio::test]
async fn provider_no_timeout_completes_normally() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(
                "event: message_stop\r\ndata: {\"type\":\"message_stop\"}\r\n\r\n",
            ),
        )
        .mount(&server)
        .await;

    let cancel = CancellationToken::new();
    let provider = opi_ai::anthropic::AnthropicProvider::new(Some(server.uri()));
    let request = make_anthropic_request(cancel, None);

    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    // Drain until we get a Done event (or error).
    let mut saw_done = false;
    while let Some(item) = stream.next().await {
        match item {
            Ok(_) => saw_done = true,
            Err(e) => panic!("unexpected error: {e}"),
        }
    }
    assert!(saw_done, "stream should produce at least one event");
}

// ---------------------------------------------------------------------------
// Extra-headers wire
// ---------------------------------------------------------------------------

#[tokio::test]
async fn extra_headers_reach_anthropic_wire() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("X-Custom", "my-value"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(
                "event: message_stop\r\ndata: {\"type\":\"message_stop\"}\r\n\r\n",
            ),
        )
        .expect(1)
        .mount(&server)
        .await;

    let cancel = CancellationToken::new();
    let provider = opi_ai::anthropic::AnthropicProvider::new(Some(server.uri()));
    let mut request = make_anthropic_request(cancel, None);
    request.extra_headers = vec![("X-Custom".to_string(), "my-value".to_string())];

    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    while let Some(item) = stream.next().await {
        match item {
            Ok(_) => {}
            Err(e) => panic!("unexpected error: {e}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Session-affinity wire mappings (OpenAI Chat)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn openai_chat_session_id_becomes_prompt_cache_key() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"delta\":{\"content\":\"hi\"},\"index\":0,\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n",
        ))
        .expect(1)
        .mount(&server)
        .await;

    let cancel = CancellationToken::new();
    let provider = opi_ai::openai_chat::OpenAiChatProvider::new(Some(server.uri()));
    let request = make_openai_chat_request(cancel, Some("sess-abc123".into()));

    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
    while let Some(item) = stream.next().await {
        match item {
            Ok(_) => {}
            Err(e) => panic!("unexpected error: {e}"),
        }
    }
    let request = server.received_requests().await.unwrap().remove(0);
    assert_eq!(
        request_body_json(&request)["prompt_cache_key"],
        "sess-abc123"
    );
    assert_eq!(header_value(&request, "prompt-cache-key"), None);
}

#[tokio::test]
async fn openai_chat_session_id_clamps_to_64_chars() {
    let session_id = "a".repeat(100);
    let clamped = session_id[..64].to_owned();

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"delta\":{\"content\":\"hi\"},\"index\":0,\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n",
        ))
        .expect(1)
        .mount(&server)
        .await;

    let cancel = CancellationToken::new();
    let provider = opi_ai::openai_chat::OpenAiChatProvider::new(Some(server.uri()));
    let request = make_openai_chat_request(cancel, Some(session_id));

    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
    while let Some(item) = stream.next().await {
        match item {
            Ok(_) => {}
            Err(e) => panic!("unexpected error: {e}"),
        }
    }
    let request = server.received_requests().await.unwrap().remove(0);
    assert_eq!(request_body_json(&request)["prompt_cache_key"], clamped);
}

#[tokio::test]
async fn openai_chat_no_session_id_no_cache_key_header() {
    let server = MockServer::start().await;
    // The mock accepts any POST without requiring a prompt-cache-key header.
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"delta\":{\"content\":\"hi\"},\"index\":0,\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n",
        ))
        .expect(1)
        .mount(&server)
        .await;

    let cancel = CancellationToken::new();
    let provider = opi_ai::openai_chat::OpenAiChatProvider::new(Some(server.uri()));
    let request = make_openai_chat_request(cancel, None);

    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
    while let Some(item) = stream.next().await {
        match item {
            Ok(_) => {}
            Err(e) => panic!("unexpected error: {e}"),
        }
    }
    let request = server.received_requests().await.unwrap().remove(0);
    assert!(
        request_body_json(&request)
            .get("prompt_cache_key")
            .is_none()
    );
    assert_eq!(header_value(&request, "prompt-cache-key"), None);
}

#[tokio::test]
async fn session_affinity_wire_mappings() {
    let chat_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string("data: [DONE]\n\n"))
        .expect(4)
        .mount(&chat_server)
        .await;
    let models = vec![ModelInfo::new(
        "model",
        "Model",
        opi_ai::WireApi::OpenAiCompletions,
        ModelCapabilities::new(8_192, 1_024).with_streaming(true),
    )];
    let direct = opi_ai::openai_chat::OpenAiChatProvider::new(Some(chat_server.uri()));
    drain(direct.stream_prepared(
        make_openai_chat_request(CancellationToken::new(), Some("session-direct".into())),
        opi_ai::test_support::resolved_bearer_auth(),
    ))
    .await;
    let compatible = opi_ai::openai_chat::OpenAiChatProvider::new_for_profile(
        chat_server.uri(),
        "compatible".into(),
        opi_ai::openai_chat::CompatConfig {
            send_session_affinity_headers: true,
            ..Default::default()
        },
        vec![],
        models.clone(),
    );
    drain(compatible.stream_prepared(
        make_openai_chat_request(CancellationToken::new(), Some("session-compatible".into())),
        opi_ai::test_support::resolved_bearer_auth(),
    ))
    .await;
    let compatible_default = opi_ai::openai_chat::OpenAiChatProvider::new_for_profile(
        chat_server.uri(),
        "compatible-default".into(),
        Default::default(),
        vec![],
        models,
    );
    drain(compatible_default.stream_prepared(
        make_openai_chat_request(CancellationToken::new(), Some("session-default".into())),
        opi_ai::test_support::resolved_bearer_auth(),
    ))
    .await;
    let mut disabled =
        make_openai_chat_request(CancellationToken::new(), Some("session-disabled".into()));
    disabled.cache_retention = CacheRetention::Disabled;
    drain(compatible.stream_prepared(disabled, opi_ai::test_support::resolved_bearer_auth())).await;

    let chat_requests = chat_server.received_requests().await.unwrap();
    assert_eq!(chat_requests.len(), 4);
    assert_eq!(
        request_body_json(&chat_requests[0])["prompt_cache_key"],
        "session-direct"
    );
    for name in ["session_id", "x-client-request-id", "x-session-affinity"] {
        assert_eq!(header_value(&chat_requests[0], name), None);
        assert_eq!(
            header_value(&chat_requests[1], name),
            Some("session-compatible")
        );
        assert_eq!(header_value(&chat_requests[2], name), None);
        assert_eq!(header_value(&chat_requests[3], name), None);
    }
    for request in &chat_requests[1..] {
        assert!(request_body_json(request).get("prompt_cache_key").is_none());
    }

    let responses_server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string("data: [DONE]\n\n"))
        .expect(5)
        .mount(&responses_server)
        .await;
    let standard =
        opi_ai::openai_responses::OpenAiResponsesProvider::new(Some(responses_server.uri()));
    drain(standard.stream_prepared(
        make_openai_responses_request("openai-responses:model", "session-standard"),
        opi_ai::test_support::resolved_bearer_auth(),
    ))
    .await;
    let direct_with_session_header =
        opi_ai::openai_responses::OpenAiResponsesProvider::new_with_config(
            Some(responses_server.uri()),
            opi_ai::openai_responses::ResponsesConfig {
                send_session_id_header: false,
                ..Default::default()
            },
        );
    drain(direct_with_session_header.stream_prepared(
        make_openai_responses_request("openai-responses:model", "session-direct-header"),
        opi_ai::test_support::resolved_bearer_auth(),
    ))
    .await;
    let mut disabled = make_openai_responses_request("openai-responses:model", "session-disabled");
    disabled.cache_retention = CacheRetention::Disabled;
    drain(standard.stream_prepared(disabled, opi_ai::test_support::resolved_bearer_auth())).await;
    let custom_model = ModelInfo::new(
        "model",
        "Model",
        opi_ai::WireApi::OpenAiResponses,
        ModelCapabilities::new(128_000, 16_384).with_streaming(true),
    );
    let custom_default = opi_ai::openai_responses::OpenAiResponsesProvider::for_route(
        Some(responses_server.uri()),
        "custom-responses".into(),
        opi_ai::ProviderHeaders::default(),
        vec![custom_model.clone()],
        Arc::new(HttpClient::new()),
    );
    drain(custom_default.stream_prepared(
        make_openai_responses_request("custom-responses:model", "session-custom-default"),
        opi_ai::test_support::resolved_bearer_auth(),
    ))
    .await;
    let custom_opt_in_model = custom_model
        .with_compat(opi_ai::WireCompat::OpenAiResponses(
            opi_ai::model_info::OpenAiResponsesCompat {
                send_session_id_header: true,
                ..Default::default()
            },
        ))
        .unwrap();
    let custom_opt_in = opi_ai::openai_responses::OpenAiResponsesProvider::for_route(
        Some(responses_server.uri()),
        "custom-responses".into(),
        opi_ai::ProviderHeaders::default(),
        vec![custom_opt_in_model],
        Arc::new(HttpClient::new()),
    );
    drain(custom_opt_in.stream_prepared(
        make_openai_responses_request("custom-responses:model", "session-custom-opt-in"),
        opi_ai::test_support::resolved_bearer_auth(),
    ))
    .await;

    let response_requests = responses_server.received_requests().await.unwrap();
    assert_eq!(response_requests.len(), 5);
    assert_eq!(
        request_body_json(&response_requests[0])["prompt_cache_key"],
        "session-standard"
    );
    let standard_request_id = header_value(&response_requests[0], "x-client-request-id")
        .expect("direct Responses request id");
    assert_eq!(
        uuid::Uuid::parse_str(standard_request_id)
            .unwrap()
            .get_version_num(),
        7
    );
    assert_ne!(standard_request_id, "session-standard");
    assert_eq!(
        header_value(&response_requests[0], "session_id"),
        Some("session-standard")
    );

    assert_eq!(
        request_body_json(&response_requests[1])["prompt_cache_key"],
        "session-direct-header"
    );
    let direct_request_id = header_value(&response_requests[1], "x-client-request-id")
        .expect("direct Responses request id");
    assert_eq!(
        uuid::Uuid::parse_str(direct_request_id)
            .unwrap()
            .get_version_num(),
        7
    );
    assert_ne!(direct_request_id, standard_request_id);
    assert_eq!(header_value(&response_requests[1], "session_id"), None);

    for request in &response_requests[2..4] {
        assert!(request_body_json(request).get("prompt_cache_key").is_none());
        for name in ["session_id", "session-id", "x-client-request-id"] {
            assert_eq!(header_value(request, name), None);
        }
    }
    assert_eq!(
        request_body_json(&response_requests[4])["prompt_cache_key"],
        "session-custom-opt-in"
    );
    assert_eq!(
        header_value(&response_requests[4], "session_id"),
        Some("session-custom-opt-in")
    );
    let custom_request_id = header_value(&response_requests[4], "x-client-request-id")
        .expect("custom opt-in request id");
    assert_eq!(
        uuid::Uuid::parse_str(custom_request_id)
            .unwrap()
            .get_version_num(),
        7
    );
    assert_ne!(custom_request_id, "session-custom-opt-in");

    let anthropic_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_string(""))
        .expect(1)
        .mount(&anthropic_server)
        .await;
    let anthropic = opi_ai::anthropic::AnthropicProvider::new(Some(anthropic_server.uri()));
    let mut request = make_anthropic_request(CancellationToken::new(), None);
    request.session_id = Some("session-anthropic".into());
    drain(anthropic.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth())).await;
    let request = anthropic_server
        .received_requests()
        .await
        .unwrap()
        .remove(0);
    for name in [
        "prompt-cache-key",
        "session-id",
        "session_id",
        "x-client-request-id",
        "x-session-affinity",
    ] {
        assert_eq!(header_value(&request, name), None);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_anthropic_request(cancel: CancellationToken, timeout: Option<Duration>) -> Request {
    Request {
        transport: Default::default(),
        model: "anthropic:claude-sonnet-4-5-20250514".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text { text: "hi".into() }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel,
        timeout,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        callbacks: Default::default(),
    }
}

fn make_openai_chat_request(cancel: CancellationToken, session_id: Option<String>) -> Request {
    Request {
        transport: Default::default(),
        model: "openai:gpt-4o".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text { text: "hi".into() }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel,
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id,
        callbacks: Default::default(),
    }
}

fn make_openai_responses_request(model: &str, session_id: &str) -> Request {
    let mut request = make_openai_chat_request(CancellationToken::new(), Some(session_id.into()));
    request.model = model.into();
    request
}

async fn drain(mut stream: opi_ai::provider::EventStream) {
    while stream.next().await.is_some() {}
}

fn header_value<'a>(request: &'a wiremock::Request, name: &str) -> Option<&'a str> {
    request
        .headers
        .get(name)
        .and_then(|value| value.to_str().ok())
}

fn request_body_json(request: &wiremock::Request) -> serde_json::Value {
    serde_json::from_slice(&request.body).expect("captured request body should be JSON")
}

// ---------------------------------------------------------------------------
// Task 20.35: request sampling parameters
// ---------------------------------------------------------------------------

#[test]
fn request_sampling_params_default_empty_and_carried() {
    let req = Request {
        transport: Default::default(),
        model: "test".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::default(),
        session_id: None,
        callbacks: Default::default(),
        sampling_params: Default::default(),
    };
    assert!(req.sampling_params.is_empty());

    let params: opi_ai::model_info::SamplingParams = serde_json::json!({"top_p": 0.9})
        .as_object()
        .cloned()
        .unwrap();
    let mut req = req;
    req.sampling_params = params;
    assert_eq!(req.sampling_params["top_p"], serde_json::json!(0.9));
}

// ---------------------------------------------------------------------------
// Task 20.51: functional headers/payload/response/raw callbacks (ONB-04)
// ---------------------------------------------------------------------------

use opi_ai::callbacks::RequestCallbacks;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

fn anthropic_sse_ok() -> String {
    "event: message_start\r\ndata: {\"type\":\"message_start\",\"message\":{\"model\":\"claude\",\"usage\":{\"input_tokens\":3,\"output_tokens\":0}}}\r\n\r\nevent: message_stop\r\ndata: {\"type\":\"message_stop\"}\r\n\r\n".into()
}

#[test]
fn request_callbacks_default_empty_and_has_no_wire_observers() {
    let callbacks = RequestCallbacks::new();
    assert!(callbacks.is_empty());
    assert!(!callbacks.has_wire_observers());
}

#[tokio::test]
async fn sdk_headers_omission_set_and_removal_reach_anthropic_wire() {
    let server = MockServer::start().await;
    let body = anthropic_sse_ok();
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .expect(1)
        .mount(&server)
        .await;

    let provider = opi_ai::anthropic::AnthropicProvider::new(Some(server.uri()));
    let mut request = make_anthropic_request(CancellationToken::new(), None);
    request.extra_headers = vec![
        ("X-Set".to_string(), "keep".to_string()),
        ("X-Drop".to_string(), "gone".to_string()),
    ];
    request.callbacks = RequestCallbacks::new().with_header_transform(Arc::new(|merged| {
        let mut out: Vec<(String, String)> = merged
            .iter()
            .filter(|(name, _)| !name.eq_ignore_ascii_case("x-drop"))
            .cloned()
            .collect();
        out.push(("X-Added".to_string(), "by-transform".to_string()));
        Ok(out)
    }));

    drain(provider.stream_prepared(request, opi_ai::test_support::resolved_auth())).await;

    let sent = &server.received_requests().await.unwrap()[0];
    // Omission: an unset header never reaches the wire.
    assert!(header_value(sent, "x-opi-never").is_none());
    // Set and transform-added headers reach the wire; removed ones do not.
    assert_eq!(header_value(sent, "x-set"), Some("keep"));
    assert!(header_value(sent, "x-drop").is_none());
    assert_eq!(header_value(sent, "x-added"), Some("by-transform"));
    // Protected selected identity stays attached and unchanged.
    assert_eq!(header_value(sent, "x-api-key"), Some("test-key"));
    server.verify().await;
}

#[tokio::test]
async fn header_transform_touching_protected_header_fails_typed_before_http() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let provider = opi_ai::anthropic::AnthropicProvider::new(Some(server.uri()));
    let mut request = make_anthropic_request(CancellationToken::new(), None);
    request.callbacks = RequestCallbacks::new().with_header_transform(Arc::new(|merged| {
        let mut out = merged.to_vec();
        out.push(("Authorization".to_string(), "Bearer attacker".to_string()));
        Ok(out)
    }));

    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    let mut saw_callback_phase = false;
    while let Some(item) = stream.next().await {
        if let Err(ProviderError::CallbackPhase(_)) = item {
            saw_callback_phase = true;
        }
    }
    assert!(
        saw_callback_phase,
        "transform touching a protected header must fail typed"
    );
    server.verify().await;
}

#[test]
fn header_transform_failure_is_typed_functional_failure() {
    // Unit-level: the transform's CallbackError becomes a typed
    // CallbackPhase error, not a silent ignore and not a retryable cause.
    let error = ProviderError::CallbackPhase(opi_ai::provider::ProviderErrorSummary::sanitized(
        "header transform failed",
    ));
    assert!(!error.is_retryable());
    assert_eq!(
        error.category(),
        opi_ai::provider::ProviderErrorCategory::Request
    );
}

#[tokio::test]
async fn transform_runs_once_per_dispatch_on_the_merged_per_call_set() {
    let server = MockServer::start().await;
    let body = anthropic_sse_ok();
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .expect(1)
        .mount(&server)
        .await;

    type HeaderSets = Arc<Mutex<Vec<Vec<(String, String)>>>>;
    let seen_inputs: HeaderSets = Arc::new(Mutex::new(Vec::new()));
    let recorder = Arc::clone(&seen_inputs);
    let provider = opi_ai::anthropic::AnthropicProvider::new(Some(server.uri()));
    let mut request = make_anthropic_request(CancellationToken::new(), None);
    request.extra_headers = vec![("X-Seed".to_string(), "1".to_string())];
    request.callbacks = RequestCallbacks::new().with_header_transform(Arc::new(move |merged| {
        recorder.lock().unwrap().push(merged.to_vec());
        Ok(merged.to_vec())
    }));

    drain(provider.stream_prepared(request, opi_ai::test_support::resolved_auth())).await;

    let seen = seen_inputs.lock().unwrap();
    assert_eq!(seen.len(), 1, "transform applies exactly once per dispatch");
    assert_eq!(
        seen[0],
        vec![("X-Seed".to_string(), "1".to_string())],
        "transform sees the merged per-call set, never the protected auth headers"
    );
}

#[test]
fn payload_replacement_retargeting_model_identity_fails_typed() {
    let callbacks = RequestCallbacks::new().with_on_payload(Arc::new(|body| {
        let mut replacement = body.clone();
        replacement["model"] = serde_json::json!("claude-evil-rewrite");
        Ok(Some(replacement))
    }));
    let outcome = callbacks.apply_payload(
        "ordinary",
        serde_json::json!({"model": "claude-sonnet", "max_tokens": 16}),
    );
    match outcome {
        Err(ProviderError::CallbackPhase(_)) => {}
        other => panic!("model retarget must fail typed, got {other:?}"),
    }
}

#[test]
fn payload_replacement_raising_output_budget_fails_typed() {
    let callbacks = RequestCallbacks::new().with_on_payload(Arc::new(|body| {
        let mut replacement = body.clone();
        replacement["max_tokens"] = serde_json::json!(999_999);
        Ok(Some(replacement))
    }));
    let outcome = callbacks.apply_payload(
        "ordinary",
        serde_json::json!({"model": "claude-sonnet", "max_tokens": 16}),
    );
    assert!(
        matches!(outcome, Err(ProviderError::CallbackPhase(_))),
        "raising the resolved output budget must fail typed"
    );
}

#[tokio::test]
async fn payload_callback_freezes_once_across_ordinary_acquisition_retry() {
    use opi_ai::auth::{AuthProvenanceSource, StaticAuthResolver};
    use opi_ai::http::HttpClientBuilder;
    use opi_ai::models::Models;
    use opi_ai::openai_chat::OpenAiChatProvider;
    use opi_ai::retry::AcquisitionPolicy;
    use opi_ai::stream::ChatStream;
    use secrecy::SecretString;

    let server = MockServer::start().await;
    // First send: retryable 429 with an immediate advisory hint.
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after-ms", "1"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    // Second send: a terminal streaming completion.
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"delta\":{\"content\":\"hi\"},\"index\":0,\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n",
        ))
        .expect(1)
        .mount(&server)
        .await;

    let client = Arc::new(HttpClientBuilder::new().build().expect("client build"));
    let provider =
        OpenAiChatProvider::with_client(Some(server.uri()), "openai".to_owned(), vec![], client);
    let mut models = Models::new();
    models
        .register_route(
            Box::new(provider),
            Arc::new(StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::Bearer,
                SecretString::from("freeze-key"),
            )),
            AuthProvenanceSource::Static,
            opi_ai::models::CompatMetadata {
                openai_compatible: true,
                profile: None,
            },
        )
        .expect("register route");

    let payload_calls = Arc::new(AtomicUsize::new(0));
    let calls = Arc::clone(&payload_calls);
    let mut request = make_openai_chat_request(CancellationToken::new(), None);
    request.callbacks = RequestCallbacks::new().with_on_payload(Arc::new(move |body| {
        calls.fetch_add(1, Ordering::SeqCst);
        let mut replacement = body.clone();
        replacement["frozen_marker"] = serde_json::json!("replaced-once");
        Ok(Some(replacement))
    }));

    let mut policy = AcquisitionPolicy::with_max_retries(1);
    policy.jitter_seed = Some(0);
    let call = models
        .prepare_call_with_policy("openai:gpt-4o", request, policy)
        .await
        .expect("prepare call");

    let first = ChatStream::new(call.start_attempt().expect("first attempt"))
        .result()
        .await
        .expect_err("first attempt is a retryable 429");
    assert!(matches!(first.cause(), ProviderError::RateLimited { .. }));
    let admission = call.retry_admission().expect("retry admitted");
    assert_eq!(admission.retry_index, 1);
    tokio::time::sleep(std::time::Duration::from_millis(admission.delay_ms.max(1))).await;

    ChatStream::new(call.start_attempt().expect("second attempt"))
        .result()
        .await
        .expect("second attempt completes");

    assert_eq!(
        payload_calls.load(Ordering::SeqCst),
        1,
        "payload callback runs exactly once per variant across ordinary retry"
    );
    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 2);
    let first_body = request_body_json(&sent[0]);
    let second_body = request_body_json(&sent[1]);
    assert_eq!(
        first_body["frozen_marker"],
        serde_json::json!("replaced-once")
    );
    assert_eq!(
        second_body, first_body,
        "the ordinary retry reuses the frozen payload byte-for-byte"
    );
}
