//! Replay through the actual Gemini and Bedrock object-argument serializers.

use super::{assistant, call, not_executed_result, request, result, user};
use opi_ai::bedrock::BedrockProvider;
use opi_ai::gemini::GeminiProvider;
use opi_ai::message::{AssistantContent, Message, ToolCall, ToolNotExecutedReason};
use opi_ai::provider::{ProviderError, Request};
use opi_ai::stream::StopReason;
use serde_json::{Value, json};
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
enum ObjectWire {
    Gemini,
    Bedrock,
}

impl ObjectWire {
    fn model(self) -> &'static str {
        match self {
            Self::Gemini => "gemini:gemini-2.5-flash",
            Self::Bedrock => "bedrock:anthropic.claude-sonnet-4-5-20250514-v1:0",
        }
    }

    fn build(self, request: &Request) -> Result<Value, ProviderError> {
        match self {
            Self::Gemini => GeminiProvider::new(None).build_request_body(request),
            Self::Bedrock => BedrockProvider::new(None, Arc::new(opi_ai::http::HttpClient::new()))
                .build_converse_body(request),
        }
    }

    fn blocks(self, body: &Value) -> Vec<(&str, &Value)> {
        let (messages, content) = match self {
            Self::Gemini => ("contents", "parts"),
            Self::Bedrock => ("messages", "content"),
        };
        body[messages]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|message| {
                message[content]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(move |block| (message["role"].as_str().unwrap(), block))
            })
            .collect()
    }
}

fn invalid_proposal(arguments: &str, marked: bool) -> AssistantContent {
    AssistantContent::ToolCall {
        tool_call: ToolCall {
            id: "private-call-id-canary".into(),
            name: "invalid_read".into(),
            arguments: arguments.into(),
            arguments_error: marked.then(|| "private-parse-diagnostic-canary".into()),
        },
    }
}

#[test]
fn object_wire_consumers_sanitize_invalid_proposals_and_preserve_valid_pairs() {
    for wire in [ObjectWire::Gemini, ObjectWire::Bedrock] {
        for (arguments, reason, reason_code) in [
            (
                r#"{"private-argument-canary""#,
                ToolNotExecutedReason::InvalidJson,
                "invalid_json",
            ),
            (
                r#"["private-argument-canary"]"#,
                ToolNotExecutedReason::NonObjectArguments,
                "non_object_arguments",
            ),
        ] {
            // Legacy histories may lack arguments_error; the actual consumer
            // must still reject an unrepresentable JSON value as a tool call.
            for marked in [false, true] {
                let mut invalid_result =
                    not_executed_result("private-call-id-canary", "invalid_read", reason);
                if let Message::ToolResult(result) = &mut invalid_result {
                    result.details = Some(json!({"diagnostic": "private-result-details-canary"}));
                }
                let req = request(
                    wire.model(),
                    vec![
                        user("question"),
                        assistant(
                            vec![invalid_proposal(arguments, marked), call("valid", "read")],
                            StopReason::ToolUse,
                        ),
                        invalid_result,
                        result("valid", "read", "real recorded result"),
                    ],
                );
                let canonical = serde_json::to_value(&req.context).unwrap();
                let body = wire.build(&req).unwrap();
                let blocks = wire.blocks(&body);
                let contexts: Vec<_> = blocks
                    .iter()
                    .filter_map(|(role, block)| {
                        block["text"]
                            .as_str()
                            .and_then(|text| text.strip_prefix("[opi:tool-call-not-executed:v1] "))
                            .map(|payload| (*role, serde_json::from_str::<Value>(payload).unwrap()))
                    })
                    .collect();
                assert_eq!(
                    contexts,
                    [(
                        "user",
                        json!({"calls": [{
                            "batch_call": 0,
                            "tool_name": "invalid_read",
                            "reason_code": reason_code,
                        }]})
                    )],
                    "{wire:?}/{reason_code}/marked={marked}: one sanitized failure context"
                );

                let (call_key, result_key, expected_call, expected_result) = match wire {
                    ObjectWire::Gemini => (
                        "functionCall",
                        "functionResponse",
                        json!({"name": "read", "args": {"path": "a.rs"}}),
                        json!({"id": "valid", "name": "read", "response": {
                            "content": "real recorded result",
                        }}),
                    ),
                    ObjectWire::Bedrock => (
                        "toolUse",
                        "toolResult",
                        json!({"toolUseId": "valid", "name": "read", "input": {"path": "a.rs"}}),
                        json!({"toolUseId": "valid", "status": "success", "content": [
                            {"text": "real recorded result"}
                        ]}),
                    ),
                };
                let calls: Vec<_> = blocks
                    .iter()
                    .enumerate()
                    .filter_map(|(index, (role, block))| {
                        block.get(call_key).map(|call| (index, *role, call))
                    })
                    .collect();
                let results: Vec<_> = blocks
                    .iter()
                    .enumerate()
                    .filter_map(|(index, (role, block))| {
                        block.get(result_key).map(|result| (index, *role, result))
                    })
                    .collect();
                assert_eq!(calls.len(), 1, "{wire:?}: no fabricated call");
                assert_eq!(results.len(), 1, "{wire:?}: invalid result consumed");
                assert_eq!(
                    calls[0].2, &expected_call,
                    "{wire:?}: no fabricated empty-object call"
                );
                assert_eq!(
                    results[0].2, &expected_result,
                    "{wire:?}: invalid linked result is consumed"
                );
                assert_eq!(
                    calls[0].1,
                    match wire {
                        ObjectWire::Gemini => "model",
                        ObjectWire::Bedrock => "assistant",
                    }
                );
                assert_eq!(results[0].1, "user");
                assert!(calls[0].0 < results[0].0, "call must precede its result");
                let encoded = body.to_string();
                for private in [
                    "private-call-id-canary",
                    "private-argument-canary",
                    "private-parse-diagnostic-canary",
                    "private execution-boundary diagnostic",
                    "private-result-details-canary",
                ] {
                    assert!(!encoded.contains(private), "{wire:?}: leaked {private}");
                }
                assert_eq!(serde_json::to_value(&req.context).unwrap(), canonical);
            }
        }
    }
}

