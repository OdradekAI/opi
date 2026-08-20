//! MCP adapter extension/package example tests (task 4.8.6).
//!
//! These tests demonstrate an MCP adapter extension that maps MCP-style
//! tools/resources through the extension API without making MCP a core feature.
//! All tool/resource data comes from fixtures — no live network calls.
//!
//! # What This Example Demonstrates
//!
//! - **Tool discovery**: List available MCP tools with schemas.
//! - **Argument validation**: Tool calls validated against input schemas.
//! - **Tool execution success/error**: Tools return results or structured errors.
//! - **Resource metadata**: List and retrieve MCP resources.
//! - **Cancellation**: Long-running tools respect cancellation tokens.
//! - **State persistence**: Tool/resource registry round-trips through serialization.
//!
//! # Example vs Core MCP
//!
//! The MCP adapter lives entirely in extension code. It uses the standard
//! [`Extension::on_command`] to dispatch MCP-style operations and does not
//! introduce any MCP protocol or transport into the core runtime.

mod common;

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use opi_agent::event::AgentEvent;
use opi_agent::extension::{Extension, ExtensionCommand, ExtensionError, ExtensionRegistry};
use opi_agent::hooks::AgentHooks;
use opi_agent::loop_types::AgentError;
use opi_agent::message::AgentMessage;
use opi_agent::tool::{ExecutionMode, Tool, ToolError, ToolResult};
use opi_ai::message::{OutputContent, ToolDef};
use opi_ai::test_support::{MockProvider, single_route_collection, text_response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// MCP fixture types
// ---------------------------------------------------------------------------

/// An MCP-style tool definition with fixture execution behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct McpToolDef {
    name: String,
    description: String,
    input_schema: Value,
    /// The kind of fixture behavior for this tool.
    #[serde(default)]
    behavior: McpToolBehavior,
}

/// Fixture behavior for an MCP tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
enum McpToolBehavior {
    /// Returns a fixed JSON response.
    Fixed(Value),
    /// Adds two numbers (expects "a" and "b" arguments).
    Calculator,
    /// Blocks until cancelled.
    BlockUntilCancelled,
    /// Always returns an error.
    AlwaysError(String),
}

impl Default for McpToolBehavior {
    fn default() -> Self {
        Self::Fixed(Value::Null)
    }
}

/// An MCP-style resource definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct McpResourceDef {
    uri: String,
    name: String,
    description: String,
    mime_type: String,
    /// The fixture content of the resource.
    content: String,
}

/// A log entry for MCP operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct McpLogEntry {
    timestamp: String,
    operation: String,
    detail: String,
}

/// State for the MCP adapter extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct McpAdapterState {
    tools: Vec<McpToolDef>,
    resources: Vec<McpResourceDef>,
    log: Vec<McpLogEntry>,
}

impl Default for McpAdapterState {
    fn default() -> Self {
        Self {
            tools: Self::fixture_tools(),
            resources: Self::fixture_resources(),
            log: Vec::new(),
        }
    }
}

impl McpAdapterState {
    /// Build the standard fixture tools.
    fn fixture_tools() -> Vec<McpToolDef> {
        vec![
            McpToolDef {
                name: "weather/get".into(),
                description: "Get weather for a location".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "location": { "type": "string", "description": "City name" }
                    },
                    "required": ["location"]
                }),
                behavior: McpToolBehavior::Fixed(serde_json::json!({
                    "temperature": 22,
                    "condition": "sunny",
                    "humidity": 45
                })),
            },
            McpToolDef {
                name: "calculator/add".into(),
                description: "Add two numbers".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "a": { "type": "number", "description": "First number" },
                        "b": { "type": "number", "description": "Second number" }
                    },
                    "required": ["a", "b"]
                }),
                behavior: McpToolBehavior::Calculator,
            },
            McpToolDef {
                name: "slow_query".into(),
                description: "A slow query tool for testing cancellation".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string" }
                    }
                }),
                behavior: McpToolBehavior::BlockUntilCancelled,
            },
            McpToolDef {
                name: "failing_tool".into(),
                description: "A tool that always errors".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
                behavior: McpToolBehavior::AlwaysError("internal server error".into()),
            },
        ]
    }

    /// Build the standard fixture resources.
    fn fixture_resources() -> Vec<McpResourceDef> {
        vec![
            McpResourceDef {
                uri: "file:///config.json".into(),
                name: "config".into(),
                description: "Application configuration".into(),
                mime_type: "application/json".into(),
                content: r#"{"version": "1.0", "debug": false}"#.into(),
            },
            McpResourceDef {
                uri: "file:///readme.md".into(),
                name: "readme".into(),
                description: "Project readme".into(),
                mime_type: "text/markdown".into(),
                content: "# Example Project\n\nThis is a fixture readme.".into(),
            },
        ]
    }
}

