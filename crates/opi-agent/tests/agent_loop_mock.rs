//! Behavioral tests for agent_loop with mock provider and tools (task 1.6).
//!
//! DoD: "mock tests cover no-tool and tool-use turns"
//!
//! Uses the shared MockProvider from `opi_ai::test_support` (task 1.17).

use std::pin::Pin;
use std::sync::{Arc, Mutex};

use opi_agent::Agent;
use opi_agent::event::{AgentEvent, AgentEventSink};
use opi_agent::hooks::{
    AgentHooks, BeforeToolCallContext, BeforeToolCallResult, ShouldStopAfterTurnContext,
};
use opi_agent::loop_types::{AgentError, AgentLoopConfig, AgentLoopContext};
use opi_agent::message::AgentMessage;
use opi_agent::tool::{Tool, ToolError, ToolResult};
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::test_support::{self, MockProvider};
use serde_json::json;
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Mock tool
// ---------------------------------------------------------------------------

struct MockTool {
    name: String,
    call_log: Arc<Mutex<Vec<serde_json::Value>>>,
}

impl MockTool {
    fn new(name: &str, call_log: Arc<Mutex<Vec<serde_json::Value>>>) -> Self {
        Self {
            name: name.to_owned(),
            call_log,
        }
    }
}

impl Tool for MockTool {
    fn definition(&self) -> opi_ai::message::ToolDef {
        opi_ai::message::ToolDef {
            name: self.name.clone(),
            description: format!("Mock tool: {}", self.name),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "arg": { "type": "string" }
                },
                "required": ["arg"]
            }),
        }
    }

    fn execute(
        &self,
        _call_id: &str,
        arguments: serde_json::Value,
        _signal: CancellationToken,
        _on_update: Option<opi_agent::tool::UpdateCallback>,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<ToolResult, ToolError>> + Send>> {
        let log = self.call_log.clone();
        log.lock().unwrap().push(arguments.clone());
        let text = arguments
            .get("arg")
            .and_then(|v| v.as_str())
            .unwrap_or("mock-result")
            .to_owned();
        Box::pin(async move {
            Ok(ToolResult {
                content: vec![opi_ai::message::OutputContent::Text {
                    text: format!("tool-result: {text}"),
                }],
                details: None,
                is_error: false,
                terminate: false,
                truncated: false,
                diagnostics: vec![],
            })
        })
    }
}

// ---------------------------------------------------------------------------
// Default hooks for testing
// ---------------------------------------------------------------------------

struct TestHooks;

impl AgentHooks for TestHooks {
    fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
        let mut result = Vec::new();
        for msg in messages {
            if let AgentMessage::Llm(m) = msg {
                result.push(m.clone());
            }
        }
        Ok(result)
    }

    fn should_stop_after_turn(
        &self,
        _ctx: ShouldStopAfterTurnContext,
    ) -> Pin<Box<dyn std::future::Future<Output = bool> + Send>> {
        Box::pin(async { false })
    }

    fn before_tool_call(
        &self,
        _ctx: BeforeToolCallContext,
    ) -> Pin<Box<dyn std::future::Future<Output = BeforeToolCallResult> + Send>> {
        Box::pin(async { BeforeToolCallResult::Allow })
    }
}

// ---------------------------------------------------------------------------
// Test: no-tool turn
// ---------------------------------------------------------------------------

