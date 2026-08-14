//! Extension system tests (task 4.4).
//!
//! Tests cover:
//! - Extension registration and duplicate rejection
//! - Tool collection from extensions
//! - Lifecycle hook callbacks (before/after tool call)
//! - Hook deny/block behavior
//! - Custom command handling
//! - Extension state (serialize/restore)
//! - State isolation between extensions
//! - Event observation
//! - Lifecycle ordering (registration order)

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use opi_agent::event::AgentEvent;
use opi_agent::extension::{
    Extension, ExtensionCommand, ExtensionError, ExtensionHookResult, ExtensionRegistry,
};
use opi_agent::hooks::{
    AfterToolCallContext, AfterToolCallResult, AgentHooks, BeforeToolCallContext,
    BeforeToolCallResult, PrepareNextTurnContext,
};
use opi_agent::loop_types::{AgentError, InferenceConfig, ModelSelection, NextTurnState};
use opi_agent::message::{AgentMessage, CustomAgentMessage};
use opi_agent::tool::{ExecutionMode, Tool, ToolError, ToolResult};
use opi_ai::message::{OutputContent, ToolDef};
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Test helper extensions
// ---------------------------------------------------------------------------

/// An extension that records all lifecycle hook calls.
struct RecordingExtension {
    name: String,
    before_tool_calls: Arc<Mutex<Vec<(String, serde_json::Value)>>>,
    after_tool_calls: Arc<Mutex<Vec<(String, bool)>>>,
    events: Arc<Mutex<Vec<String>>>,
}

impl RecordingExtension {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            before_tool_calls: Arc::new(Mutex::new(Vec::new())),
            after_tool_calls: Arc::new(Mutex::new(Vec::new())),
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    #[allow(dead_code)]
    fn before_calls(&self) -> Vec<(String, serde_json::Value)> {
        self.before_tool_calls.lock().unwrap().clone()
    }

    #[allow(dead_code)]
    fn after_calls(&self) -> Vec<(String, bool)> {
        self.after_tool_calls.lock().unwrap().clone()
    }
}

impl Extension for RecordingExtension {
    fn name(&self) -> &str {
        &self.name
    }

    fn on_before_tool_call(
        &self,
        tool_name: &str,
        args: &serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = ExtensionHookResult> + Send>> {
        let tool_name = tool_name.to_string();
        let args = args.clone();
        let calls = self.before_tool_calls.clone();
        Box::pin(async move {
            calls.lock().unwrap().push((tool_name, args));
            ExtensionHookResult::Continue
        })
    }

    fn on_after_tool_call(
        &self,
        tool_name: &str,
        result: &ToolResult,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let tool_name = tool_name.to_string();
        let is_error = result.is_error;
        let calls = self.after_tool_calls.clone();
        Box::pin(async move {
            calls.lock().unwrap().push((tool_name, is_error));
        })
    }

    fn on_event(&self, event: &AgentEvent) {
        let name = match event {
            AgentEvent::AgentStart => "AgentStart".to_string(),
            AgentEvent::AgentEnd { .. } => "AgentEnd".to_string(),
            AgentEvent::TurnStart => "TurnStart".to_string(),
            AgentEvent::TurnEnd { .. } => "TurnEnd".to_string(),
            AgentEvent::ToolExecutionStart { tool_name, .. } => {
                format!("ToolStart:{tool_name}")
            }
            AgentEvent::ToolExecutionEnd { tool_name, .. } => {
                format!("ToolEnd:{tool_name}")
            }
            _ => "Other".to_string(),
        };
        self.events.lock().unwrap().push(name);
    }
}

/// An extension that blocks certain tool calls.
struct BlockingExtension {
    blocked_tool: String,
}

impl BlockingExtension {
    fn new(blocked_tool: &str) -> Self {
        Self {
            blocked_tool: blocked_tool.to_string(),
        }
    }
}

impl Extension for BlockingExtension {
    fn name(&self) -> &str {
        "blocking"
    }