// ---------------------------------------------------------------------------
// McpAdapterExtension
// ---------------------------------------------------------------------------

/// An MCP adapter extension that maps MCP-style tools/resources through the
/// extension API. This is an **example** — it is NOT core MCP support.
///
/// # Commands
///
/// - `mcp/list_tools` — List available MCP tools with schemas.
/// - `mcp/call_tool` — Call an MCP tool with arguments.
/// - `mcp/list_resources` — List available MCP resources.
/// - `mcp/get_resource` — Get a resource by URI.
struct McpAdapterExtension {
    state: Arc<Mutex<McpAdapterState>>,
    events_received: Arc<Mutex<Vec<String>>>,
    /// Optional cancellation token for long-running tool execution.
    cancel_token: Arc<Mutex<Option<CancellationToken>>>,
}

impl McpAdapterExtension {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(McpAdapterState::default())),
            events_received: Arc::new(Mutex::new(Vec::new())),
            cancel_token: Arc::new(Mutex::new(None)),
        }
    }

    /// Create with a custom cancellation token for testing.
    fn with_cancel_token(token: CancellationToken) -> Self {
        let ext = Self::new();
        *ext.cancel_token.lock().unwrap() = Some(token);
        ext
    }

    /// Validate arguments against a tool's required fields.
    fn validate_args(tool: &McpToolDef, args: &Value) -> Result<(), ExtensionError> {
        let required = tool.input_schema["required"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let args_obj = args.as_object();
        for field in &required {
            let present = args_obj.is_some_and(|obj| obj.contains_key(field));
            if !present {
                return Err(ExtensionError::CommandError(format!(
                    "missing required argument: {field}"
                )));
            }
        }

        // Type-check numeric fields.
        if let Some(props) = tool.input_schema["properties"].as_object()
            && let Some(obj) = args_obj
        {
            for (key, schema) in props {
                let expected_type = schema["type"].as_str().unwrap_or("string");
                if let Some(val) = obj.get(key) {
                    match expected_type {
                        "number" if !val.is_number() => {
                            return Err(ExtensionError::CommandError(format!(
                                "argument '{key}' must be a number"
                            )));
                        }
                        "string" if !val.is_string() => {
                            return Err(ExtensionError::CommandError(format!(
                                "argument '{key}' must be a string"
                            )));
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }
}

impl Extension for McpAdapterExtension {
    fn name(&self) -> &str {
        "mcp-adapter"
    }

    fn on_event(&self, event: &AgentEvent) {
        let label = match event {
            AgentEvent::AgentStart => "AgentStart".to_string(),
            AgentEvent::AgentEnd { .. } => "AgentEnd".to_string(),
            AgentEvent::TurnStart => "TurnStart".to_string(),
            AgentEvent::ToolExecutionStart { tool_name, .. } => {
                format!("ToolExecutionStart({tool_name})")
            }
            AgentEvent::ToolExecutionEnd { tool_name, .. } => {
                format!("ToolExecutionEnd({tool_name})")
            }
            _ => "Other".to_string(),
        };
        self.events_received.lock().unwrap().push(label);
    }

    fn on_command(
        &self,
        command: &ExtensionCommand,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Value>, ExtensionError>> + Send>> {
        let cmd = command.name.clone();
        let args = command.args.clone();
        let state = self.state.clone();
        let cancel_token_arc = self.cancel_token.clone();

        Box::pin(async move {
            match cmd.as_str() {
                "mcp/list_tools" => {
                    let s = state.lock().unwrap();
                    let tools: Vec<Value> = s
                        .tools
                        .iter()
                        .map(|t| {
                            serde_json::json!({
                                "name": t.name,
                                "description": t.description,
                                "input_schema": t.input_schema,
                            })
                        })
                        .collect();

                    Ok(Some(serde_json::json!({
                        "tools": tools,
                        "total": tools.len(),
                    })))
                }
                "mcp/call_tool" => {
                    let tool_name = args["name"]
                        .as_str()
                        .ok_or_else(|| {
                            ExtensionError::CommandError("tool name is required".into())
                        })?
                        .to_string();
                    let tool_args = args["arguments"].clone();

                    // Look up the tool.
                    let behavior = {
                        let s = state.lock().unwrap();
                        let tool =
                            s.tools
                                .iter()
                                .find(|t| t.name == tool_name)
                                .ok_or_else(|| {
                                    ExtensionError::CommandError(format!(
                                        "tool '{tool_name}' not found"
                                    ))
                                })?;

                        // Validate arguments.
                        Self::validate_args(tool, &tool_args)?;

                        tool.behavior.clone()
                    };

                    // Execute fixture behavior.
                    let result = match behavior {
                        McpToolBehavior::Fixed(val) => {
                            serde_json::json!({
                                "tool": tool_name,
                                "result": val,
                                "is_error": false,
                            })
                        }
                        McpToolBehavior::Calculator => {
                            let a = tool_args["a"].as_f64().unwrap_or(0.0);
                            let b = tool_args["b"].as_f64().unwrap_or(0.0);
                            serde_json::json!({
                                "tool": tool_name,
                                "result": { "sum": a + b },
                                "is_error": false,
                            })
                        }
                        McpToolBehavior::BlockUntilCancelled => {
                            let token = cancel_token_arc.lock().unwrap().clone();
                            if let Some(ct) = token {
                                ct.cancelled().await;
                            }
                            serde_json::json!({
                                "tool": tool_name,
                                "result": "cancelled",
                                "is_error": false,
                            })
                        }
                        McpToolBehavior::AlwaysError(msg) => {
                            serde_json::json!({
                                "tool": tool_name,
                                "error": msg,
                                "is_error": true,
                            })
                        }
                    };

                    // Log the call.
                    {
                        let mut s = state.lock().unwrap();
                        s.log.push(McpLogEntry {
                            timestamp: "2026-06-04T00:00:00Z".into(),
                            operation: "call_tool".into(),
                            detail: tool_name.clone(),
                        });
                    }

                    Ok(Some(result))
                }
                "mcp/list_resources" => {
                    let s = state.lock().unwrap();
                    let resources: Vec<Value> = s
                        .resources
                        .iter()
                        .map(|r| {
                            serde_json::json!({
                                "uri": r.uri,
                                "name": r.name,
                                "description": r.description,
                                "mime_type": r.mime_type,
                            })
                        })
                        .collect();

                    Ok(Some(serde_json::json!({
                        "resources": resources,
                        "total": resources.len(),
                    })))
                }
                "mcp/get_resource" => {
                    let uri = args["uri"]
                        .as_str()
                        .ok_or_else(|| ExtensionError::CommandError("uri is required".into()))?
                        .to_string();

                    let s = state.lock().unwrap();
                    let resource = s.resources.iter().find(|r| r.uri == uri).ok_or_else(|| {
                        ExtensionError::CommandError(format!("resource '{uri}' not found"))
                    })?;

                    let content = resource.content.clone();
                    let mime_type = resource.mime_type.clone();
                    let name = resource.name.clone();

                    Ok(Some(serde_json::json!({
                        "uri": uri,
                        "name": name,
                        "mime_type": mime_type,
                        "content": content,
                    })))
                }
                _ => Ok(None),
            }
        })
    }

    fn serialize_state(&self) -> Result<Option<Value>, ExtensionError> {
        let s = self.state.lock().unwrap();
        let val = serde_json::to_value(McpAdapterState {
            tools: s.tools.clone(),
            resources: s.resources.clone(),
            log: s.log.clone(),
        })
        .map_err(|e| ExtensionError::StateSerialization {
            name: "mcp-adapter".into(),
            reason: e.to_string(),
        })?;
        Ok(Some(val))
    }

    fn restore_state(&self, state_val: Value) -> Result<(), ExtensionError> {
        let parsed: McpAdapterState =
            serde_json::from_value(state_val).map_err(|e| ExtensionError::StateRestoration {
                name: "mcp-adapter".into(),
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
        _arguments: Value,
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
// Tests: Tool discovery
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_tools_returns_all_fixture_tools() {
    let ext = McpAdapterExtension::new();

    let cmd = ExtensionCommand::new("mcp/list_tools", serde_json::json!({}));
    let result = ext.on_command(&cmd).await.unwrap().unwrap();

    let tools = result["tools"].as_array().unwrap();
    assert_eq!(result["total"], 4);
    assert_eq!(tools.len(), 4);

    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"weather/get"));
    assert!(names.contains(&"calculator/add"));
    assert!(names.contains(&"slow_query"));
    assert!(names.contains(&"failing_tool"));
}

#[tokio::test]
async fn tool_schemas_are_exposed() {
    let ext = McpAdapterExtension::new();

    let cmd = ExtensionCommand::new("mcp/list_tools", serde_json::json!({}));
    let result = ext.on_command(&cmd).await.unwrap().unwrap();

    let tools = result["tools"].as_array().unwrap();
    let weather = tools.iter().find(|t| t["name"] == "weather/get").unwrap();

    assert!(!weather["description"].as_str().unwrap().is_empty());
    assert!(weather["input_schema"]["properties"]["location"].is_object());
    assert!(weather["input_schema"]["required"].is_array());
}

// ---------------------------------------------------------------------------
// Tests: Argument validation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn call_tool_validates_required_args() {
    let ext = McpAdapterExtension::new();

    // Missing "location" for weather/get.
    let cmd = ExtensionCommand::new(
        "mcp/call_tool",
        serde_json::json!({
            "name": "weather/get",
            "arguments": {}
        }),
    );
    let result = ext.on_command(&cmd).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("missing required"));
}

#[tokio::test]
async fn call_tool_validates_argument_types() {
    let ext = McpAdapterExtension::new();

    // "a" should be a number but we pass a string.
    let cmd = ExtensionCommand::new(
        "mcp/call_tool",
        serde_json::json!({
            "name": "calculator/add",
            "arguments": { "a": "not_a_number", "b": 2 }
        }),
    );
    let result = ext.on_command(&cmd).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("must be a number"));
}

#[tokio::test]
async fn call_tool_requires_tool_name() {
    let ext = McpAdapterExtension::new();

    let cmd = ExtensionCommand::new("mcp/call_tool", serde_json::json!({}));
    let result = ext.on_command(&cmd).await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("tool name is required")
    );
}

// ---------------------------------------------------------------------------
// Tests: Tool execution success
// ---------------------------------------------------------------------------

#[tokio::test]
async fn call_tool_returns_fixed_fixture_result() {
    let ext = McpAdapterExtension::new();

    let cmd = ExtensionCommand::new(
        "mcp/call_tool",
        serde_json::json!({
            "name": "weather/get",
            "arguments": { "location": "London" }
        }),
    );
    let result = ext.on_command(&cmd).await.unwrap().unwrap();

    assert_eq!(result["is_error"], false);
    assert_eq!(result["result"]["temperature"], 22);
    assert_eq!(result["result"]["condition"], "sunny");
}

#[tokio::test]
async fn call_tool_calculator_adds_numbers() {
    let ext = McpAdapterExtension::new();

    let cmd = ExtensionCommand::new(
        "mcp/call_tool",
        serde_json::json!({
            "name": "calculator/add",
            "arguments": { "a": 3.0, "b": 4.0 }
        }),
    );
    let result = ext.on_command(&cmd).await.unwrap().unwrap();

    assert_eq!(result["is_error"], false);
    assert_eq!(result["result"]["sum"], 7.0);
}

#[tokio::test]
async fn call_tool_logs_execution() {
    let ext = McpAdapterExtension::new();
    let state = ext.state.clone();

    let cmd = ExtensionCommand::new(
        "mcp/call_tool",
        serde_json::json!({
            "name": "calculator/add",
            "arguments": { "a": 1, "b": 1 }
        }),
    );
    ext.on_command(&cmd).await.unwrap();

    let s = state.lock().unwrap();
    assert_eq!(s.log.len(), 1);
    assert_eq!(s.log[0].operation, "call_tool");
    assert_eq!(s.log[0].detail, "calculator/add");
}

// ---------------------------------------------------------------------------
// Tests: Tool execution error
// ---------------------------------------------------------------------------

#[tokio::test]
async fn call_tool_returns_error_for_unknown_tool() {
    let ext = McpAdapterExtension::new();

    let cmd = ExtensionCommand::new(
        "mcp/call_tool",
        serde_json::json!({
            "name": "nonexistent",
            "arguments": {}
        }),
    );
    let result = ext.on_command(&cmd).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[tokio::test]
async fn failing_tool_returns_structured_error() {
    let ext = McpAdapterExtension::new();

    let cmd = ExtensionCommand::new(
        "mcp/call_tool",
        serde_json::json!({
            "name": "failing_tool",
            "arguments": {}
        }),
    );
    let result = ext.on_command(&cmd).await.unwrap().unwrap();

    assert_eq!(result["is_error"], true);
    assert_eq!(result["error"], "internal server error");
}

// ---------------------------------------------------------------------------
// Tests: Resource metadata
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_resources_returns_all_fixture_resources() {
    let ext = McpAdapterExtension::new();

    let cmd = ExtensionCommand::new("mcp/list_resources", serde_json::json!({}));
    let result = ext.on_command(&cmd).await.unwrap().unwrap();

    let resources = result["resources"].as_array().unwrap();
    assert_eq!(result["total"], 2);
    assert_eq!(resources.len(), 2);

    let uris: Vec<&str> = resources
        .iter()
        .map(|r| r["uri"].as_str().unwrap())
        .collect();
    assert!(uris.contains(&"file:///config.json"));
    assert!(uris.contains(&"file:///readme.md"));
}

#[tokio::test]
async fn get_resource_returns_content() {
    let ext = McpAdapterExtension::new();

    let cmd = ExtensionCommand::new(
        "mcp/get_resource",
        serde_json::json!({ "uri": "file:///config.json" }),
    );
    let result = ext.on_command(&cmd).await.unwrap().unwrap();

    assert_eq!(result["name"], "config");
    assert_eq!(result["mime_type"], "application/json");
    assert!(result["content"].as_str().unwrap().contains("version"));
}

#[tokio::test]
async fn get_resource_rejects_unknown_uri() {
    let ext = McpAdapterExtension::new();

    let cmd = ExtensionCommand::new(
        "mcp/get_resource",
        serde_json::json!({ "uri": "file:///nonexistent.txt" }),
    );
    let result = ext.on_command(&cmd).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[tokio::test]
async fn get_resource_requires_uri() {
    let ext = McpAdapterExtension::new();

    let cmd = ExtensionCommand::new("mcp/get_resource", serde_json::json!({}));
    let result = ext.on_command(&cmd).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("uri is required"));
}

// ---------------------------------------------------------------------------
// Tests: Cancellation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn slow_tool_respects_cancellation() {
    let token = CancellationToken::new();
    let ext = McpAdapterExtension::with_cancel_token(token.clone());
    let cancel_token_arc = ext.cancel_token.clone();

    let cmd = ExtensionCommand::new(
        "mcp/call_tool",
        serde_json::json!({
            "name": "slow_query",
            "arguments": { "query": "select *" }
        }),
    );

    // Spawn the command in a background task.
    // Build an extension that shares state but has the cancel token.
    let mut ext_for_task = McpAdapterExtension::with_cancel_token(token.clone());
    ext_for_task.state = ext.state.clone();
    ext_for_task.events_received = ext.events_received.clone();
    ext_for_task.cancel_token = cancel_token_arc.clone();

    let handle = tokio::spawn(async move { ext_for_task.on_command(&cmd).await.unwrap().unwrap() });

    // Give it a moment to start, then cancel.
    tokio::task::yield_now().await;
    token.cancel();

    let result = handle.await.unwrap();
    assert_eq!(result["result"], "cancelled");
}

// ---------------------------------------------------------------------------
// Tests: State persistence
// ---------------------------------------------------------------------------

#[tokio::test]
async fn state_round_trips_through_serialization() {
    let ext = McpAdapterExtension::new();

    // Execute a tool to populate the log.
    let cmd = ExtensionCommand::new(
        "mcp/call_tool",
        serde_json::json!({
            "name": "weather/get",
            "arguments": { "location": "Paris" }
        }),
    );
    ext.on_command(&cmd).await.unwrap();

    // Serialize.
    let serialized = ext.serialize_state().unwrap().unwrap();
    assert!(!serialized["tools"].as_array().unwrap().is_empty());
    assert!(!serialized["resources"].as_array().unwrap().is_empty());
    assert_eq!(serialized["log"].as_array().unwrap().len(), 1);

    // Restore into a new extension.
    let ext2 = McpAdapterExtension::new();
    ext2.restore_state(serialized).unwrap();

    let s = ext2.state.lock().unwrap();
    assert_eq!(s.tools.len(), 4);
    assert_eq!(s.resources.len(), 2);
    assert_eq!(s.log.len(), 1);
    assert_eq!(s.log[0].detail, "weather/get");
}

// ---------------------------------------------------------------------------
// Tests: Event observation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn extension_receives_parent_agent_events() {
    let ext = McpAdapterExtension::new();
    let events = ext.events_received.clone();

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

    let received = events.lock().unwrap();
    assert!(received.contains(&"AgentStart".to_string()));
    assert!(received.contains(&"TurnStart".to_string()));
    assert!(received.contains(&"ToolExecutionStart(read)".to_string()));
}

// ---------------------------------------------------------------------------
// Tests: Session integration
// ---------------------------------------------------------------------------

#[tokio::test]
async fn session_integration_with_agent() {
    let ext = McpAdapterExtension::new();
    let state = ext.state.clone();

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    let provider = MockProvider::new("mock", vec![text_response("done")]);
    let hooks = registry.wrap_hooks(Box::new(TestHooks));
    let mut agent = opi_agent::Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        common::registrations_from(vec![Box::new(DummyTool::new("read"))]),
        Some(common::permissive_authorizer()),
        "mock:mock-model".into(),
        None,
        Default::default(),
        Default::default(),
        hooks,
    )
    .expect("agent");

    let _result = agent.prompt("test").await.into_execution_result().unwrap();

    // Fixture tools should still be in state after agent run.
    let s = state.lock().unwrap();
    assert_eq!(s.tools.len(), 4);
    assert_eq!(s.resources.len(), 2);
}

// ---------------------------------------------------------------------------
// Tests: No live network
// ---------------------------------------------------------------------------

#[tokio::test]
async fn no_network_calls_during_operations() {
    let ext = McpAdapterExtension::new();

    // All operations use fixtures — no network. This test verifies
    // the extension works without any external connectivity by
    // exercising all commands and asserting fixture data.
    let list = ExtensionCommand::new("mcp/list_tools", serde_json::json!({}));
    let tools_result = ext.on_command(&list).await.unwrap().unwrap();
    assert_eq!(tools_result["total"], 4);

    let call = ExtensionCommand::new(
        "mcp/call_tool",
        serde_json::json!({
            "name": "calculator/add",
            "arguments": { "a": 10, "b": 20 }
        }),
    );
    let call_result = ext.on_command(&call).await.unwrap().unwrap();
    assert_eq!(call_result["result"]["sum"], 30.0);

    let resources = ExtensionCommand::new("mcp/list_resources", serde_json::json!({}));
    let res_result = ext.on_command(&resources).await.unwrap().unwrap();
    assert_eq!(res_result["total"], 2);
}

// ---------------------------------------------------------------------------
// Tests: Unknown command passthrough
// ---------------------------------------------------------------------------

#[tokio::test]
async fn unknown_command_returns_none() {
    let ext = McpAdapterExtension::new();

    let cmd = ExtensionCommand::new("mcp/unknown", serde_json::json!({}));
    let result = ext.on_command(&cmd).await.unwrap();
    assert!(result.is_none());
}
