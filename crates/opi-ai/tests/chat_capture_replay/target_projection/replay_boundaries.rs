//! Opaque metadata, replay identity and compatibility on the target wire.

use opi_ai::message::{AssistantContent, AssistantMessage, Message};
use opi_ai::model_info::WireCompat;
use opi_ai::{ChatModelRecord, Models, ThinkingLevel, WireApi};
use serde_json::{Value, json};
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

use crate::fixtures::{capture_from_source, request, responses_reply, user};

async fn capture(
    provider: &str,
    wire: WireApi,
    reply: String,
) -> (MockServer, Models, ChatModelRecord, AssistantMessage) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(reply),
        )
        .mount(&server)
        .await;
    let (models, record, actual) = capture_from_source(provider, wire, &server).await;
    (server, models, record, actual)
}

async fn continue_from(models: &mut Models, record: ChatModelRecord, restored: &AssistantMessage) {
    models
        .complete_simple(
            record,
            request(
                vec![user(), Message::Assistant(restored.clone()), user()],
                ThinkingLevel::Low,
            ),
            Default::default(),
        )
        .await
        .unwrap();
}

fn backfilled_reply() -> String {
    let a = json!({"type":"reasoning","id":"rs-empty-a","summary":[],"encrypted_content":"OPAQUE-BACKFILL-A"});
    let b = json!({"type":"reasoning","id":"rs-empty-b","summary":[],"encrypted_content":"OPAQUE-BACKFILL-B"});
    let answer = json!({"type":"message","id":"msg-between","role":"assistant","status":"completed","content":[{"type":"output_text","text":"answer","annotations":[]}]});
    [
        json!({"type":"response.output_item.added","output_index":0,"item":{"type":"reasoning","id":"rs-empty-a"}}),
        json!({"type":"response.output_item.done","output_index":0,"item":{"type":"reasoning","id":"rs-empty-a","summary":[]}}),
        json!({"type":"response.output_item.added","output_index":1,"item":{"type":"message","id":"msg-between","role":"assistant","status":"in_progress","content":[]}}),
        json!({"type":"response.output_text.delta","output_index":1,"content_index":0,"item_id":"msg-between","delta":"answer"}),
        json!({"type":"response.output_item.done","output_index":1,"item":answer.clone()}),
        json!({"type":"response.output_item.added","output_index":2,"item":{"type":"reasoning","id":"rs-empty-b"}}),
        json!({"type":"response.output_item.done","output_index":2,"item":{"type":"reasoning","id":"rs-empty-b","summary":[]}}),
        json!({"type":"response.completed","response":{"status":"completed","id":"terminal-backfill","model":"reported-model","output":[a,answer,b],"usage":{"input_tokens":3,"output_tokens":2}}}),
    ]
    .into_iter()
    .map(|frame| format!("data: {frame}\n\n"))
    .collect()
}

