//! Sub-agent extension/package example tests (task 4.8.3).
//!
//! These tests demonstrate a sub-agent extension that runs nested agent
//! workflows through the SDK/RPC command model with isolated state, bounded
//! cancellation, and no extra core sub-agent feature flags. This is an
//! **example** showing how to build sub-agent orchestration as an extension —
//! it is NOT core sub-agent functionality and does not add feature flags to the
//! agent runtime.
//!
//! # What This Example Demonstrates
//!
//! - **Child run completion**: Parent dispatches a child prompt, child
//!   completes, result routed back.
//! - **Child error propagation**: Child provider errors surface to the parent.
//! - **Cancellation**: Child runs can be cancelled via their cancellation
//!   token.
//! - **Event routing**: Child agent events are observable by the parent
//!   extension.
//! - **Isolated state**: Each child run has independent state.
//! - **Session visibility**: Run history is queryable and persists through
//!   serialization.
//!
//! # Example vs Core Sub-Agent
//!
//! The sub-agent lives entirely in extension code. It uses the standard
//! [`Extension::on_command`] to dispatch child runs and the standard
//! [`Extension::on_event`] to observe agent lifecycle events. No core runtime
//! changes or feature flags are needed.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use opi_agent::event::AgentEvent;
use opi_agent::extension::{Extension, ExtensionCommand, ExtensionError, ExtensionRegistry};
use opi_agent::hooks::AgentHooks;
use opi_agent::loop_types::{AgentError, AgentLoopConfig, InferenceConfig};
use opi_agent::message::AgentMessage;
use opi_agent::sdk::SDK_SCHEMA_VERSION;
use opi_agent::tool::{ExecutionMode, Tool, ToolError, ToolResult};
use opi_ai::message::{AssistantContent, OutputContent, ToolDef};
use opi_ai::provider::Provider;
use opi_ai::test_support::{
    MockProvider, single_route_collection, text_response, tool_call_response,
};
use serde_json::Value;
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Run record
// ---------------------------------------------------------------------------

/// A record of a completed (or cancelled) child agent run.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SubAgentRunRecord {
    /// Unique run ID.
    id: String,
    /// The prompt sent to the child agent.
    prompt: String,
    /// Status: "completed", "error", or "cancelled".
    status: String,
    /// Final text output (if completed successfully).
    result: Option<String>,
    /// Error message (if error or cancelled).
    error: Option<String>,
}

// ---------------------------------------------------------------------------
// Sub-agent extension
// ---------------------------------------------------------------------------

/// A sub-agent extension that runs nested agent workflows via custom commands.
///
/// This is an **example extension** demonstrating how to orchestrate child
/// agents through the extension API. It is NOT core sub-agent functionality.
///
/// # Commands
///
/// - `sub-agent/run { "prompt": "..." }` — Run a child agent and return the
///   result.
/// - `sub-agent/list` — Return the run history.
///
/// # Isolation
///
/// Each child run creates a fresh agent with its own provider, tools, and
/// cancellation token. Child state does not leak to the parent.
struct SubAgentExtension {
    /// Factory for creating child providers.
    provider_factory: Arc<dyn Fn() -> Box<dyn Provider> + Send + Sync>,
    /// Factory for creating child tools.
    tools_factory: Arc<dyn Fn() -> Vec<Box<dyn Tool>> + Send + Sync>,
    /// Model spec for child agents.
    model: String,
    /// Completed run records.
    runs: Arc<Mutex<Vec<SubAgentRunRecord>>>,
    /// Parent agent events received via `on_event`.
    parent_events: Arc<Mutex<Vec<String>>>,
    /// Child agent events collected during runs.
    child_events: Arc<Mutex<Vec<String>>>,
    /// Active child cancellation token (set during a run).
    active_child_cancel: Arc<Mutex<Option<CancellationToken>>>,
    /// Run ID counter.
    next_run_id: AtomicU64,
}

