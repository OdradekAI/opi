//! Actual builtin producer output survives persistence and target-wire projection.
use opi_ai::auth::{AuthContext, AuthContextError};
use opi_ai::message::{AssistantContent, AssistantMessage, InputContent, Message, UserMessage};
use opi_ai::model_info::{OpenAiResponsesCompat, WireCompat};
use opi_ai::provider::{Request, ThinkingConfig};
use opi_ai::provider_collection::{BuiltinOverrides, register_builtin};
use opi_ai::{
    ChatModelRecord, ModelCapabilities, ModelInfo, Models, ThinkingLevel, ThinkingLevelMap, WireApi,
};
use serde_json::{Value, json};
use std::sync::Arc;
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

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
fn request(messages: Vec<Message>, effort: ThinkingLevel) -> Request {
    Request {
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
fn user() -> Message {
    Message::User(UserMessage {
        content: vec![InputContent::Text {
            text: "continue".into(),
        }],
        timestamp_ms: 0,
    })
}
fn model(wire: WireApi, base: String) -> ModelInfo {
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
            supports_mid_convo_effort: true,
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
fn chat_reply(details: bool) -> String {
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
fn responses_reply() -> String {
    let item = json!({"type":"reasoning","id":"rs-captured","summary":[{"type":"summary_text","text":"Plan"}],"encrypted_content":"OPAQUE-RESPONSES"});
    [json!({"type":"response.output_item.added","output_index":0,"item":{"type":"reasoning","id":"rs-captured"}}),
     json!({"type":"response.reasoning_summary_text.delta","item_id":"rs-captured","output_index":0,"summary_index":0,"delta":"Plan"}),
     json!({"type":"response.output_item.done","output_index":0,"item":item.clone()}),
     json!({"type":"response.output_text.delta","delta":"answer"}),
     json!({"type":"response.completed","response":{"id":"capture","model":"resolved-model","output":[item],"usage":{"input_tokens":3,"output_tokens":2}}})]
        .into_iter().map(|frame| format!("data: {frame}\n\n")).collect()
}
async fn exercise(provider: &str, wire: WireApi, details: bool) {
    let server = MockServer::start().await;
    let responses = wire != WireApi::OpenAiCompletions;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(if responses {
                    responses_reply()
                } else {
                    chat_reply(details)
                }),
        )
        .expect(3)
        .mount(&server)
        .await;
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
    assert_eq!(captured.provider, provider);
    assert_eq!(captured.model, "requested-alias");
    assert_eq!(captured.wire, Some(wire));
    assert_eq!(captured.response_model.as_deref(), Some("resolved-model"));
    assert_eq!(captured.usage.input_tokens, 3);
    let (thinking, signature) = captured
        .content
        .iter()
        .find_map(|block| match block {
            AssistantContent::Thinking {
                thinking,
                signature,
            } => Some((thinking, signature)),
            _ => None,
        })
        .expect("actual adapter must capture its thinking output");
    assert_eq!(thinking, "Plan");
    if !responses && !details {
        assert_eq!(signature.as_deref(), Some("reasoning_content"));
    }
    if responses {
        let expected = (wire != WireApi::AzureOpenAiResponses).then_some("high");
        assert_eq!(captured.provider_thinking_level.as_deref(), expected);
    }
    let encoded = serde_json::to_vec(&captured).unwrap();
    let restored: AssistantMessage = serde_json::from_slice(&encoded).unwrap();
    let canonical = serde_json::to_value(&restored).unwrap();
    let history = vec![user(), Message::Assistant(restored.clone()), user()];
    models
        .complete_simple(
            record.clone(),
            request(history.clone(), ThinkingLevel::Low),
            Default::default(),
        )
        .await
        .unwrap();
    let mut foreign = record.info().clone();
    foreign.id = "different-model".into();
    models
        .complete_simple(
            ChatModelRecord::new(provider, foreign),
            request(history, ThinkingLevel::Low),
            Default::default(),
        )
        .await
        .unwrap();
    let cross_server = MockServer::start().await;
    let (cross_provider, cross_wire, reply) = if responses {
        ("groq", WireApi::OpenAiCompletions, chat_reply(false))
    } else {
        ("openai", WireApi::OpenAiResponses, responses_reply())
    };
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(reply),
        )
        .expect(1)
        .mount(&cross_server)
        .await;
    models
        .complete_simple(
            ChatModelRecord::new(cross_provider, model(cross_wire, cross_server.uri())),
            request(
                vec![user(), Message::Assistant(restored.clone()), user()],
                ThinkingLevel::Low,
            ),
            Default::default(),
        )
        .await
        .unwrap();
    let cross = cross_server.received_requests().await.unwrap();
    let cross_body: Value = serde_json::from_slice(&cross[0].body).unwrap();
    assert!(
        !cross_body.to_string().contains("OPAQUE"),
        "foreign provider/API receives no opaque replay data"
    );
    assert!(
        !cross_body.to_string().contains("Plan"),
        "foreign provider/API receives no thinking projection"
    );
    assert!(
        cross_body.to_string().contains("answer"),
        "readable assistant output survives the projection"
    );
    assert_eq!(
        serde_json::to_value(&restored).unwrap(),
        canonical,
        "projection cannot rewrite canonical data"
    );
    let sent = server.received_requests().await.unwrap();
    let first: Value = serde_json::from_slice(&sent[0].body).unwrap();
    let replay: Value = serde_json::from_slice(&sent[1].body).unwrap();
    let foreign: Value = serde_json::from_slice(&sent[2].body).unwrap();
    if responses {
        let input = replay["input"].as_array().unwrap();
        let reasoning = input
            .iter()
            .find(|item| item["type"] == "reasoning")
            .expect("captured encrypted item replays");
        assert_eq!(reasoning["id"], "rs-captured");
        assert_eq!(reasoning["encrypted_content"], "OPAQUE-RESPONSES");
        let effort: Vec<_> = input
            .iter()
            .filter(|item| item["type"] == "configuration_update")
            .map(|item| item["reasoning"]["effort"].as_str().unwrap())
            .collect();
        if wire == WireApi::AzureOpenAiResponses {
            assert!(
                effort.is_empty(),
                "Azure source does not emit history effort stamps"
            );
            assert_eq!(first["reasoning"]["effort"], "high");
            assert_eq!(replay["reasoning"]["effort"], "low");
        } else {
            assert_eq!(effort, ["high", "low"]);
            assert_eq!(first["reasoning"]["effort"], "medium");
        }
        assert!(!foreign.to_string().contains("OPAQUE-RESPONSES"));
        assert!(
            !foreign["input"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["reasoning"]["effort"] == "high")
        );
    } else {
        assert_eq!(first["reasoning_effort"], "high");
        assert_eq!(replay["reasoning_effort"], "low");
        let assistant = replay["messages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|message| message["role"] == "assistant")
            .unwrap();
        if details {
            assert_eq!(
                assistant["reasoning_details"],
                json!([
                {"type":"reasoning.text","text":"detail","id":"trace","signature":"signed","index":0},
                {"type":"reasoning.encrypted","data":"OPAQUE-A"},{"type":"reasoning.encrypted","data":"OPAQUE-B"}])
            );
        } else {
            assert_eq!(assistant["reasoning_content"], "Plan");
        }
        assert_eq!(assistant["content"], "answer");
        let foreign_assistant = foreign["messages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|message| message["role"] == "assistant")
            .unwrap();
        assert!(foreign_assistant.get("reasoning_details").is_none());
        assert!(foreign_assistant.get("reasoning_content").is_none());
    }
}
#[tokio::test]
async fn chat_reasoning_capture_reload_reaches_same_and_changed_model_requests() {
    exercise("groq", WireApi::OpenAiCompletions, false).await;
}
#[tokio::test]
async fn chat_opaque_details_capture_reload_preserves_delta_merging_and_order() {
    exercise("groq", WireApi::OpenAiCompletions, true).await;
}
#[tokio::test]
async fn responses_capture_reload_preserves_encrypted_reasoning_and_effort_history() {
    exercise("openai", WireApi::OpenAiResponses, false).await;
}
#[tokio::test]
async fn azure_chat_capture_reload_preserves_azure_identity() {
    exercise("azure", WireApi::OpenAiCompletions, false).await;
}
#[tokio::test]
async fn azure_responses_capture_reload_preserves_azure_identity_and_effort() {
    exercise("azure", WireApi::AzureOpenAiResponses, false).await;
}