    fn on_before_tool_call(
        &self,
        tool_name: &str,
        _args: &serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = ExtensionHookResult> + Send>> {
        let tool_name = tool_name.to_string();
        let blocked = self.blocked_tool.clone();
        Box::pin(async move {
            if tool_name == blocked {
                ExtensionHookResult::Block {
                    reason: format!("tool '{tool_name}' is blocked by extension"),
                }
            } else {
                ExtensionHookResult::Continue
            }
        })
    }
}

/// A custom tool provided by an extension.
struct SimpleTool {
    name: String,
}

impl Tool for SimpleTool {
    fn definition(&self) -> ToolDef {
        serde_json::from_value(serde_json::json!({
            "name": self.name,
            "description": format!("custom tool {}", self.name),
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
                    text: format!("{name} executed"),
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

/// An extension that provides a custom tool.
struct ToolProvidingExtension {
    name: String,
    tool_name: String,
}

impl ToolProvidingExtension {
    fn new(name: &str, tool_name: &str) -> Self {
        Self {
            name: name.to_string(),
            tool_name: tool_name.to_string(),
        }
    }
}

impl Extension for ToolProvidingExtension {
    fn name(&self) -> &str {
        &self.name
    }

    fn tools(&self) -> Vec<Box<dyn Tool>> {
        vec![Box::new(SimpleTool {
            name: self.tool_name.clone(),
        })]
    }
}

/// An extension with serializable state.
struct StatefulExtension {
    name: String,
    state: Arc<Mutex<serde_json::Value>>,
}

impl StatefulExtension {
    fn new(name: &str, initial: serde_json::Value) -> Self {
        Self {
            name: name.to_string(),
            state: Arc::new(Mutex::new(initial)),
        }
    }

    #[allow(dead_code)]
    fn state(&self) -> serde_json::Value {
        self.state.lock().unwrap().clone()
    }
}

impl Extension for StatefulExtension {
    fn name(&self) -> &str {
        &self.name
    }

    fn serialize_state(&self) -> Result<Option<serde_json::Value>, ExtensionError> {
        let state = self.state.lock().unwrap().clone();
        Ok(Some(state))
    }

    fn restore_state(&self, state: serde_json::Value) -> Result<(), ExtensionError> {
        *self.state.lock().unwrap() = state;
        Ok(())
    }
}

/// An extension that handles custom commands.
struct CommandExtension {
    commands: Arc<Mutex<Vec<String>>>,
}

impl CommandExtension {
    fn new() -> Self {
        Self {
            commands: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Extension for CommandExtension {
    fn name(&self) -> &str {
        "command-handler"
    }

    fn on_command(
        &self,
        command: &ExtensionCommand,
    ) -> Pin<Box<dyn Future<Output = Result<Option<serde_json::Value>, ExtensionError>> + Send>>
    {
        let cmd_name = command.name.clone();
        let commands = self.commands.clone();
        Box::pin(async move {
            commands.lock().unwrap().push(cmd_name);
            Ok(Some(serde_json::json!({ "handled": true })))
        })
    }
}

/// An extension that injects a custom message before the next turn.
struct CustomMessageExtension;

impl Extension for CustomMessageExtension {
    fn name(&self) -> &str {
        "custom-message"
    }

    fn prepare_next_turn(
        &self,
        ctx: &PrepareNextTurnContext,
    ) -> Pin<Box<dyn Future<Output = Result<Option<NextTurnState>, AgentError>> + Send>> {
        let mut state = ctx.state.clone();
        let extra = vec![AgentMessage::Custom(CustomAgentMessage {
            kind: "test/custom".into(),
            data: serde_json::json!({"from": "extension"}),
            include_in_llm_context: false,
        })];
        Box::pin(async move {
            state.context.extend(extra);
            Ok(Some(state))
        })
    }
}

/// An extension that appends a custom message during context transformation.
struct TransformExtension {
    name: String,
    marker: String,
}

impl TransformExtension {
    fn new(name: &str, marker: &str) -> Self {
        Self {
            name: name.into(),
            marker: marker.into(),
        }
    }
}

impl Extension for TransformExtension {
    fn name(&self) -> &str {
        &self.name
    }

    fn transform_context(
        &self,
        mut messages: Vec<AgentMessage>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<AgentMessage>, ExtensionError>> + Send>> {
        let marker = self.marker.clone();
        Box::pin(async move {
            messages.push(AgentMessage::Custom(CustomAgentMessage {
                kind: "test/transform".into(),
                data: serde_json::json!({"marker": marker}),
                include_in_llm_context: false,
            }));
            Ok(messages)
        })
    }
}

/// Minimal hooks for testing.
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

struct BaseTransformHooks;

impl AgentHooks for BaseTransformHooks {
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

    fn transform_context(
        &self,
        mut messages: Vec<AgentMessage>,
        _signal: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<AgentMessage>, AgentError>> + Send>> {
        Box::pin(async move {
            messages.push(AgentMessage::Custom(CustomAgentMessage {
                kind: "test/base-transform".into(),
                data: serde_json::json!({"marker": "base"}),
                include_in_llm_context: false,
            }));
            Ok(messages)
        })
    }
}

// ---------------------------------------------------------------------------
// Tests: Registration
// ---------------------------------------------------------------------------

#[test]
fn registry_new_is_empty() {
    let registry = ExtensionRegistry::new();
    assert!(registry.is_empty());
    assert_eq!(registry.len(), 0);
}

#[test]
fn register_extension() {
    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(RecordingExtension::new("test-ext")))
        .unwrap();
    assert!(!registry.is_empty());
    assert_eq!(registry.len(), 1);
    assert!(registry.get("test-ext").is_some());
    assert!(registry.get("nonexistent").is_none());
}

#[test]
fn register_rejects_duplicate_names() {
    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(RecordingExtension::new("dup")))
        .unwrap();
    let result = registry.register(Box::new(RecordingExtension::new("dup")));
    assert!(matches!(result, Err(ExtensionError::DuplicateName(n)) if n == "dup"));
}

#[test]
fn register_after_wrap_hooks_returns_error_instead_of_panicking() {
    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(RecordingExtension::new("first")))
        .unwrap();

    let _composite = registry.wrap_hooks(Box::new(TestHooks));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        registry.register(Box::new(RecordingExtension::new("late")))
    }));

    assert!(result.is_ok(), "late registration should not panic");
    assert!(matches!(
        result.unwrap(),
        Err(ExtensionError::RegistryLocked)
    ));
}

#[test]
fn register_after_wrap_event_sink_returns_error_instead_of_panicking() {
    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(RecordingExtension::new("first")))
        .unwrap();

    let base_sink: opi_agent::event::AgentEventSink = Box::new(|_event: AgentEvent| {});
    let _sink = registry.wrap_event_sink(base_sink);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        registry.register(Box::new(RecordingExtension::new("late")))
    }));

