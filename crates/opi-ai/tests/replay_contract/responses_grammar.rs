//! Replay Responses grammar calls and outputs with their custom tool item IDs.

use opi_ai::constrained_sampling::ConstrainedSampling;
use opi_ai::message::{AssistantContent, Message, ToolDef};
use opi_ai::provider::Request;
use opi_ai::providers::openai_responses::{OpenAiResponsesProvider, ResponsesConfig};
use opi_ai::stream::StopReason;
use serde_json::json;

use super::fixtures::{assistant, call, request, result, user};

fn grammar_tool() -> ToolDef {
    ToolDef {
        name: "grammar-tool".into(),
        description: "grammar constrained".into(),
        input_schema: json!({
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

fn grammar_request(messages: Vec<Message>) -> Request {
    let mut request = request("openai-responses:gpt-4o", messages);
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: request
            .context
            .messages()
            .iter()
            .filter(|message| !matches!(message, opi_ai::message::Message::System(_)))
            .cloned()
            .collect(),
        tools: vec![grammar_tool()],
    })
    .expect("valid context declarations");
    request
}

#[test]
fn grammar_call_and_result_replay_as_custom_items_with_ctc_ids() {
    let mut same_identity_call = call("call_g|ctc_1", "grammar-tool");
    if let AssistantContent::ToolCall { tool_call } = &mut same_identity_call {
        tool_call.arguments = json!({"input": "raw input"}).to_string();
    }
    let mut assistant_message = assistant(vec![same_identity_call], StopReason::ToolUse);
    if let Message::Assistant(inner) = &mut assistant_message {
        inner.provider = "openai-responses".into();
        inner.model = "gpt-4o".into();
        inner.wire = Some(opi_ai::model_info::WireApi::OpenAiResponses);
    }
    let messages = vec![
        user("q"),
        assistant_message,
        result("call_g|ctc_1", "grammar-tool", "raw grammar output"),
    ];
    let provider = OpenAiResponsesProvider::new_with_config(
        None,
        ResponsesConfig {
            source: opi_ai::model_info::OpenAiResponsesSourceCompat {
                supports_grammar_tools: Some(true),
                ..Default::default()
            },
            ..Default::default()
        },
    );
    let body = provider
        .build_request_body(&grammar_request(messages))
        .expect("grammar replay body builds");
    let serialized = body.to_string();
    assert!(
        serialized.contains("\"type\":\"custom_tool_call\""),
        "the grammar call replays as a custom call item: {serialized}"
    );
    assert!(
        serialized.contains("\"id\":\"ctc_1\""),
        "the packed ctc_ item id restores on replay: {serialized}"
    );
    assert!(
        serialized.contains("\"type\":\"custom_tool_call_output\""),
        "the grammar result replays as a custom output item: {serialized}"
    );
    assert!(
        !serialized.contains("\"type\":\"function_call\""),
        "grammar tools never replay as function items: {serialized}"
    );
}
