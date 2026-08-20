//! Coding-agent extension integration tests (task 4.4).
//!
//! Tests verify that extensions work through the full agent loop path:
//! - Extension tools are available and executable
//! - Extension hooks compose with base hooks during agent loop execution
//! - Extension events are observed during agent lifecycle
//! - Extension state persists across agent invocations

mod common;

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use opi_agent::Agent;
use opi_agent::event::AgentEvent;
use opi_agent::extension::{Extension, ExtensionError, ExtensionHookResult, ExtensionRegistry};
use opi_agent::hooks::AgentHooks;
use opi_agent::loop_types::{AgentError, AgentLoopConfig, InferenceConfig};
use opi_agent::message::AgentMessage;
use opi_agent::tool::{ExecutionMode, Tool, ToolError, ToolResult};
use opi_ai::message::{OutputContent, ToolDef};
use opi_ai::provider::ModelInfo;
use opi_ai::registry::ModelCapabilities;
use opi_ai::test_support::{
    MockProvider, single_route_collection, text_response, tool_call_response,
};
use opi_coding_agent::config::OpiConfig;
use opi_coding_agent::harness::CodingHarness;
use opi_coding_agent::policy::{RunMode, ToolRuntimeConfig, ToolSelection};
use opi_coding_agent::project_trust::TrustDecision;
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A simple extension tool that echoes its arguments.
struct EchoExtensionTool;

impl Tool for EchoExtensionTool {
    fn definition(&self) -> ToolDef {
        serde_json::from_value(serde_json::json!({
            "name": "ext_echo",
            "description": "extension echo tool",
            "input_schema": {
                "type": "object",
                "properties": { "text": { "type": "string" } },
                "required": ["text"]
            }
        }))
        .unwrap()
    }

