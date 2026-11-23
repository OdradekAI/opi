//! Shared OpenAI Responses request, model and stream fixtures.

use super::*;

/// Helper: create an OpenAI Responses provider.
pub(crate) fn responses_provider() -> OpenAiResponsesProvider {
    OpenAiResponsesProvider::new(None)
}

/// Helper: collect stream events asynchronously.
pub(crate) async fn collect_stream(stream: EventStream) -> Vec<AssistantStreamEvent> {
    stream.filter_map(|r| async move { r.ok() }).collect().await
}

// ---------------------------------------------------------------------------
// Phase 12 task 12.3 — native Responses request semantics
//
// DoD: OpenAI Responses fixture tests assert native Responses request semantics
// for store, strict JSON schema/tool schema, previous_response_id, and
// reasoning.effort where supported by opi's existing request model, or 12.9
// documents the unsupported bits as deferred provider correctness work.
// ---------------------------------------------------------------------------

pub(crate) fn responses_tool_request() -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openai-responses:gpt-4o".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: Some("You are helpful.".into()),
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "list files".into(),
                }],
                timestamp_ms: 0,
            })],
            tools: vec![ToolDef {
                name: "list_dir".into(),
                description: "list a directory".into(),
                input_schema: serde_json::json!({"type":"object","properties":{}}),
                constrained_sampling: None,
            }],
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
    }
}

pub(crate) fn responses_tool_context() -> opi_ai::transcript::Context {
    opi_ai::transcript::Context {
        system_prompt: Some("You are helpful.".into()),
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "list files".into(),
            }],
            timestamp_ms: 0,
        })],
        tools: vec![ToolDef {
            name: "list_dir".into(),
            description: "list a directory".into(),
            input_schema: serde_json::json!({"type":"object","properties":{}}),
            constrained_sampling: None,
        }],
    }
}

// --- Reasoning tokens (Phase 14 task 14.5) ---

pub(crate) fn responses_reasoning_fixture(
    reasoning_tokens: Option<u64>,
    output_tokens: u32,
) -> String {
    let mut usage = serde_json::json!({
        "input_tokens": 100,
        "output_tokens": output_tokens,
    });
    if let Some(tokens) = reasoning_tokens {
        usage["output_tokens_details"] = serde_json::json!({"reasoning_tokens": tokens});
    }
    let payload = serde_json::json!({
        "type": "response.completed",
        "response": {
            "id": "resp_reason",
            "status": "completed",
            "model": "o3",
            "output": [],
            "usage": usage,
        },
    });
    format!("event: response.completed\ndata: {payload}\n\n")
}

pub(crate) fn responses_text_request() -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openai-responses:gpt-4o".into(),
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
        max_tokens: None,
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
    }
}

pub(crate) fn o3_responses_model() -> ModelInfo {
    ModelInfo::new(
        "o3",
        "o3",
        WireApi::OpenAiResponses,
        ModelCapabilities::new(200_000, 100_000)
            .with_images(true)
            .with_streaming(true)
            .with_thinking(true),
    )
}

pub(crate) fn responses_public_request(model: &str, thinking: Option<ThinkingLevel>) -> Request {
    let mut request = responses_tool_request();
    request.model = format!("openai-responses:{model}");
    request.thinking = match thinking {
        Some(level) => ThinkingConfig {
            enabled: true,
            budget_tokens: None,
            level,
        },
        None => ThinkingConfig::default(),
    };
    request
}

pub(crate) async fn registered_responses_models(
    server_uri: String,
    model: ModelInfo,
) -> (
    Models,
    opi_ai::api::contract::ChatApiHandle<OpenAiResponsesOptions>,
) {
    let provider = std::sync::Arc::new(OpenAiResponsesProvider::new(Some(server_uri)));
    let api = OpenAiResponsesChatApi::new(provider, model).expect("valid responses record");
    let mut models = Models::new();
    let handle = models
        .register_chat_api(
            std::sync::Arc::new(api),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::Bearer,
                secrecy::SecretString::from("offline-responses-key"),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .expect("register responses api");
    (models, handle)
}

pub(crate) fn responses_record() -> ChatModelRecord {
    ChatModelRecord::new("openai-responses", o3_responses_model())
}

pub(crate) fn responses_sse_ok() -> String {
    concat!(
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_1\",\"status\":\"in_progress\",\"model\":\"o3\",\"output\":[]}}\n\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_1\",\"status\":\"completed\",\"model\":\"o3\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"Hi\"}]}],\"usage\":{\"input_tokens\":5,\"output_tokens\":2}}}\n\n",
    )
    .to_owned()
}

pub(crate) fn reasoning_sse() -> String {
    concat!(
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_r\",\"status\":\"in_progress\",\"model\":\"o3\",\"output\":[]}}\n\n",
        "data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"type\":\"reasoning\",\"id\":\"rs_1\"}}\n\n",
        "data: {\"type\":\"response.reasoning_summary_text.delta\",\"output_index\":0,\"summary_index\":0,\"delta\":\"Think\"}\n\n",
        "data: {\"type\":\"response.reasoning_summary_text.delta\",\"output_index\":0,\"summary_index\":0,\"delta\":\"ing\"}\n\n",
        "data: {\"type\":\"response.output_item.done\",\"output_index\":0,\"item\":{\"type\":\"reasoning\",\"id\":\"rs_1\",\"summary\":[{\"type\":\"summary_text\",\"text\":\"Thinking\"}]}}\n\n",
        "data: {\"type\":\"response.output_item.added\",\"output_index\":1,\"item\":{\"type\":\"message\",\"status\":\"in_progress\",\"role\":\"assistant\",\"content\":[]}}\n\n",
        "data: {\"type\":\"response.output_text.delta\",\"output_index\":1,\"content_index\":0,\"delta\":\"Answer\"}\n\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_r\",\"status\":\"completed\",\"model\":\"o3\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"Answer\"}]}],\"usage\":{\"input_tokens\":5,\"output_tokens\":2}}}\n\n",
    )
    .to_owned()
}

pub(crate) fn message_thinking(message: &opi_ai::message::AssistantMessage) -> Option<String> {
    message.content.iter().find_map(|content| match content {
        opi_ai::message::AssistantContent::Thinking { thinking, .. } => Some(thinking.clone()),
        _ => None,
    })
}

pub(crate) fn history_assistant_message(
    content: Vec<opi_ai::message::AssistantContent>,
    provider_thinking_level: Option<String>,
) -> opi_ai::message::AssistantMessage {
    opi_ai::message::AssistantMessage {
        projection: None,
        content,
        api: opi_ai::ApiKind::OpenAi,
        provider: "openai-responses".into(),
        deferred: None,
        model: "o3".into(),
        response_model: None,
        wire: Some(WireApi::OpenAiResponses),
        response_id: None,
        usage: Default::default(),
        stop_reason: opi_ai::stream::StopReason::Stop,
        error_message: None,
        timestamp_ms: 0,
        diagnostics: Vec::new(),
        duration_ms: None,
        raw_stop_reason: None,
        provider_thinking_level,
    }
}
