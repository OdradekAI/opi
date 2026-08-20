mod common;

use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use opi_agent::Agent;
use opi_agent::compaction::{
    CompactionConfig, CompactionEngine, CompactionHooks, DefaultCompactionHooks, Entry,
};
use opi_agent::diagnostic::code::CODE_TOOL_EXECUTION_FAILED;
use opi_agent::event::{AgentEvent, AgentEventSink};
use opi_agent::hooks::{
    AgentHooks, BeforeToolCallContext, BeforeToolCallResult, ShouldStopAfterTurnContext,
};
use opi_agent::loop_types::{
    AgentError, AgentLoopConfig, AgentLoopContext, InferenceConfig, ModelSelection, NextTurnState,
};
use opi_agent::message::AgentMessage;
use opi_agent::session_event::{CompactionReason, CompactionResult};
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

struct CanaryCompactionHook;

impl CompactionHooks for CanaryCompactionHook {
    fn generate_summary(&self, _messages: &[AgentMessage]) -> Option<String> {
        Some("hook copied OPI_HOOK_PROMPT_CANARY and OPI_HOOK_TOOL_CANARY from context".into())
    }
}

fn agent_with_provider_and_tools(provider: MockProvider, tools: Vec<Box<dyn Tool>>) -> Agent {
    Agent::new(
        Arc::new(test_support::single_route_collection(Box::new(provider))),
        common::registrations_from(tools),
        Some(common::permissive_authorizer()),
        "mock:mock-model".into(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig {
            max_turns: 3,
            ..Default::default()
        },
        Box::new(AllowHooks),
    )
    .expect("valid test agent")
}

fn subscribe_as_json(agent: &mut Agent) -> Arc<Mutex<Vec<String>>> {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let seen_clone = seen.clone();
    agent.subscribe(Box::new(move |event| {
        seen_clone
            .lock()
            .unwrap()
            .push(serde_json::to_string(event).unwrap());
    }));
    seen
}

#[test]
fn loop_tool_event_producers_do_not_redact_before_the_public_boundary() {
    let source = include_str!("../src/agent_loop.rs");
    assert!(
        !source.contains("redact_public_value"),
        "tool-event producers must emit raw arguments into the sole public boundary"
    );
}

#[test]
fn subscriber_panic_does_not_poison_future_fanout() {
    let provider = MockProvider::new("mock", vec![]);
    let mut agent = agent_with_provider_and_tools(provider, vec![]);
    let should_panic = Arc::new(AtomicBool::new(true));
    agent.subscribe(Box::new({
        let should_panic = should_panic.clone();
        move |_| {
            if should_panic.swap(false, Ordering::SeqCst) {
                panic!("subscriber panic canary");
            }
        }
    }));
    let delivered = Arc::new(AtomicUsize::new(0));
    agent.subscribe(Box::new({
        let delivered = delivered.clone();
        move |_| {
            delivered.fetch_add(1, Ordering::SeqCst);
        }
    }));

    let first = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        agent.emit_event(AgentEvent::AgentStart);
    }));
    assert!(
        first.is_err(),
        "subscriber panic remains visible to its caller"
    );

    agent.emit_event(AgentEvent::TurnStart);
    assert_eq!(
        delivered.load(Ordering::SeqCst),
        1,
        "a callback panic must not poison later fan-out"
    );
}

#[test]
fn compaction_entry_identity_is_redacted_only_when_untrusted() {
    let provider = MockProvider::new("mock", vec![]);
    let mut agent = agent_with_provider_and_tools(provider, vec![]);
    let seen = subscribe_as_json(&mut agent);

    for first_kept_entry_id in [
        "C:\\Users\\private\\entry-42",
        "sk-proj-secret1234567890",
        "entry-42",
    ] {
        agent.emit_event(AgentEvent::CompactionEnd {
            reason: CompactionReason::Threshold,
            result: Some(CompactionResult {
                summary: "untrusted summary".into(),
                first_kept_entry_id: first_kept_entry_id.into(),
                tokens_before: 2,
                tokens_after: 1,
            }),
            aborted: false,
            error_message: None,
        });
    }

    let seen = seen.lock().unwrap();
    let untrusted: serde_json::Value = serde_json::from_str(&seen[0]).unwrap();
    assert_eq!(untrusted["result"]["first_kept_entry_id"], "[REDACTED]");
    let credential: serde_json::Value = serde_json::from_str(&seen[1]).unwrap();
    assert_eq!(credential["result"]["first_kept_entry_id"], "[REDACTED]");
    let ordinary: serde_json::Value = serde_json::from_str(&seen[2]).unwrap();
    assert_eq!(ordinary["result"]["first_kept_entry_id"], "entry-42");
}