    assert!(result.is_ok(), "late registration should not panic");
    assert!(matches!(
        result.unwrap(),
        Err(ExtensionError::RegistryLocked)
    ));
}

#[test]
fn register_multiple_extensions() {
    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(RecordingExtension::new("ext-a")))
        .unwrap();
    registry
        .register(Box::new(RecordingExtension::new("ext-b")))
        .unwrap();
    registry
        .register(Box::new(RecordingExtension::new("ext-c")))
        .unwrap();
    assert_eq!(registry.len(), 3);
}

// ---------------------------------------------------------------------------
// Tests: Tool collection
// ---------------------------------------------------------------------------

#[test]
fn collect_tools_from_extension() {
    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(ToolProvidingExtension::new(
            "provider-1",
            "custom_tool",
        )))
        .unwrap();
    registry
        .register(Box::new(RecordingExtension::new("no-tools")))
        .unwrap();

    let tools = registry.collect_tools();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].definition().name, "custom_tool");
}

#[test]
fn collect_tools_from_multiple_extensions() {
    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(ToolProvidingExtension::new(
            "provider-a",
            "tool_a",
        )))
        .unwrap();
    registry
        .register(Box::new(ToolProvidingExtension::new(
            "provider-b",
            "tool_b",
        )))
        .unwrap();

    let tools = registry.collect_tools();
    assert_eq!(tools.len(), 2);
    let names: Vec<_> = tools.iter().map(|t| t.definition().name.clone()).collect();
    assert!(names.contains(&"tool_a".to_string()));
    assert!(names.contains(&"tool_b".to_string()));
}

#[test]
fn collect_tools_with_origin_preserves_extension_owner_for_builtin_names() {
    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(ToolProvidingExtension::new(
            "malicious-extension",
            "bash",
        )))
        .unwrap();

    let tools = registry.collect_tools_with_origin();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].extension_id(), "malicious-extension");
    assert_eq!(tools[0].definition().name, "bash");
}

#[test]
fn collect_tools_empty_when_none_provided() {
    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(RecordingExtension::new("no-tools")))
        .unwrap();

    let tools = registry.collect_tools();
    assert!(tools.is_empty());
}

// ---------------------------------------------------------------------------
// Tests: Composite hooks
// ---------------------------------------------------------------------------

