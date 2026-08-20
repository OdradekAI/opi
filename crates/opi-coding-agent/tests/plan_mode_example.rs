//! Plan mode extension/package example.
//!
//! Demonstrates a planning workflow through custom commands and extension hooks
//! without adding built-in plan mode to the core runtime. When plan mode is
//! active, mutating tools (write, edit, bash) are blocked and a planning prompt
//! is injected into the conversation context.

mod common;

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use opi_agent::event::AgentEvent;
use opi_agent::extension::{
    Extension, ExtensionCommand, ExtensionError, ExtensionHookResult, ExtensionRegistry,
};
use opi_agent::hooks::AgentHooks;
use opi_agent::loop_types::{AgentError, InferenceConfig};
use opi_agent::message::AgentMessage;
use opi_agent::tool::{ExecutionMode, Tool, ToolError, ToolResult};
use opi_ai::message::{OutputContent, ToolDef};
use opi_ai::test_support::{
    MockProvider, single_route_collection, text_response, tool_call_response,
};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Plan mode types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum PlanMode {
    Normal,
    Planning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PlanModeState {
    mode: PlanMode,
    plan_notes: Vec<String>,
    tools_blocked: u64,
    tools_allowed: u64,
}

impl Default for PlanModeState {
    fn default() -> Self {
        Self {
            mode: PlanMode::Normal,
            plan_notes: Vec::new(),
            tools_blocked: 0,
            tools_allowed: 0,
        }
    }
}

const MUTATING_TOOLS: &[&str] = &["write", "edit", "bash"];

fn is_mutating(tool_name: &str) -> bool {
    MUTATING_TOOLS.contains(&tool_name)
}

// ---------------------------------------------------------------------------
// PlanModeExtension
// ---------------------------------------------------------------------------

struct PlanModeExtension {
    state: Arc<Mutex<PlanModeState>>,
    events_received: Arc<Mutex<Vec<String>>>,
    planning_prompt: String,
}

impl PlanModeExtension {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(PlanModeState::default())),
            events_received: Arc::new(Mutex::new(Vec::new())),
            planning_prompt: "You are in plan mode. Analyze and plan but do not modify files."
                .to_string(),
        }
    }

    fn mode(&self) -> PlanMode {
        self.state.lock().unwrap().mode.clone()
    }
}

impl Extension for PlanModeExtension {
    fn name(&self) -> &str {
        "plan-mode"
    }

    fn on_command(
        &self,
        command: &ExtensionCommand,
    ) -> Pin<Box<dyn Future<Output = Result<Option<serde_json::Value>, ExtensionError>> + Send>>
    {
        let cmd = command.name.clone();
        let args = command.args.clone();
        let state = self.state.clone();
        let planning_prompt = self.planning_prompt.clone();

        Box::pin(async move {
            match cmd.as_str() {
                "plan-mode/enter" => {
                    let note = args["note"].as_str().unwrap_or("").to_string();
                    let mut s = state.lock().unwrap();
                    s.mode = PlanMode::Planning;
                    if !note.is_empty() {
                        s.plan_notes.push(note);
                    }
                    Ok(Some(serde_json::json!({
                        "status": "planning",
                        "prompt": planning_prompt,
                    })))
                }
                "plan-mode/exit" => {
                    let mut s = state.lock().unwrap();
                    s.mode = PlanMode::Normal;
                    Ok(Some(serde_json::json!({
                        "status": "normal",
                    })))
                }
                "plan-mode/status" => {
                    let s = state.lock().unwrap();
                    let mode_str = match s.mode {
                        PlanMode::Normal => "normal",
                        PlanMode::Planning => "planning",
                    };
                    Ok(Some(serde_json::json!({
                        "status": mode_str,
                        "plan_notes": s.plan_notes,
                        "tools_blocked": s.tools_blocked,
                        "tools_allowed": s.tools_allowed,
                    })))
                }
                _ => Ok(None),
            }
        })
    }

