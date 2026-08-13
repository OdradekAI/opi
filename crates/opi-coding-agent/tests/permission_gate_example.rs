//! Permission gate extension/package example tests (task 4.8.1).
//!
//! These tests demonstrate a permission gate extension that gates mutating tool
//! calls through extension hooks. This is an **example** showing how to build a
//! permission gate as an extension — it is NOT core policy and does not add a
//! permanent permission-popup subsystem to the agent runtime.
//!
//! # What This Example Demonstrates
//!
//! - **Allow**: Tools pass through when the policy permits them.
//! - **Deny**: Tools are blocked when the policy denies them, with a reason.
//! - **Audit/event output**: Every allow/deny decision is recorded in an audit
//!   log and the extension receives agent events.
//! - **Non-interactive behavior**: The extension makes automatic decisions
//!   (auto-allow or auto-deny) without prompting the user.
//!
//! # Example vs Core Policy
//!
//! The permission gate lives entirely in extension code. It uses the standard
//! [`Extension::on_before_tool_call`] hook to intercept tool calls and the
//! standard [`Extension::on_event`] callback to observe agent lifecycle events.
//! No core runtime changes are needed.

mod common;

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use opi_agent::extension::{Extension, ExtensionHookResult, ExtensionRegistry};
use opi_agent::hooks::AgentHooks;
use opi_agent::loop_types::{AgentError, AgentLoopConfig, InferenceConfig};
use opi_agent::message::AgentMessage;
use opi_agent::tool::{ExecutionMode, Tool, ToolError, ToolResult};
use opi_ai::message::{OutputContent, ToolDef};
use opi_ai::test_support::{
    MockProvider, single_route_collection, text_response, tool_call_response,
};
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Permission policy
// ---------------------------------------------------------------------------

/// Policy controlling how the permission gate handles tool calls.
#[derive(Debug, Clone)]
enum PermissionPolicy {
    /// Allow all tool calls unconditionally.
    AllowAll,
    /// Deny all tool calls unconditionally.
    DenyAll,
    /// Deny only tools whose names appear in the list.
    DenyList(Vec<String>),
    /// Allow only tools whose names appear in the list; deny everything else.
    AllowList(Vec<String>),
}

// ---------------------------------------------------------------------------
// Audit entry
// ---------------------------------------------------------------------------

/// A recorded permission decision.
#[derive(Debug, Clone)]
struct AuditEntry {
    /// Name of the tool that was evaluated.
    tool_name: String,
    /// Decision: "allowed" or "denied".
    decision: String,
    /// Reason for denial (None for allow).
    reason: Option<String>,
}

// ---------------------------------------------------------------------------
// Permission gate extension
// ---------------------------------------------------------------------------

/// A permission gate extension that gates tool calls based on a configurable
/// policy.
///
/// This is an **example extension** demonstrating how to use extension hooks
/// for permission gating. It is NOT core policy. In a real deployment, the
/// policy could be driven by user input, configuration files, or external
/// services.
///
/// # Modes
///
/// - **Interactive** (hypothetical): would prompt the user for each tool call.
/// - **Non-interactive** (demonstrated here): makes automatic decisions based
///   on the configured policy.
struct PermissionGateExtension {
    policy: PermissionPolicy,
    /// Audit log of all allow/deny decisions.
    audit_log: Arc<Mutex<Vec<AuditEntry>>>,
    /// Agent events received via `on_event`.
    events_received: Arc<Mutex<Vec<String>>>,
}

