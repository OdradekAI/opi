//! Keep known non-execution and unknown effects distinct across replay and storage.

use opi_ai::message::{AssistantContent, InputContent, Message, ToolNotExecutedReason};
use opi_ai::provider::ProviderError;
use opi_ai::providers::openai::OpenAiChatProvider;
use opi_ai::stream::StopReason;
use serde_json::json;

use super::fixtures::{anthropic, assistant, call, not_executed_result, request, result, user};

/// Close an unambiguous missing result in the request with an unknown execution effect.
/// The synthetic error leaves the canonical history intact.
#[test]
fn missing_results_close_request_locally_with_unknown_effect() {
    let messages = vec![
        user("q"),
        assistant(vec![call("tc_9", "read")], StopReason::ToolUse),
        user("next turn"),
    ];
    let canonical = serde_json::to_string(
        &opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: messages.clone(),
            tools: vec![],
        })
        .expect("valid context declarations"),
    )
    .unwrap();
    let body = anthropic()
        .build_request_body(&request(
            "anthropic:claude-sonnet-4-5-20250514",
            messages.clone(),
        ))
        .unwrap();

    let tool_results: Vec<&serde_json::Value> = body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|m| {
            m["content"]
                .as_array()
                .map(|a| a.iter())
                .into_iter()
                .flatten()
        })
        .filter(|block| block["type"] == "tool_result")
        .collect();
    assert_eq!(tool_results.len(), 1, "exactly one synthetic result");
    assert_eq!(tool_results[0]["is_error"], json!(true));
    let text = tool_results[0]["content"].as_str().unwrap();
    assert!(text.contains("Result unavailable"), "{text}");
    assert!(text.contains("execution effect is unknown"), "{text}");
    assert!(!text.to_lowercase().contains("was not executed"));

    // The canonical history is byte-identical after projection (the
    // projection cannot mutate a borrowed chronology).
    let replayed = serde_json::to_string(
        &opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages,
            tools: vec![],
        })
        .expect("valid context declarations"),
    )
    .unwrap();
    assert_eq!(canonical, replayed);
}

/// Omit failed turns with linked non-execution results and keep complete Length pairs.
#[test]
fn failed_turns_omit_with_linked_results_and_length_keeps_pairs() {
    for stop_reason in [StopReason::Error, StopReason::Aborted] {
        let messages = vec![
            user("q"),
            assistant(vec![call("f_1", "read")], stop_reason),
            not_executed_result(
                "f_1",
                "read",
                ToolNotExecutedReason::InterruptedBeforeExecution,
            ),
            assistant(
                vec![AssistantContent::Text {
                    text: "recovered".into(),
                    signature: None,
                }],
                StopReason::Stop,
            ),
        ];
        let body = anthropic()
            .build_request_body(&request("anthropic:claude-sonnet-4-5-20250514", messages))
            .unwrap();
        let serialized = body.to_string();
        assert!(
            !serialized.contains("f_1"),
            "{stop_reason:?}: failed turn and linked results omitted"
        );
        assert!(!serialized.contains("not executed record"));
        assert!(serialized.contains("recovered"));
    }

    let messages = vec![
        user("q"),
        assistant(vec![call("len_1", "read")], StopReason::Length),
        result("len_1", "read", "error result from the trusted path"),
    ];
    let body = anthropic()
        .build_request_body(&request("anthropic:claude-sonnet-4-5-20250514", messages))
        .unwrap();
    assert!(body.to_string().contains("len_1"), "Length pair kept");
}

