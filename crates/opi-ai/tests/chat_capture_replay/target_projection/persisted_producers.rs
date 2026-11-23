//! Codex and Anthropic continuation requests from native producer results.

use opi_ai::api::anthropic::{AnthropicMessagesChatApi, AnthropicMessagesOptions};
use opi_ai::api::openai_codex_responses::{CodexChatApi, CodexOptions};
use opi_ai::api::openai_completions::{OpenAiCompletionsChatApi, OpenAiCompletionsOptions};
use opi_ai::api::openai_responses::{OpenAiResponsesChatApi, OpenAiResponsesOptions};
use opi_ai::auth::{AuthResolver, AuthScheme, ResolvedAuth};
use opi_ai::http::HttpClient;
use opi_ai::message::{AssistantMessage, InputContent, Message, UserMessage};
use opi_ai::provider::{Request, TransportPreference};
use opi_ai::providers::anthropic::AnthropicProvider;
use opi_ai::providers::openai::OpenAiChatProvider;
use opi_ai::providers::openai_codex::OpenAiCodexResponsesProvider;
use opi_ai::providers::openai_responses::OpenAiResponsesProvider;
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
        projection: Default::default(),
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
    ) -> opi_ai::BoxAuthFuture<'a, Result<ResolvedAuth, opi_ai::provider::ProviderError>> {
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
    let reasoning =
        json!({"type":"reasoning","id":"rs-empty","summary":[],"encrypted_content":"OPAQUE-EMPTY"});
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