impl SubAgentExtension {
    /// Create a new sub-agent extension with the given factories.
    fn new(
        provider_factory: Arc<dyn Fn() -> Box<dyn Provider> + Send + Sync>,
        tools_factory: Arc<dyn Fn() -> Vec<Box<dyn Tool>> + Send + Sync>,
        model: String,
    ) -> Self {
        Self {
            provider_factory,
            tools_factory,
            model,
            runs: Arc::new(Mutex::new(Vec::new())),
            parent_events: Arc::new(Mutex::new(Vec::new())),
            child_events: Arc::new(Mutex::new(Vec::new())),
            active_child_cancel: Arc::new(Mutex::new(None)),
            next_run_id: AtomicU64::new(1),
        }
    }

    /// Generate the next run ID.
    fn alloc_run_id(&self) -> String {
        let id = self.next_run_id.fetch_add(1, Ordering::SeqCst);
        format!("run-{id}")
    }

    /// Extract the last assistant text from agent messages.
    fn extract_final_text(messages: &[AgentMessage]) -> String {
        messages
            .iter()
            .rev()
            .find_map(|m| {
                if let AgentMessage::Llm(opi_ai::message::Message::Assistant(a)) = m {
                    let text: String = a
                        .content
                        .iter()
                        .filter_map(|c| match c {
                            AssistantContent::Text { text } => Some(text.clone()),
                            _ => None,
                        })
                        .collect();
                    if !text.is_empty() {
                        return Some(text);
                    }
                }
                None
            })
            .unwrap_or_default()
    }
}

impl Extension for SubAgentExtension {
    fn name(&self) -> &str {
        "sub-agent"
    }

    fn on_event(&self, event: &AgentEvent) {
        let label = match event {
            AgentEvent::AgentStart => "AgentStart".to_string(),
            AgentEvent::AgentEnd { .. } => "AgentEnd".to_string(),
            AgentEvent::TurnStart => "TurnStart".to_string(),
            AgentEvent::TurnEnd { .. } => "TurnEnd".to_string(),
            AgentEvent::ToolExecutionStart { tool_name, .. } => {
                format!("ToolExecutionStart({tool_name})")
            }
            AgentEvent::ToolExecutionEnd { tool_name, .. } => {
                format!("ToolExecutionEnd({tool_name})")
            }
            _ => "Other".to_string(),
        };
        self.parent_events.lock().unwrap().push(label);
    }

