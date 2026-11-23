//! Project malformed proposals through string and object argument wires.

use opi_ai::message::{AssistantContent, Message, ToolCall, ToolNotExecutedReason};
use opi_ai::provider::ProviderError;
use opi_ai::providers::openai::OpenAiChatProvider;
use opi_ai::stream::StopReason;
use serde_json::json;

use super::fixtures::{anthropic, assistant, call, not_executed_result, request, result, user};

/// String wires keep raw malformed arguments. Object wires use explicit failure context.
#[test]
fn malformed_proposals_split_by_wire_argument_shape() {
    let malformed = || AssistantContent::ToolCall {
        tool_call: ToolCall {
            id: "tc_bad".into(),
            name: "read".into(),
            arguments: "{\"broken".into(),
            arguments_error: Some("not a complete JSON object".into()),
            signature: None,
        },
    };
    let history = |mut a: Message, result_text: &str| {
        if let Message::Assistant(am) = &mut a {
            am.api = opi_ai::ApiKind::OpenAi;
            am.provider = "openai".into();
            am.model = "gpt-4o".into();
            am.wire = Some(opi_ai::model_info::WireApi::OpenAiCompletions);
        }
        vec![user("q"), a, result("tc_bad", "read", result_text)]
    };

    // Chat: raw malformed text on the wire.
    let chat = OpenAiChatProvider::new(None);
    let body = chat
        .build_request_body(&request(
            "openai:gpt-4o",
            history(
                assistant(vec![malformed()], StopReason::ToolUse),
                "error result",
            ),
        ))
        .unwrap();
    let serialized = body.to_string();
    assert!(serialized.contains("broken"), "raw text preserved");
    assert!(
        !serialized.contains("\"arguments\":\"{}\""),
        "no fabricated empty object"
    );

    // Anthropic (object wire): explicit failure context replaces the pair.
    let anthropic_history = vec![
        user("q"),
        assistant(vec![malformed()], StopReason::ToolUse),
        not_executed_result("tc_bad", "read", ToolNotExecutedReason::InvalidJson),
    ];
    let body = anthropic()
        .build_request_body(&request(
            "anthropic:claude-sonnet-4-5-20250514",
            anthropic_history,
        ))
        .unwrap();
    let serialized = body.to_string();
    assert!(serialized.contains("[opi:tool-call-not-executed:v1]"));
    assert!(!serialized.contains("broken"));
    assert!(
        !serialized.contains("tool_result"),
        "linked result consumed by the context"
    );
}

#[test]
fn malformed_object_wire_does_not_infer_non_execution_from_error_text() {
    let malformed = AssistantContent::ToolCall {
        tool_call: ToolCall {
            id: "malformed".into(),
            name: "read".into(),
            arguments: "[1]".into(),
            arguments_error: Some("sensitive parse diagnostic".into()),
            signature: None,
        },
    };
    let mut recorded = result("malformed", "read", "was not executed");
    if let Message::ToolResult(result) = &mut recorded {
        result.is_error = true;
    }
    let error = anthropic()
        .build_request_body(&request(
            "anthropic:claude-sonnet-4-5-20250514",
            vec![
                user("q"),
                assistant(vec![malformed], StopReason::ToolUse),
                recorded,
            ],
        ))
        .expect_err("ordinary error text cannot supply a non-execution fact");
    assert!(matches!(error, ProviderError::Config(_)));
}

#[test]
fn malformed_non_execution_context_is_one_sanitized_user_block_after_serde() {
    let malformed = |id: &str, arguments: &str| AssistantContent::ToolCall {
        tool_call: ToolCall {
            id: id.into(),
            name: "read\"\n".into(),
            arguments: arguments.into(),
            arguments_error: Some("private-error-canary".into()),
            signature: None,
        },
    };
    let req = request(
        "anthropic:claude-sonnet-4-5-20250514",
        vec![
            user("canonical input"),
            assistant(
                vec![
                    malformed("private-id-canary-a", "{\"private-argument-canary"),
                    call("valid", "read"),
                    malformed("private-id-canary-b", "[1]"),
                ],
                StopReason::ToolUse,
            ),
            not_executed_result(
                "private-id-canary-a",
                "read\"\n",
                ToolNotExecutedReason::InvalidJson,
            ),
            result("valid", "read", "real recorded result"),
            not_executed_result(
                "private-id-canary-b",
                "read\"\n",
                ToolNotExecutedReason::NonObjectArguments,
            ),
        ],
    );
    let encoded = serde_json::to_string(&req.context).unwrap();
    let mut restored = req.clone();
    restored.context = serde_json::from_str(&encoded).unwrap();
    let body = anthropic().build_request_body(&restored).unwrap();
    let encoded_body = body.to_string();
    for private in [
        "private-error-canary",
        "private-argument-canary",
        "private-id-canary",
        "private execution-boundary diagnostic",
    ] {
        assert!(!encoded_body.contains(private), "{private}");
    }
    let contexts: Vec<_> = body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|message| {
            message["content"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(move |block| {
                    block["text"]
                        .as_str()
                        .filter(|text| text.starts_with("[opi:tool-call-not-executed:v1]"))
                        .map(|text| (&message["role"], text))
                })
        })
        .collect();
    assert_eq!(contexts.len(), 1);
    assert_eq!(contexts[0].0, "user");
    let payload: serde_json::Value =
        serde_json::from_str(contexts[0].1.split_once(' ').unwrap().1).unwrap();
    assert_eq!(
        payload,
        json!({"calls": [
            {"batch_call":0,"tool_name":"read\"\n","reason_code":"invalid_json"},
            {"batch_call":2,"tool_name":"read\"\n","reason_code":"non_object_arguments"}
        ]})
    );
    assert!(encoded_body.contains("real recorded result"));
    assert!(encoded_body.contains("tool_use_id"));
    assert_eq!(serde_json::to_string(&req.context).unwrap(), encoded);
}
