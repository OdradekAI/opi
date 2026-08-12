//! Todo extension/package example.
//!
//! Demonstrates a task-tracking workflow through extension state and custom
//! commands without adding core runtime state management. The todo extension
//! stores items, tracks status changes, emits observable events, and persists
//! state through serialization.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use opi_agent::event::AgentEvent;
use opi_agent::extension::{Extension, ExtensionCommand, ExtensionError, ExtensionRegistry};
use opi_agent::hooks::AgentHooks;
use opi_agent::loop_types::{AgentError, InferenceConfig};
use opi_agent::message::AgentMessage;
use opi_agent::tool::{ExecutionMode, Tool, ToolError, ToolResult};
use opi_ai::message::{OutputContent, ToolDef};
use opi_ai::test_support::{MockProvider, single_route_collection, text_response};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Todo types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum TodoStatus {
    Pending,
    InProgress,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TodoItem {
    id: String,
    title: String,
    description: String,
    status: TodoStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TodoState {
    items: Vec<TodoItem>,
    next_id: u64,
    events_log: Vec<String>,
}

impl Default for TodoState {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            next_id: 1,
            events_log: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// TodoExtension
// ---------------------------------------------------------------------------

struct TodoExtension {
    state: Arc<Mutex<TodoState>>,
    events_received: Arc<Mutex<Vec<String>>>,
}

impl TodoExtension {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(TodoState::default())),
            events_received: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Extension for TodoExtension {
    fn name(&self) -> &str {
        "todo"
    }

    fn on_command(
        &self,
        command: &ExtensionCommand,
    ) -> Pin<Box<dyn Future<Output = Result<Option<serde_json::Value>, ExtensionError>> + Send>>
    {
        let cmd = command.name.clone();
        let args = command.args.clone();
        let state = self.state.clone();

        Box::pin(async move {
            match cmd.as_str() {
                "todo/add" => {
                    let title = args["title"]
                        .as_str()
                        .ok_or_else(|| ExtensionError::CommandError("title is required".into()))?
                        .to_string();
                    let description = args["description"].as_str().unwrap_or("").to_string();

                    let mut s = state.lock().unwrap();
                    let id = format!("todo-{}", s.next_id);
                    s.next_id += 1;

                    let item = TodoItem {
                        id: id.clone(),
                        title: title.clone(),
                        description,
                        status: TodoStatus::Pending,
                    };
                    s.items.push(item);
                    s.events_log.push(format!("added: {} ({})", title, id));

                    Ok(Some(serde_json::json!({
                        "id": id,
                        "status": "pending",
                    })))
                }
                "todo/update" => {
                    let id = args["id"]
                        .as_str()
                        .ok_or_else(|| ExtensionError::CommandError("id is required".into()))?
                        .to_string();

                    let (title, status_str) = {
                        let mut s = state.lock().unwrap();
                        let item = s.items.iter_mut().find(|i| i.id == id).ok_or_else(|| {
                            ExtensionError::CommandError(format!("todo '{}' not found", id))
                        })?;

                        if let Some(title) = args["title"].as_str() {
                            item.title = title.to_string();
                        }
                        if let Some(desc) = args["description"].as_str() {
                            item.description = desc.to_string();
                        }
                        if let Some(status_str) = args["status"].as_str() {
                            let new_status = match status_str {
                                "pending" => TodoStatus::Pending,
                                "in_progress" => TodoStatus::InProgress,
                                "completed" => TodoStatus::Completed,
                                _ => {
                                    return Err(ExtensionError::CommandError(format!(
                                        "invalid status: {}",
                                        status_str
                                    )));
                                }
                            };
                            item.status = new_status;
                        }

                        let status_str = match item.status {
                            TodoStatus::Pending => "pending",
                            TodoStatus::InProgress => "in_progress",
                            TodoStatus::Completed => "completed",
                        };
                        let title_out = item.title.clone();
                        s.events_log.push(format!("updated: {}", id));
                        (title_out, status_str.to_string())
                    };

                    Ok(Some(serde_json::json!({
                        "id": id,
                        "status": status_str,
                        "title": title,
                    })))
                }
                "todo/list" => {
                    let s = state.lock().unwrap();
                    let items: Vec<serde_json::Value> = s
                        .items
                        .iter()
                        .map(|i| {
                            let status_str = match i.status {
                                TodoStatus::Pending => "pending",
                                TodoStatus::InProgress => "in_progress",
                                TodoStatus::Completed => "completed",
                            };
                            serde_json::json!({
                                "id": i.id,
                                "title": i.title,
                                "description": i.description,
                                "status": status_str,
                            })
                        })
                        .collect();

                    Ok(Some(serde_json::json!({
                        "items": items,
                        "total": items.len(),
                    })))
                }
                "todo/complete" => {
                    let id = args["id"]
                        .as_str()
                        .ok_or_else(|| ExtensionError::CommandError("id is required".into()))?
                        .to_string();

                    let mut s = state.lock().unwrap();
                    let item = s.items.iter_mut().find(|i| i.id == id).ok_or_else(|| {
                        ExtensionError::CommandError(format!("todo '{}' not found", id))
                    })?;

                    item.status = TodoStatus::Completed;
                    s.events_log.push(format!("completed: {}", id));

                    Ok(Some(serde_json::json!({
                        "id": id,
                        "status": "completed",
                    })))
                }
                _ => Ok(None),
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
        let val = serde_json::to_value(TodoState {
            items: s.items.clone(),
            next_id: s.next_id,
            events_log: s.events_log.clone(),
        })
        .map_err(|e| ExtensionError::StateSerialization {
            name: "todo".into(),
            reason: e.to_string(),
        })?;
        Ok(Some(val))
    }

    fn restore_state(&self, state_val: serde_json::Value) -> Result<(), ExtensionError> {
        let parsed: TodoState =
            serde_json::from_value(state_val).map_err(|e| ExtensionError::StateRestoration {
                name: "todo".into(),
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn add_creates_todo_item() {
    let ext = TodoExtension::new();
    let state = ext.state.clone();

    let cmd = ExtensionCommand::new(
        "todo/add",
        serde_json::json!({"title": "write tests", "description": "unit and integration"}),
    );
    let result = ext.on_command(&cmd).await.unwrap().unwrap();

    assert_eq!(result["status"], "pending");
    assert_eq!(result["id"], "todo-1");

    let s = state.lock().unwrap();
    assert_eq!(s.items.len(), 1);
    assert_eq!(s.items[0].title, "write tests");
    assert_eq!(s.items[0].description, "unit and integration");
    assert_eq!(s.items[0].status, TodoStatus::Pending);
    assert_eq!(s.next_id, 2);
}

#[tokio::test]
async fn add_requires_title() {
    let ext = TodoExtension::new();

    let cmd = ExtensionCommand::new("todo/add", serde_json::json!({}));
    let result = ext.on_command(&cmd).await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("title is required"));
}

#[tokio::test]
async fn update_changes_item_fields() {
    let ext = TodoExtension::new();
    let state = ext.state.clone();

    // Add an item first
    let add_cmd = ExtensionCommand::new("todo/add", serde_json::json!({"title": "original"}));
    ext.on_command(&add_cmd).await.unwrap();

    // Update it
    let update_cmd = ExtensionCommand::new(
        "todo/update",
        serde_json::json!({"id": "todo-1", "title": "updated", "status": "in_progress"}),
    );
    let result = ext.on_command(&update_cmd).await.unwrap().unwrap();

    assert_eq!(result["status"], "in_progress");
    assert_eq!(result["title"], "updated");

    let s = state.lock().unwrap();
    assert_eq!(s.items[0].title, "updated");
    assert_eq!(s.items[0].status, TodoStatus::InProgress);
}

#[tokio::test]
async fn update_rejects_unknown_id() {
    let ext = TodoExtension::new();

    let cmd = ExtensionCommand::new(
        "todo/update",
        serde_json::json!({"id": "todo-999", "title": "nope"}),
    );
    let result = ext.on_command(&cmd).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[tokio::test]
async fn update_rejects_invalid_status() {
    let ext = TodoExtension::new();

    let add_cmd = ExtensionCommand::new("todo/add", serde_json::json!({"title": "task"}));
    ext.on_command(&add_cmd).await.unwrap();

    let update_cmd = ExtensionCommand::new(
        "todo/update",
        serde_json::json!({"id": "todo-1", "status": "bogus"}),
    );
    let result = ext.on_command(&update_cmd).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("invalid status"));
}

#[tokio::test]
async fn list_returns_all_items() {
    let ext = TodoExtension::new();

    let add1 = ExtensionCommand::new("todo/add", serde_json::json!({"title": "task a"}));
    let add2 = ExtensionCommand::new("todo/add", serde_json::json!({"title": "task b"}));
    ext.on_command(&add1).await.unwrap();
    ext.on_command(&add2).await.unwrap();

    let list_cmd = ExtensionCommand::new("todo/list", serde_json::json!({}));
    let result = ext.on_command(&list_cmd).await.unwrap().unwrap();

    assert_eq!(result["total"], 2);
    let items = result["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["title"], "task a");
    assert_eq!(items[1]["title"], "task b");
}

#[tokio::test]
async fn complete_marks_item_done() {
    let ext = TodoExtension::new();
    let state = ext.state.clone();

    let add_cmd = ExtensionCommand::new("todo/add", serde_json::json!({"title": "finish thing"}));
    ext.on_command(&add_cmd).await.unwrap();

    let complete_cmd = ExtensionCommand::new("todo/complete", serde_json::json!({"id": "todo-1"}));
    let result = ext.on_command(&complete_cmd).await.unwrap().unwrap();

    assert_eq!(result["status"], "completed");
    assert_eq!(result["id"], "todo-1");

    let s = state.lock().unwrap();
    assert_eq!(s.items[0].status, TodoStatus::Completed);
}

#[tokio::test]
async fn complete_rejects_unknown_id() {
    let ext = TodoExtension::new();

    let cmd = ExtensionCommand::new("todo/complete", serde_json::json!({"id": "todo-999"}));
    let result = ext.on_command(&cmd).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[tokio::test]
async fn state_round_trips_through_serialization() {
    let ext = TodoExtension::new();

    let add1 = ExtensionCommand::new(
        "todo/add",
        serde_json::json!({"title": "task one", "description": "desc"}),
    );
    ext.on_command(&add1).await.unwrap();

    let add2 = ExtensionCommand::new("todo/add", serde_json::json!({"title": "task two"}));
    ext.on_command(&add2).await.unwrap();

    let complete_cmd = ExtensionCommand::new("todo/complete", serde_json::json!({"id": "todo-1"}));
    ext.on_command(&complete_cmd).await.unwrap();

    // Serialize
    let serialized = ext.serialize_state().unwrap().unwrap();
    assert_eq!(serialized["items"].as_array().unwrap().len(), 2);
    assert_eq!(serialized["next_id"], 3);

    // Restore into a new extension
    let ext2 = TodoExtension::new();
    ext2.restore_state(serialized).unwrap();

    let s = ext2.state.lock().unwrap();
    assert_eq!(s.items.len(), 2);
    assert_eq!(s.items[0].title, "task one");
    assert_eq!(s.items[0].status, TodoStatus::Completed);
    assert_eq!(s.items[1].title, "task two");
    assert_eq!(s.items[1].status, TodoStatus::Pending);
    assert_eq!(s.next_id, 3);
    assert!(
        s.events_log
            .contains(&"added: task one (todo-1)".to_string())
    );
}

#[tokio::test]
async fn extension_receives_parent_agent_events() {
    let ext = TodoExtension::new();
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
async fn session_integration_with_agent() {
    let ext = TodoExtension::new();
    let state = ext.state.clone();

    // Add a todo via command before registering
    let add_cmd = ExtensionCommand::new(
        "todo/add",
        serde_json::json!({"title": "pre-existing task"}),
    );
    ext.on_command(&add_cmd).await.unwrap();

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    // Provider returns a text response
    let provider = MockProvider::new("mock", vec![text_response("done")]);

    let hooks = registry.wrap_hooks(Box::new(TestHooks));
    let mut agent = opi_agent::Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        vec![Box::new(DummyTool::new("read"))],
        "mock:mock-model".into(),
        None,
        InferenceConfig::default(),
        Default::default(),
        hooks,
    )
    .expect("agent");

    let _result = agent.prompt("test").await.unwrap();

    // The pre-existing task should still be in state after agent run
    let s = state.lock().unwrap();
    assert_eq!(s.items.len(), 1);
    assert_eq!(s.items[0].title, "pre-existing task");
}

#[tokio::test]
async fn failure_recovery_preserves_state() {
    let ext = TodoExtension::new();

    let add_cmd = ExtensionCommand::new("todo/add", serde_json::json!({"title": "important"}));
    ext.on_command(&add_cmd).await.unwrap();

    // Serialize state (simulate checkpoint before failure)
    let checkpoint = ext.serialize_state().unwrap().unwrap();

    // Simulate corruption by creating a fresh extension
    let ext2 = TodoExtension::new();
    assert!(ext2.state.lock().unwrap().items.is_empty());

    // Restore from checkpoint (simulate recovery)
    ext2.restore_state(checkpoint).unwrap();

    let s = ext2.state.lock().unwrap();
    assert_eq!(s.items.len(), 1);
    assert_eq!(s.items[0].title, "important");
}

#[tokio::test]
async fn unknown_command_returns_none() {
    let ext = TodoExtension::new();
    let cmd = ExtensionCommand::new("todo/unknown", serde_json::json!({}));
    let result = ext.on_command(&cmd).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn events_log_tracks_operations() {
    let ext = TodoExtension::new();
    let state = ext.state.clone();

    // Add
    let add_cmd = ExtensionCommand::new("todo/add", serde_json::json!({"title": "task x"}));
    ext.on_command(&add_cmd).await.unwrap();

    // Update
    let update_cmd = ExtensionCommand::new(
        "todo/update",
        serde_json::json!({"id": "todo-1", "status": "in_progress"}),
    );
    ext.on_command(&update_cmd).await.unwrap();

    // Complete
    let complete_cmd = ExtensionCommand::new("todo/complete", serde_json::json!({"id": "todo-1"}));
    ext.on_command(&complete_cmd).await.unwrap();

    let s = state.lock().unwrap();
    assert_eq!(s.events_log.len(), 3);
    assert_eq!(s.events_log[0], "added: task x (todo-1)");
    assert_eq!(s.events_log[1], "updated: todo-1");
    assert_eq!(s.events_log[2], "completed: todo-1");
}

#[tokio::test]
async fn multiple_items_have_sequential_ids() {
    let ext = TodoExtension::new();

    for i in 0..3 {
        let add_cmd = ExtensionCommand::new(
            "todo/add",
            serde_json::json!({"title": format!("task {}", i)}),
        );
        let result = ext.on_command(&add_cmd).await.unwrap().unwrap();
        assert_eq!(result["id"], format!("todo-{}", i + 1));
    }

    let list_cmd = ExtensionCommand::new("todo/list", serde_json::json!({}));
    let result = ext.on_command(&list_cmd).await.unwrap().unwrap();
    assert_eq!(result["total"], 3);
}

#[tokio::test]
async fn add_without_description_defaults_empty() {
    let ext = TodoExtension::new();
    let state = ext.state.clone();

    let add_cmd = ExtensionCommand::new("todo/add", serde_json::json!({"title": "no desc"}));
    ext.on_command(&add_cmd).await.unwrap();

    let s = state.lock().unwrap();
    assert_eq!(s.items[0].description, "");
}