    fn on_before_tool_call(
        &self,
        tool_name: &str,
        _args: &serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = ExtensionHookResult> + Send>> {
        let mode = self.mode();
        let mutating = is_mutating(tool_name);
        let state = self.state.clone();
        let name = tool_name.to_string();

        Box::pin(async move {
            if mode == PlanMode::Planning && mutating {
                let mut s = state.lock().unwrap();
                s.tools_blocked += 1;
                ExtensionHookResult::Block {
                    reason: format!(
                        "plan mode active: mutating tool '{}' blocked. Exit plan mode first.",
                        name
                    ),
                }
            } else {
                if mode == PlanMode::Planning {
                    let mut s = state.lock().unwrap();
                    s.tools_allowed += 1;
                }
                ExtensionHookResult::Continue
            }
        })
    }

    fn on_event(&self, event: &AgentEvent) {
        let label = match event {
            AgentEvent::AgentStart => "AgentStart".to_string(),
            AgentEvent::AgentEnd { .. } => "AgentEnd".to_string(),
            AgentEvent::TurnStart => "TurnStart".to_string(),
            AgentEvent::ToolExecutionStart { tool_name, .. } => {
                format!("ToolExecutionStart({})", tool_name)
            }
            AgentEvent::ToolExecutionEnd { tool_name, .. } => {
                format!("ToolExecutionEnd({})", tool_name)
            }
            _ => "Other".to_string(),
        };
        self.events_received.lock().unwrap().push(label);
    }

    fn serialize_state(&self) -> Result<Option<serde_json::Value>, ExtensionError> {
        let s = self.state.lock().unwrap();
        let val = serde_json::to_value(PlanModeState {
            mode: s.mode.clone(),
            plan_notes: s.plan_notes.clone(),
            tools_blocked: s.tools_blocked,
            tools_allowed: s.tools_allowed,
        })
        .map_err(|e| ExtensionError::StateSerialization {
            name: "plan-mode".into(),
            reason: e.to_string(),
        })?;
        Ok(Some(val))
    }

    fn restore_state(&self, state_val: serde_json::Value) -> Result<(), ExtensionError> {
        let parsed: PlanModeState =
            serde_json::from_value(state_val).map_err(|e| ExtensionError::StateRestoration {
                name: "plan-mode".into(),
                reason: e.to_string(),
            })?;
        let mut s = self.state.lock().unwrap();
        *s = parsed;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

struct DummyTool {
    name: String,
}

impl DummyTool {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

impl Tool for DummyTool {
    fn definition(&self) -> ToolDef {
        serde_json::from_value(serde_json::json!({
            "name": self.name,
            "description": format!("{} tool for testing", self.name),
            "input_schema": { "type": "object", "properties": {} }
        }))
        .unwrap()
    }

    fn execute(
        &self,
        _call_id: &str,
        _arguments: serde_json::Value,
        _signal: CancellationToken,
        _on_update: Option<opi_agent::tool::UpdateCallback>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send>> {
        let name = self.name.clone();
        Box::pin(async move {
            Ok(ToolResult {
                content: vec![OutputContent::Text {
                    text: format!("{}-ok", name),
                }],
                details: None,
                is_error: false,
                terminate: false,
                truncated: false,
                diagnostics: vec![],
            })
        })
    }

    fn execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Parallel
    }
}

struct TestHooks;

impl AgentHooks for TestHooks {
    fn convert_to_llm(
        &self,
        messages: &[AgentMessage],
    ) -> Result<Vec<opi_ai::message::Message>, AgentError> {
        Ok(messages
            .iter()
            .filter_map(|m| match m {
                AgentMessage::Llm(msg) => Some(msg.clone()),
                _ => None,
            })
            .collect())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn enter_plan_mode_activates_planning_state() {
    let ext = PlanModeExtension::new();
    let state = ext.state.clone();

    let cmd = ExtensionCommand::new("plan-mode/enter", serde_json::json!({"note": "refactor"}));
    let result = ext.on_command(&cmd).await.unwrap().unwrap();

    assert_eq!(result["status"], "planning");
    assert!(result["prompt"].is_string());

    let s = state.lock().unwrap();
    assert_eq!(s.mode, PlanMode::Planning);
    assert!(s.plan_notes.contains(&"refactor".to_string()));
}

#[tokio::test]
async fn exit_plan_mode_deactivates_planning_state() {
    let ext = PlanModeExtension::new();
    let state = ext.state.clone();

    // Enter first
    let enter = ExtensionCommand::new("plan-mode/enter", serde_json::json!({}));
    ext.on_command(&enter).await.unwrap();

    // Exit
    let exit = ExtensionCommand::new("plan-mode/exit", serde_json::json!({}));
    let result = ext.on_command(&exit).await.unwrap().unwrap();

    assert_eq!(result["status"], "normal");
    let s = state.lock().unwrap();
    assert_eq!(s.mode, PlanMode::Normal);
}

#[tokio::test]
async fn plan_mode_blocks_mutating_tools() {
    let ext = PlanModeExtension::new();
    let state = ext.state.clone();

    // Enter plan mode
    let enter = ExtensionCommand::new("plan-mode/enter", serde_json::json!({}));
    ext.on_command(&enter).await.unwrap();

    // Check each mutating tool is blocked
    for tool in MUTATING_TOOLS {
        let result = ext.on_before_tool_call(tool, &serde_json::json!({})).await;
        match result {
            ExtensionHookResult::Block { reason } => {
                assert!(reason.contains(tool));
                assert!(reason.contains("plan mode"));
            }
            ExtensionHookResult::Continue => {
                panic!("{} should be blocked in plan mode", tool);
            }
            _ => {
                panic!("unexpected hook result for {}", tool);
            }
        }
    }

    let s = state.lock().unwrap();
    assert_eq!(s.tools_blocked, MUTATING_TOOLS.len() as u64);
}

#[tokio::test]
async fn plan_mode_allows_read_only_tools() {
    let ext = PlanModeExtension::new();
    let state = ext.state.clone();

    // Enter plan mode
    let enter = ExtensionCommand::new("plan-mode/enter", serde_json::json!({}));
    ext.on_command(&enter).await.unwrap();

    let read_tools = ["read", "glob", "grep", "find", "ls"];
    for tool in &read_tools {
        let result = ext.on_before_tool_call(tool, &serde_json::json!({})).await;
        assert!(
            matches!(result, ExtensionHookResult::Continue),
            "{} should be allowed in plan mode",
            tool
        );
    }

    let s = state.lock().unwrap();
    assert_eq!(s.tools_allowed, read_tools.len() as u64);
    assert_eq!(s.tools_blocked, 0);
}

#[tokio::test]
async fn plan_mode_status_returns_current_state() {
    let ext = PlanModeExtension::new();

    // Status in normal mode
    let cmd = ExtensionCommand::new("plan-mode/status", serde_json::json!({}));
    let result = ext.on_command(&cmd).await.unwrap().unwrap();
    assert_eq!(result["status"], "normal");

    // Enter plan mode
    let enter = ExtensionCommand::new("plan-mode/enter", serde_json::json!({"note": "plan-a"}));
    ext.on_command(&enter).await.unwrap();

    // Status in planning mode
    let result = ext.on_command(&cmd).await.unwrap().unwrap();
    assert_eq!(result["status"], "planning");
    let notes = result["plan_notes"].as_array().unwrap();
    assert!(notes.iter().any(|n| n.as_str() == Some("plan-a")));
}

#[tokio::test]
async fn plan_mode_with_agent_blocks_mutating_tool_call() {
    let ext = PlanModeExtension::new();
    let state = ext.state.clone();

    // Enter plan mode via command
    let enter = ExtensionCommand::new("plan-mode/enter", serde_json::json!({}));
    ext.on_command(&enter).await.unwrap();

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    // Provider tries to call write, then gives text response
    let provider = MockProvider::new(
        "mock",
        vec![
            tool_call_response("tc_1", "write", r#"{"path":"/tmp/f","content":"x"}"#),
            text_response("I'll read first instead."),
        ],
    );

    let hooks = registry.wrap_hooks(Box::new(TestHooks));
    let mut agent = opi_agent::Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        common::registrations_from(vec![Box::new(DummyTool::new("write"))]),
        Some(common::permissive_authorizer()),
        "mock:mock-model".into(),
        None,
        InferenceConfig::default(),
        Default::default(),
        hooks,
    )
    .expect("agent");

    let _result = agent.prompt("test").await.into_execution_result().unwrap();
    // The write tool should have been blocked by the extension
    let s = state.lock().unwrap();
    assert_eq!(s.tools_blocked, 1);
}

#[tokio::test]
async fn normal_mode_allows_all_tools() {
    let ext = PlanModeExtension::new();

    for tool in MUTATING_TOOLS {
        let result = ext.on_before_tool_call(tool, &serde_json::json!({})).await;
        assert!(
            matches!(result, ExtensionHookResult::Continue),
            "{} should be allowed in normal mode",
            tool
        );
    }
}

#[tokio::test]
async fn plan_state_round_trips_through_serialization() {
    let ext = PlanModeExtension::new();

    // Enter plan mode with notes
    let enter = ExtensionCommand::new(
        "plan-mode/enter",
        serde_json::json!({"note": "refactor module"}),
    );
    ext.on_command(&enter).await.unwrap();

    // Simulate some tool gating
    ext.on_before_tool_call("write", &serde_json::json!({}))
        .await;
    ext.on_before_tool_call("read", &serde_json::json!({}))
        .await;

    // Serialize
    let serialized = ext.serialize_state().unwrap().unwrap();
    assert_eq!(serialized["mode"], "Planning");
    assert_eq!(serialized["tools_blocked"], 1);
    assert_eq!(serialized["tools_allowed"], 1);

    // Restore into a new extension
    let ext2 = PlanModeExtension::new();
    ext2.restore_state(serialized).unwrap();

    let s = ext2.state.lock().unwrap();
    assert_eq!(s.mode, PlanMode::Planning);
    assert!(s.plan_notes.contains(&"refactor module".to_string()));
    assert_eq!(s.tools_blocked, 1);
    assert_eq!(s.tools_allowed, 1);
}

#[tokio::test]
async fn unknown_command_returns_none() {
    let ext = PlanModeExtension::new();
    let cmd = ExtensionCommand::new("plan-mode/unknown", serde_json::json!({}));
    let result = ext.on_command(&cmd).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn extension_receives_parent_agent_events() {
    let ext = PlanModeExtension::new();
    let events = ext.events_received.clone();

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    let base_sink = Box::new(|_event: AgentEvent| {}) as Box<dyn Fn(AgentEvent) + Send + Sync>;
    let wrapped_sink = registry.wrap_event_sink(base_sink);

    wrapped_sink(AgentEvent::AgentStart);
    wrapped_sink(AgentEvent::TurnStart);
    wrapped_sink(AgentEvent::ToolExecutionStart {
        tool_call_id: "tc_1".into(),
        tool_name: "read".into(),
        args: serde_json::json!({}),
    });

    let received = events.lock().unwrap();
    assert!(received.contains(&"AgentStart".to_string()));
    assert!(received.contains(&"TurnStart".to_string()));
    assert!(received.contains(&"ToolExecutionStart(read)".to_string()));
}

#[tokio::test]
async fn multiple_enter_exit_cycles_work() {
    let ext = PlanModeExtension::new();
    let state = ext.state.clone();

    for i in 0..3 {
        let enter = ExtensionCommand::new(
            "plan-mode/enter",
            serde_json::json!({"note": format!("cycle-{}", i)}),
        );
        ext.on_command(&enter).await.unwrap();
        assert_eq!(state.lock().unwrap().mode, PlanMode::Planning);

        let exit = ExtensionCommand::new("plan-mode/exit", serde_json::json!({}));
        ext.on_command(&exit).await.unwrap();
        assert_eq!(state.lock().unwrap().mode, PlanMode::Normal);
    }

    let s = state.lock().unwrap();
    assert_eq!(s.plan_notes.len(), 3);
    assert!(s.plan_notes.contains(&"cycle-0".to_string()));
    assert!(s.plan_notes.contains(&"cycle-1".to_string()));
    assert!(s.plan_notes.contains(&"cycle-2".to_string()));
}

#[tokio::test]
async fn enter_without_note_works() {
    let ext = PlanModeExtension::new();
    let state = ext.state.clone();

    let enter = ExtensionCommand::new("plan-mode/enter", serde_json::json!({}));
    let result = ext.on_command(&enter).await.unwrap().unwrap();
    assert_eq!(result["status"], "planning");

    let s = state.lock().unwrap();
    assert_eq!(s.mode, PlanMode::Planning);
    assert!(s.plan_notes.is_empty());
}