    fn execute(
        &self,
        _call_id: &str,
        arguments: serde_json::Value,
        _signal: CancellationToken,
        _on_update: Option<opi_agent::tool::UpdateCallback>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send>> {
        let text = arguments["text"].as_str().unwrap_or("").to_string();
        Box::pin(async move {
            Ok(ToolResult {
                content: vec![OutputContent::Text {
                    text: format!("ext_echo: {text}"),
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

/// Extension that provides a tool and records hook calls.
struct IntegrationExtension {
    before_calls: Arc<Mutex<Vec<String>>>,
    after_calls: Arc<Mutex<Vec<String>>>,
}

impl IntegrationExtension {
    fn new() -> Self {
        Self {
            before_calls: Arc::new(Mutex::new(Vec::new())),
            after_calls: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Extension for IntegrationExtension {
    fn name(&self) -> &str {
        "integration-ext"
    }

    fn tools(&self) -> Vec<Box<dyn Tool>> {
        vec![Box::new(EchoExtensionTool)]
    }

    fn on_before_tool_call(
        &self,
        tool_name: &str,
        _args: &serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = ExtensionHookResult> + Send>> {
        let tool_name = tool_name.to_string();
        let calls = self.before_calls.clone();
        Box::pin(async move {
            calls.lock().unwrap().push(tool_name);
            ExtensionHookResult::Continue
        })
    }

    fn on_after_tool_call(
        &self,
        tool_name: &str,
        _result: &ToolResult,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let tool_name = tool_name.to_string();
        let calls = self.after_calls.clone();
        Box::pin(async move {
            calls.lock().unwrap().push(tool_name);
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn extension_tool_executes_through_agent_loop() {
    let mut registry = ExtensionRegistry::new();
    let ext = IntegrationExtension::new();
    let before_calls = ext.before_calls.clone();
    let after_calls = ext.after_calls.clone();
    registry.register(Box::new(ext)).unwrap();

    let ext_tools = registry.collect_tools();

    let provider = MockProvider::new(
        "mock",
        vec![
            tool_call_response("tc_ext_1", "ext_echo", r#"{"text":"hello"}"#),
            text_response("Done"),
        ],
    );
    let hooks = registry.wrap_hooks(Box::new(TestHooks));
    let mut agent = Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        common::registrations_from(ext_tools),
        Some(common::permissive_authorizer()),
        "mock:mock-model".into(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig {
            max_turns: 10,
            ..Default::default()
        },
        hooks,
    )
    .expect("agent");

    let result = agent.prompt("test").await.into_execution_result().unwrap();

    // Verify extension tool was executed: user + assistant(tool_call) + tool_result + assistant(text)
    assert!(result.len() >= 3);

    // Verify extension hooks were called.
    let before = before_calls.lock().unwrap();
    assert!(before.contains(&"ext_echo".to_string()));

    let after = after_calls.lock().unwrap();
    assert!(after.contains(&"ext_echo".to_string()));
}

#[tokio::test]
async fn extension_hooks_observe_builtin_tool_calls() {
    let observed_tools: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    struct ObserverExtension {
        observed: Arc<Mutex<Vec<String>>>,
    }

    impl Extension for ObserverExtension {
        fn name(&self) -> &str {
            "observer"
        }

        fn on_after_tool_call(
            &self,
            tool_name: &str,
            _result: &ToolResult,
        ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
            let tool_name = tool_name.to_string();
            let observed = self.observed.clone();
            Box::pin(async move {
                observed.lock().unwrap().push(tool_name);
            })
        }
    }

    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(ObserverExtension {
            observed: observed_tools.clone(),
        }))
        .unwrap();

    // Dummy tool that's not an extension tool — extension should still observe it.
    struct DummyTool;

    impl Tool for DummyTool {
        fn definition(&self) -> ToolDef {
            serde_json::from_value(serde_json::json!({
                "name": "dummy",
                "description": "dummy tool",
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
            Box::pin(async {
                Ok(ToolResult {
                    content: vec![OutputContent::Text { text: "ok".into() }],
                    details: None,
                    is_error: false,
                    terminate: false,
                    truncated: false,
                    diagnostics: vec![],
                })
            })
        }
    }

    let provider = MockProvider::new(
        "mock",
        vec![
            tool_call_response("tc_d1", "dummy", "{}"),
            text_response("Done"),
        ],
    );
    let hooks = registry.wrap_hooks(Box::new(TestHooks));
    let mut agent = Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        common::registrations_from(vec![Box::new(DummyTool)]),
        Some(common::permissive_authorizer()),
        "mock:mock-model".into(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig {
            max_turns: 10,
            ..Default::default()
        },
        hooks,
    )
    .expect("agent");

    let result = agent.prompt("test").await.into_execution_result().unwrap();
    assert!(result.len() >= 3);

    // Extension should have observed the "dummy" tool call.
    let observed = observed_tools.lock().unwrap();
    assert!(observed.contains(&"dummy".to_string()));
}

#[tokio::test]
async fn extension_can_block_tool_in_agent_loop() {
    struct BlockAllExtension;

    impl Extension for BlockAllExtension {
        fn name(&self) -> &str {
            "blocker"
        }

        fn on_before_tool_call(
            &self,
            _tool_name: &str,
            _args: &serde_json::Value,
        ) -> Pin<Box<dyn Future<Output = ExtensionHookResult> + Send>> {
            Box::pin(async {
                ExtensionHookResult::Block {
                    reason: "all tools blocked".into(),
                }
            })
        }
    }

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(BlockAllExtension)).unwrap();

    struct AlwaysOkTool;

    impl Tool for AlwaysOkTool {
        fn definition(&self) -> ToolDef {
            serde_json::from_value(serde_json::json!({
                "name": "ok",
                "description": "ok tool",
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
            Box::pin(async {
                Ok(ToolResult {
                    content: vec![OutputContent::Text {
                        text: "executed".into(),
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

    let provider = MockProvider::new(
        "mock",
        vec![
            tool_call_response("tc_b1", "ok", "{}"),
            text_response("Done"),
        ],
    );
    let hooks = registry.wrap_hooks(Box::new(TestHooks));
    let mut agent = Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        common::registrations_from(vec![Box::new(AlwaysOkTool)]),
        Some(common::permissive_authorizer()),
        "mock:mock-model".into(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig {
            max_turns: 10,
            ..Default::default()
        },
        hooks,
    )
    .expect("agent");

    let result = agent.prompt("test").await.into_execution_result().unwrap();

    // The tool result should contain the block reason (not "executed").
    let tool_text: String = result
        .iter()
        .filter_map(|m| {
            if let AgentMessage::Llm(opi_ai::message::Message::ToolResult(trm)) = m {
                Some(trm.content.clone())
            } else {
                None
            }
        })
        .flat_map(|c| {
            c.into_iter().filter_map(|c| match c {
                OutputContent::Text { text } => Some(text),
                _ => None,
            })
        })
        .collect();

    assert!(
        tool_text.contains("all tools blocked"),
        "tool result should contain block reason, got: {tool_text}"
    );
}

#[tokio::test]
async fn extension_state_round_trip_through_agent() {
    struct CountingExtension {
        count: Arc<Mutex<u64>>,
    }

    impl CountingExtension {
        fn new() -> Self {
            Self {
                count: Arc::new(Mutex::new(0)),
            }
        }
    }

    impl Extension for CountingExtension {
        fn name(&self) -> &str {
            "counter"
        }

        fn on_after_tool_call(
            &self,
            _tool_name: &str,
            _result: &ToolResult,
        ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
            let count = self.count.clone();
            Box::pin(async move {
                *count.lock().unwrap() += 1;
            })
        }

        fn serialize_state(&self) -> Result<Option<serde_json::Value>, ExtensionError> {
            let count = *self.count.lock().unwrap();
            Ok(Some(serde_json::json!({ "count": count })))
        }

        fn restore_state(&self, state: serde_json::Value) -> Result<(), ExtensionError> {
            if let Some(c) = state["count"].as_u64() {
                *self.count.lock().unwrap() = c;
            }
            Ok(())
        }
    }

    let ext = CountingExtension::new();
    let count = ext.count.clone();
    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    // Serialize (initial state).
    let states = registry.serialize_states().unwrap();
    assert_eq!(states["counter"]["count"], 0);

    // Simulate a tool call incrementing the counter.
    *count.lock().unwrap() = 5;

    // Serialize (after increment).
    let states = registry.serialize_states().unwrap();
    assert_eq!(states["counter"]["count"], 5);

    // Create a new extension and restore.
    let ext2 = CountingExtension::new();
    let count2 = ext2.count.clone();
    let mut registry2 = ExtensionRegistry::new();
    registry2.register(Box::new(ext2)).unwrap();
    registry2.restore_states(states).unwrap();

    assert_eq!(*count2.lock().unwrap(), 5);
}

#[tokio::test]
async fn harness_builder_wraps_extension_registry_hooks_and_tools() {
    struct BuilderTool;

    impl Tool for BuilderTool {
        fn definition(&self) -> ToolDef {
            serde_json::from_value(serde_json::json!({
                "name": "builder_echo",
                "description": "builder extension echo",
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
            Box::pin(async {
                Ok(ToolResult {
                    content: vec![OutputContent::Text {
                        text: "builder tool executed".into(),
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

    struct BuilderExtension {
        before_calls: Arc<Mutex<Vec<String>>>,
    }

    impl Extension for BuilderExtension {
        fn name(&self) -> &str {
            "builder-extension"
        }

        fn tools(&self) -> Vec<Box<dyn Tool>> {
            vec![Box::new(BuilderTool)]
        }

        fn on_before_tool_call(
            &self,
            tool_name: &str,
            _args: &serde_json::Value,
        ) -> Pin<Box<dyn Future<Output = ExtensionHookResult> + Send>> {
            let tool_name = tool_name.to_string();
            let before_calls = self.before_calls.clone();
            Box::pin(async move {
                before_calls.lock().unwrap().push(tool_name);
                ExtensionHookResult::Continue
            })
        }
    }

    let before_calls = Arc::new(Mutex::new(Vec::new()));
    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(BuilderExtension {
            before_calls: before_calls.clone(),
        }))
        .unwrap();

    let provider = MockProvider::new(
        "mock",
        vec![
            tool_call_response("tc_builder", "builder_echo", "{}"),
            text_response("Done"),
        ],
    );
    let workspace = tempfile::tempdir().unwrap();
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .extension_registry(registry)
    .tool_config(ToolRuntimeConfig {
        run_mode: RunMode::Interactive,
        active_tool_names: Vec::new(),
    })
    .build();

    assert!(
        !harness.system_prompt().contains("builder_echo"),
        "Phase 17.4 excludes extension tools from the model-visible projection"
    );
    assert!(
        harness
            .resource_metadata()
            .extensions
            .iter()
            .any(|entry| entry.name == "builder-extension"),
        "the extension is still registered for hooks/events/metadata"
    );

    let messages = harness.prompt("use builder tool").await.unwrap();
    assert!(messages.len() >= 3);
    assert!(
        before_calls.lock().unwrap().is_empty(),
        "Phase 17.4 excludes the extension tool, so it never resolves and its before_tool_call hook never runs"
    );
}

#[tokio::test]
async fn harness_builder_extension_observes_agent_events() {
    struct EventRecorderExtension {
        events: Arc<Mutex<Vec<&'static str>>>,
    }

    impl Extension for EventRecorderExtension {
        fn name(&self) -> &str {
            "event-recorder"
        }

        fn on_event(&self, event: &AgentEvent) {
            let label = match event {
                AgentEvent::AgentStart => "AgentStart",
                AgentEvent::TurnStart => "TurnStart",
                AgentEvent::MessageStart { .. } => "MessageStart",
                AgentEvent::AgentEnd { .. } => "AgentEnd",
                _ => return,
            };
            self.events.lock().unwrap().push(label);
        }
    }

    let events = Arc::new(Mutex::new(Vec::new()));
    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(EventRecorderExtension {
            events: events.clone(),
        }))
        .unwrap();

    let provider = MockProvider::new("mock", vec![text_response("Done")]);
    let workspace = tempfile::tempdir().unwrap();
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .extension_registry(registry)
    .tool_selection(ToolSelection::Disabled)
    .build();

    harness.prompt("hello").await.unwrap();

    let recorded = events.lock().unwrap().clone();
    assert!(
        recorded.contains(&"AgentStart"),
        "recorded events: {recorded:?}"
    );
    assert!(
        recorded.contains(&"TurnStart"),
        "recorded events: {recorded:?}"
    );
    assert!(
        recorded.contains(&"MessageStart"),
        "recorded events: {recorded:?}"
    );
    assert!(
        recorded.contains(&"AgentEnd"),
        "recorded events: {recorded:?}"
    );
}

#[tokio::test]
async fn harness_builder_dispatches_extension_commands() {
    struct CommandExtension;

    impl Extension for CommandExtension {
        fn name(&self) -> &str {
            "command-extension"
        }

        fn on_command(
            &self,
            command: &opi_agent::extension::ExtensionCommand,
        ) -> Pin<Box<dyn Future<Output = Result<Option<serde_json::Value>, ExtensionError>> + Send>>
        {
            let command = command.clone();
            Box::pin(async move {
                if command.name != "echo/upper" {
                    return Ok(None);
                }
                let text = command
                    .args
                    .get("text")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_ascii_uppercase();
                Ok(Some(serde_json::json!({
                    "id": command.id,
                    "text": text,
                })))
            })
        }
    }

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(CommandExtension)).unwrap();
    let workspace = tempfile::tempdir().unwrap();

    let mut harness = CodingHarness::builder(
        Box::new(MockProvider::new("mock", vec![text_response("Done")])),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .extension_registry(registry)
    .tool_selection(ToolSelection::Disabled)
    .build();

    let response = harness
        .dispatch_extension_command(
            "echo/upper",
            Some("cmd-1"),
            serde_json::json!({ "text": "hello" }),
        )
        .await
        .unwrap();

    assert_eq!(
        response,
        Some(serde_json::json!({
            "id": "cmd-1",
            "text": "HELLO",
        }))
    );

    let unhandled = harness
        .dispatch_extension_command("unknown/command", None, serde_json::json!({}))
        .await
        .unwrap();
    assert_eq!(unhandled, None);
}

#[tokio::test]
async fn harness_builder_tool_selection_disabled_filters_extension_tools() {
    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(IntegrationExtension::new()))
        .unwrap();
    let provider = MockProvider::new("mock", vec![text_response("Done")]);
    let workspace = tempfile::tempdir().unwrap();

    let harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .extension_registry(registry)
    .tool_selection(ToolSelection::Disabled)
    .build();

    assert!(!harness.system_prompt().contains("ext_echo"));
}

#[tokio::test]
async fn harness_builder_tool_selection_allowlist_filters_extension_tools_by_name() {
    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(IntegrationExtension::new()))
        .unwrap();
    let provider = MockProvider::new("mock", vec![text_response("Done")]);
    let workspace = tempfile::tempdir().unwrap();

    let harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .extension_registry(registry)
    .tool_selection(ToolSelection::Allowlist(vec!["ext_echo".to_owned()]))
    .build();

    assert!(
        !harness.system_prompt().contains("ext_echo"),
        "Phase 17.4 excludes extension tools even when named in the allowlist"
    );
    assert!(!harness.system_prompt().contains("read_file"));
}

#[tokio::test]
async fn harness_builder_tool_selection_allowlist_excludes_unlisted_extension_tools() {
    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(IntegrationExtension::new()))
        .unwrap();
    let provider = MockProvider::new("mock", vec![text_response("Done")]);
    let workspace = tempfile::tempdir().unwrap();

    let harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .extension_registry(registry)
    .tool_selection(ToolSelection::Allowlist(vec!["read".to_owned()]))
    .build();

    assert!(!harness.system_prompt().contains("ext_echo"));
}

#[test]
fn harness_builder_model_picker_includes_current_provider_extension_overrides() {
    struct ModelOverrideExtension;

    impl Extension for ModelOverrideExtension {
        fn name(&self) -> &str {
            "model-override-extension"
        }

        fn model_overrides(&self) -> Vec<(String, ModelInfo)> {
            vec![(
                "mock".into(),
                ModelInfo::new(
                    "custom-model",
                    "Custom Model",
                    opi_ai::WireApi::OpenAiCompletions,
                    ModelCapabilities::new(100_000, 4_096)
                        .with_images(true)
                        .with_streaming(true),
                ),
            )]
        }
    }

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ModelOverrideExtension)).unwrap();
    let workspace = tempfile::tempdir().unwrap();

    let harness = CodingHarness::builder(
        Box::new(MockProvider::new("mock", vec![text_response("Done")])),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .extension_registry(registry)
    .build();

    let items = harness.model_picker_items();

    assert!(
        items
            .iter()
            .any(|item| item.id == "mock:custom-model" && item.display == "Custom Model")
    );
}

#[test]
fn harness_builder_set_model_validated_accepts_current_provider_extension_overrides() {
    struct ModelOverrideExtension;

    impl Extension for ModelOverrideExtension {
        fn name(&self) -> &str {
            "model-override-extension"
        }

        fn model_overrides(&self) -> Vec<(String, ModelInfo)> {
            vec![(
                "mock".into(),
                ModelInfo::new(
                    "custom-model",
                    "Custom Model",
                    opi_ai::WireApi::OpenAiCompletions,
                    ModelCapabilities::new(100_000, 4_096)
                        .with_images(true)
                        .with_streaming(true),
                ),
            )]
        }
    }

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ModelOverrideExtension)).unwrap();
    let workspace = tempfile::tempdir().unwrap();

    let mut harness = CodingHarness::builder(
        Box::new(MockProvider::new("mock", vec![text_response("Done")])),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .extension_registry(registry)
    .build();

    let selected = harness
        .set_model_validated("mock:custom-model".into())
        .unwrap();

    assert_eq!(selected, "mock:custom-model");
}
