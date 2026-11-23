//! Shared Pi Messages fixture builders and stream helpers.

use super::*;

pub(crate) const MODEL_44: &str = "claude-sonnet-4-5";

pub(crate) fn model_44(id: &str) -> opi_ai::provider::ModelInfo {
    opi_ai::provider::ModelInfo::new(
        id,
        id,
        WireApi::PiMessages,
        ModelCapabilities::new(200_000, 16_384).with_streaming(true),
    )
}

pub(crate) fn thinking_model_44() -> opi_ai::provider::ModelInfo {
    let mut model = model_44(MODEL_44);
    model.capabilities = model.capabilities.with_thinking(true);
    model.thinking_level_map = model.thinking_level_map.with_mapping(
        ThinkingLevel::High,
        ThinkingLevelMapping::Mapped("high".into()),
    );
    model
}

pub(crate) fn clamped_model_44() -> opi_ai::provider::ModelInfo {
    // An explicit map that would clamp High to "max" if any clamp ran; the
    // pi-messages simple bypass must ignore it entirely.
    let mut map = ThinkingLevelMap::disabled();
    map = map.with_mapping(
        ThinkingLevel::High,
        ThinkingLevelMapping::Mapped("max".into()),
    );
    let mut model = model_44(MODEL_44);
    model.thinking_level_map = map;
    model
}

pub(crate) fn req_44(user: &str) -> Request {
    let mut request = base_req_44();
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text { text: user.into() }],
            timestamp_ms: 0,
        })],
        tools: vec![],
    })
    .expect("valid context declarations");
    request
}

pub(crate) fn base_req_44() -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: format!("radius:{MODEL_44}"),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

pub(crate) fn sse_ok_44() -> ResponseTemplate {
    ResponseTemplate::new(200)
        .set_body_string(concat!(
            "data: {\"type\":\"start\"}\n\n",
            "data: {\"type\":\"text_start\",\"contentIndex\":0}\n\n",
            "data: {\"type\":\"text_delta\",\"contentIndex\":0,\"delta\":\"Hel\"}\n\n",
            "data: {\"type\":\"text_end\",\"contentIndex\":0,\"content\":\"Hello\"}\n\n",
            "data: {\"type\":\"done\",\"reason\":\"stop\",\"usage\":{\"input\":10,\"output\":5,\"cacheRead\":0,\"cacheWrite\":0,\"totalTokens\":15,\"cost\":{\"input\":0.1,\"output\":0.2,\"cacheRead\":0,\"cacheWrite\":0,\"total\":0.3}},\"responseId\":\"resp-1\"}\n\n",
            "data: [DONE]\n\n",
        ))
        .insert_header("content-type", "text/event-stream")
}

pub(crate) async fn mount_44(server: &MockServer, response: ResponseTemplate) {
    Mock::given(method("POST"))
        .and(path("/messages"))
        .and(header("authorization", "Bearer test-key"))
        .respond_with(response)
        .expect(1)
        .mount(server)
        .await;
}

pub(crate) async fn body_44(server: &MockServer) -> serde_json::Value {
    let requests = server.received_requests().await.expect("captured requests");
    serde_json::from_slice(&requests[0].body).expect("request body is JSON")
}

pub(crate) async fn run_44(
    provider: &impl Provider,
    request: Request,
) -> Vec<Result<AssistantStreamEvent, ProviderError>> {
    let stream = provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
    stream.collect().await
}

pub(crate) async fn done_message_44(
    events: &[Result<AssistantStreamEvent, ProviderError>],
) -> opi_ai::message::AssistantMessage {
    events
        .iter()
        .find_map(|item| match item {
            Ok(AssistantStreamEvent::Done { message, .. }) => Some(message.clone()),
            _ => None,
        })
        .expect("stream reaches Done")
}

pub(crate) async fn first_error_44(
    events: &[Result<AssistantStreamEvent, ProviderError>],
) -> ProviderError {
    events
        .iter()
        .find_map(|item| match item {
            Err(ProviderError::StreamFailure(failure)) => Some(failure.cause().clone()),
            Err(error) => Some(error.clone()),
            _ => None,
        })
        .expect("stream fails typed")
}

pub(crate) static ENV_LOCK_44: std::sync::OnceLock<tokio::sync::Mutex<()>> =
    std::sync::OnceLock::new();
pub(crate) fn env_lock_44() -> &'static tokio::sync::Mutex<()> {
    ENV_LOCK_44.get_or_init(|| tokio::sync::Mutex::new(()))
}

pub(crate) fn assert_direct_projection_prefix(
    message: &opi_ai::message::AssistantMessage,
    request: &opi_ai::provider::Request,
) {
    use sha2::Digest as _;
    let facts = message
        .projection
        .as_ref()
        .expect("accepted projection is known");
    let bytes = serde_json::to_vec(request.context.messages()).unwrap();
    assert_eq!(
        facts.source_prefix.message_count,
        request.context.messages().len()
    );
    assert_eq!(
        facts.source_prefix.sha256,
        format!("{:x}", sha2::Sha256::digest(bytes))
    );
    assert_eq!(
        facts.temporal.mode,
        opi_ai::transcript::ProjectionMode::Lossless
    );
    assert!(facts.temporal.affected_ranges.is_empty());
    assert!(facts.replay.losses.is_empty());
}
