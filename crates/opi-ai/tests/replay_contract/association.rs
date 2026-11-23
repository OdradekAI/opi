//! Validate call/result association during request preparation and wire serialization.

use opi_ai::message::{AssistantContent, ToolNotExecutedReason};
use opi_ai::provider::ProviderError;
use opi_ai::providers::openai::OpenAiChatProvider;
use opi_ai::stream::StopReason;
use opi_ai::test_support::resolved_auth;

use super::fixtures::{anthropic, assistant, call, not_executed_result, request, result, user};

/// Closed batches can reuse a source ID. Association errors surface at body construction.
#[test]
fn association_matrix_through_real_adapters() {
    // Reused IDs across closed batches serialize fine on every family.
    let messages = vec![
        user("q"),
        assistant(vec![call("fc_0", "read")], StopReason::ToolUse),
        result("fc_0", "read", "one"),
        assistant(vec![call("fc_0", "read")], StopReason::ToolUse),
        result("fc_0", "read", "two"),
    ];
    let anthropic_body = anthropic()
        .build_request_body(&request(
            "anthropic:claude-sonnet-4-5-20250514",
            messages.clone(),
        ))
        .unwrap();
    let chat_body = OpenAiChatProvider::new(None)
        .build_request_body(&request("openai:gpt-4o", messages))
        .unwrap();
    let anthropic_messages = anthropic_body["messages"].as_array().unwrap();
    let chat_messages = chat_body["messages"].as_array().unwrap();
    assert_eq!(anthropic_messages.len(), 5);
    assert_eq!(chat_messages.len(), 5);
    for (batch, output) in ["one", "two"].into_iter().enumerate() {
        let call_index = 1 + 2 * batch;
        let result_index = call_index + 1;
        let anthropic_call = &anthropic_messages[call_index]["content"][0];
        let anthropic_result = &anthropic_messages[result_index]["content"][0];
        assert_eq!(anthropic_call["type"], "tool_use");
        assert_eq!(anthropic_call["name"], "read");
        assert_eq!(anthropic_result["type"], "tool_result");
        assert_eq!(
            anthropic_result["tool_use_id"],
            anthropic_call["id"].as_str().unwrap()
        );
        assert_eq!(anthropic_result["content"], output);

        let chat_call = &chat_messages[call_index]["tool_calls"][0];
        let chat_result = &chat_messages[result_index];
        assert_eq!(chat_call["function"]["name"], "read");
        assert_eq!(chat_result["role"], "tool");
        assert_eq!(
            chat_result["tool_call_id"],
            chat_call["id"].as_str().unwrap()
        );
        assert_eq!(chat_result["content"], output);
    }

    // Duplicate call ID inside one batch fails typed.
    let error = anthropic()
        .build_request_body(&request(
            "anthropic:claude-sonnet-4-5-20250514",
            vec![
                user("q"),
                assistant(
                    vec![call("x", "read"), call("x", "bash")],
                    StopReason::ToolUse,
                ),
            ],
        ))
        .unwrap_err();
    assert!(error.to_string().contains("duplicate tool call id"));

    // Duplicate result for one call fails typed.
    let error = anthropic()
        .build_request_body(&request(
            "anthropic:claude-sonnet-4-5-20250514",
            vec![
                user("q"),
                assistant(vec![call("x", "read")], StopReason::ToolUse),
                result("x", "read", "one"),
                result("x", "read", "two"),
            ],
        ))
        .unwrap_err();
    assert!(error.to_string().contains("duplicate tool result"));

    // Orphan result fails typed.
    let error = anthropic()
        .build_request_body(&request(
            "anthropic:claude-sonnet-4-5-20250514",
            vec![user("q"), result("nope", "read", "one")],
        ))
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("does not belong to any open tool-call batch")
    );

    // Result/call name mismatch fails typed.
    let error = anthropic()
        .build_request_body(&request(
            "anthropic:claude-sonnet-4-5-20250514",
            vec![
                user("q"),
                assistant(vec![call("x", "read")], StopReason::ToolUse),
                result("x", "bash", "one"),
            ],
        ))
        .unwrap_err();
    assert!(error.to_string().contains("names"));

    // The typed failure is a ProviderError::Config, not a panic: the
    // request never reaches authorization or the network.
    assert!(matches!(error, ProviderError::Config(_)));
}

#[test]
fn omitted_batches_still_validate_association() {
    for stop_reason in [StopReason::Error, StopReason::Aborted, StopReason::ToolUse] {
        let proposal = || {
            let mut proposal = call("x", "read");
            if stop_reason == StopReason::ToolUse
                && let AssistantContent::ToolCall { tool_call } = &mut proposal
            {
                tool_call.arguments = "[1]".into();
                tool_call.arguments_error = Some("non-object".into());
            }
            proposal
        };
        let trusted = || not_executed_result("x", "read", ToolNotExecutedReason::InvalidJson);
        for (calls, results, expected) in [
            (
                vec![proposal(), proposal()],
                vec![],
                "duplicate tool call id",
            ),
            (
                vec![proposal()],
                vec![trusted(), trusted()],
                "duplicate tool result",
            ),
            (
                vec![proposal()],
                vec![not_executed_result(
                    "x",
                    "other",
                    ToolNotExecutedReason::InvalidJson,
                )],
                "names",
            ),
            (
                vec![proposal()],
                vec![not_executed_result(
                    "orphan",
                    "read",
                    ToolNotExecutedReason::InvalidJson,
                )],
                "does not belong",
            ),
        ] {
            let mut messages = vec![user("q"), assistant(calls, stop_reason)];
            messages.extend(results);
            let error = anthropic()
                .build_request_body(&request("anthropic:claude-sonnet-4-5-20250514", messages))
                .expect_err("omission cannot bypass association checks");
            assert!(matches!(error, ProviderError::Config(_)));
            assert!(
                error.to_string().contains(expected),
                "{stop_reason:?}: {error}"
            );
        }
    }
}

/// The prepared stream delivers one typed association failure followed by EOF.
#[tokio::test]
async fn association_failure_surfaces_typed_before_network() {
    use futures_util::StreamExt;
    let provider = anthropic();
    let request = request(
        "anthropic:claude-sonnet-4-5-20250514",
        vec![user("q"), result("orphan", "read", "x")],
    );
    use opi_ai::provider::Provider;
    let mut stream = provider.stream_prepared(request, resolved_auth());
    let event = stream.next().await.unwrap();
    match event {
        Err(ProviderError::Config(summary)) => {
            let text = summary.to_string();
            assert!(text.contains("does not belong to any open tool-call batch"));
        }
        other => panic!("expected typed association failure, got {other:?}"),
    }
    // No further events: nothing was dispatched.
    assert!(stream.next().await.is_none());
}