#[test]
fn object_wire_consumers_require_trusted_non_execution_facts() {
    for wire in [ObjectWire::Gemini, ObjectWire::Bedrock] {
        for arguments in [
            r#"{"private-argument-canary""#,
            r#"["private-argument-canary"]"#,
        ] {
            let mut recorded = result("private-call-id-canary", "invalid_read", "was not executed");
            if let Message::ToolResult(result) = &mut recorded {
                result.is_error = true;
            }
            let req = request(
                wire.model(),
                vec![
                    user("question"),
                    assistant(
                        vec![invalid_proposal(arguments, false)],
                        StopReason::ToolUse,
                    ),
                    recorded,
                ],
            );
            let canonical = serde_json::to_value(&req.context).unwrap();
            let error = wire
                .build(&req)
                .expect_err("error text cannot authorize omission or a fabricated successful call");
            assert!(
                matches!(error, ProviderError::Config(_)),
                "{wire:?}: {error:?}"
            );
            assert!(
                error
                    .to_string()
                    .contains("trusted non-execution error fact")
            );
            assert_eq!(serde_json::to_value(&req.context).unwrap(), canonical);
        }
    }
}

#[test]
fn bedrock_wire_id_mapping_is_bijective_synchronized_and_request_local() {
    let provider = BedrockProvider::new(None, Arc::new(opi_ai::http::HttpClient::new()));
    let probe = request(
        ObjectWire::Bedrock.model(),
        vec![
            user("question"),
            assistant(vec![call("x/y", "read")], StopReason::ToolUse),
            result("x/y", "read", "probe result"),
        ],
    );
    let probe_body = provider.build_converse_body(&probe).unwrap();
    let generated_alias = ObjectWire::Bedrock
        .blocks(&probe_body)
        .into_iter()
        .find_map(|(_, block)| block.pointer("/toolUse/toolUseId").and_then(Value::as_str))
        .unwrap()
        .to_owned();
    assert_ne!(generated_alias, "x/y");
    assert!(!generated_alias.is_empty() && generated_alias.len() <= 64);
    assert!(
        generated_alias
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    );
    let legal_source = if generated_alias == "legal_read" {
        "legal_read_other"
    } else {
        "legal_read"
    };
    let long_id = "long-call-".repeat(10);
    // Use a public result to construct a collision, without depending on
    // the private hash algorithm. Uniqueness below is an independent oracle.
    let source_ids = [
        "x/y",
        generated_alias.as_str(),
        long_id.as_str(),
        legal_source,
    ];
    let mut messages = vec![
        user("question"),
        assistant(
            source_ids.iter().map(|id| call(id, "read")).collect(),
            StopReason::ToolUse,
        ),
    ];
    for (index, id) in source_ids.iter().enumerate() {
        messages.push(result(id, "read", &format!("recorded-result-{index}")));
    }
    let req = request(ObjectWire::Bedrock.model(), messages);
    let canonical = serde_json::to_value(&req.context).unwrap();
    let body = provider.build_converse_body(&req).unwrap();
    assert_eq!(
        body,
        provider.build_converse_body(&req).unwrap(),
        "stable retries"
    );
    let blocks = ObjectWire::Bedrock.blocks(&body);
    let calls: Vec<_> = blocks
        .iter()
        .enumerate()
        .filter_map(|(index, (role, block))| block.get("toolUse").map(|call| (index, *role, call)))
        .collect();
    let results: Vec<_> = blocks
        .iter()
        .enumerate()
        .filter_map(|(index, (role, block))| {
            block.get("toolResult").map(|result| (index, *role, result))
        })
        .collect();
    assert_eq!(calls.len(), source_ids.len());
    assert_eq!(results.len(), source_ids.len());
    let ids: Vec<_> = calls
        .iter()
        .map(|(_, _, call)| call["toolUseId"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids.iter()
            .copied()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        source_ids.len()
    );
    assert_ne!(ids[0], source_ids[0], "illegal characters are legalized");
    assert_ne!(ids[2], source_ids[2], "long IDs are bounded");
    assert_eq!(
        ids[3], legal_source,
        "an available valid spelling is preserved"
    );
    for (index, id) in ids.iter().enumerate() {
        assert!(!id.is_empty() && id.len() <= 64);
        assert!(
            id.bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        );
        assert_eq!(calls[index].1, "assistant");
        assert_eq!(results[index].1, "user");
        assert!(
            calls[index].0 < results[index].0,
            "call precedes its result"
        );
        assert_eq!(calls[index].2["name"], "read");
        assert_eq!(calls[index].2["input"], json!({"path": "a.rs"}));
        assert_eq!(
            results[index].2["toolUseId"], *id,
            "each result stays paired"
        );
        assert_eq!(results[index].2["status"], "success");
        assert_eq!(
            results[index].2["content"],
            json!([{"text": format!("recorded-result-{index}")}])
        );
    }
    assert_eq!(serde_json::to_value(&req.context).unwrap(), canonical);
}
