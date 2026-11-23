//! Real continuation requests projected from persisted producer output.

use opi_ai::message::{AssistantContent, AssistantMessage, Message};
use opi_ai::{ChatModelRecord, ThinkingLevel, WireApi};
use serde_json::{Value, json};
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

use super::fixtures::{
    AZURE_CHAT, AZURE_RESPONSES, BuiltinCapture, BuiltinCase, CHAT_OPAQUE_DETAILS, CHAT_REASONING,
    OPENAI_RESPONSES, capture_builtin, chat_reply, model, reload_message, request, responses_reply,
    sse_peer, user,
};

#[path = "target_projection/persisted_producers.rs"]
mod persisted_producers;
#[path = "target_projection/replay_boundaries.rs"]
mod replay_boundaries;

async fn assert_builtin_replay(case @ (provider, wire, details): BuiltinCase) {
    let BuiltinCapture {
        peer: server,
        models,
        record,
        message: captured,
    } = capture_builtin(case, 3).await;
    let responses = wire != WireApi::OpenAiCompletions;
    let (_, restored) = reload_message(&captured);
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
    let (cross_provider, cross_wire, reply) = if responses {
        ("groq", WireApi::OpenAiCompletions, chat_reply(false))
    } else {
        ("openai", WireApi::OpenAiResponses, responses_reply())
    };
    let cross_server = sse_peer(reply, 1).await;
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
    assert_builtin_replay(CHAT_REASONING).await;
}
#[tokio::test]
async fn chat_opaque_details_capture_reload_preserves_delta_merging_and_order() {
    assert_builtin_replay(CHAT_OPAQUE_DETAILS).await;
}
#[tokio::test]
async fn responses_capture_reload_preserves_encrypted_reasoning_and_effort_history() {
    assert_builtin_replay(OPENAI_RESPONSES).await;
}
#[tokio::test]
async fn azure_chat_capture_reload_preserves_azure_identity() {
    assert_builtin_replay(AZURE_CHAT).await;
}
#[tokio::test]
async fn azure_responses_capture_reload_preserves_azure_identity_and_effort() {
    assert_builtin_replay(AZURE_RESPONSES).await;
}

#[tokio::test]
async fn chat_source_grammar_custom_input_survives_capture_store_and_replay() {
    use futures_util::StreamExt;
    use opi_ai::constrained_sampling::ConstrainedSampling;
    use opi_ai::message::{OutputContent, ToolDef, ToolResultMessage};
    use opi_ai::provider::Provider;
    use opi_ai::providers::openai::{CompatConfig, OpenAiChatProvider};
    use opi_ai::stream::AssistantStreamEvent;
    let server = MockServer::start().await;
    let sse = format!(
        "data: {}\n\ndata: {}\n\n",
        json!({"id":"grammar-response","model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant","tool_calls":[{"index":0,"id":"grammar-call","type":"custom","custom":{"name":"search","input":"line one\n\"two\""}}]},"finish_reason":null}]}),
        json!({"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":4,"completion_tokens":2}})
    );
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(2)
        .mount(&server)
        .await;
    let provider = OpenAiChatProvider::new_with_compat(
        Some(server.uri()),
        CompatConfig {
            supports_grammar_tools: Some(true),
            ..Default::default()
        },
    );
    let tool = ToolDef {
        name: "search".into(),
        description: "Find source".into(),
        input_schema: json!({"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}),
        constrained_sampling: Some(ConstrainedSampling::Grammar {
            openai_lark: Some("start: /.+/".into()),
            openai_regex: Some("regex-loses".into()),
        }),
    };
    let mut call = request(vec![user()], ThinkingLevel::None);
    call.model = "openai:gpt-4o".into();
    call.thinking = Default::default();
    call.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: vec![user()],
        tools: vec![tool.clone()],
    })
    .unwrap();
    let events = provider
        .stream_prepared(call.clone(), opi_ai::test_support::resolved_bearer_auth())
        .collect::<Vec<_>>()
        .await;
    let captured = events
        .into_iter()
        .find_map(|event| match event.unwrap() {
            AssistantStreamEvent::Done { message, .. } => Some(message),
            _ => None,
        })
        .unwrap();
    let found = captured
        .content
        .iter()
        .find_map(|part| match part {
            AssistantContent::ToolCall { tool_call } => Some(tool_call),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&found.arguments).unwrap(),
        json!({"query":"line one\n\"two\""})
    );
    assert_eq!(found.arguments_error, None);
    let persisted = serde_json::to_vec(&captured).unwrap();
    let restored: AssistantMessage = serde_json::from_slice(&persisted).unwrap();
    let canonical = serde_json::to_vec(&restored).unwrap();
    call.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        tools: vec![tool],
        messages: vec![
            user(),
            Message::Assistant(restored.clone()),
            Message::ToolResult(ToolResultMessage {
                tool_call_id: found.id.clone(),
                tool_name: found.name.clone(),
                content: vec![OutputContent::Text {
                    text: "result".into(),
                }],
                details: None,
                is_error: false,
                not_executed: None,
                truncated: false,
                timestamp_ms: 1,
            }),
            user(),
        ],
    })
    .unwrap();
    let next = provider
        .stream_prepared(call, opi_ai::test_support::resolved_bearer_auth())
        .collect::<Vec<_>>()
        .await;
    assert!(next.iter().all(Result::is_ok));
    let sent = server.received_requests().await.unwrap();
    for request in &sent {
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(body["tools"][0]["type"], "custom");
        assert_eq!(body["tools"][0]["custom"]["format"]["syntax"], "lark");
    }
    let replay: Value = serde_json::from_slice(&sent[1].body).unwrap();
    let assistant = replay["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|message| message["role"] == "assistant")
        .unwrap();
    assert_eq!(
        assistant["tool_calls"][0],
        json!({"id":"grammar-call","type":"custom","custom":{"name":"search","input":"line one\n\"two\""}})
    );
    assert_eq!(serde_json::to_vec(&restored).unwrap(), canonical);
}