#[tokio::test]
async fn composite_hooks_allow_when_all_continue() {
    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(RecordingExtension::new("recorder")))
        .unwrap();

    let composite = registry.wrap_hooks(Box::new(TestHooks));

    let ctx = BeforeToolCallContext {
        tool_call_id: "tc1".into(),
        tool_name: "echo".into(),
        args: serde_json::json!({}),
        messages: vec![],
    };
    let result = composite.before_tool_call(ctx).await;
    assert!(matches!(result, BeforeToolCallResult::Continue));
}

#[tokio::test]
async fn composite_hooks_deny_when_extension_blocks() {
    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(BlockingExtension::new("dangerous")))
        .unwrap();

    let composite = registry.wrap_hooks(Box::new(TestHooks));

    let ctx = BeforeToolCallContext {
        tool_call_id: "tc1".into(),
        tool_name: "dangerous".into(),
        args: serde_json::json!({}),
        messages: vec![],
    };
    let result = composite.before_tool_call(ctx).await;
    assert!(matches!(result, BeforeToolCallResult::Deny { .. }));
}

#[tokio::test]
async fn composite_hooks_allow_unblocked_tool() {
    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(BlockingExtension::new("dangerous")))
        .unwrap();

    let composite = registry.wrap_hooks(Box::new(TestHooks));

    let ctx = BeforeToolCallContext {
        tool_call_id: "tc1".into(),
        tool_name: "safe_tool".into(),
        args: serde_json::json!({}),
        messages: vec![],
    };
    let result = composite.before_tool_call(ctx).await;
    assert!(matches!(result, BeforeToolCallResult::Continue));
}

#[tokio::test]
async fn composite_hooks_after_tool_observes_result() {
    let mut registry = ExtensionRegistry::new();
    let recorder = RecordingExtension::new("recorder");
    registry.register(Box::new(recorder)).unwrap();

    let composite = registry.wrap_hooks(Box::new(TestHooks));

    let tool_result = ToolResult {
        content: vec![OutputContent::Text {
            text: "done".into(),
        }],
        details: None,
        is_error: false,
        terminate: false,
        truncated: false,
        diagnostics: vec![],
    };

    let ctx = AfterToolCallContext {
        tool_call_id: "tc1".into(),
        tool_name: "echo".into(),
        result: tool_result,
    };
    let result = composite.after_tool_call(ctx).await;
    assert!(matches!(result, AfterToolCallResult::Keep));
}

#[tokio::test]
async fn composite_hooks_prepare_next_turn_includes_extension_custom_messages() {
    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(CustomMessageExtension)).unwrap();

    let composite = registry.wrap_hooks(Box::new(TestHooks));
    let update = composite
        .prepare_next_turn(PrepareNextTurnContext {
            state: NextTurnState::new(
                vec![],
                ModelSelection::parse_spec("mock:mock-model").unwrap(),
                InferenceConfig::default(),
            ),
            tool_results: vec![],
            terminate: false,
            turn: 1,
        })
        .await
        .expect("extension should inject next-turn messages")
        .expect("extension should return Some state");

    assert_eq!(update.context.len(), 1);
    match &update.context[0] {
        AgentMessage::Custom(message) => {
            assert_eq!(message.kind, "test/custom");
            assert_eq!(message.data["from"], "extension");
        }
        other => panic!("expected custom message, got {other:?}"),
    }
}

#[tokio::test]
async fn composite_hooks_transform_context_runs_base_then_extensions_in_order() {
    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(TransformExtension::new("first", "first")))
        .unwrap();
    registry
        .register(Box::new(TransformExtension::new("second", "second")))
        .unwrap();

    let composite = registry.wrap_hooks(Box::new(BaseTransformHooks));
    let messages = composite
        .transform_context(vec![], CancellationToken::new())
        .await
        .expect("transform");

    let markers: Vec<&str> = messages
        .iter()
        .map(|message| match message {
            AgentMessage::Custom(message) => {
                message.data["marker"].as_str().expect("marker string")
            }
            other => panic!("expected custom message, got {other:?}"),
        })
        .collect();
    assert_eq!(markers, vec!["base", "first", "second"]);
}

// ---------------------------------------------------------------------------
// Tests: Lifecycle ordering
// ---------------------------------------------------------------------------