#[test]
fn harness_events_cross_one_redacting_subscriber_fanout() {
    let provider = MockProvider::new("mock", vec![]);
    let mut agent = agent_with_provider_and_tools(provider, vec![]);
    let seen = subscribe_as_json(&mut agent);

    let entries = vec![
        Entry {
            id: "compacted".into(),
            message: AgentMessage::Llm(Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "OPI_PROMPT_CANARY".into(),
                }],
                timestamp_ms: 0,
            })),
        },
        Entry {
            id: "tool-output".into(),
            message: AgentMessage::Llm(Message::ToolResult(opi_ai::message::ToolResultMessage {
                tool_call_id: "tc-compaction".into(),
                tool_name: "bash".into(),
                content: vec![OutputContent::Text {
                    text: "OPI_TOOL_RESULT_CANARY".into(),
                }],
                details: None,
                is_error: false,
                truncated: false,
                timestamp_ms: 0,
            })),
        },
        Entry {
            id: "kept".into(),
            message: AgentMessage::Llm(Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "safe tail".into(),
                }],
                timestamp_ms: 0,
            })),
        },
    ];
    let engine = CompactionEngine::new(CompactionConfig::default());
    let hook_output = engine
        .compact(&entries, CompactionReason::Threshold, &CanaryCompactionHook)
        .expect("custom compaction summary");
    let core_output = engine
        .compact(
            &entries,
            CompactionReason::Threshold,
            &DefaultCompactionHooks,
        )
        .expect("core compaction summary");

    agent.emit_event(AgentEvent::CompactionStart {
        reason: CompactionReason::Threshold,
    });
    agent.emit_event(AgentEvent::CompactionEnd {
        reason: hook_output.reason,
        result: Some(CompactionResult {
            summary: hook_output.summary_text,
            first_kept_entry_id: hook_output.first_kept_entry_id,
            tokens_before: hook_output.tokens_before,
            tokens_after: hook_output.tokens_after,
        }),
        aborted: false,
        error_message: Some(
            "compaction failed at C:\\Users\\private\\opi with sk-proj-secret1234567890".into(),
        ),
    });
    agent.emit_event(AgentEvent::CompactionEnd {
        reason: core_output.reason,
        result: Some(CompactionResult {
            summary: core_output.summary_text,
            first_kept_entry_id: core_output.first_kept_entry_id,
            tokens_before: core_output.tokens_before,
            tokens_after: core_output.tokens_after,
        }),
        aborted: false,
        error_message: None,
    });
    agent.emit_event(AgentEvent::CompactionEnd {
        reason: CompactionReason::Threshold,
        result: None,
        aborted: true,
        error_message: Some("compaction produced no output".into()),
    });
    agent.emit_event(AgentEvent::SessionPersistError {
        message: "OPI_PERSIST_CANARY".into(),
    });

    let seen = seen.lock().unwrap();
    assert_eq!(
        seen.len(),
        5,
        "each producer event must fan out exactly once"
    );
    let hook_compaction: serde_json::Value = serde_json::from_str(&seen[1]).unwrap();
    assert_eq!(hook_compaction["result"]["summary"], "[REDACTED]");
    assert_eq!(hook_compaction["error_message"], "[REDACTED]");
    let core_compaction: serde_json::Value = serde_json::from_str(&seen[2]).unwrap();
    assert_eq!(core_compaction["result"]["summary"], "[REDACTED]");
    let closed_compaction: serde_json::Value = serde_json::from_str(&seen[3]).unwrap();
    assert_eq!(
        closed_compaction["error_message"],
        "compaction produced no output"
    );
    let persist_error: serde_json::Value = serde_json::from_str(&seen[4]).unwrap();
    assert_eq!(persist_error["message"], "[REDACTED]");
    let rendered = seen.join("\n");
    for canary in [
        "OPI_PROMPT_CANARY",
        "OPI_TOOL_RESULT_CANARY",
        "OPI_HOOK_PROMPT_CANARY",
        "OPI_HOOK_TOOL_CANARY",
        "OPI_PERSIST_CANARY",
        "C:\\\\Users",
        "sk-proj-secret1234567890",
    ] {
        assert!(!rendered.contains(canary), "leaked {canary}: {rendered}");
    }
    assert!(
        rendered.contains("\"first_kept_entry_id\":\"kept\""),
        "structured compaction identity must survive redaction: {rendered}"
    );
}

#[tokio::test]
async fn loop_events_cross_the_same_subscriber_fanout_exactly_once() {
    let first = test_support::tool_call_response(
        "tc1",
        "bash",
        r#"{"command":"echo OPI_SUBSCRIBER_TOOL_CANARY"}"#,
    );
    let second = test_support::text_response("done");
    let provider = MockProvider::new("mock", vec![first, second]);
    let mut agent = agent_with_provider_and_tools(provider, vec![Box::new(SecretTool)]);
    let seen = subscribe_as_json(&mut agent);

    let result = agent.prompt("OPI_SUBSCRIBER_PROMPT_CANARY").await;
    assert!(result.error().is_none(), "agent run failed: {result:?}");

    let seen = seen.lock().unwrap();
    assert_eq!(
        seen.iter()
            .filter(|event| event.contains("\"type\":\"AgentStart\""))
            .count(),
        1,
        "loop producer must not bypass or duplicate subscriber fan-out: {seen:?}"
    );
    assert_eq!(
        seen.iter()
            .filter(|event| event.contains("\"type\":\"ToolExecutionStart\""))
            .count(),
        1,
        "tool start must fan out exactly once: {seen:?}"
    );
    let rendered = seen.join("\n");
    assert!(
        !rendered.contains("OPI_SUBSCRIBER_TOOL_CANARY"),
        "{rendered}"
    );
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
        session_id: None,
        evidence_sink: None,
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
    .into_execution_result()
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
