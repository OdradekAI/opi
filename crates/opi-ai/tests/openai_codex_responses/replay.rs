use super::*;
use opi_ai::message::{OutputContent, ToolCall, ToolResultMessage};

fn provider() -> OpenAiCodexResponsesProvider {
    OpenAiCodexResponsesProvider::new(None, vec![model(None)], Arc::new(HttpClient::new()))
}

fn assistant() -> AssistantMessage {
    AssistantMessage {
        content: vec![AssistantContent::ToolCall {
            tool_call: ToolCall {
                id: "call_codex|fc_codex".into(),
                name: "read".into(),
                arguments: r#"{"path":"a.rs"}"#.into(),
                arguments_error: None,
                signature: None,
            },
        }],
        api: opi_ai::ApiKind::OpenAi,
        provider: "openai-codex".into(),
        model: "gpt-5.4".into(),
        response_model: Some("reported-gpt-5.4-revision".into()),
        wire: Some(WireApi::OpenAiCodexResponses),
        response_id: Some("response-codex".into()),
        usage: Usage::unknown(),
        stop_reason: StopReason::ToolUse,
        error_message: None,
        timestamp_ms: 0,
        diagnostics: Vec::new(),
        duration_ms: None,
        provider_thinking_level: None,
    }
}

fn result(is_error: bool) -> ToolResultMessage {
    ToolResultMessage {
        tool_call_id: "call_codex|fc_codex".into(),
        tool_name: "read".into(),
        content: vec![OutputContent::Text {
            text: "result payload".into(),
        }],
        details: Some(serde_json::json!({"private": "detail-canary"})),
        is_error,
        not_executed: None,
        truncated: false,
        timestamp_ms: 0,
    }
}

fn replay_request(assistant: AssistantMessage, result: Option<ToolResultMessage>) -> Request {
    let mut request = request();
    let mut messages = vec![
        Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "read a.rs".into(),
            }],
            timestamp_ms: 0,
        }),
        Message::Assistant(assistant),
    ];
    if let Some(result) = result {
        messages.push(Message::ToolResult(result));
    }
    messages.push(Message::User(UserMessage {
        content: vec![InputContent::Text {
            text: "continue".into(),
        }],
        timestamp_ms: 0,
    }));
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: Some("system prompt".into()),
        messages,
        tools: vec![],
    })
    .expect("valid Codex replay context");
    request
}

fn item<'a>(body: &'a serde_json::Value, kind: &str) -> &'a serde_json::Value {
    let matching: Vec<_> = body["input"]
        .as_array()
        .expect("Codex input items")
        .iter()
        .filter(|item| item["type"] == kind)
        .collect();
    assert_eq!(matching.len(), 1, "one {kind} item: {body}");
    matching[0]
}