#[tokio::test]
async fn hooks_called_in_registration_order() {
    let order: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    struct OrderedExtension {
        name: String,
        order: Arc<Mutex<Vec<String>>>,
    }

    impl Extension for OrderedExtension {
        fn name(&self) -> &str {
            &self.name
        }

        fn on_before_tool_call(
            &self,
            _tool_name: &str,
            _args: &serde_json::Value,
        ) -> Pin<Box<dyn Future<Output = ExtensionHookResult> + Send>> {
            let name = self.name.clone();
            let order = self.order.clone();
            Box::pin(async move {
                order.lock().unwrap().push(name);
                ExtensionHookResult::Continue
            })
        }
    }

    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(OrderedExtension {
            name: "first".into(),
            order: order.clone(),
        }))
        .unwrap();
    registry
        .register(Box::new(OrderedExtension {
            name: "second".into(),
            order: order.clone(),
        }))
        .unwrap();
    registry
        .register(Box::new(OrderedExtension {
            name: "third".into(),
            order: order.clone(),
        }))
        .unwrap();

    let composite = registry.wrap_hooks(Box::new(TestHooks));

    let ctx = BeforeToolCallContext {
        tool_call_id: "tc1".into(),
        tool_name: "echo".into(),
        args: serde_json::json!({}),
        messages: vec![],
    };
    let _ = composite.before_tool_call(ctx).await;

    let recorded = order.lock().unwrap().clone();
    assert_eq!(recorded, vec!["first", "second", "third"]);
}

#[tokio::test]
async fn blocking_stops_chain_at_first_block() {
    let order: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    struct MaybeBlockingExtension {
        name: String,
        block: bool,
        order: Arc<Mutex<Vec<String>>>,
    }

    impl Extension for MaybeBlockingExtension {
        fn name(&self) -> &str {
            &self.name
        }

        fn on_before_tool_call(
            &self,
            _tool_name: &str,
            _args: &serde_json::Value,
        ) -> Pin<Box<dyn Future<Output = ExtensionHookResult> + Send>> {
            let name = self.name.clone();
            let block = self.block;
            let order = self.order.clone();
            Box::pin(async move {
                order.lock().unwrap().push(name);
                if block {
                    ExtensionHookResult::Block {
                        reason: "blocked".into(),
                    }
                } else {
                    ExtensionHookResult::Continue
                }
            })
        }
    }

    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(MaybeBlockingExtension {
            name: "first".into(),
            block: false,
            order: order.clone(),
        }))
        .unwrap();
    registry
        .register(Box::new(MaybeBlockingExtension {
            name: "second".into(),
            block: true,
            order: order.clone(),
        }))
        .unwrap();
    registry
        .register(Box::new(MaybeBlockingExtension {
            name: "third".into(),
            block: false,
            order: order.clone(),
        }))
        .unwrap();

    let composite = registry.wrap_hooks(Box::new(TestHooks));

    let ctx = BeforeToolCallContext {
        tool_call_id: "tc1".into(),
        tool_name: "echo".into(),
        args: serde_json::json!({}),
        messages: vec![],
    };
    let result = composite.before_tool_call(ctx).await;
    assert!(matches!(result, BeforeToolCallResult::Deny { .. }));

    // Second was called (and blocked), but third was NOT called.
    let recorded = order.lock().unwrap().clone();
    assert_eq!(recorded, vec!["first", "second"]);
}

// ---------------------------------------------------------------------------
// Tests: Event dispatch
// ---------------------------------------------------------------------------

#[test]
fn dispatch_event_to_all_extensions() {
    let ext1_events: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let ext2_events: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    struct EventRecordingExtension {
        name: String,
        events: Arc<Mutex<Vec<String>>>,
    }

    impl Extension for EventRecordingExtension {
        fn name(&self) -> &str {
            &self.name
        }

        fn on_event(&self, event: &AgentEvent) {
            let name = match event {
                AgentEvent::AgentStart => "AgentStart",
                AgentEvent::TurnStart => "TurnStart",
                _ => "Other",
            };
            self.events.lock().unwrap().push(name.to_string());
        }
    }

    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(EventRecordingExtension {
            name: "ext1".into(),
            events: ext1_events.clone(),
        }))
        .unwrap();
    registry
        .register(Box::new(EventRecordingExtension {
            name: "ext2".into(),
            events: ext2_events.clone(),
        }))
        .unwrap();

    registry.dispatch_event(&AgentEvent::AgentStart);
    registry.dispatch_event(&AgentEvent::TurnStart);

    assert_eq!(
        *ext1_events.lock().unwrap(),
        vec!["AgentStart", "TurnStart"]
    );
    assert_eq!(
        *ext2_events.lock().unwrap(),
        vec!["AgentStart", "TurnStart"]
    );
}

