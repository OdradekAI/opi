//! Shared builtin peers, route setup, input records and AssistantMessage JSON reload.

use std::sync::Arc;

use opi_ai::auth::{AuthContext, AuthContextError};
use opi_ai::message::{AssistantMessage, InputContent, Message, UserMessage};
use opi_ai::model_info::{OpenAiResponsesCompat, WireCompat};
use opi_ai::provider::{Request, ThinkingConfig};
use opi_ai::providers::collection::{BuiltinOverrides, register_builtin};
use opi_ai::{
    ChatModelRecord, ModelCapabilities, ModelInfo, Models, ThinkingLevel, ThinkingLevelMap, WireApi,
};
use serde_json::json;
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

pub(super) type BuiltinCase = (&'static str, WireApi, bool);
pub(super) const CHAT_REASONING: BuiltinCase = ("groq", WireApi::OpenAiCompletions, false);
pub(super) const CHAT_OPAQUE_DETAILS: BuiltinCase = ("groq", WireApi::OpenAiCompletions, true);
pub(super) const OPENAI_RESPONSES: BuiltinCase = ("openai", WireApi::OpenAiResponses, false);
pub(super) const AZURE_CHAT: BuiltinCase = ("azure", WireApi::OpenAiCompletions, false);
pub(super) const AZURE_RESPONSES: BuiltinCase = ("azure", WireApi::AzureOpenAiResponses, false);
pub(super) const BUILTIN_CASES: [BuiltinCase; 5] = [
    CHAT_REASONING,
    CHAT_OPAQUE_DETAILS,
    OPENAI_RESPONSES,
    AZURE_CHAT,
    AZURE_RESPONSES,
];

struct Source;
impl AuthContext for Source {
    fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
        Ok(matches!(
            name,
            "GROQ_API_KEY" | "OPENAI_API_KEY" | "AZURE_OPENAI_API_KEY"
        )
        .then(|| "fixture-key".into()))
    }
    fn file_exists(&self, _: &str) -> Result<bool, AuthContextError> {
        panic!("unselected source")
    }
}

pub(super) fn request(messages: Vec<Message>, effort: ThinkingLevel) -> Request {
    Request {
        projection: Default::default(),
        model: "record-owned".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages,
            tools: vec![],
        })
        .unwrap(),
        max_tokens: Some(32),
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig {
            enabled: true,
            level: effort,
            budget_tokens: None,
        },
        transport: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: Default::default(),
        timeout: Some(std::time::Duration::from_secs(5)),
        extra_headers: vec![],
        cache_retention: Default::default(),
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

pub(super) fn user() -> Message {
    Message::User(UserMessage {
        content: vec![InputContent::Text {
            text: "continue".into(),
        }],
        timestamp_ms: 0,
    })
}

pub(super) fn model(wire: WireApi, base: String) -> ModelInfo {
    let mut model = ModelInfo::new(
        "requested-alias",
        "Fixture",
        wire,
        ModelCapabilities::new(8192, 64)
            .with_thinking(true)
            .with_streaming(true),
    )
    .with_thinking_level_map(ThinkingLevelMap::reasoning_default())
    .with_base_url(base);
    if matches!(
        wire,
        WireApi::OpenAiResponses | WireApi::AzureOpenAiResponses
    ) {
        let compat = OpenAiResponsesCompat {
            source: opi_ai::model_info::OpenAiResponsesSourceCompat {
                supports_mid_convo_effort: Some(true),
                ..Default::default()
            },
            ..Default::default()
        };
        model.compat = if wire == WireApi::AzureOpenAiResponses {
            WireCompat::AzureOpenAiResponses(compat)
        } else {
            WireCompat::OpenAiResponses(compat)
        };
    }
    model
}