impl PermissionGateExtension {
    /// Create a new permission gate with the given policy.
    fn new(policy: PermissionPolicy) -> Self {
        Self {
            policy,
            audit_log: Arc::new(Mutex::new(Vec::new())),
            events_received: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Evaluate whether a tool call should be allowed.
    fn evaluate(&self, tool_name: &str) -> ExtensionHookResult {
        let decision = match &self.policy {
            PermissionPolicy::AllowAll => "allowed".to_string(),
            PermissionPolicy::DenyAll => "denied".to_string(),
            PermissionPolicy::DenyList(list) => if list.iter().any(|t| t == tool_name) {
                "denied"
            } else {
                "allowed"
            }
            .to_string(),
            PermissionPolicy::AllowList(list) => if list.iter().any(|t| t == tool_name) {
                "allowed"
            } else {
                "denied"
            }
            .to_string(),
        };

        let reason = if decision == "denied" {
            Some(format!(
                "permission gate denied '{}' based on {:?} policy",
                tool_name, self.policy
            ))
        } else {
            None
        };

        self.audit_log.lock().unwrap().push(AuditEntry {
            tool_name: tool_name.to_string(),
            decision: decision.clone(),
            reason: reason.clone(),
        });

        match decision.as_str() {
            "denied" => ExtensionHookResult::Block {
                reason: reason.unwrap(),
            },
            _ => ExtensionHookResult::Continue,
        }
    }
}

impl Extension for PermissionGateExtension {
    fn name(&self) -> &str {
        "permission-gate"
    }

    fn on_before_tool_call(
        &self,
        tool_name: &str,
        _args: &serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = ExtensionHookResult> + Send>> {
        let result = self.evaluate(tool_name);
        Box::pin(async move { result })
    }

    fn on_event(&self, event: &opi_agent::event::AgentEvent) {
        let label = match event {
            opi_agent::event::AgentEvent::AgentStart => "AgentStart".to_string(),
            opi_agent::event::AgentEvent::AgentEnd { .. } => "AgentEnd".to_string(),
            opi_agent::event::AgentEvent::TurnStart => "TurnStart".to_string(),
            opi_agent::event::AgentEvent::ToolExecutionStart { tool_name, .. } => {
                format!("ToolExecutionStart({tool_name})")
            }
            opi_agent::event::AgentEvent::ToolExecutionEnd { tool_name, .. } => {
                format!("ToolExecutionEnd({tool_name})")
            }
            _ => "Other".to_string(),
        };
        self.events_received.lock().unwrap().push(label);
    }

    fn serialize_state(
        &self,
    ) -> Result<Option<serde_json::Value>, opi_agent::extension::ExtensionError> {
        let log = self.audit_log.lock().unwrap();
        let entries: Vec<serde_json::Value> = log
            .iter()
            .map(|e| {
                serde_json::json!({
                    "tool_name": e.tool_name,
                    "decision": e.decision,
                    "reason": e.reason,
                })
            })
            .collect();
        Ok(Some(serde_json::json!({ "audit_log": entries })))
    }

    fn restore_state(
        &self,
        state: serde_json::Value,
    ) -> Result<(), opi_agent::extension::ExtensionError> {
        if let Some(entries) = state["audit_log"].as_array() {
            let mut log = self.audit_log.lock().unwrap();
            log.clear();
            for entry in entries {
                log.push(AuditEntry {
                    tool_name: entry["tool_name"].as_str().unwrap_or("").to_string(),
                    decision: entry["decision"].as_str().unwrap_or("").to_string(),
                    reason: entry["reason"].as_str().map(|s| s.to_string()),
                });
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/// A dummy tool that succeeds with "ok".
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

    fn execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Parallel
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

/// Extract tool result text from agent messages.
fn extract_tool_result_text(messages: &[AgentMessage]) -> String {
    messages
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
        .collect()
}

// ---------------------------------------------------------------------------
// Tests: Allow
// ---------------------------------------------------------------------------

#[tokio::test]
async fn allow_all_policy_permits_tool_call() {
    let ext = PermissionGateExtension::new(PermissionPolicy::AllowAll);
    let audit = ext.audit_log.clone();

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    let provider = MockProvider::new(
        "mock",
        vec![
            tool_call_response("tc_1", "write", r#"{"path":"/tmp/f","content":"x"}"#),
            text_response("Done"),
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
        AgentLoopConfig {
            max_turns: 10,
            ..Default::default()
        },
        hooks,
    )
    .expect("agent");

    let result = agent.prompt("test").await.unwrap();
    assert!(result.len() >= 3);

    // Tool should have executed successfully (not blocked).
    let tool_text = extract_tool_result_text(&result);
    assert!(
        tool_text.contains("ok"),
        "tool should have executed, got: {tool_text}"
    );

    // Audit log should record the allow decision.
    let log = audit.lock().unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].tool_name, "write");
    assert_eq!(log[0].decision, "allowed");
    assert!(log[0].reason.is_none());
}

#[tokio::test]
async fn allow_list_policy_permits_listed_tool() {
    let ext = PermissionGateExtension::new(PermissionPolicy::AllowList(vec![
        "read".to_string(),
        "glob".to_string(),
    ]));
    let audit = ext.audit_log.clone();

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    let provider = MockProvider::new(
        "mock",
        vec![
            tool_call_response("tc_1", "read", r#"{"path":"/tmp/f"}"#),
            text_response("Done"),
        ],
    );
    let hooks = registry.wrap_hooks(Box::new(TestHooks));
    let mut agent = opi_agent::Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        common::registrations_from(vec![Box::new(DummyTool::new("read"))]),
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

    let result = agent.prompt("test").await.unwrap();
    let tool_text = extract_tool_result_text(&result);
    assert!(tool_text.contains("ok"), "listed tool should execute");

    let log = audit.lock().unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].decision, "allowed");
}

// ---------------------------------------------------------------------------
// Tests: Deny
// ---------------------------------------------------------------------------

#[tokio::test]
async fn deny_all_policy_blocks_tool_call() {
    let ext = PermissionGateExtension::new(PermissionPolicy::DenyAll);
    let audit = ext.audit_log.clone();

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    let provider = MockProvider::new(
        "mock",
        vec![
            tool_call_response("tc_1", "bash", r#"{"command":"rm -rf /"}"#),
            text_response("Done"),
        ],
    );
    let hooks = registry.wrap_hooks(Box::new(TestHooks));
    let mut agent = opi_agent::Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        common::registrations_from(vec![Box::new(DummyTool::new("bash"))]),
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

    let result = agent.prompt("test").await.unwrap();

    // Tool should NOT have executed — result should contain the block reason.
    let tool_text = extract_tool_result_text(&result);
    assert!(
        tool_text.contains("permission gate denied"),
        "tool should be blocked, got: {tool_text}"
    );
    assert!(!tool_text.contains("ok"), "tool should NOT have executed");

    // Audit log should record the deny decision.
    let log = audit.lock().unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].tool_name, "bash");
    assert_eq!(log[0].decision, "denied");
    assert!(log[0].reason.is_some());
}

#[tokio::test]
async fn deny_list_policy_blocks_specific_tools() {
    let ext = PermissionGateExtension::new(PermissionPolicy::DenyList(vec![
        "write".to_string(),
        "edit".to_string(),
        "bash".to_string(),
    ]));
    let audit = ext.audit_log.clone();

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    let provider = MockProvider::new(
        "mock",
        vec![
            tool_call_response("tc_1", "write", r#"{"path":"/tmp/f","content":"x"}"#),
            text_response("Done"),
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
        AgentLoopConfig {
            max_turns: 10,
            ..Default::default()
        },
        hooks,
    )
    .expect("agent");

    let result = agent.prompt("test").await.unwrap();
    let tool_text = extract_tool_result_text(&result);
    assert!(
        tool_text.contains("permission gate denied"),
        "denied tool should be blocked, got: {tool_text}"
    );

    let log = audit.lock().unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].tool_name, "write");
    assert_eq!(log[0].decision, "denied");
}

#[tokio::test]
async fn deny_list_policy_allows_non_listed_tools() {
    let ext = PermissionGateExtension::new(PermissionPolicy::DenyList(vec![
        "write".to_string(),
        "edit".to_string(),
        "bash".to_string(),
    ]));
    let audit = ext.audit_log.clone();

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    let provider = MockProvider::new(
        "mock",
        vec![
            tool_call_response("tc_1", "read", r#"{"path":"/tmp/f"}"#),
            text_response("Done"),
        ],
    );
    let hooks = registry.wrap_hooks(Box::new(TestHooks));
    let mut agent = opi_agent::Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        common::registrations_from(vec![Box::new(DummyTool::new("read"))]),
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

    let result = agent.prompt("test").await.unwrap();
    let tool_text = extract_tool_result_text(&result);
    assert!(
        tool_text.contains("ok"),
        "non-listed tool should execute, got: {tool_text}"
    );

    let log = audit.lock().unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].decision, "allowed");
}

// ---------------------------------------------------------------------------
// Tests: Audit/event output
// ---------------------------------------------------------------------------

#[tokio::test]
async fn audit_log_records_allow_and_deny_across_turns() {
    // Use an AllowList policy so "read" is allowed but "write" is denied.
    let ext = PermissionGateExtension::new(PermissionPolicy::AllowList(vec!["read".to_string()]));
    let audit = ext.audit_log.clone();

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    let provider = MockProvider::new(
        "mock",
        vec![
            // Turn 1: read (allowed)
            tool_call_response("tc_1", "read", r#"{"path":"/tmp/f"}"#),
            // Turn 2: write (denied)
            tool_call_response("tc_2", "write", r#"{"path":"/tmp/f","content":"x"}"#),
            text_response("Done"),
        ],
    );
    let hooks = registry.wrap_hooks(Box::new(TestHooks));
    let mut agent = opi_agent::Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        common::registrations_from(vec![
            Box::new(DummyTool::new("read")),
            Box::new(DummyTool::new("write")),
        ]),
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

    let _ = agent.prompt("test").await.unwrap();

    let log = audit.lock().unwrap();
    assert!(log.len() >= 2, "should have at least 2 audit entries");

    // First entry: read allowed.
    assert_eq!(log[0].tool_name, "read");
    assert_eq!(log[0].decision, "allowed");

    // Second entry: write denied.
    assert_eq!(log[1].tool_name, "write");
    assert_eq!(log[1].decision, "denied");
    assert!(log[1].reason.is_some());
}

#[tokio::test]
async fn extension_receives_agent_events() {
    let ext = PermissionGateExtension::new(PermissionPolicy::AllowAll);
    let events = ext.events_received.clone();

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    // Use wrap_event_sink to dispatch events to the extension.
    let base_sink =
        Box::new(|_event: opi_agent::event::AgentEvent| {}) as opi_agent::event::AgentEventSink;
    let wrapped_sink = registry.wrap_event_sink(base_sink);

    // Simulate emitting events through the wrapped sink.
    wrapped_sink(opi_agent::event::AgentEvent::AgentStart);
    wrapped_sink(opi_agent::event::AgentEvent::TurnStart);
    wrapped_sink(opi_agent::event::AgentEvent::ToolExecutionStart {
        tool_call_id: "tc_1".into(),
        tool_name: "read".into(),
        args: serde_json::json!({}),
    });

    let received = events.lock().unwrap();
    assert!(
        received.contains(&"AgentStart".to_string()),
        "should have received AgentStart"
    );
    assert!(
        received.contains(&"TurnStart".to_string()),
        "should have received TurnStart"
    );
    assert!(
        received.contains(&"ToolExecutionStart(read)".to_string()),
        "should have received ToolExecutionStart(read)"
    );
}

#[tokio::test]
async fn audit_state_round_trips_through_serialization() {
    let ext = PermissionGateExtension::new(PermissionPolicy::DenyList(vec!["bash".to_string()]));

    // Manually trigger an evaluate to populate the audit log.
    ext.evaluate("read");
    ext.evaluate("bash");

    // Serialize.
    let state = ext.serialize_state().unwrap().unwrap();
    assert_eq!(state["audit_log"].as_array().unwrap().len(), 2);

    // Restore into a new extension.
    let ext2 = PermissionGateExtension::new(PermissionPolicy::AllowAll);
    ext2.restore_state(state).unwrap();

    let log = ext2.audit_log.lock().unwrap();
    assert_eq!(log.len(), 2);
    assert_eq!(log[0].tool_name, "read");
    assert_eq!(log[0].decision, "allowed");
    assert_eq!(log[1].tool_name, "bash");
    assert_eq!(log[1].decision, "denied");
}

// ---------------------------------------------------------------------------
// Tests: Non-interactive behavior
// ---------------------------------------------------------------------------

#[tokio::test]
async fn non_interactive_auto_approves_with_allow_all() {
    // In non-interactive mode with AllowAll, the extension auto-approves
    // everything without prompting the user. This is verified by running
    // through the agent loop — no TUI interaction needed.
    let ext = PermissionGateExtension::new(PermissionPolicy::AllowAll);
    let audit = ext.audit_log.clone();

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    let provider = MockProvider::new(
        "mock",
        vec![
            tool_call_response("tc_1", "write", r#"{"path":"/tmp/a","content":"data"}"#),
            tool_call_response("tc_2", "bash", r#"{"command":"ls"}"#),
            text_response("Done"),
        ],
    );
    let hooks = registry.wrap_hooks(Box::new(TestHooks));
    let mut agent = opi_agent::Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        common::registrations_from(vec![
            Box::new(DummyTool::new("write")),
            Box::new(DummyTool::new("bash")),
        ]),
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

    let result = agent.prompt("test").await.unwrap();
    assert!(result.len() >= 3);

    // Both tools should execute without any prompt.
    let log = audit.lock().unwrap();
    assert!(log.len() >= 2, "should have audited both tool calls");
    assert!(log.iter().all(|e| e.decision == "allowed"));
}

#[tokio::test]
async fn non_interactive_auto_denies_with_deny_all() {
    // In non-interactive mode with DenyAll, the extension auto-denies
    // everything without prompting the user.
    let ext = PermissionGateExtension::new(PermissionPolicy::DenyAll);
    let audit = ext.audit_log.clone();

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    let provider = MockProvider::new(
        "mock",
        vec![
            tool_call_response("tc_1", "write", r#"{"path":"/tmp/a","content":"data"}"#),
            text_response("Done"),
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
        AgentLoopConfig {
            max_turns: 10,
            ..Default::default()
        },
        hooks,
    )
    .expect("agent");

    let result = agent.prompt("test").await.unwrap();

    // Tool should be blocked — no TUI prompt occurred.
    let tool_text = extract_tool_result_text(&result);
    assert!(
        tool_text.contains("permission gate denied"),
        "should be auto-denied, got: {tool_text}"
    );

    let log = audit.lock().unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].decision, "denied");
}

#[tokio::test]
async fn non_interactive_auto_denies_with_allow_list_for_unlisted() {
    // Non-interactive mode: tools not in the allow list are auto-denied.
    let ext = PermissionGateExtension::new(PermissionPolicy::AllowList(vec!["read".to_string()]));
    let audit = ext.audit_log.clone();

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    let provider = MockProvider::new(
        "mock",
        vec![
            tool_call_response("tc_1", "bash", r#"{"command":"ls"}"#),
            text_response("Done"),
        ],
    );
    let hooks = registry.wrap_hooks(Box::new(TestHooks));
    let mut agent = opi_agent::Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        common::registrations_from(vec![Box::new(DummyTool::new("bash"))]),
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

    let result = agent.prompt("test").await.unwrap();

    let tool_text = extract_tool_result_text(&result);
    assert!(
        tool_text.contains("permission gate denied"),
        "unlisted tool should be auto-denied, got: {tool_text}"
    );

    let log = audit.lock().unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].tool_name, "bash");
    assert_eq!(log[0].decision, "denied");
}