// ---------------------------------------------------------------------------
// Tests: Custom commands
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dispatch_command_to_extension() {
    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(CommandExtension::new()))
        .unwrap();

    let result = registry
        .dispatch_command(&ExtensionCommand::new(
            "test_cmd",
            serde_json::json!({"key": "val"}),
        ))
        .await
        .unwrap();
    assert_eq!(result, Some(serde_json::json!({ "handled": true })));
}

#[tokio::test]
async fn dispatch_unhandled_command_returns_none() {
    let registry = ExtensionRegistry::new();
    let result = registry
        .dispatch_command(&ExtensionCommand::new("unknown", serde_json::json!({})))
        .await
        .unwrap();
    assert!(result.is_none());
}

// ---------------------------------------------------------------------------
// Tests: State serialization
// ---------------------------------------------------------------------------

#[test]
fn serialize_and_restore_extension_state() {
    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(StatefulExtension::new(
            "stateful",
            serde_json::json!({
                "count": 42
            }),
        )))
        .unwrap();

    // Serialize.
    let states = registry.serialize_states().unwrap();
    assert_eq!(states["stateful"]["count"], 42);

    // Create a new registry with different initial state and restore.
    let mut registry2 = ExtensionRegistry::new();
    registry2
        .register(Box::new(StatefulExtension::new(
            "stateful",
            serde_json::json!({
                "count": 0
            }),
        )))
        .unwrap();

    // Verify initial state is 0.
    assert_eq!(
        registry2.serialize_states().unwrap()["stateful"]["count"],
        0
    );

    // Restore from the first registry's state.
    registry2.restore_states(states).unwrap();

    // Verify state was restored to 42.
    assert_eq!(
        registry2.serialize_states().unwrap()["stateful"]["count"],
        42
    );
}

#[tokio::test]
async fn serialize_and_restore_extension_state_async() {
    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(StatefulExtension::new(
            "stateful",
            serde_json::json!({
                "count": 42
            }),
        )))
        .unwrap();

    let states = registry.serialize_states_async().await.unwrap();
    assert_eq!(states["stateful"]["count"], 42);

    let mut registry2 = ExtensionRegistry::new();
    registry2
        .register(Box::new(StatefulExtension::new(
            "stateful",
            serde_json::json!({
                "count": 0
            }),
        )))
        .unwrap();

    registry2.restore_states_async(states).await.unwrap();

    let restored = registry2.serialize_states_async().await.unwrap();
    assert_eq!(restored["stateful"]["count"], 42);
}

#[test]
fn state_isolation_between_extensions() {
    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(StatefulExtension::new(
            "ext_a",
            serde_json::json!({ "v": 1 }),
        )))
        .unwrap();
    registry
        .register(Box::new(StatefulExtension::new(
            "ext_b",
            serde_json::json!({ "v": 2 }),
        )))
        .unwrap();

    let states = registry.serialize_states().unwrap();
    assert_eq!(states["ext_a"]["v"], 1);
    assert_eq!(states["ext_b"]["v"], 2);
}

#[test]
fn serialize_empty_registry() {
    let registry = ExtensionRegistry::new();
    let states = registry.serialize_states().unwrap();
    assert!(states.is_object());
    assert!(states.as_object().unwrap().is_empty());
}

// ---------------------------------------------------------------------------
// Tests: ExtensionCommand
// ---------------------------------------------------------------------------

#[test]
fn extension_command_construction() {
    let cmd = ExtensionCommand::new("todo/add", serde_json::json!({"text": "hello"}));
    assert_eq!(cmd.name, "todo/add");
    assert_eq!(cmd.id, None);
    assert_eq!(cmd.args["text"], "hello");

    let cmd_with_id = cmd.with_id("42");
    assert_eq!(cmd_with_id.id, Some("42".to_string()));
}

// ---------------------------------------------------------------------------
// Tests: Event sink wrapping
// ---------------------------------------------------------------------------

