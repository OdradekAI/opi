//! Shared native Mistral Conversations builders.

use super::*;

pub(crate) const SMALL_NATIVE: &str = "mistral-small-latest";

pub(crate) fn model_native(id: &str, thinking: bool, images: bool) -> opi_ai::provider::ModelInfo {
    let mut caps = ModelCapabilities::new(32_000, 8_192).with_streaming(true);
    if thinking {
        caps = caps.with_thinking(true);
    }
    if images {
        caps = caps.with_images(true);
    }
    opi_ai::provider::ModelInfo::new(id, id, WireApi::MistralConversations, caps)
}

pub(crate) fn effort_model_native(id: &str, map: ThinkingLevelMap) -> opi_ai::provider::ModelInfo {
    let mut model = model_native(id, true, false);
    model.thinking_level_map = map;
    model
}

pub(crate) fn map_native(pairs: &[(ThinkingLevel, &str)]) -> ThinkingLevelMap {
    let mut map = ThinkingLevelMap::disabled();
    for (level, value) in pairs {
        map = map.with_mapping(*level, ThinkingLevelMapping::Mapped((*value).into()));
    }
    map
}

pub(crate) fn provider_native(
    server_uri: Option<&str>,
    model: opi_ai::provider::ModelInfo,
    options: MistralOptions,
) -> MistralConversationsWireAdapter {
    MistralConversationsProvider::new(server_uri.map(str::to_owned))
        .with_models(vec![model])
        .with_native_options(options)
}

pub(crate) fn req_native(user: &str) -> Request {
    let mut request = base_req_native();
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

pub(crate) fn base_req_native() -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: format!("mistral:{SMALL_NATIVE}"),
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

pub(crate) fn sse_ok_native() -> ResponseTemplate {
    ResponseTemplate::new(200)
        .set_body_string(concat!(
            "data: {\"id\":\"chatcmpl-1\",\"choices\":[{\"delta\":{\"role\":\"assistant\",\"content\":\"\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"Hi\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5,\"total_tokens\":15}}\n\n",
            "data: [DONE]\n\n",
        ))
        .insert_header("content-type", "text/event-stream")
}

pub(crate) async fn mount_native(server: &MockServer, response: ResponseTemplate) {
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer test-key"))
        .respond_with(response)
        .expect(1)
        .mount(server)
        .await;
}

pub(crate) async fn body_native(server: &MockServer) -> serde_json::Value {
    let requests = server.received_requests().await.expect("captured requests");
    serde_json::from_slice(&requests[0].body).expect("request body is JSON")
}

pub(crate) async fn run_native(
    provider: &impl Provider,
    request: Request,
) -> Vec<Result<AssistantStreamEvent, ProviderError>> {
    let stream = provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
    stream.collect().await
}

pub(crate) async fn done_message_native(
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

pub(crate) async fn first_error_native(
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
