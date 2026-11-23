use super::*;
use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};

fn provider() -> OpenAiCodexResponsesProvider {
    OpenAiCodexResponsesProvider::new(None, vec![model(None)], Arc::new(HttpClient::new()))
}

fn request_with(tools: Vec<ToolDef>) -> Request {
    let mut request = request();
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: Some("system prompt".into()),
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "hello".into(),
            }],
            timestamp_ms: 0,
        })],
        tools,
    })
    .expect("valid context declarations");
    request
}

fn json_schema_tool(strict: StrictPreference) -> ToolDef {
    ToolDef {
        name: "structured".into(),
        description: "d".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {"path": {"type": "string"}},
            "required": ["path"]
        }),
        constrained_sampling: Some(ConstrainedSampling::JsonSchema { strict }),
    }
}

fn grammar_tool() -> ToolDef {
    ToolDef {
        name: "grammar-tool".into(),
        description: "d".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {"input": {"type": "string"}},
            "required": ["input"]
        }),
        constrained_sampling: Some(ConstrainedSampling::Grammar {
            openai_lark: Some("start: WORD".into()),
            openai_regex: None,
        }),
    }
}

#[test]
fn plain_tool_keeps_the_null_strict_default() {
    let body = provider()
        .build_request_body(&request_with(vec![ToolDef {
            name: "read".into(),
            description: "Read a file".into(),
            input_schema: serde_json::json!({"type": "object"}),
            constrained_sampling: None,
        }]))
        .unwrap();
    assert_eq!(body["tools"][0]["strict"], serde_json::json!(null));
    assert_eq!(
        body["tools"][0]["parameters"],
        serde_json::json!({"type": "object"})
    );
}

#[test]
fn json_schema_tool_resolves_strict_over_the_null_default() {
    for preference in [StrictPreference::Prefer, StrictPreference::Require] {
        let body = provider()
            .build_request_body(&request_with(vec![json_schema_tool(preference)]))
            .unwrap();
        assert_eq!(body["tools"][0]["strict"], serde_json::json!(true));
        assert_eq!(
            body["tools"][0]["parameters"]["additionalProperties"],
            serde_json::json!(false)
        );
    }
}

#[tokio::test]
async fn grammar_tool_falls_back_to_plain_json_with_recorded_loss() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_1\",\"model\":\"gpt-5.4\",\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}}\n\n",
            ))
            .mount(&server)
            .await;
    let provider = OpenAiCodexResponsesProvider::new(
        Some(server.uri()),
        vec![model(None)],
        Arc::new(HttpClient::new()),
    );
    let mut events = Vec::new();
    let mut stream = provider.stream_prepared(
        request_with(vec![grammar_tool()]),
        auth("k", None, Some("acct".into())),
    );
    use futures_util::StreamExt;
    while let Some(item) = stream.next().await {
        match item {
            Ok(event) => events.push(event),
            Err(_) => break,
        }
    }
    let sent = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
    assert_eq!(body["tools"][0]["type"], serde_json::json!("function"));
    assert!(body["tools"][0].get("format").is_none());
    let done = events
        .iter()
        .find_map(|event| match event {
            AssistantStreamEvent::Done { message, .. } => Some(message.clone()),
            _ => None,
        })
        .expect("stream completes");
    assert!(
        done.diagnostics.iter().any(|diagnostic| diagnostic.kind
            == "constrained_sampling_fallback"
            && diagnostic.reason == "grammar_target_unsupported"
            && diagnostic.detail.as_deref() == Some("grammar-tool")),
        "the unsupported grammar target records its loss on the message"
    );
}