    fn on_command(
        &self,
        command: &ExtensionCommand,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Value>, ExtensionError>> + Send>> {
        let name = command.name.clone();
        let args = command.args.clone();
        let provider_factory = self.provider_factory.clone();
        let tools_factory = self.tools_factory.clone();
        let model = self.model.clone();
        let runs = self.runs.clone();
        let child_events = self.child_events.clone();
        let active_cancel = self.active_child_cancel.clone();
        let run_id = self.alloc_run_id();

        Box::pin(async move {
            match name.as_str() {
                "sub-agent/run" => {
                    let prompt = args["prompt"].as_str().unwrap_or("").to_string();

                    // Create child agent with isolated state.
                    let child_provider = provider_factory();
                    let child_tools = tools_factory();

                    let child_hooks = Box::new(ChildHooks) as Box<dyn AgentHooks>;
                    let mut child_agent = opi_agent::Agent::new(
                        Arc::new(single_route_collection(child_provider)),
                        child_tools,
                        model,
                        None,
                        InferenceConfig::default(),
                        AgentLoopConfig {
                            max_turns: 10,
                            ..Default::default()
                        },
                        child_hooks,
                    )
                    .expect("agent");

                    // Subscribe to child events.
                    let ce = child_events.clone();
                    child_agent.subscribe(Box::new(move |event: &AgentEvent| {
                        let label = match event {
                            AgentEvent::AgentStart => "ChildAgentStart".to_string(),
                            AgentEvent::AgentEnd { .. } => "ChildAgentEnd".to_string(),
                            AgentEvent::TurnStart => "ChildTurnStart".to_string(),
                            AgentEvent::TurnEnd { .. } => "ChildTurnEnd".to_string(),
                            AgentEvent::ToolExecutionStart { tool_name, .. } => {
                                format!("ChildToolExecutionStart({tool_name})")
                            }
                            AgentEvent::ToolExecutionEnd { tool_name, .. } => {
                                format!("ChildToolExecutionEnd({tool_name})")
                            }
                            _ => "ChildOther".to_string(),
                        };
                        ce.lock().unwrap().push(label);
                    }));

                    // Store the child cancel token so tests can cancel it.
                    let child_token = child_agent.cancel_token();
                    *active_cancel.lock().unwrap() = Some(child_token);

                    let result = child_agent.prompt(&prompt).await;

                    // Clear the active cancel token.
                    *active_cancel.lock().unwrap() = None;

                    let record = match result {
                        Ok(messages) => {
                            let text = Self::extract_final_text(&messages);
                            SubAgentRunRecord {
                                id: run_id,
                                prompt,
                                status: "completed".to_string(),
                                result: Some(text),
                                error: None,
                            }
                        }
                        Err(AgentError::Cancelled) => SubAgentRunRecord {
                            id: run_id,
                            prompt,
                            status: "cancelled".to_string(),
                            result: None,
                            error: Some("cancelled".to_string()),
                        },
                        Err(e) => SubAgentRunRecord {
                            id: run_id,
                            prompt,
                            status: "error".to_string(),
                            result: None,
                            error: Some(e.to_string()),
                        },
                    };

                    let response_text = record
                        .result
                        .clone()
                        .unwrap_or_else(|| record.error.clone().unwrap_or_default());
                    let status = record.status.clone();
                    let rid = record.id.clone();

                    runs.lock().unwrap().push(record);

                    Ok(Some(serde_json::json!({
                        "run_id": rid,
                        "status": status,
                        "result": response_text,
                        "sdk_schema_version": SDK_SCHEMA_VERSION,
                    })))
                }
                "sub-agent/list" => {
                    let runs_guard = runs.lock().unwrap();
                    let run_list: Vec<Value> = runs_guard
                        .iter()
                        .map(|r| {
                            serde_json::json!({
                                "id": r.id,
                                "prompt": r.prompt,
                                "status": r.status,
                                "result": r.result,
                                "error": r.error,
                            })
                        })
                        .collect();
                    Ok(Some(serde_json::json!({ "runs": run_list })))
                }
                _ => Ok(None),
            }
        })
    }

    fn serialize_state(&self) -> Result<Option<Value>, ExtensionError> {
        let runs = self.runs.lock().unwrap();
        Ok(Some(serde_json::json!({
            "runs": serde_json::to_value(&*runs).unwrap_or(serde_json::json!([])),
            "sdk_schema_version": SDK_SCHEMA_VERSION,
        })))
    }

    fn restore_state(&self, state: Value) -> Result<(), ExtensionError> {
        if let Some(runs_val) = state["runs"].as_array() {
            let mut runs = self.runs.lock().unwrap();
            runs.clear();
            for r in runs_val {
                if let Ok(record) = serde_json::from_value::<SubAgentRunRecord>(r.clone()) {
                    runs.push(record);
                }
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Child hooks (minimal)
// ---------------------------------------------------------------------------

/// Minimal hooks for child agents.
struct ChildHooks;

impl AgentHooks for ChildHooks {
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
// Test helpers
// ---------------------------------------------------------------------------

/// A dummy tool that succeeds with "child-ok".
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
            "description": format!("{} tool", self.name),
            "input_schema": { "type": "object", "properties": {} }
        }))
        .unwrap()
    }

    fn execute(
        &self,
        _call_id: &str,
        _arguments: Value,
        _signal: CancellationToken,
        _on_update: Option<opi_agent::tool::UpdateCallback>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send>> {
        Box::pin(async {
            Ok(ToolResult {
                content: vec![OutputContent::Text {
                    text: "child-ok".into(),
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

/// A dummy tool that blocks until cancelled.
struct BlockingTool;

impl Tool for BlockingTool {
    fn definition(&self) -> ToolDef {
        serde_json::from_value(serde_json::json!({
            "name": "blocking",
            "description": "A tool that blocks until cancelled",
            "input_schema": { "type": "object", "properties": {} }
        }))
        .unwrap()
    }

    fn execute(
        &self,
        _call_id: &str,
        _arguments: Value,
        signal: CancellationToken,
        _on_update: Option<opi_agent::tool::UpdateCallback>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send>> {
        Box::pin(async move {
            signal.cancelled().await;
            Ok(ToolResult {
                content: vec![OutputContent::Text {
                    text: "cancelled".into(),
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
        ExecutionMode::Sequential
    }
}

// ---------------------------------------------------------------------------
// Tests: Child completion
// ---------------------------------------------------------------------------

#[tokio::test]
async fn child_run_completes_and_result_routed_to_parent() {
    let ext = SubAgentExtension::new(
        Arc::new(|| {
            Box::new(MockProvider::new(
                "child",
                vec![text_response("Child says hello")],
            )) as Box<dyn Provider>
        }),
        Arc::new(Vec::new as fn() -> Vec<Box<dyn Tool>>),
        "child:mock-model".into(),
    );
    let runs = ext.runs.clone();
    let child_events = ext.child_events.clone();

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    let cmd = ExtensionCommand::new("sub-agent/run", serde_json::json!({ "prompt": "hello" }));
    let result = registry.dispatch_command(&cmd).await.unwrap().unwrap();

    assert_eq!(result["status"], "completed");
    assert_eq!(result["result"], "Child says hello");
    assert_eq!(result["sdk_schema_version"], SDK_SCHEMA_VERSION);

    // Run recorded in history.
    let runs_guard = runs.lock().unwrap();
    assert_eq!(runs_guard.len(), 1);
    assert_eq!(runs_guard[0].status, "completed");
    assert_eq!(runs_guard[0].result.as_deref(), Some("Child says hello"));

    // Child events were collected.
    let ce = child_events.lock().unwrap();
    assert!(
        ce.iter().any(|e| e == "ChildAgentStart"),
        "should have ChildAgentStart, got: {ce:?}"
    );
    assert!(
        ce.iter().any(|e| e == "ChildAgentEnd"),
        "should have ChildAgentEnd, got: {ce:?}"
    );
}

#[tokio::test]
async fn child_run_with_tool_call_completes() {
    let ext = SubAgentExtension::new(
        Arc::new(|| {
            Box::new(MockProvider::new(
                "child",
                vec![
                    tool_call_response("tc_1", "read", r#"{"path":"/tmp/f"}"#),
                    text_response("Read result: contents"),
                ],
            )) as Box<dyn Provider>
        }),
        Arc::new(|| vec![Box::new(DummyTool::new("read"))]),
        "child:mock-model".into(),
    );
    let child_events = ext.child_events.clone();

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    let cmd = ExtensionCommand::new(
        "sub-agent/run",
        serde_json::json!({ "prompt": "read the file" }),
    );
    let result = registry.dispatch_command(&cmd).await.unwrap().unwrap();

    assert_eq!(result["status"], "completed");
    assert_eq!(result["result"], "Read result: contents");

    // Child tool events were collected.
    let ce = child_events.lock().unwrap();
    assert!(
        ce.iter().any(|e| e == "ChildToolExecutionStart(read)"),
        "should have ChildToolExecutionStart(read), got: {ce:?}"
    );
    assert!(
        ce.iter().any(|e| e == "ChildToolExecutionEnd(read)"),
        "should have ChildToolExecutionEnd(read), got: {ce:?}"
    );
}

// ---------------------------------------------------------------------------
// Tests: Child error propagation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn child_provider_error_propagates_to_parent() {
    let ext = SubAgentExtension::new(
        Arc::new(|| {
            Box::new(MockProvider::new_with_errors(
                "child",
                vec![opi_ai::test_support::MockResponse::Error(
                    opi_ai::provider::ProviderError::AuthFailed("bad key".into()),
                )],
            )) as Box<dyn Provider>
        }),
        Arc::new(Vec::new as fn() -> Vec<Box<dyn Tool>>),
        "child:mock-model".into(),
    );
    let runs = ext.runs.clone();

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    let cmd = ExtensionCommand::new("sub-agent/run", serde_json::json!({ "prompt": "test" }));
    let result = registry.dispatch_command(&cmd).await.unwrap().unwrap();

    assert_eq!(result["status"], "error");
    assert!(
        result["result"]
            .as_str()
            .unwrap()
            .contains("authentication failed"),
        "error should mention auth failure, got: {}",
        result["result"]
    );

    // Error recorded in history.
    let runs_guard = runs.lock().unwrap();
    assert_eq!(runs_guard.len(), 1);
    assert_eq!(runs_guard[0].status, "error");
    assert!(runs_guard[0].error.is_some());
}

// ---------------------------------------------------------------------------
// Tests: Cancellation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn child_run_cancelled_mid_execution() {
    let ext = SubAgentExtension::new(
        Arc::new(|| {
            Box::new(MockProvider::new(
                "child",
                vec![
                    // First turn: call the blocking tool.
                    tool_call_response("tc_1", "blocking", "{}"),
                    // Second turn (never reached if cancelled): final text.
                    text_response("done"),
                ],
            )) as Box<dyn Provider>
        }),
        Arc::new(|| vec![Box::new(BlockingTool)]),
        "child:mock-model".into(),
    );
    let child_events = ext.child_events.clone();
    let runs = ext.runs.clone();

    // The extension sets active_child_cancel when the child agent is created.
    // We poll it to grab the cancellation token.
    let ext_active_cancel = ext.active_child_cancel.clone();

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    let cmd = ExtensionCommand::new("sub-agent/run", serde_json::json!({ "prompt": "run" }));

    // Spawn the dispatch in a background task.
    let registry_handle = Arc::new(registry);
    let registry_for_task = registry_handle.clone();
    let cmd_for_task = cmd.clone();

    let task = tokio::spawn(async move {
        registry_for_task
            .dispatch_command(&cmd_for_task)
            .await
            .unwrap()
            .unwrap()
    });

    // Wait for the child agent to start and set its cancel token.
    let cancel_token = {
        loop {
            tokio::task::yield_now().await;
            let guard = ext_active_cancel.lock().unwrap();
            if guard.is_some() {
                break guard.clone().unwrap();
            }
        }
    };

    // Cancel the child.
    cancel_token.cancel();

    // Await the task result.
    let result = task.await.unwrap();
    assert_eq!(result["status"], "cancelled");

    // Cancellation recorded in history.
    let runs_guard = runs.lock().unwrap();
    assert_eq!(runs_guard.len(), 1);
    assert_eq!(runs_guard[0].status, "cancelled");

    // Child events should show start but end with cancellation.
    let ce = child_events.lock().unwrap();
    assert!(
        ce.iter().any(|e| e == "ChildAgentStart"),
        "should have ChildAgentStart, got: {ce:?}"
    );
}

// ---------------------------------------------------------------------------
// Tests: Event routing
// ---------------------------------------------------------------------------

#[tokio::test]
async fn child_events_observable_by_parent() {
    let ext = SubAgentExtension::new(
        Arc::new(|| {
            Box::new(MockProvider::new(
                "child",
                vec![
                    tool_call_response("tc_1", "search", r#"{"query":"test"}"#),
                    text_response("Found results"),
                ],
            )) as Box<dyn Provider>
        }),
        Arc::new(|| vec![Box::new(DummyTool::new("search"))]),
        "child:mock-model".into(),
    );
    let child_events = ext.child_events.clone();

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    let cmd = ExtensionCommand::new(
        "sub-agent/run",
        serde_json::json!({ "prompt": "search for X" }),
    );
    let _ = registry.dispatch_command(&cmd).await.unwrap().unwrap();

    let ce = child_events.lock().unwrap();

    // Verify full lifecycle.
    assert!(ce.iter().any(|e| e == "ChildAgentStart"));
    assert!(ce.iter().any(|e| e == "ChildTurnStart"));
    assert!(ce.iter().any(|e| e == "ChildToolExecutionStart(search)"));
    assert!(ce.iter().any(|e| e == "ChildToolExecutionEnd(search)"));
    assert!(ce.iter().any(|e| e == "ChildTurnEnd"));
    assert!(ce.iter().any(|e| e == "ChildAgentEnd"));
}

#[tokio::test]
async fn extension_receives_parent_agent_events() {
    let ext = SubAgentExtension::new(
        Arc::new(|| {
            Box::new(MockProvider::new("child", vec![text_response("ok")])) as Box<dyn Provider>
        }),
        Arc::new(Vec::new as fn() -> Vec<Box<dyn Tool>>),
        "child:mock-model".into(),
    );
    let parent_events = ext.parent_events.clone();

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    let base_sink = Box::new(|_: AgentEvent| {}) as Box<dyn Fn(AgentEvent) + Send + Sync>;
    let wrapped_sink = registry.wrap_event_sink(base_sink);

    wrapped_sink(AgentEvent::AgentStart);
    wrapped_sink(AgentEvent::TurnStart);
    wrapped_sink(AgentEvent::ToolExecutionStart {
        tool_call_id: "tc_1".into(),
        tool_name: "read".into(),
        args: serde_json::json!({}),
    });

    let received = parent_events.lock().unwrap();
    assert!(
        received.contains(&"AgentStart".to_string()),
        "should have AgentStart"
    );
    assert!(
        received.contains(&"TurnStart".to_string()),
        "should have TurnStart"
    );
    assert!(
        received.contains(&"ToolExecutionStart(read)".to_string()),
        "should have ToolExecutionStart(read)"
    );
}

// ---------------------------------------------------------------------------
// Tests: Isolated state
// ---------------------------------------------------------------------------

#[tokio::test]
async fn multiple_child_runs_have_isolated_state() {
    let call_count = Arc::new(AtomicU64::new(0));
    let call_count_clone = call_count.clone();

    let ext = SubAgentExtension::new(
        Arc::new(move || {
            let count = call_count_clone.fetch_add(1, Ordering::SeqCst);
            // First call returns "alpha", second returns "beta".
            if count == 0 {
                Box::new(MockProvider::new("child", vec![text_response("alpha")]))
                    as Box<dyn Provider>
            } else {
                Box::new(MockProvider::new("child", vec![text_response("beta")]))
                    as Box<dyn Provider>
            }
        }),
        Arc::new(Vec::new as fn() -> Vec<Box<dyn Tool>>),
        "child:mock-model".into(),
    );
    let runs = ext.runs.clone();

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    // First run.
    let cmd1 = ExtensionCommand::new("sub-agent/run", serde_json::json!({ "prompt": "first" }));
    let result1 = registry.dispatch_command(&cmd1).await.unwrap().unwrap();
    assert_eq!(result1["status"], "completed");
    assert_eq!(result1["result"], "alpha");

    // Second run — fresh provider, independent state.
    let cmd2 = ExtensionCommand::new("sub-agent/run", serde_json::json!({ "prompt": "second" }));
    let result2 = registry.dispatch_command(&cmd2).await.unwrap().unwrap();
    assert_eq!(result2["status"], "completed");
    assert_eq!(result2["result"], "beta");

    // Both runs recorded independently.
    let runs_guard = runs.lock().unwrap();
    assert_eq!(runs_guard.len(), 2);
    assert_eq!(runs_guard[0].prompt, "first");
    assert_eq!(runs_guard[0].result.as_deref(), Some("alpha"));
    assert_eq!(runs_guard[1].prompt, "second");
    assert_eq!(runs_guard[1].result.as_deref(), Some("beta"));
}

// ---------------------------------------------------------------------------
// Tests: Session visibility
// ---------------------------------------------------------------------------

#[tokio::test]
async fn run_history_visible_via_list_command() {
    let ext = SubAgentExtension::new(
        Arc::new(|| {
            Box::new(MockProvider::new("child", vec![text_response("ok")])) as Box<dyn Provider>
        }),
        Arc::new(Vec::new as fn() -> Vec<Box<dyn Tool>>),
        "child:mock-model".into(),
    );

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    // Run two child agents.
    let cmd1 = ExtensionCommand::new("sub-agent/run", serde_json::json!({ "prompt": "task A" }));
    let _ = registry.dispatch_command(&cmd1).await.unwrap().unwrap();

    let cmd2 = ExtensionCommand::new("sub-agent/run", serde_json::json!({ "prompt": "task B" }));
    let _ = registry.dispatch_command(&cmd2).await.unwrap().unwrap();

    // List runs.
    let list_cmd = ExtensionCommand::new("sub-agent/list", serde_json::json!({}));
    let list_result = registry.dispatch_command(&list_cmd).await.unwrap().unwrap();

    let runs = list_result["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0]["prompt"], "task A");
    assert_eq!(runs[0]["status"], "completed");
    assert_eq!(runs[1]["prompt"], "task B");
    assert_eq!(runs[1]["status"], "completed");
}

// ---------------------------------------------------------------------------
// Tests: State round-trip
// ---------------------------------------------------------------------------

#[tokio::test]
async fn run_history_round_trips_through_serialization() {
    let ext = SubAgentExtension::new(
        Arc::new(|| {
            Box::new(MockProvider::new("child", vec![text_response("ok")])) as Box<dyn Provider>
        }),
        Arc::new(Vec::new as fn() -> Vec<Box<dyn Tool>>),
        "child:mock-model".into(),
    );

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    // Run a child agent to populate history.
    let cmd = ExtensionCommand::new("sub-agent/run", serde_json::json!({ "prompt": "test" }));
    let _ = registry.dispatch_command(&cmd).await.unwrap().unwrap();

    // Serialize extension states.
    let states = registry.serialize_states().unwrap();
    assert!(states["sub-agent"]["runs"].is_array());

    // Create a new extension and restore state.
    let ext2 = SubAgentExtension::new(
        Arc::new(|| {
            Box::new(MockProvider::new("child", vec![text_response("ok")])) as Box<dyn Provider>
        }),
        Arc::new(Vec::new as fn() -> Vec<Box<dyn Tool>>),
        "child:mock-model".into(),
    );

    let mut registry2 = ExtensionRegistry::new();
    registry2.register(Box::new(ext2)).unwrap();

    registry2.restore_states(states).unwrap();

    // List should show the restored run.
    let list_cmd = ExtensionCommand::new("sub-agent/list", serde_json::json!({}));
    let list_result = registry2
        .dispatch_command(&list_cmd)
        .await
        .unwrap()
        .unwrap();

    let runs = list_result["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0]["prompt"], "test");
    assert_eq!(runs[0]["status"], "completed");
}

// ---------------------------------------------------------------------------
// Tests: Unknown command passthrough
// ---------------------------------------------------------------------------

#[tokio::test]
async fn unknown_command_returns_none() {
    let ext = SubAgentExtension::new(
        Arc::new(|| {
            Box::new(MockProvider::new("child", vec![text_response("ok")])) as Box<dyn Provider>
        }),
        Arc::new(Vec::new as fn() -> Vec<Box<dyn Tool>>),
        "child:mock-model".into(),
    );

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    let cmd = ExtensionCommand::new("unknown/command", serde_json::json!({}));
    let result = registry.dispatch_command(&cmd).await.unwrap();
    assert!(
        result.is_none(),
        "unknown command should return None, got: {result:?}"
    );
}
