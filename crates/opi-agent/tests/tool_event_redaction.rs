mod common;

use std::pin::Pin;
use std::sync::{Arc, Mutex};

use opi_agent::diagnostic::code::CODE_TOOL_EXECUTION_FAILED;
use opi_agent::event::{AgentEvent, AgentEventSink};
use opi_agent::hooks::{
    AgentHooks, BeforeToolCallContext, BeforeToolCallResult, ShouldStopAfterTurnContext,
};
use opi_agent::loop_types::{
    AgentError, AgentLoopConfig, AgentLoopContext, InferenceConfig, ModelSelection, NextTurnState,
};
use opi_agent::message::AgentMessage;
use opi_agent::tool::{ExecutionMode, Tool, ToolDiagnostic, ToolError, ToolResult, result};
use opi_ai::message::{
    AssistantContent, InputContent, Message, OutputContent, ToolCall, UserMessage,
};
use opi_ai::stream::AssistantStreamEvent;
use opi_ai::test_support::{self, MockProvider};
use serde_json::json;
use tokio_util::sync::CancellationToken;

struct SecretTool;

impl Tool for SecretTool {
    fn definition(&self) -> opi_ai::message::ToolDef {
        opi_ai::message::ToolDef {
            name: "bash".into(),
            description: "test tool".into(),
            input_schema: json!({"type":"object"}),
        }
    }

    fn execute(
        &self,
        _call_id: &str,
        _args: serde_json::Value,
        _signal: CancellationToken,
        _on_update: Option<opi_agent::tool::UpdateCallback>,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<ToolResult, ToolError>> + Send>> {
        Box::pin(async {
            let mut out = result::ok(
                vec![OutputContent::Text {
                    text: "VISIBLE_PROVIDER_TOOL_OUTPUT".into(),
                }],
                json!({
                    "command": "echo OPI_COMMAND_SECRET_CANARY",
                    "cwd": "C:\\Users\\private\\repo",
                    "exit_code": 1,
                    "timed_out": false,
                    "cancelled": false,
                    "truncated": false
                }),
            );
            out.is_error = true;
            out.diagnostics.push(ToolDiagnostic {
                code: CODE_TOOL_EXECUTION_FAILED.to_string(),
                message: "command exited non-zero".into(),
                context: json!({
                    "command": "echo OPI_COMMAND_SECRET_CANARY",
                    "exit_code": 1
                }),
            });
            Ok(out)
        })
    }

    fn execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Sequential
    }
}

struct AllowHooks;

impl AgentHooks for AllowHooks {
    fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
        Ok(messages
            .iter()
            .filter_map(|m| match m {
                AgentMessage::Llm(m) => Some(m.clone()),
                _ => None,
            })
            .collect())
    }

    fn should_stop_after_turn(
        &self,
        _: ShouldStopAfterTurnContext,
    ) -> Pin<Box<dyn std::future::Future<Output = bool> + Send>> {
        Box::pin(async { false })
    }

    fn before_tool_call(
        &self,
        _: BeforeToolCallContext,
    ) -> Pin<Box<dyn std::future::Future<Output = BeforeToolCallResult> + Send>> {
        Box::pin(async { BeforeToolCallResult::Continue })
    }
}

#[tokio::test]
async fn tool_events_redact_command_context_and_provider_content_stays_unchanged() {
    let first = test_support::tool_call_response(
        "tc1",
        "bash",
        r#"{"command":"echo OPI_COMMAND_SECRET_CANARY"}"#,
    );
    let second = test_support::text_response("done");
    let provider = MockProvider::new("mock", vec![first, second]);
    let call_log = provider.call_log_handle();

    let seen = Arc::new(Mutex::new(Vec::<AgentEvent>::new()));
    let seen_clone = seen.clone();
    let events: AgentEventSink = Box::new(move |event| {
        seen_clone.lock().unwrap().push(event);
    });

    let context = AgentLoopContext {
        collection: Arc::new(test_support::single_route_collection(Box::new(provider))),
        registry: common::test_registry(vec![Box::new(SecretTool)]),
        authorizer: Some(common::permissive_authorizer()),
        evidence_health: opi_agent::evidence::EvidenceHealth::healthy(),
        state: NextTurnState::new(
            vec![AgentMessage::Llm(Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "use bash".into(),
                }],
                timestamp_ms: 0,
            }))],
            ModelSelection::parse_spec("mock:mock-model").unwrap(),
            InferenceConfig::default(),
        ),
        system: None,
        steering_queue: None,
        follow_up_queue: None,
        diagnostic_sink: None,
        trace: None,
        session_id: None,
    };
    let messages = opi_agent::agent_loop(
        context,
        AgentLoopConfig {
            max_turns: 3,
            ..Default::default()
        },
        &AllowHooks,
        events,
        CancellationToken::new(),
    )
    .await
    .expect("agent loop should finish");

    let rendered_events = serde_json::to_string(&*seen.lock().unwrap()).unwrap();
    assert!(
        !rendered_events.contains("OPI_COMMAND_SECRET_CANARY"),
        "{rendered_events}"
    );

    let calls = call_log.lock().unwrap();
    let second_request = calls.get(1).expect("second provider request");
    let provider_tool_result = second_request
        .messages
        .iter()
        .find_map(|message| match message {
            Message::ToolResult(tool_result) => Some(tool_result),
            _ => None,
        })
        .expect("tool result sent back to provider");
    assert_eq!(
        provider_tool_result.content,
        vec![OutputContent::Text {
            text: "VISIBLE_PROVIDER_TOOL_OUTPUT".into()
        }]
    );

    let returned_tool_result = messages
        .context
        .iter()
        .find_map(|message| match message {
            AgentMessage::Llm(Message::ToolResult(tool_result)) => Some(tool_result),
            _ => None,
        })
        .expect("returned agent state keeps tool result");
    assert_eq!(
        returned_tool_result.content,
        vec![OutputContent::Text {
            text: "VISIBLE_PROVIDER_TOOL_OUTPUT".into()
        }]
    );
}