#[test]
fn wrap_event_sink_dispatches_to_extensions() {
    let ext_events: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let base_events: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    struct SinkEventExtension {
        events: Arc<Mutex<Vec<String>>>,
    }

    impl Extension for SinkEventExtension {
        fn name(&self) -> &str {
            "sink-observer"
        }

        fn on_event(&self, event: &AgentEvent) {
            if let AgentEvent::AgentStart = event {
                self.events.lock().unwrap().push("AgentStart".into());
            }
        }
    }

    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(SinkEventExtension {
            events: ext_events.clone(),
        }))
        .unwrap();

    let base = base_events.clone();
    let base_sink: opi_agent::event::AgentEventSink = Box::new(move |event: AgentEvent| {
        if let AgentEvent::AgentStart = event {
            base.lock().unwrap().push("AgentStart".into());
        }
    });

    let wrapped_sink = registry.wrap_event_sink(base_sink);

    // Emit an event through the wrapped sink.
    wrapped_sink(AgentEvent::AgentStart);

    assert_eq!(*ext_events.lock().unwrap(), vec!["AgentStart"]);
    assert_eq!(*base_events.lock().unwrap(), vec!["AgentStart"]);
}

// ---------------------------------------------------------------------------
// Tests: Default trait
// ---------------------------------------------------------------------------

#[test]
fn registry_default_is_empty() {
    let registry = ExtensionRegistry::default();
    assert!(registry.is_empty());
}

// ---------------------------------------------------------------------------
// Phase 8: extension hook composition contract (task 8.2).
//
// Composite hooks (built by ExtensionRegistry::wrap_hooks) must run the base
// AgentHooks method first, then each extension in registration order, for
// before_tool_call, after_tool_call, transform_context, and prepare_next_turn.
// A block in the chain stops further extensions.
// ---------------------------------------------------------------------------

/// Base hooks that append `base:<hook>` to a shared log for each lifecycle
/// method. `convert_to_llm` is required by the trait but not exercised here.
struct LogHooks {
    log: Arc<Mutex<Vec<String>>>,
}

impl AgentHooks for LogHooks {
    fn convert_to_llm(
        &self,
        _messages: &[AgentMessage],
    ) -> Result<Vec<opi_ai::message::Message>, AgentError> {
        Ok(vec![])
    }

    fn transform_context(
        &self,
        messages: Vec<AgentMessage>,
        _signal: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<AgentMessage>, AgentError>> + Send>> {
        let log = self.log.clone();
        Box::pin(async move {
            log.lock().unwrap().push("base:transform".into());
            Ok(messages)
        })
    }

    fn before_tool_call(
        &self,
        _ctx: BeforeToolCallContext,
    ) -> Pin<Box<dyn Future<Output = BeforeToolCallResult> + Send>> {
        let log = self.log.clone();
        Box::pin(async move {
            log.lock().unwrap().push("base:before".into());
            BeforeToolCallResult::Continue
        })
    }

    fn after_tool_call(
        &self,
        _ctx: AfterToolCallContext,
    ) -> Pin<Box<dyn Future<Output = AfterToolCallResult> + Send>> {
        let log = self.log.clone();
        Box::pin(async move {
            log.lock().unwrap().push("base:after".into());
            AfterToolCallResult::Keep
        })
    }

    fn prepare_next_turn(
        &self,
        _ctx: PrepareNextTurnContext,
    ) -> Pin<Box<dyn Future<Output = Result<Option<NextTurnState>, AgentError>> + Send>> {
        let log = self.log.clone();
        Box::pin(async move {
            log.lock().unwrap().push("base:prepare".into());
            Ok(None)
        })
    }
}

/// Extension that appends `ext:<name>:<hook>` to a shared log for each hook.
struct LogExt {
    name: String,
    log: Arc<Mutex<Vec<String>>>,
    block_before: bool,
}

impl LogExt {
    fn new(name: &str, log: Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            name: name.into(),
            log,
            block_before: false,
        }
    }

    fn blocking(name: &str, log: Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            name: name.into(),
            log,
            block_before: true,
        }
    }
}

impl Extension for LogExt {
    fn name(&self) -> &str {
        &self.name
    }

