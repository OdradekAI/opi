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
    assert_eq!(
        serde_json::to_value(&captured).unwrap()["raw_stop_reason"],
        if responses {
            json!("completed")
        } else {
            json!("stop")
        },
        "the result retains the provider's original finish fact"
    );
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

mod replay_boundaries {
    use super::*;

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
        let mut models = Models::new();
        register_builtin(
            &mut models,
            Arc::new(Source),
            Arc::new(opi_ai::http::HttpClient::new()),
            &BuiltinOverrides::default(),
        )
        .unwrap();
        let record = ChatModelRecord::new(provider, model(wire, server.uri()));
        let actual = models
            .complete_simple(
                record.clone(),
                request(vec![user()], ThinkingLevel::High),
                Default::default(),
            )
            .await
            .unwrap();
        (server, models, record, actual)
    }

    async fn continue_from(
        models: &mut Models,
        record: ChatModelRecord,
        restored: &AssistantMessage,
    ) {
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
                    compat.supports_mid_convo_effort = false;
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
            let (server, mut models, record, actual) =
                capture(provider, wire, responses_reply()).await;
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
}

mod persisted_producers {

    use opi_ai::anthropic::{
        AnthropicMessagesChatApi, AnthropicMessagesOptions, AnthropicProvider,
    };
    use opi_ai::auth::{AuthResolver, AuthScheme, ResolvedAuth};
    use opi_ai::http::HttpClient;
    use opi_ai::message::{AssistantMessage, InputContent, Message, UserMessage};
    use opi_ai::openai_chat::{
        OpenAiChatProvider, OpenAiCompletionsChatApi, OpenAiCompletionsOptions,
    };
    use opi_ai::openai_codex_responses::{
        CodexChatApi, CodexOptions, OpenAiCodexResponsesProvider,
    };
    use opi_ai::openai_responses::{
        OpenAiResponsesChatApi, OpenAiResponsesOptions, OpenAiResponsesProvider,
    };
    use opi_ai::provider::{Request, TransportPreference};
    use opi_ai::{
        AuthProvenanceSource, ChatModelRecord, ModelCapabilities, ModelInfo, Models, WireApi,
    };
    use serde_json::{Value, json};
    use std::sync::Arc;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};
    fn user() -> Message {
        Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "continue".into(),
            }],
            timestamp_ms: 0,
        })
    }
    fn req(messages: Vec<Message>) -> Request {
        Request {
            model: "record-owned".into(),
            context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
                system_prompt: None,
                messages,
                tools: vec![],
            })
            .unwrap(),
            max_tokens: Some(4096),
            temperature: None,
            sampling_params: Default::default(),
            thinking: Default::default(),
            transport: TransportPreference::Sse,
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
    fn info(wire: WireApi) -> ModelInfo {
        ModelInfo::new(
            "requested-alias",
            "Fixture",
            wire,
            ModelCapabilities::new(32000, 4096)
                .with_streaming(true)
                .with_thinking(true),
        )
    }
    struct FixtureAuth(AuthScheme);
    impl AuthResolver for FixtureAuth {
        fn resolve<'a>(
            &'a self,
        ) -> opi_ai::BoxAuthFuture<'a, Result<ResolvedAuth, opi_ai::provider::ProviderError>>
        {
            Box::pin(async {
                Ok(ResolvedAuth {
                    env: Default::default(),
                    scheme: self.0.clone(),
                    secret: secrecy::SecretString::from("fixture-key"),
                    account_id: Some("fixture-account".into()),
                    base_url: None,
                    provenance: Default::default(),
                })
            })
        }
    }
    fn auth(scheme: AuthScheme) -> Arc<FixtureAuth> {
        Arc::new(FixtureAuth(scheme))
    }
    fn responses() -> String {
        let item = json!({"type":"reasoning","id":"rs-captured","summary":[{"type":"summary_text","text":"PRIVATE-PLAN"}],"encrypted_content":"PRIVATE-OPAQUE"});
        let answer = json!({"type":"message","id":"msg-captured","role":"assistant","status":"completed","content":[{"type":"output_text","text":"answer","annotations":[]}]});
        [json!({"type":"response.created","response":{"id":"response-captured","model":"reported-model"}}),
json!({"type":"response.output_item.added","output_index":0,"item":{"type":"reasoning","id":"rs-captured"}}),
json!({"type":"response.reasoning_summary_text.delta","item_id":"rs-captured","output_index":0,"summary_index":0,"delta":"PRIVATE-PLAN"}),
json!({"type":"response.output_item.done","output_index":0,"item":item.clone()}),
json!({"type":"response.output_item.added","output_index":1,"item":{"type":"message","id":"msg-captured","role":"assistant","status":"in_progress","content":[]}}),
json!({"type":"response.output_text.delta","output_index":1,"content_index":0,"item_id":"msg-captured","delta":"answer"}),
json!({"type":"response.output_item.done","output_index":1,"item":answer.clone()}),
json!({"type":"response.completed","response":{"id":"response-captured","model":"reported-model","status":"completed","output":[item,answer],"usage":{"input_tokens":3,"output_tokens":2}}})]
.into_iter().map(|x|format!("data: {x}\n\n")).collect()
    }
    fn anthropic() -> String {
        [("message_start",json!({"type":"message_start","message":{"id":"capture","model":"reported-model"}})),
("content_block_start",json!({"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":"","signature":""}})),
("content_block_delta",json!({"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"PRIVATE-PLAN"}})),
("content_block_delta",json!({"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"PRIVATE-SIGNATURE"}})),
("content_block_stop",json!({"type":"content_block_stop","index":0})),
("content_block_start",json!({"type":"content_block_start","index":1,"content_block":{"type":"redacted_thinking","data":"PRIVATE-REDACTED"}})),
("content_block_stop",json!({"type":"content_block_stop","index":1})),
("content_block_start",json!({"type":"content_block_start","index":2,"content_block":{"type":"text","text":"answer"}})),
("content_block_delta",json!({"type":"content_block_delta","index":2,"delta":{"type":"text_delta","text":"answer"}})),
("content_block_stop",json!({"type":"content_block_stop","index":2})),
("message_delta",json!({"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":2}})),
("message_stop",json!({"type":"message_stop"}))]
.into_iter().map(|(event,x)|format!("event: {event}\ndata: {x}\n\n")).collect()
    }
    async fn server(body: String) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(body),
            )
            .mount(&server)
            .await;
        server
    }
    async fn foreign_request(models: &mut Models, captured: &AssistantMessage) {
        let remote=server("data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n".into()).await;
        let model = info(WireApi::OpenAiCompletions);
        let api = OpenAiCompletionsChatApi::new(
            Arc::new(OpenAiChatProvider::new(Some(remote.uri()))),
            model.clone(),
        )
        .unwrap();
        let handle = models
            .register_chat_api(
                Arc::new(api),
                auth(AuthScheme::Bearer),
                AuthProvenanceSource::Static,
            )
            .unwrap();
        let canonical = serde_json::to_value(captured).unwrap();
        models
            .complete(
                ChatModelRecord::new("openai", model),
                req(vec![user(), Message::Assistant(captured.clone()), user()]),
                handle.options(OpenAiCompletionsOptions::default()),
            )
            .await
            .unwrap();
        let requests = remote.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        let body = String::from_utf8(requests[0].body.clone()).unwrap();
        assert!(
            !body.contains("PRIVATE-"),
            "cross provider/API must strip thinking: {body}"
        );
        assert!(body.contains("answer"));
        assert_eq!(serde_json::to_value(captured).unwrap(), canonical);
    }

    async fn provider_only_anthropic_request(
        models: &mut Models,
        remote: &MockServer,
        captured: &AssistantMessage,
    ) {
        let model = info(WireApi::AnthropicMessages);
        let provider_id = "mapped-anthropic";
        assert_ne!(provider_id, captured.provider);
        assert_eq!(model.id, captured.model);
        assert_eq!(Some(model.wire_api), captured.wire);
        let api = AnthropicMessagesChatApi::new(
            Arc::new(AnthropicProvider::for_route(
                provider_id.into(),
                vec![model.clone()],
                Some(remote.uri()),
                Default::default(),
                Arc::new(HttpClient::new()),
                false,
            )),
            model.clone(),
        )
        .unwrap();
        let handle = models
            .register_chat_api(
                Arc::new(api),
                auth(AuthScheme::ApiKey),
                AuthProvenanceSource::Static,
            )
            .unwrap();
        let canonical = serde_json::to_vec(captured).unwrap();
        let before = remote.received_requests().await.unwrap().len();
        let result = models
            .complete(
                ChatModelRecord::new(provider_id, model),
                req(vec![user(), Message::Assistant(captured.clone()), user()]),
                handle.options(AnthropicMessagesOptions::default()),
            )
            .await
            .unwrap();
        assert_eq!(result.provider, provider_id);
        assert_eq!(result.model, captured.model);
        assert_eq!(result.wire, captured.wire);
        let sent = remote.received_requests().await.unwrap();
        assert_eq!(sent.len(), before + 1);
        let request = sent.last().unwrap();
        assert_eq!(request.url.path(), "/v1/messages");
        assert_eq!(request.headers["x-api-key"], "fixture-key");
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(body["model"], captured.model);
        assert!(!body.to_string().contains("PRIVATE-"));
        let assistant = body["messages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|message| message["role"] == "assistant")
            .unwrap();
        assert_eq!(
            assistant["content"],
            json!([{"type":"text","text":"answer"}])
        );
        assert_eq!(serde_json::to_vec(captured).unwrap(), canonical);
    }

    async fn api_only_codex_to_responses_request(
        models: &mut Models,
        remote: &MockServer,
        captured: &AssistantMessage,
    ) {
        let model = info(WireApi::OpenAiResponses);
        assert_eq!(model.id, captured.model);
        assert_ne!(Some(model.wire_api), captured.wire);
        let api = OpenAiResponsesChatApi::new(
            Arc::new(OpenAiResponsesProvider::for_route(
                Some(remote.uri()),
                captured.provider.clone(),
                Default::default(),
                vec![model.clone()],
                Arc::new(HttpClient::new()),
            )),
            model.clone(),
        )
        .unwrap();
        let handle = models
            .register_chat_api(
                Arc::new(api),
                auth(AuthScheme::Bearer),
                AuthProvenanceSource::Static,
            )
            .unwrap();
        let canonical = serde_json::to_vec(captured).unwrap();
        let before = remote.received_requests().await.unwrap().len();
        let result = models
            .complete(
                ChatModelRecord::new(captured.provider.clone(), model),
                req(vec![user(), Message::Assistant(captured.clone()), user()]),
                handle.options(OpenAiResponsesOptions::default()),
            )
            .await
            .unwrap();
        assert_eq!(result.provider, captured.provider);
        assert_eq!(result.model, captured.model);
        assert_eq!(result.wire, Some(WireApi::OpenAiResponses));
        let sent = remote.received_requests().await.unwrap();
        assert_eq!(sent.len(), before + 1);
        let request = sent.last().unwrap();
        assert_eq!(request.url.path(), "/v1/responses");
        assert_eq!(request.headers["authorization"], "Bearer fixture-key");
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(body["model"], captured.model);
        assert!(!body.to_string().contains("PRIVATE-"));
        assert!(
            !body["input"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["type"] == "reasoning")
        );
        assert!(body.to_string().contains("answer"));
        assert_eq!(serde_json::to_vec(captured).unwrap(), canonical);
    }
    #[tokio::test]
    async fn codex_producer_persist_reload_same_changed_model_and_foreign_wire() {
        let remote = server(responses()).await;
        let mut models = Models::new();
        let model = info(WireApi::OpenAiCodexResponses);
        let api = CodexChatApi::new(
            Arc::new(OpenAiCodexResponsesProvider::new(
                Some(remote.uri()),
                vec![],
                Arc::new(HttpClient::new()),
            )),
            model.clone(),
        )
        .unwrap();
        let handle = models
            .register_chat_api(
                Arc::new(api),
                auth(AuthScheme::Bearer),
                AuthProvenanceSource::Static,
            )
            .unwrap();
        let record = ChatModelRecord::new("openai-codex", model.clone());
        let actual = models
            .complete(
                record.clone(),
                req(vec![user()]),
                handle.options(CodexOptions::default()),
            )
            .await
            .unwrap();
        assert_eq!(actual.provider, "openai-codex");
        assert_eq!(actual.model, "requested-alias");
        assert_eq!(actual.wire, Some(WireApi::OpenAiCodexResponses));
        let stored = serde_json::to_vec(&actual).unwrap();
        let restored: AssistantMessage = serde_json::from_slice(&stored).unwrap();
        assert_eq!(actual, restored);
        assert!(
            String::from_utf8(stored.clone())
                .unwrap()
                .contains("PRIVATE-OPAQUE")
        );
        models
            .complete(
                record,
                req(vec![user(), Message::Assistant(restored.clone()), user()]),
                handle.options(CodexOptions::default()),
            )
            .await
            .unwrap();
        let mut changed = model;
        changed.id = "changed-model".into();
        models
            .complete(
                ChatModelRecord::new("openai-codex", changed),
                req(vec![user(), Message::Assistant(restored.clone()), user()]),
                handle.options(CodexOptions::default()),
            )
            .await
            .unwrap();
        let requests = remote.received_requests().await.unwrap();
        assert_eq!(requests.len(), 3);
        let same: Value = serde_json::from_slice(&requests[1].body).unwrap();
        assert_eq!(requests[1].url.path(), "/codex/responses");
        let reasoning = same["input"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["type"] == "reasoning")
            .unwrap();
        assert_eq!(reasoning["id"], "rs-captured");
        assert_eq!(reasoning["encrypted_content"], "PRIVATE-OPAQUE");
        assert!(same.to_string().contains("PRIVATE-PLAN"));
        assert!(
            !String::from_utf8(requests[2].body.clone())
                .unwrap()
                .contains("PRIVATE-")
        );
        foreign_request(&mut models, &restored).await;
        api_only_codex_to_responses_request(&mut models, &remote, &restored).await;
        assert_eq!(serde_json::to_vec(&restored).unwrap(), stored);
        println!("Codex producer reload: same/model/API-only/foreign actual requests PASS");
    }
    #[tokio::test]
    async fn anthropic_producer_persist_reload_same_changed_model_and_foreign_wire() {
        let remote = server(anthropic()).await;
        let mut models = Models::new();
        let model = info(WireApi::AnthropicMessages);
        let api = AnthropicMessagesChatApi::new(
            Arc::new(AnthropicProvider::new(Some(remote.uri()))),
            model.clone(),
        )
        .unwrap();
        let handle = models
            .register_chat_api(
                Arc::new(api),
                auth(AuthScheme::ApiKey),
                AuthProvenanceSource::Static,
            )
            .unwrap();
        let record = ChatModelRecord::new("anthropic", model.clone());
        let actual = models
            .complete(
                record.clone(),
                req(vec![user()]),
                handle.options(AnthropicMessagesOptions::default()),
            )
            .await
            .unwrap();
        assert_eq!(actual.provider, "anthropic");
        assert_eq!(actual.wire, Some(WireApi::AnthropicMessages));
        let stored = serde_json::to_vec(&actual).unwrap();
        let restored: AssistantMessage = serde_json::from_slice(&stored).unwrap();
        assert_eq!(restored, actual);
        models
            .complete(
                record,
                req(vec![user(), Message::Assistant(restored.clone()), user()]),
                handle.options(AnthropicMessagesOptions::default()),
            )
            .await
            .unwrap();
        let mut changed = model;
        changed.id = "changed-model".into();
        models
            .complete(
                ChatModelRecord::new("anthropic", changed),
                req(vec![user(), Message::Assistant(restored.clone()), user()]),
                handle.options(AnthropicMessagesOptions::default()),
            )
            .await
            .unwrap();
        let requests = remote.received_requests().await.unwrap();
        assert_eq!(requests.len(), 3);
        let same: Value = serde_json::from_slice(&requests[1].body).unwrap();
        let assistant = same["messages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|message| message["role"] == "assistant")
            .unwrap();
        assert_eq!(
            assistant["content"],
            json!([
                {"type":"thinking","thinking":"PRIVATE-PLAN","signature":"PRIVATE-SIGNATURE"},
                {"type":"redacted_thinking","data":"PRIVATE-REDACTED"},
                {"type":"text","text":"answer"},
            ])
        );
        assert!(
            !String::from_utf8(requests[2].body.clone())
                .unwrap()
                .contains("PRIVATE-")
        );
        foreign_request(&mut models, &restored).await;
        provider_only_anthropic_request(&mut models, &remote, &restored).await;
        assert_eq!(serde_json::to_vec(&restored).unwrap(), stored);
        println!(
            "Anthropic producer reload: exact blocks and same/model/provider-only/foreign requests PASS"
        );
    }

    #[tokio::test]
    async fn anthropic_opaque_only_producer_reload_preserves_empty_signed_block() {
        let frames = [
            ("message_start", json!({"type":"message_start","message":{"id":"opaque-only","model":"reported-model"}})),
            ("content_block_start", json!({"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":"","signature":""}})),
            ("content_block_delta", json!({"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"PRIVATE-EMPTY-SIGNATURE"}})),
            ("content_block_stop", json!({"type":"content_block_stop","index":0})),
            ("content_block_start", json!({"type":"content_block_start","index":1,"content_block":{"type":"redacted_thinking","data":"PRIVATE-OPAQUE-ONLY"}})),
            ("content_block_stop", json!({"type":"content_block_stop","index":1})),
            ("message_delta", json!({"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":2}})),
            ("message_stop", json!({"type":"message_stop"})),
        ].into_iter().map(|(event, body)| format!("event: {event}\ndata: {body}\n\n")).collect();
        let remote = server(frames).await;
        let model = info(WireApi::AnthropicMessages);
        let api = AnthropicMessagesChatApi::new(
            Arc::new(AnthropicProvider::new(Some(remote.uri()))),
            model.clone(),
        )
        .unwrap();
        let mut models = Models::new();
        let handle = models
            .register_chat_api(
                Arc::new(api),
                auth(AuthScheme::ApiKey),
                AuthProvenanceSource::Static,
            )
            .unwrap();
        let record = ChatModelRecord::new("anthropic", model);
        let captured = models
            .complete(
                record.clone(),
                req(vec![user()]),
                handle.options(AnthropicMessagesOptions::default()),
            )
            .await
            .unwrap();
        assert_eq!(
            captured.content,
            vec![
                opi_ai::message::AssistantContent::Thinking {
                    thinking: String::new(),
                    signature: Some("PRIVATE-EMPTY-SIGNATURE".into())
                },
                opi_ai::message::AssistantContent::RedactedThinking {
                    data: "PRIVATE-OPAQUE-ONLY".into()
                },
            ]
        );
        let canonical = serde_json::to_vec(&captured).unwrap();
        let reload: AssistantMessage = serde_json::from_slice(&canonical).unwrap();
        assert_eq!(reload, captured);
        models
            .complete(
                record,
                req(vec![user(), Message::Assistant(reload.clone()), user()]),
                handle.options(AnthropicMessagesOptions::default()),
            )
            .await
            .unwrap();
        let sent = remote.received_requests().await.unwrap();
        assert_eq!(sent.len(), 2);
        let body: Value = serde_json::from_slice(&sent[1].body).unwrap();
        let assistant = body["messages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|message| message["role"] == "assistant")
            .unwrap();
        assert_eq!(
            assistant["content"],
            json!([
                {"type":"thinking","thinking":"","signature":"PRIVATE-EMPTY-SIGNATURE"},
                {"type":"redacted_thinking","data":"PRIVATE-OPAQUE-ONLY"},
            ])
        );
        assert_eq!(serde_json::to_vec(&reload).unwrap(), canonical);
    }

    #[tokio::test]
    async fn codex_terminal_backfill_and_dual_tool_ids_survive_real_reload_request() {
        use opi_ai::message::{AssistantContent, OutputContent, ToolDef, ToolResultMessage};
        let reasoning = json!({"type":"reasoning","id":"rs-empty","summary":[],"encrypted_content":"OPAQUE-EMPTY"});
        let tool = json!({"type":"function_call","id":"fc_real","call_id":"call-real","name":"read","arguments":"{\"path\":\"fixture\"}"});
        let frames=[
        json!({"type":"response.output_item.added","output_index":0,"item":{"type":"reasoning","id":"rs-empty"}}),
        json!({"type":"response.output_item.done","output_index":0,"item":{"type":"reasoning","id":"rs-empty","summary":[]}}),
        json!({"type":"response.output_item.added","output_index":1,"item":{"type":"function_call","id":"fc_real","call_id":"call-real","name":"read","arguments":""}}),
        json!({"type":"response.output_item.done","output_index":1,"item":tool.clone()}),
        json!({"type":"response.completed","response":{"status":"completed","id":"actual-codex","model":"reported-model","output":[reasoning,tool],"usage":{"input_tokens":3,"output_tokens":2}}}),
    ].into_iter().map(|v|format!("data: {v}\n\n")).collect();
        let remote = server(frames).await;
        let mut models = Models::new();
        let model = info(WireApi::OpenAiCodexResponses);
        let api = CodexChatApi::new(
            Arc::new(OpenAiCodexResponsesProvider::new(
                Some(remote.uri()),
                vec![],
                Arc::new(HttpClient::new()),
            )),
            model.clone(),
        )
        .unwrap();
        let handle = models
            .register_chat_api(
                Arc::new(api),
                auth(AuthScheme::Bearer),
                AuthProvenanceSource::Static,
            )
            .unwrap();
        let record = ChatModelRecord::new("openai-codex", model);
        let request = |messages| {
            let mut request = req(vec![]);
            request.context=opi_ai::transcript::normalize_context(opi_ai::transcript::Context{
            system_prompt:None,messages,tools:vec![ToolDef{name:"read".into(),description:"Read fixture".into(),input_schema:json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}),constrained_sampling:None}],
        }).unwrap();
            request
        };
        let actual = models
            .complete(
                record.clone(),
                request(vec![user()]),
                handle.options(CodexOptions::default()),
            )
            .await
            .unwrap();
        let AssistantContent::Thinking {
            thinking,
            signature,
        } = &actual.content[0]
        else {
            panic!("the empty encrypted reasoning item must survive capture");
        };
        assert!(thinking.is_empty());
        let signature: Value = serde_json::from_str(signature.as_deref().unwrap()).unwrap();
        assert_eq!(signature["id"], "rs-empty");
        assert_eq!(signature["encrypted_content"], "OPAQUE-EMPTY");
        let captured_call = actual
            .content
            .iter()
            .find_map(|c| {
                if let AssistantContent::ToolCall { tool_call } = c {
                    Some(tool_call.clone())
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(captured_call.id, "call-real|fc_real");
        let canonical = serde_json::to_vec(&actual).unwrap();
        let restored: AssistantMessage = serde_json::from_slice(&canonical).unwrap();
        models
            .complete(
                record,
                request(vec![
                    user(),
                    Message::Assistant(restored.clone()),
                    Message::ToolResult(ToolResultMessage {
                        tool_call_id: captured_call.id,
                        tool_name: captured_call.name,
                        content: vec![OutputContent::Text {
                            text: "fixture result".into(),
                        }],
                        details: None,
                        is_error: false,
                        not_executed: None,
                        truncated: false,
                        timestamp_ms: 0,
                    }),
                    user(),
                ]),
                handle.options(CodexOptions::default()),
            )
            .await
            .unwrap();
        let sent = remote.received_requests().await.unwrap();
        assert_eq!(sent.len(), 2);
        let wire: Value = serde_json::from_slice(&sent[1].body).unwrap();
        let input = wire["input"].as_array().unwrap();
        assert_eq!(
            input
                .iter()
                .filter_map(|item| item["type"].as_str())
                .collect::<Vec<_>>(),
            ["reasoning", "function_call", "function_call_output"]
        );
        let reasoning = input.iter().find(|v| v["type"] == "reasoning").unwrap();
        assert_eq!(reasoning["id"], "rs-empty");
        assert_eq!(reasoning["encrypted_content"], "OPAQUE-EMPTY");
        let call = input.iter().find(|v| v["type"] == "function_call").unwrap();
        assert_eq!(call["id"], "fc_real");
        assert_eq!(call["call_id"], "call-real");
        let result = input
            .iter()
            .find(|v| v["type"] == "function_call_output")
            .unwrap();
        assert_eq!(result["call_id"], "call-real");
        assert_eq!(result["output"], "fixture result");
        assert_eq!(serde_json::to_vec(&restored).unwrap(), canonical);
    }
}