#[test]
fn tool_execution_update_redacts_args_and_partial_result() {
    let event = AgentEvent::ToolExecutionUpdate {
        tool_call_id: "tc-update".into(),
        tool_name: "bash".into(),
        args: json!({ "command": "echo OPI_UPDATE_COMMAND_SECRET_CANARY" }),
        partial_result: json!({ "stdout": "OPI_UPDATE_STDOUT_SECRET_CANARY" }),
    };

    let rendered = serde_json::to_string(&event.redacted_for_public()).unwrap();
    assert!(
        !rendered.contains("OPI_UPDATE_COMMAND_SECRET_CANARY"),
        "{rendered}"
    );
    assert!(
        !rendered.contains("OPI_UPDATE_STDOUT_SECRET_CANARY"),
        "{rendered}"
    );
}

#[test]
fn tool_execution_end_redacts_diagnostic_message_text() {
    let event = AgentEvent::ToolExecutionEnd {
        tool_call_id: "tc-end".into(),
        tool_name: "bash".into(),
        result: json!({"status": "failed"}),
        details: None,
        is_error: true,
        truncated: false,
        diagnostics: vec![ToolDiagnostic {
            code: CODE_TOOL_EXECUTION_FAILED.into(),
            message: "adapter failed at C:\\Users\\private\\repo with sk-proj-secret1234567890"
                .into(),
            context: json!({}),
        }],
    };

    let rendered = serde_json::to_string(&event.redacted_for_public()).unwrap();
    assert!(!rendered.contains("C:\\\\Users"), "{rendered}");
    assert!(!rendered.contains("sk-proj-secret1234567890"), "{rendered}");
    assert!(rendered.contains("[REDACTED]"), "{rendered}");
}

#[test]
fn public_message_events_redact_assistant_tool_call_arguments() {
    let mut assistant = test_support::base_assistant();
    assistant.content.push(AssistantContent::ToolCall {
        tool_call: ToolCall {
            id: "tc-message".into(),
            name: "bash".into(),
            arguments: r#"{"command":"echo OPI_MESSAGE_COMMAND_SECRET_CANARY","safe":true}"#.into(),
        },
    });

    let event = AgentEvent::MessageEnd {
        message: AgentMessage::Llm(Message::Assistant(assistant)),
    };

    let rendered = serde_json::to_string(&event.redacted_for_public()).unwrap();
    assert!(
        !rendered.contains("OPI_MESSAGE_COMMAND_SECRET_CANARY"),
        "{rendered}"
    );
    assert!(rendered.contains("[REDACTED]"), "{rendered}");
}

#[test]
fn public_stream_tool_call_delta_redacts_delta_and_partial_arguments() {
    let mut partial = test_support::base_assistant();
    partial.content.push(AssistantContent::ToolCall {
        tool_call: ToolCall {
            id: "tc-delta".into(),
            name: "bash".into(),
            arguments: r#"{"command":"echo OPI_DELTA_PARTIAL_SECRET_CANARY"}"#.into(),
        },
    });

    let event = AgentEvent::MessageUpdate {
        message: AgentMessage::Llm(Message::Assistant(partial.clone())),
        assistant_event: Box::new(AssistantStreamEvent::ToolCallDelta {
            content_index: 0,
            delta: r#"{"command":"echo OPI_DELTA_COMMAND_SECRET_CANARY"}"#.into(),
            partial,
        }),
    };

    let rendered = serde_json::to_string(&event.redacted_for_public()).unwrap();
    assert!(
        !rendered.contains("OPI_DELTA_COMMAND_SECRET_CANARY"),
        "{rendered}"
    );
    assert!(
        !rendered.contains("OPI_DELTA_PARTIAL_SECRET_CANARY"),
        "{rendered}"
    );
    assert!(rendered.contains("[REDACTED]"), "{rendered}");
}