/// A failed proposal's missing result stays visible as user context with unknown effect.
#[test]
fn failed_turn_missing_results_keep_unknown_effect_across_batch_boundaries() {
    use opi_ai::replay::{ReplayError, ReplayLoss, ReplayProjection, ReplayRoute, project_replay};
    use opi_ai::transcript::SystemMessage;

    let route = ReplayRoute {
        wire: opi_ai::model_info::WireApi::AnthropicMessages,
        provider: "anthropic".into(),
        model: "claude-sonnet-4-5-20250514".into(),
        supports_images: true,
    };
    for stop_reason in [StopReason::Error, StopReason::Aborted] {
        for boundary in ["end", "user", "assistant", "system", "partial"] {
            let mut missing = call("private-missing-id", "read");
            if let AssistantContent::ToolCall { tool_call } = &mut missing {
                tool_call.arguments = "private-raw-arguments".into();
                tool_call.arguments_error = Some("private-parse-error".into());
            }
            let mut calls = vec![missing];
            if boundary == "partial" {
                calls.insert(0, call("private-known-id", "read"));
            }
            let mut messages = vec![user("q"), assistant(calls, stop_reason)];
            if boundary == "partial" {
                messages.push(not_executed_result(
                    "private-known-id",
                    "read",
                    ToolNotExecutedReason::InterruptedBeforeExecution,
                ));
            }
            if matches!(boundary, "system" | "partial") {
                messages.push(Message::System(SystemMessage {
                    content: "updated instructions".into(),
                    ..Default::default()
                }));
            }
            match boundary {
                "user" | "partial" => messages.push(user("continue")),
                "assistant" => messages.push(assistant(
                    vec![AssistantContent::Text {
                        text: "recovered".into(),
                        signature: None,
                    }],
                    StopReason::Stop,
                )),
                _ => {}
            }
            let mut req = request("anthropic:claude-sonnet-4-5-20250514", messages);
            let canonical = serde_json::to_string(&req.context).unwrap();
            req.context = serde_json::from_str(&canonical).unwrap();
            let out = project_replay(
                req.context.messages(),
                &route,
                ReplayProjection::NativeOrCollapse,
            )
            .unwrap();
            let missing_ids: Vec<_> = out
                .facts
                .losses
                .iter()
                .filter_map(|loss| match loss {
                    ReplayLoss::SynthesizedMissingResult { tool_call_id } => {
                        Some(tool_call_id.as_str())
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(
                missing_ids,
                ["private-missing-id"],
                "{stop_reason:?}/{boundary}: only the missing result needs closure"
            );
            let failure_position = out
                .messages
                .iter()
                .position(|message| match message {
                    Message::User(user) => user.content.iter().any(|content| {
                        matches!(content, InputContent::Text { text } if text.contains("Result unavailable") && text.contains("execution effect is unknown"))
                    }),
                    _ => false,
                })
                .expect("failed missing result remains as ordinary User failure context");
            assert!(out.messages.iter().all(|message| {
                !matches!(message, Message::ToolResult(_))
                    && !matches!(message, Message::Assistant(assistant) if assistant.content.iter().any(|block| matches!(block, AssistantContent::ToolCall { .. })))
            }));
            if let Some(system_position) = out
                .messages
                .iter()
                .position(|message| matches!(message, Message::System(_)))
            {
                assert!(
                    failure_position < system_position,
                    "system stays batch-transparent"
                );
            }
            let ReplayError::StrictLoss(losses) =
                project_replay(req.context.messages(), &route, ReplayProjection::Strict)
                    .expect_err("Strict rejects both omission and missing-result synthesis")
            else {
                panic!("expected StrictLoss");
            };
            assert!(
                losses
                    .iter()
                    .any(|loss| matches!(loss, ReplayLoss::OmittedFailedAssistant { .. }))
            );
            assert_eq!(
                losses.iter().filter(|loss| matches!(loss, ReplayLoss::SynthesizedMissingResult { tool_call_id } if tool_call_id == "private-missing-id")).count(),
                1
            );
            let anthropic_body = anthropic().build_request_body(&req).unwrap();
            let chat_body = OpenAiChatProvider::new(None)
                .build_request_body(&req)
                .unwrap();
            for body in [anthropic_body, chat_body] {
                let serialized = body.to_string();
                assert!(serialized.contains("Result unavailable"));
                assert!(serialized.contains("execution effect is unknown"));
                for private in [
                    "private-missing-id",
                    "private-known-id",
                    "private-raw-arguments",
                    "private-parse-error",
                    "private execution-boundary diagnostic",
                    "tool_use",
                    "tool_calls",
                    "not-executed",
                    "not executed",
                ] {
                    assert!(
                        !serialized.contains(private),
                        "{stop_reason:?}/{boundary}: {private}"
                    );
                }
            }
            assert_eq!(serde_json::to_string(&req.context).unwrap(), canonical);
        }
    }
}

#[test]
fn failed_turns_reject_results_without_non_execution_facts() {
    for stop_reason in [StopReason::Error, StopReason::Aborted] {
        for (is_error, not_executed) in [
            (false, None),
            (true, None),
            (
                false,
                Some(ToolNotExecutedReason::InterruptedBeforeExecution),
            ),
        ] {
            let mut recorded = result("failed", "read", "tool changed external state");
            if let Message::ToolResult(result) = &mut recorded {
                result.is_error = is_error;
                result.not_executed = not_executed;
            }
            let error = anthropic()
                .build_request_body(&request(
                    "anthropic:claude-sonnet-4-5-20250514",
                    vec![
                        user("q"),
                        assistant(vec![call("failed", "read")], stop_reason),
                        recorded,
                    ],
                ))
                .expect_err("failed assistant projection must not erase possible effects");
            assert!(matches!(error, ProviderError::Config(_)));
        }
    }
}

#[test]
fn malformed_missing_result_keeps_unknown_effect_distinct_from_non_execution() {
    let mut malformed = call("missing", "read");
    if let AssistantContent::ToolCall { tool_call } = &mut malformed {
        tool_call.arguments = "null".into();
        tool_call.arguments_error = Some("private diagnostic".into());
    }
    let req = request(
        "anthropic:claude-sonnet-4-5-20250514",
        vec![user("q"), assistant(vec![malformed], StopReason::ToolUse)],
    );
    let body = anthropic().build_request_body(&req).unwrap().to_string();
    assert!(body.contains("Result unavailable"));
    assert!(body.contains("execution effect is unknown"));
    assert!(!body.contains("not-executed"));
    assert!(!body.contains("not executed"));
    assert!(!body.contains("private diagnostic"));
    assert!(!body.contains("tool_use"));
}

#[test]
fn legacy_result_serde_has_no_non_execution_fact() {
    let message = result("legacy", "read", "was not executed");
    let mut encoded = serde_json::to_value(message).unwrap();
    encoded.as_object_mut().unwrap().remove("not_executed");
    let Message::ToolResult(restored) = serde_json::from_value::<Message>(encoded).unwrap() else {
        panic!("tool result expected");
    };
    assert_eq!(restored.not_executed, None);
}