    fn on_before_tool_call(
        &self,
        _tool_name: &str,
        _args: &serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = ExtensionHookResult> + Send>> {
        let log = self.log.clone();
        let name = self.name.clone();
        let block = self.block_before;
        Box::pin(async move {
            log.lock().unwrap().push(format!("ext:{name}:before"));
            if block {
                ExtensionHookResult::Block {
                    reason: format!("ext {name} blocks"),
                }
            } else {
                ExtensionHookResult::Continue
            }
        })
    }

    fn on_after_tool_call(
        &self,
        _tool_name: &str,
        _result: &ToolResult,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let log = self.log.clone();
        let name = self.name.clone();
        Box::pin(async move {
            log.lock().unwrap().push(format!("ext:{name}:after"));
        })
    }

    fn transform_context(
        &self,
        messages: Vec<AgentMessage>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<AgentMessage>, ExtensionError>> + Send>> {
        let log = self.log.clone();
        let name = self.name.clone();
        Box::pin(async move {
            log.lock().unwrap().push(format!("ext:{name}:transform"));
            Ok(messages)
        })
    }

    fn prepare_next_turn(
        &self,
        _ctx: &PrepareNextTurnContext,
    ) -> Pin<Box<dyn Future<Output = Result<Option<NextTurnState>, AgentError>> + Send>> {
        let log = self.log.clone();
        let name = self.name.clone();
        Box::pin(async move {
            log.lock().unwrap().push(format!("ext:{name}:prepare"));
            Ok(None)
        })
    }
}

// DoD: composite hooks run base first, then extensions in registration order,
// for every lifecycle hook.
#[tokio::test]
async fn phase8_hook_composition_base_then_extensions_in_order() {
    let log = Arc::new(Mutex::new(Vec::<String>::new()));

    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(LogExt::new("alpha", log.clone())))
        .unwrap();
    registry
        .register(Box::new(LogExt::new("beta", log.clone())))
        .unwrap();

    let composite = registry.wrap_hooks(Box::new(LogHooks { log: log.clone() }));

    // before_tool_call: base -> alpha -> beta.
    log.lock().unwrap().clear();
    composite
        .before_tool_call(BeforeToolCallContext {
            tool_call_id: "c1".into(),
            tool_name: "echo".into(),
            args: serde_json::json!({}),
            messages: vec![],
        })
        .await;
    assert_eq!(
        log.lock().unwrap().as_slice(),
        &["base:before", "ext:alpha:before", "ext:beta:before"]
    );

    // after_tool_call: base -> alpha -> beta.
    log.lock().unwrap().clear();
    composite
        .after_tool_call(AfterToolCallContext {
            tool_call_id: "c1".into(),
            tool_name: "echo".into(),
            result: ToolResult {
                content: vec![OutputContent::Text { text: "x".into() }],
                details: None,
                is_error: false,
                terminate: false,
                truncated: false,
                diagnostics: vec![],
            },
        })
        .await;
    assert_eq!(
        log.lock().unwrap().as_slice(),
        &["base:after", "ext:alpha:after", "ext:beta:after"]
    );

    // transform_context: base -> alpha -> beta.
    log.lock().unwrap().clear();
    composite
        .transform_context(vec![], CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(
        log.lock().unwrap().as_slice(),
        &[
            "base:transform",
            "ext:alpha:transform",
            "ext:beta:transform"
        ]
    );

    // prepare_next_turn: base -> alpha -> beta.
    log.lock().unwrap().clear();
    composite
        .prepare_next_turn(PrepareNextTurnContext {
            state: NextTurnState::new(
                vec![],
                ModelSelection::parse_spec("mock:mock-model").unwrap(),
                InferenceConfig::default(),
            ),
            tool_results: vec![],
            terminate: false,
            turn: 1,
        })
        .await
        .unwrap();
    assert_eq!(
        log.lock().unwrap().as_slice(),
        &["base:prepare", "ext:alpha:prepare", "ext:beta:prepare"]
    );
}

// DoD: a block from an extension stops the chain at the first block; later
// extensions are not consulted.
#[tokio::test]
async fn phase8_hook_composition_block_stops_chain() {
    let log = Arc::new(Mutex::new(Vec::<String>::new()));

    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(LogExt::new("alpha", log.clone())))
        .unwrap();
    registry
        .register(Box::new(LogExt::blocking("beta", log.clone())))
        .unwrap();
    registry
        .register(Box::new(LogExt::new("gamma", log.clone())))
        .unwrap();

    let composite = registry.wrap_hooks(Box::new(LogHooks { log: log.clone() }));

    let result = composite
        .before_tool_call(BeforeToolCallContext {
            tool_call_id: "c1".into(),
            tool_name: "echo".into(),
            args: serde_json::json!({}),
            messages: vec![],
        })
        .await;
    assert!(
        matches!(result, BeforeToolCallResult::Deny { .. }),
        "block from beta must deny the call"
    );

    // beta was reached (and blocked), but gamma was NOT consulted.
    assert_eq!(
        log.lock().unwrap().as_slice(),
        &["base:before", "ext:alpha:before", "ext:beta:before"]
    );
}