#[test]
fn codex_replay_restores_dual_ids_only_for_exact_continuation_identity() {
    let provider = provider();
    for (label, provider_id, wire, model_id, same) in [
        (
            "same requested identity with different reported model",
            "openai-codex",
            Some(WireApi::OpenAiCodexResponses),
            "gpt-5.4",
            true,
        ),
        (
            "foreign provider",
            "openai-responses",
            Some(WireApi::OpenAiCodexResponses),
            "gpt-5.4",
            false,
        ),
        (
            "ordinary Responses is a different wire",
            "openai-codex",
            Some(WireApi::OpenAiResponses),
            "gpt-5.4",
            false,
        ),
        (
            "foreign requested model",
            "openai-codex",
            Some(WireApi::OpenAiCodexResponses),
            "other-model",
            false,
        ),
        (
            "legacy unknown wire",
            "openai-codex",
            None,
            "gpt-5.4",
            false,
        ),
    ] {
        let mut assistant = assistant();
        assistant.provider = provider_id.into();
        assistant.wire = wire;
        assistant.model = model_id.into();
        let request = replay_request(assistant, Some(result(false)));
        let canonical = serde_json::to_vec(&request.context).unwrap();
        let body = provider.build_request_body(&request).unwrap();
        let call = item(&body, "function_call");
        assert_eq!(call["call_id"], "call_codex", "{label}");
        assert_eq!(call["name"], "read", "{label}");
        assert_eq!(call["arguments"], r#"{"path":"a.rs"}"#, "{label}");
        if same {
            assert_eq!(call["id"], "fc_codex", "{label}");
        } else {
            assert!(call.get("id").is_none(), "{label}: {call}");
        }
        let output = item(&body, "function_call_output");
        assert_eq!(output["call_id"], "call_codex", "{label}");
        assert_eq!(output["output"], "result payload", "{label}");
        assert_eq!(provider.build_request_body(&request).unwrap(), body);
        assert_eq!(serde_json::to_vec(&request.context).unwrap(), canonical);
    }
}

#[test]
fn codex_replay_preserves_success_and_marks_tool_error_results() {
    for (is_error, expected) in [
        (false, "result payload"),
        (true, "[tool_error] result payload"),
    ] {
        let request = replay_request(assistant(), Some(result(is_error)));
        let canonical = serde_json::to_vec(&request.context).unwrap();
        let body = provider().build_request_body(&request).unwrap();
        let output = item(&body, "function_call_output");
        assert_eq!(output["call_id"], "call_codex");
        assert_eq!(output["output"], expected);
        assert!(output.get("status").is_none());
        assert!(!body.to_string().contains("detail-canary"));
        assert_eq!(serde_json::to_vec(&request.context).unwrap(), canonical);
    }
}

#[test]
fn codex_replay_keeps_malformed_string_arguments_without_fabricating_an_object() {
    let mut assistant = assistant();
    let AssistantContent::ToolCall { tool_call } = &mut assistant.content[0] else {
        panic!("tool-call fixture");
    };
    tool_call.arguments = r#"{"path":"unfinished"#.into();
    tool_call.arguments_error = Some("incomplete JSON object".into());
    let request = replay_request(assistant, Some(result(true)));
    let canonical = serde_json::to_vec(&request.context).unwrap();
    let body = provider().build_request_body(&request).unwrap();

    assert_eq!(
        item(&body, "function_call")["arguments"],
        r#"{"path":"unfinished"#
    );
    assert_eq!(
        item(&body, "function_call_output")["output"],
        "[tool_error] result payload"
    );
    assert!(!body.to_string().contains("incomplete JSON object"));
    assert_eq!(serde_json::to_vec(&request.context).unwrap(), canonical);
}

#[test]
fn codex_replay_closes_missing_results_with_unknown_effect_without_rewriting_history() {
    let request = replay_request(assistant(), None);
    let canonical = serde_json::to_vec(&request.context).unwrap();
    let body = provider().build_request_body(&request).unwrap();
    assert_eq!(item(&body, "function_call")["call_id"], "call_codex");
    let output = item(&body, "function_call_output");
    assert_eq!(output["call_id"], "call_codex");
    let text = output["output"].as_str().expect("text result");
    assert!(text.starts_with("[tool_error] Result unavailable:"));
    assert!(text.contains("execution effect is unknown"));
    assert!(!text.contains("was not executed"));
    assert_eq!(serde_json::to_vec(&request.context).unwrap(), canonical);
    assert!(
        request
            .context
            .messages()
            .iter()
            .all(|message| !matches!(message, Message::ToolResult(_)))
    );
}

#[test]
fn codex_replay_rejects_ambiguous_call_batches_instead_of_synthesizing_results() {
    let mut assistant = assistant();
    assistant.content.push(assistant.content[0].clone());
    let request = replay_request(assistant, None);
    let canonical = serde_json::to_vec(&request.context).unwrap();
    let error = provider()
        .build_request_body(&request)
        .expect_err("duplicate call IDs are ambiguous");

    assert!(matches!(&error, ProviderError::Config(_)), "{error:?}");
    assert!(error.to_string().contains("duplicate tool call id"));
    assert_eq!(serde_json::to_vec(&request.context).unwrap(), canonical);
}