#[tokio::test]
async fn no_tool_turn_emits_lifecycle_events() {
    let response_events = test_support::text_response("Hello!");

    let provider = MockProvider::new("mock", vec![response_events]);
    let collected_events: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let event_collector = collected_events.clone();

    let sink: AgentEventSink = Box::new(move |event| {
        let name = match &event {
            AgentEvent::AgentStart => "AgentStart",
            AgentEvent::AgentEnd { .. } => "AgentEnd",
            AgentEvent::TurnStart => "TurnStart",
            AgentEvent::TurnEnd { .. } => "TurnEnd",
            AgentEvent::MessageStart { .. } => "MessageStart",
            AgentEvent::MessageUpdate { .. } => "MessageUpdate",
            AgentEvent::MessageEnd { .. } => "MessageEnd",
            AgentEvent::ToolExecutionStart { .. } => "ToolExecutionStart",
            AgentEvent::ToolExecutionUpdate { .. } => "ToolExecutionUpdate",
            AgentEvent::ToolExecutionEnd { .. } => "ToolExecutionEnd",
            AgentEvent::QueueUpdate { .. } => "QueueUpdate",
            _ => "Unknown",
        };
        event_collector.lock().unwrap().push(name.to_owned());
    });

    let context = AgentLoopContext {
        provider: Box::new(provider),
        tools: vec![],
        messages: vec![AgentMessage::Llm(Message::User(UserMessage {
            content: vec![InputContent::Text { text: "Hi".into() }],
            timestamp_ms: 0,
        }))],
        model: "mock-model".into(),
        system: None,
        steering_queue: None,
        follow_up_queue: None,
        diagnostic_sink: None,
        trace: None,
        session_id: None,
    };

    let config = AgentLoopConfig {
        max_turns: 10,
        ..Default::default()
    };

    let hooks = TestHooks;
    let result = opi_agent::agent_loop(context, config, &hooks, sink, CancellationToken::new())
        .await
        .unwrap();

    assert!(result.len() >= 2);

    let events = collected_events.lock().unwrap();
    assert!(
        events.contains(&"AgentStart".to_owned()),
        "missing AgentStart"
    );
    assert!(events.contains(&"AgentEnd".to_owned()), "missing AgentEnd");
    assert!(
        events.contains(&"TurnStart".to_owned()),
        "missing TurnStart"
    );
    assert!(events.contains(&"TurnEnd".to_owned()), "missing TurnEnd");
}

// ---------------------------------------------------------------------------
// Test: provider AccountIdMissing maps to a typed AgentError (C-3.2)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn account_id_missing_provider_error_maps_to_typed_agent_error() {
    use opi_ai::provider::ProviderError;
    use opi_ai::test_support::MockResponse;

    let provider = MockProvider::new_with_errors(
        "openai-codex",
        vec![MockResponse::Error(ProviderError::AccountIdMissing {
            provider_id: "openai-codex".into(),
        })],
    );

    let context = AgentLoopContext {
        provider: Box::new(provider),
        tools: vec![],
        messages: vec![AgentMessage::Llm(Message::User(UserMessage {
            content: vec![InputContent::Text { text: "Hi".into() }],
            timestamp_ms: 0,
        }))],
        model: "mock-model".into(),
        system: None,
        steering_queue: None,
        follow_up_queue: None,
        diagnostic_sink: None,
        trace: None,
        session_id: None,
    };
    let config = AgentLoopConfig {
        max_turns: 10,
        ..Default::default()
    };
    let sink: AgentEventSink = Box::new(|_| {});
    let err = opi_agent::agent_loop(context, config, &TestHooks, sink, CancellationToken::new())
        .await
        .expect_err("AccountIdMissing must surface as a typed AgentError, not Ok");
    match err {
        AgentError::AccountIdMissing { provider_id } => {
            assert_eq!(provider_id, "openai-codex");
        }
        other => panic!("expected AgentError::AccountIdMissing, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Test: tool-use turn
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tool_use_turn_executes_tool_and_loops() {
    let tool_call_log: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(Vec::new()));

    let first_response = test_support::tool_call_response("tc-1", "greet", r#"{"arg":"world"}"#);
    let second_response = test_support::text_response("Done!");

    let provider = MockProvider::new("mock", vec![first_response, second_response]);

    let tool = MockTool::new("greet", tool_call_log.clone());

    let collected_events: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let event_collector = collected_events.clone();

    let sink: AgentEventSink = Box::new(move |event| {
        let name = match &event {
            AgentEvent::AgentStart => "AgentStart",
            AgentEvent::AgentEnd { .. } => "AgentEnd",
            AgentEvent::TurnStart => "TurnStart",
            AgentEvent::TurnEnd { .. } => "TurnEnd",
            AgentEvent::MessageStart { .. } => "MessageStart",
            AgentEvent::MessageUpdate { .. } => "MessageUpdate",
            AgentEvent::MessageEnd { .. } => "MessageEnd",
            AgentEvent::ToolExecutionStart { tool_name, .. } => {
                // Store tool name for assertion
                event_collector
                    .lock()
                    .unwrap()
                    .push(format!("ToolExecutionStart:{tool_name}"));
                return;
            }
            AgentEvent::ToolExecutionUpdate { .. } => "ToolExecutionUpdate",
            AgentEvent::ToolExecutionEnd {
                tool_name,
                is_error,
                ..
            } => {
                let status = if *is_error { "err" } else { "ok" };
                event_collector
                    .lock()
                    .unwrap()
                    .push(format!("ToolExecutionEnd:{tool_name}:{status}"));
                return;
            }
            AgentEvent::QueueUpdate { .. } => "QueueUpdate",
            _ => "Unknown",
        };
        event_collector.lock().unwrap().push(name.to_owned());
    });

    let context = AgentLoopContext {
        provider: Box::new(provider),
        tools: vec![Box::new(tool)],
        messages: vec![AgentMessage::Llm(Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "Use the tool".into(),
            }],
            timestamp_ms: 0,
        }))],
        model: "mock-model".into(),
        system: None,
        steering_queue: None,
        follow_up_queue: None,
        diagnostic_sink: None,
        trace: None,
        session_id: None,
    };

    let config = AgentLoopConfig {
        max_turns: 10,
        ..Default::default()
    };

    let hooks = TestHooks;
    let result = opi_agent::agent_loop(context, config, &hooks, sink, CancellationToken::new())
        .await
        .unwrap();

    // Tool should have been called
    let log = tool_call_log.lock().unwrap();
    assert_eq!(log.len(), 1, "tool should have been called exactly once");
    assert_eq!(log[0]["arg"], "world");

    assert!(
        result.len() >= 3,
        "expected at least 3 messages, got {}",
        result.len()
    );

    let events = collected_events.lock().unwrap();
    assert!(
        events.iter().any(|e| e == "ToolExecutionStart:greet"),
        "missing ToolExecutionStart for greet"
    );
    assert!(
        events.iter().any(|e| e == "ToolExecutionEnd:greet:ok"),
        "missing ToolExecutionEnd(ok) for greet"
    );

    let turn_starts = events.iter().filter(|e| *e == "TurnStart").count();
    assert!(
        turn_starts >= 2,
        "expected at least 2 TurnStart events, got {}",
        turn_starts
    );
}

