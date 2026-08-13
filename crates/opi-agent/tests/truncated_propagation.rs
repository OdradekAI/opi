//! Phase 11.1: `ToolResult.truncated` propagates through `agent_loop` into
//! `ToolResultMessage` and `AgentEvent::ToolExecutionEnd` for both execution
//! modes. The sequential and parallel batches are near-duplicates; this guards
//! against a field landing in one but not the other.

mod common;

use std::pin::Pin;
use std::sync::{Arc, Mutex};

use opi_agent::event::{AgentEvent, AgentEventSink};
use opi_agent::hooks::{
    AgentHooks, BeforeToolCallContext, BeforeToolCallResult, ShouldStopAfterTurnContext,
};
use opi_agent::loop_types::{
    AgentError, AgentLoopConfig, AgentLoopContext, InferenceConfig, ModelSelection, NextTurnState,
};
use opi_agent::message::AgentMessage;
use opi_agent::tool::{ExecutionMode, Tool, ToolError, ToolResult};
use opi_ai::message::{InputContent, Message, OutputContent, UserMessage};
use opi_ai::test_support::{self, MockProvider, single_route_collection};
use serde_json::json;
use tokio_util::sync::CancellationToken;

struct TruncatingTool {
    name: &'static str,
    mode: ExecutionMode,
}

impl Tool for TruncatingTool {
    fn definition(&self) -> opi_ai::message::ToolDef {
        opi_ai::message::ToolDef {
            name: self.name.into(),
            description: "returns a truncated result".into(),
            input_schema: json!({ "type": "object", "properties": {} }),
        }
    }

    fn execute(
        &self,
        _call_id: &str,
        _arguments: serde_json::Value,
        _signal: CancellationToken,
        _on_update: Option<opi_agent::tool::UpdateCallback>,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<ToolResult, ToolError>> + Send>> {
        Box::pin(async move {
            Ok(ToolResult {
                content: vec![OutputContent::Text {
                    text: "partial".into(),
                }],
                details: None,
                is_error: false,
                terminate: false,
                truncated: true,
                diagnostics: vec![],
            })
        })
    }

    fn execution_mode(&self) -> ExecutionMode {
        self.mode
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

/// Run one tool-call turn through `agent_loop` with a tool that returns
/// `truncated: true`, returning `(event_truncated, message_truncated)`.
async fn run_truncated(mode: ExecutionMode) -> (Option<bool>, Option<bool>) {
    let name = match mode {
        ExecutionMode::Sequential => "seqtool",
        ExecutionMode::Parallel => "partool",
    };
    let first = test_support::tool_call_response("tc-1", name, "{}");
    let second = test_support::text_response("ok");
    let provider = MockProvider::new("mock", vec![first, second]);
    let tool = TruncatingTool { name, mode };

    let event_truncated: Arc<Mutex<Option<bool>>> = Arc::new(Mutex::new(None));
    let event_clone = event_truncated.clone();
    let sink: AgentEventSink = Box::new(move |event| {
        if let AgentEvent::ToolExecutionEnd { truncated, .. } = event {
            *event_clone.lock().unwrap() = Some(truncated);
        }
    });

    let messages = vec![AgentMessage::Llm(Message::User(UserMessage {
        content: vec![InputContent::Text {
            text: "use the tool".into(),
        }],
        timestamp_ms: 0,
    }))];
    let context = AgentLoopContext {
        collection: Arc::new(single_route_collection(Box::new(provider))),
        registry: common::test_registry(vec![Box::new(tool)]),
        authorizer: Some(common::permissive_authorizer()),
        evidence_health: opi_agent::evidence::EvidenceHealth::healthy(),
        state: NextTurnState::new(
            messages,
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
    let config = AgentLoopConfig {
        max_turns: 10,
        ..Default::default()
    };
    let hooks = AllowHooks;
    let result = opi_agent::agent_loop(context, config, &hooks, sink, CancellationToken::new())
        .await
        .unwrap();

    let message_truncated = result.context.iter().find_map(|m| match m {
        AgentMessage::Llm(Message::ToolResult(trm)) => Some(trm.truncated),
        _ => None,
    });
    let event = *event_truncated.lock().unwrap();
    (event, message_truncated)
}

#[tokio::test]
async fn truncated_propagates_through_sequential_batch() {
    let (event, message) = run_truncated(ExecutionMode::Sequential).await;
    assert_eq!(
        event,
        Some(true),
        "AgentEvent::ToolExecutionEnd.truncated must propagate (sequential batch)"
    );
    assert_eq!(
        message,
        Some(true),
        "ToolResultMessage.truncated must propagate (sequential batch)"
    );
}

#[tokio::test]
async fn truncated_propagates_through_parallel_batch() {
    let (event, message) = run_truncated(ExecutionMode::Parallel).await;
    assert_eq!(
        event,
        Some(true),
        "AgentEvent::ToolExecutionEnd.truncated must propagate (parallel batch)"
    );
    assert_eq!(
        message,
        Some(true),
        "ToolResultMessage.truncated must propagate (parallel batch)"
    );
}
