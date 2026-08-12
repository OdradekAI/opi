//! E2E integration test using MockProvider (task 1.17).
//!
//! DoD: "mock-provider E2E runs in CI"
//!
//! Demonstrates the full workflow: MockProvider → Agent → prompt → verify messages.
//! This test runs without any live API calls and exercises the cross-crate
//! integration path that cli-runtime tasks (1.11, 1.14, 1.15, 1.16) will build on.

use std::pin::Pin;
use std::sync::{Arc, Mutex};

use opi_agent::Agent;
use opi_agent::hooks::AgentHooks;
use opi_agent::loop_types::{AgentError, AgentLoopConfig, InferenceConfig};
use opi_agent::message::AgentMessage;
use opi_agent::tool::{Tool, ToolError, ToolResult};
use opi_ai::message::{InputContent, Message};
use opi_ai::test_support::{self, MockProvider, single_route_collection};
use serde_json::json;
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Record event type names for assertion without requiring Clone on AgentEvent.
fn event_name(event: &opi_agent::event::AgentEvent) -> &'static str {
    use opi_agent::event::AgentEvent::*;
    match event {
        AgentStart => "AgentStart",
        AgentEnd { .. } => "AgentEnd",
        TurnStart => "TurnStart",
        TurnEnd { .. } => "TurnEnd",
        MessageStart { .. } => "MessageStart",
        MessageUpdate { .. } => "MessageUpdate",
        MessageEnd { .. } => "MessageEnd",
        ToolExecutionStart { .. } => "ToolExecutionStart",
        ToolExecutionUpdate { .. } => "ToolExecutionUpdate",
        ToolExecutionEnd { is_error, .. } => {
            if *is_error {
                "ToolExecutionEnd(error)"
            } else {
                "ToolExecutionEnd(ok)"
            }
        }
        QueueUpdate { .. } => "QueueUpdate",
        _ => "Unknown",
    }
}

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
}

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
// E2E test: text-only prompt through Agent
// ---------------------------------------------------------------------------

#[tokio::test]
async fn e2e_text_prompt_returns_assistant_message() {
    let response = test_support::text_response("Hello from mock!");
    let provider = MockProvider::new("mock", vec![response]);

    let mut agent = Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        vec![],
        "mock:mock-model".into(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig::default(),
        Box::new(TestHooks),
    )
    .expect("agent");

    let events: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let ev = events.clone();
    agent.subscribe(Box::new(move |event| {
        ev.lock().unwrap().push(event_name(event).to_owned());
    }));

    let result = agent.prompt("Hi there").await.unwrap();

    // Should have at least user message + assistant response
    assert!(
        result.len() >= 2,
        "expected >= 2 messages, got {}",
        result.len()
    );

    // First message should be user
    if let AgentMessage::Llm(Message::User(user)) = &result[0] {
        let text = &user.content[0];
        assert!(
            matches!(text, InputContent::Text { text } if text == "Hi there"),
            "first message should be the user prompt"
        );
    } else {
        panic!("first message should be a User message");
    }

    // Should have an assistant message (the loop accumulates Done event's message)
    let has_assistant = result
        .iter()
        .any(|m| matches!(m, AgentMessage::Llm(Message::Assistant(_))));
    assert!(has_assistant, "should have at least one Assistant message");

    // Events should include lifecycle
    let ev_lock = events.lock().unwrap();
    assert!(
        ev_lock.contains(&"AgentStart".to_owned()),
        "missing AgentStart"
    );
    assert!(ev_lock.contains(&"AgentEnd".to_owned()), "missing AgentEnd");
}

// ---------------------------------------------------------------------------
// E2E test: tool call prompt through Agent
// ---------------------------------------------------------------------------

#[tokio::test]
async fn e2e_tool_call_prompt_executes_tool() {
    let tool_call_log: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(Vec::new()));

    let first = test_support::tool_call_response("tc-1", "greet", r#"{"arg":"world"}"#);
    let second = test_support::text_response("Tool executed successfully!");

    let provider = MockProvider::new("mock", vec![first, second]);

    let tool = MockTool::new("greet", tool_call_log.clone());

    let mut agent = Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        vec![Box::new(tool)],
        "mock:mock-model".into(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig::default(),
        Box::new(TestHooks),
    )
    .expect("agent");

    let events: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let ev = events.clone();
    agent.subscribe(Box::new(move |event| {
        ev.lock().unwrap().push(event_name(event).to_owned());
    }));

    let result = agent.prompt("Use the greet tool").await.unwrap();

    // Tool should have been called
    let log = tool_call_log.lock().unwrap();
    assert_eq!(log.len(), 1, "tool should have been called exactly once");
    assert_eq!(log[0]["arg"], "world");

    // Should have messages: user → assistant(tool_call) → tool_result → assistant(text)
    assert!(
        result.len() >= 4,
        "expected >= 4 messages, got {}",
        result.len()
    );

    // Events should include tool execution
    let ev_lock = events.lock().unwrap();
    assert!(
        ev_lock.contains(&"ToolExecutionStart".to_owned()),
        "missing ToolExecutionStart"
    );
    assert!(
        ev_lock.contains(&"ToolExecutionEnd(ok)".to_owned()),
        "missing ToolExecutionEnd(ok)"
    );
}

// ---------------------------------------------------------------------------
// E2E test: multi-turn conversation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn e2e_multi_turn_conversation_accumulates_state() {
    let first = test_support::text_response("First response");
    let second = test_support::text_response("Second response");

    let provider = MockProvider::new("mock", vec![first, second]);

    let mut agent = Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        vec![],
        "mock:mock-model".into(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig::default(),
        Box::new(TestHooks),
    )
    .expect("agent");

    let result1 = agent.prompt("Hello").await.unwrap();
    assert!(result1.len() >= 2);

    let result2 = agent.continue_("Tell me more").await.unwrap();

    // After two turns: user1 + asst1 + user2 + asst2
    assert!(
        result2.len() >= 4,
        "expected >= 4 messages after two turns, got {}",
        result2.len()
    );
}

// ---------------------------------------------------------------------------
// E2E test: error response from provider
// ---------------------------------------------------------------------------

#[tokio::test]
async fn e2e_error_response_from_provider() {
    let response = test_support::error_response("rate limited");
    let provider = MockProvider::new("mock", vec![response]);

    let mut agent = Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        vec![],
        "mock:mock-model".into(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig::default(),
        Box::new(TestHooks),
    )
    .expect("agent");

    let result = agent.prompt("Hello").await.unwrap();

    // Should still have messages (user + error assistant)
    assert!(result.len() >= 2);

    // The assistant message should have error_message set
    let has_error_msg = result.iter().any(|m| {
        if let AgentMessage::Llm(Message::Assistant(a)) = m {
            a.error_message.is_some()
        } else {
            false
        }
    });
    assert!(
        has_error_msg,
        "should have an assistant message with error_message set"
    );
}