#[tokio::test]
async fn responses_and_azure_terminal_backfill_survives_reload_and_compat_change() {
    for (provider, wire) in [
        ("openai", WireApi::OpenAiResponses),
        ("azure", WireApi::AzureOpenAiResponses),
    ] {
        let (server, mut models, record, actual) =
            capture(provider, wire, backfilled_reply()).await;
        assert_eq!(actual.provider, provider);
        assert_eq!(actual.wire, Some(wire));
        assert_eq!(actual.raw_stop_reason.as_deref(), Some("completed"));
        assert_eq!(actual.response_model.as_deref(), Some("reported-model"));
        assert_eq!(actual.content.len(), 3);
        for (index, id, encrypted) in [
            (0, "rs-empty-a", "OPAQUE-BACKFILL-A"),
            (2, "rs-empty-b", "OPAQUE-BACKFILL-B"),
        ] {
            let AssistantContent::Thinking {
                thinking,
                signature,
            } = &actual.content[index]
            else {
                panic!("the empty signed item keeps its block position");
            };
            assert!(thinking.is_empty());
            let signature: Value = serde_json::from_str(signature.as_deref().unwrap()).unwrap();
            assert_eq!(signature["id"], id);
            assert_eq!(signature["encrypted_content"], encrypted);
        }
        let bytes = serde_json::to_vec(&actual).unwrap();
        let restored: AssistantMessage = serde_json::from_slice(&bytes).unwrap();
        continue_from(&mut models, record.clone(), &restored).await;
        let mut changed = record.info().clone();
        match &mut changed.compat {
            WireCompat::OpenAiResponses(compat) | WireCompat::AzureOpenAiResponses(compat) => {
                compat.source.supports_mid_convo_effort = Some(false);
            }
            _ => unreachable!(),
        }
        continue_from(
            &mut models,
            ChatModelRecord::new(provider, changed),
            &restored,
        )
        .await;
        let mut foreign = record.info().clone();
        foreign.id = "changed-model".into();
        continue_from(
            &mut models,
            ChatModelRecord::new(provider, foreign),
            &restored,
        )
        .await;
        let sent = server.received_requests().await.unwrap();
        assert_eq!(sent.len(), 4);
        let body: Value = serde_json::from_slice(&sent[1].body).unwrap();
        let without_stamps: Vec<_> = body["input"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|item| item["type"] != "configuration_update")
            .filter(|item| item["role"] != "user")
            .collect();
        assert_eq!(
            without_stamps
                .iter()
                .map(|item| item["type"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["reasoning", "message", "reasoning"]
        );
        assert_eq!(without_stamps[0]["encrypted_content"], "OPAQUE-BACKFILL-A");
        assert_eq!(without_stamps[2]["encrypted_content"], "OPAQUE-BACKFILL-B");
        assert_eq!(without_stamps[1]["content"][0]["text"], "answer");
        let changed: Value = serde_json::from_slice(&sent[2].body).unwrap();
        assert!(
            !changed["input"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["type"] == "configuration_update")
        );
        assert_eq!(changed["model"], "requested-alias");
        assert!(changed.to_string().contains("OPAQUE-BACKFILL-A"));
        let foreign: Value = serde_json::from_slice(&sent[3].body).unwrap();
        assert!(!foreign.to_string().contains("OPAQUE-BACKFILL"));
        assert!(foreign.to_string().contains("answer"));
        assert_eq!(serde_json::to_vec(&restored).unwrap(), bytes);
    }
}

#[tokio::test]
async fn responses_invalid_and_unknown_metadata_projects_without_rewriting_capture() {
    for (provider, wire) in [
        ("openai", WireApi::OpenAiResponses),
        ("azure", WireApi::AzureOpenAiResponses),
    ] {
        let (server, mut models, record, actual) = capture(provider, wire, responses_reply()).await;
        let original = serde_json::to_vec(&actual).unwrap();
        for invalid in [
            "{invalid",
            "null",
            r#"{"type":"message","encrypted_content":"INVALID-OPAQUE"}"#,
        ] {
            let mut restored: AssistantMessage = serde_json::from_slice(&original).unwrap();
            let AssistantContent::Thinking { signature, .. } = &mut restored.content[0] else {
                panic!("producer thinking")
            };
            *signature = Some(invalid.into());
            let canonical = serde_json::to_vec(&restored).unwrap();
            continue_from(&mut models, record.clone(), &restored).await;
            assert_eq!(serde_json::to_vec(&restored).unwrap(), canonical);
        }
        let mut unknown: AssistantMessage = serde_json::from_slice(&original).unwrap();
        unknown.wire = None;
        let canonical = serde_json::to_vec(&unknown).unwrap();
        continue_from(&mut models, record, &unknown).await;
        assert_eq!(serde_json::to_vec(&unknown).unwrap(), canonical);
        assert_eq!(serde_json::to_vec(&actual).unwrap(), original);
        let sent = server.received_requests().await.unwrap();
        assert_eq!(sent.len(), 5);
        for request in &sent[1..] {
            let body: Value = serde_json::from_slice(&request.body).unwrap();
            assert!(
                !body["input"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|item| item["type"] == "reasoning")
            );
            assert!(!body.to_string().contains("OPAQUE"));
            assert!(!body.to_string().contains("Plan"));
            assert!(body.to_string().contains("answer"));
        }
    }
}

#[tokio::test]
async fn chat_opaque_only_details_survive_actual_capture_reload_and_next_send() {
    let details = json!([
        {"type":"reasoning.encrypted","id":"opaque-a","data":"OPAQUE-ONLY-A"},
        {"type":"reasoning.encrypted","id":"opaque-b","data":"OPAQUE-ONLY-B"}
    ]);
    let reply = [
        json!({"id":"opaque-only","choices":[{"index":0,"delta":{"reasoning_details":details.clone(),"content":"answer"}}]}),
        json!({"choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}),
    ].into_iter().map(|frame| format!("data: {frame}\n\n")).collect::<String>() + "data: [DONE]\n\n";
    let (server, mut models, record, actual) =
        capture("groq", WireApi::OpenAiCompletions, reply).await;
    let AssistantContent::Thinking {
        thinking,
        signature,
    } = &actual.content[0]
    else {
        panic!("actual opaque-only capture")
    };
    assert!(thinking.is_empty());
    assert_eq!(
        serde_json::from_str::<Value>(signature.as_deref().unwrap()).unwrap(),
        details
    );
    let original = serde_json::to_vec(&actual).unwrap();
    let restored: AssistantMessage = serde_json::from_slice(&original).unwrap();
    continue_from(&mut models, record.clone(), &restored).await;
    for invalid in [
        "{invalid",
        "null",
        "[]",
        "[{}]",
        r#"{"type":"reasoning.encrypted","data":"INVALID"}"#,
    ] {
        let mut damaged: AssistantMessage = serde_json::from_slice(&original).unwrap();
        let AssistantContent::Thinking { signature, .. } = &mut damaged.content[0] else {
            unreachable!()
        };
        *signature = Some(invalid.into());
        let canonical = serde_json::to_vec(&damaged).unwrap();
        continue_from(&mut models, record.clone(), &damaged).await;
        assert_eq!(serde_json::to_vec(&damaged).unwrap(), canonical);
    }
    let mut unknown: AssistantMessage = serde_json::from_slice(&original).unwrap();
    unknown.wire = None;
    let canonical = serde_json::to_vec(&unknown).unwrap();
    continue_from(&mut models, record.clone(), &unknown).await;
    assert_eq!(serde_json::to_vec(&unknown).unwrap(), canonical);
    let mut foreign = record.info().clone();
    foreign.id = "changed-model".into();
    continue_from(
        &mut models,
        ChatModelRecord::new("groq", foreign),
        &restored,
    )
    .await;
    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 9);
    let body: Value = serde_json::from_slice(&sent[1].body).unwrap();
    let assistant = body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|message| message["role"] == "assistant")
        .unwrap();
    assert_eq!(assistant["reasoning_details"], details);
    assert_eq!(assistant["content"], "answer");
    for request in &sent[2..] {
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        let assistant = body["messages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|message| message["role"] == "assistant")
            .unwrap();
        assert!(assistant.get("reasoning_details").is_none());
        assert_eq!(assistant["content"], "answer");
        assert!(!assistant.to_string().contains("OPAQUE"));
    }
    assert_eq!(serde_json::to_vec(&restored).unwrap(), original);
}