// ---------------------------------------------------------------------------
// Test: text content is preserved in assistant messages
// ---------------------------------------------------------------------------

#[tokio::test]
async fn text_content_preserved_in_assistant_message() {
    let response_events = test_support::text_response("Hello, world!");

    let provider = MockProvider::new("mock", vec![response_events]);

    let sink: AgentEventSink = Box::new(|_| {});

    let context = AgentLoopContext {
        provider: Box::new(provider),
        tools: vec![],
        messages: vec![AgentMessage::Llm(Message::User(UserMessage {
            content: vec![InputContent::Text { text: "Hi".into() }],
            timestamp_ms: 0,
        }))],
        model: "mock-model".into(),
        system: None,
        steering_queue: None,
        follow_up_queue: None,
        diagnostic_sink: None,
        trace: None,
        session_id: None,
    };

    let config = AgentLoopConfig {
        max_turns: 10,
        ..Default::default()
    };

    let hooks = TestHooks;
    let result = opi_agent::agent_loop(context, config, &hooks, sink, CancellationToken::new())
        .await
        .unwrap();

    // Find the assistant message
    let assistant = result
        .iter()
        .find_map(|m| match m {
            AgentMessage::Llm(Message::Assistant(a)) => Some(a),
            _ => None,
        })
        .expect("should have assistant message");

    // Verify text content is preserved
    let has_text = assistant.content.iter().any(
        |c| matches!(c, opi_ai::message::AssistantContent::Text { text } if text.contains("Hello")),
    );
    assert!(
        has_text,
        "assistant message must contain text, got: {:?}",
        assistant.content
    );
}

#[tokio::test]
async fn session_id_reaches_every_request() {
    let provider = MockProvider::new(
        "mock",
        vec![
            test_support::text_response("first"),
            test_support::text_response("second"),
        ],
    );
    let call_log = provider.call_log_handle();
    let mut agent = Agent::new(
        Box::new(provider),
        vec![],
        "mock-model".into(),
        None,
        AgentLoopConfig::default(),
        Box::new(TestHooks),
    );

    agent.set_session_id(Some("session-a".into()));
    agent.prompt("first").await.unwrap();
    agent.set_session_id(Some("session-b".into()));
    agent.continue_("second").await.unwrap();

    let calls = call_log.lock().unwrap();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].session_id.as_deref(), Some("session-a"));
    assert_eq!(calls[1].session_id.as_deref(), Some("session-b"));
}