pub(super) fn chat_reply(details: bool) -> String {
    let deltas = if details {
        vec![
            json!({"reasoning_content":"Plan","reasoning":"duplicate","reasoning_details":[{"type":"reasoning.text","text":"de","id":"trace"}]}),
            json!({"reasoning_details":[{"type":"reasoning.text","text":"tail","signature":"signed","index":0},{"type":"reasoning.encrypted","data":"OPAQUE-A"},{"type":"reasoning.encrypted","data":"OPAQUE-B"}],"content":"answer"}),
        ]
    } else {
        vec![json!({"reasoning_content":"Plan","reasoning":"duplicate","content":"answer"})]
    };
    let mut frames: Vec<_> = deltas.into_iter().map(|delta| json!({"id":"capture","model":"resolved-model","choices":[{"index":0,"delta":delta}]})).collect();
    frames.push(json!({"id":"capture","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":3,"completion_tokens":2}}));
    frames
        .into_iter()
        .map(|frame| format!("data: {frame}\n\n"))
        .collect::<String>()
        + "data: [DONE]\n\n"
}

pub(super) fn responses_reply() -> String {
    let item = json!({"type":"reasoning","id":"rs-captured","summary":[{"type":"summary_text","text":"Plan"}],"encrypted_content":"OPAQUE-RESPONSES"});
    let answer = json!({"type":"message","id":"msg-captured","role":"assistant","status":"completed","content":[{"type":"output_text","text":"answer","annotations":[]}]});
    [json!({"type":"response.output_item.added","output_index":0,"item":{"type":"reasoning","id":"rs-captured"}}),
     json!({"type":"response.reasoning_summary_text.delta","item_id":"rs-captured","output_index":0,"summary_index":0,"delta":"Plan"}),
     json!({"type":"response.output_item.done","output_index":0,"item":item.clone()}),
     json!({"type":"response.output_item.added","output_index":1,"item":{"type":"message","id":"msg-captured","role":"assistant","status":"in_progress","content":[]}}),
     json!({"type":"response.output_text.delta","output_index":1,"content_index":0,"item_id":"msg-captured","delta":"answer"}),
     json!({"type":"response.output_item.done","output_index":1,"item":answer.clone()}),
     json!({"type":"response.completed","response":{"id":"capture","model":"resolved-model","status":"completed","output":[item,answer],"usage":{"input_tokens":3,"output_tokens":2}}})]
        .into_iter().map(|frame| format!("data: {frame}\n\n")).collect()
}

pub(super) struct BuiltinCapture {
    pub(super) peer: MockServer,
    pub(super) models: Models,
    pub(super) record: ChatModelRecord,
    pub(super) message: AssistantMessage,
}

pub(super) async fn capture_builtin(
    (provider, wire, details): BuiltinCase,
    expected_sends: u64,
) -> BuiltinCapture {
    let reply = if wire == WireApi::OpenAiCompletions {
        chat_reply(details)
    } else {
        responses_reply()
    };
    let peer = sse_peer(reply, expected_sends).await;
    let (models, record, message) = capture_from_source(provider, wire, &peer).await;
    BuiltinCapture {
        peer,
        models,
        record,
        message,
    }
}

pub(super) async fn sse_peer(reply: String, expected_sends: u64) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(reply),
        )
        .expect(expected_sends)
        .mount(&server)
        .await;
    server
}

pub(super) async fn capture_from_source(
    provider: &str,
    wire: WireApi,
    server: &MockServer,
) -> (Models, ChatModelRecord, AssistantMessage) {
    let mut models = Models::new();
    register_builtin(
        &mut models,
        Arc::new(Source),
        Arc::new(opi_ai::http::HttpClient::new()),
        &BuiltinOverrides::default(),
    )
    .unwrap();
    let record = ChatModelRecord::new(provider, model(wire, server.uri()));
    let captured = models
        .complete_simple(
            record.clone(),
            request(vec![user()], ThinkingLevel::High),
            Default::default(),
        )
        .await
        .unwrap();
    (models, record, captured)
}

pub(super) fn reload_message(captured: &AssistantMessage) -> (Vec<u8>, AssistantMessage) {
    let encoded = serde_json::to_vec(captured).unwrap();
    let restored = serde_json::from_slice(&encoded).unwrap();
    (encoded, restored)
}
