//! Protected paths extension/package example tests (task 4.8.2).
//!
//! These tests demonstrate a protected paths extension that blocks or allows
//! file-tool operations based on configured path rules. This is an **example**
//! showing how to build path-based access control as an extension — it is NOT
//! core file-tool policy and does not modify opi's built-in file tool behavior.
//!
//! # What This Example Demonstrates
//!
//! - **Allow/deny by path**: File tools (read, write, edit) are gated by path
//!   rules. Bash is gated by its implicit cwd (the workspace root).
//! - **Path normalization**: Relative paths, `..` traversal, and symlinks are
//!   resolved before checking against rules.
//! - **Audit output**: Every allow/deny decision is recorded in an audit log.
//! - **Non-file tools**: Tools without a `path` argument (glob, grep, etc.)
//!   pass through unaffected.

mod common;

use std::future::Future;
use std::path::{Path, PathBuf};
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
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Path policy
// ---------------------------------------------------------------------------

/// Policy controlling path access for file tools.
#[derive(Debug, Clone)]
enum PathPolicy {
    /// Allow all paths (no restrictions).
    AllowAll,
    /// Deny access to listed paths and their children. Everything else allowed.
    DenyPaths(Vec<PathBuf>),
    /// Only allow access to listed paths and their children. Everything else
    /// denied.
    AllowPaths(Vec<PathBuf>),
}

// ---------------------------------------------------------------------------
// Audit entry
// ---------------------------------------------------------------------------

/// A recorded path access decision.
#[derive(Debug, Clone)]
struct PathAuditEntry {
    tool_name: String,
    path: String,
    decision: String,
    reason: Option<String>,
}

// ---------------------------------------------------------------------------
// Protected paths extension
// ---------------------------------------------------------------------------

/// A protected paths extension that gates file-tool operations based on
/// configured path rules.
///
/// This is an **example extension** demonstrating how to use extension hooks
/// for path-based access control. It is NOT core policy. In a real deployment,
/// the rules could be driven by configuration files, project manifests, or
/// external services.
struct ProtectedPathsExtension {
    policy: PathPolicy,
    workspace_root: PathBuf,
    audit_log: Arc<Mutex<Vec<PathAuditEntry>>>,
    events_received: Arc<Mutex<Vec<String>>>,
}

impl ProtectedPathsExtension {
    /// Create a new protected paths extension with the given policy and
    /// workspace root.
    fn new(policy: PathPolicy, workspace_root: PathBuf) -> Self {
        Self {
            policy,
            workspace_root,
            audit_log: Arc::new(Mutex::new(Vec::new())),
            events_received: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Extract the file path from tool arguments based on tool name.
    ///
    /// For read/write/edit: extracts the `path` argument.
    /// For bash: uses the workspace root as the implicit cwd.
    fn extract_path(&self, tool_name: &str, args: &serde_json::Value) -> Option<String> {
        match tool_name {
            "bash" => Some(self.workspace_root.to_string_lossy().to_string()),
            _ => args["path"].as_str().map(|s| s.to_string()),
        }
    }

    /// Normalize a path: resolve relative paths against workspace root,
    /// resolve `..` components, and canonicalize symlinks when possible.
    ///
    /// Falls back to parent-directory canonicalization + filename when the
    /// target file does not exist yet, ensuring consistent path forms across
    /// existing and non-existing targets.
    fn normalize_path(&self, path_str: &str) -> PathBuf {
        let path = Path::new(path_str);
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.workspace_root.join(path)
        };

        // Try canonicalize directly.
        if let Ok(canonical) = absolute.canonicalize() {
            return canonical;
        }

        // If the file doesn't exist, try canonicalizing the parent and
        // appending the filename. This keeps the path form consistent with
        // other canonicalized paths (e.g., UNC prefix on Windows).
        if let Some(parent) = absolute.parent()
            && let Ok(canonical_parent) = parent.canonicalize()
            && let Some(name) = absolute.file_name()
        {
            return canonical_parent.join(name);
        }

        // Last resort: resolve . and .. manually.
        Self::resolve_components(&absolute)
    }

    /// Normalize a rule path (may be relative to workspace root).
    fn normalize_rule_path(&self, rule: &Path) -> PathBuf {
        let absolute = if rule.is_absolute() {
            rule.to_path_buf()
        } else {
            self.workspace_root.join(rule)
        };
        absolute
            .canonicalize()
            .unwrap_or_else(|_| Self::resolve_components(&absolute))
    }

    /// Resolve `.` and `..` components without requiring the path to exist.
    fn resolve_components(path: &Path) -> PathBuf {
        let mut components = Vec::new();
        for comp in path.components() {
            match comp {
                std::path::Component::ParentDir => {
                    components.pop();
                }
                std::path::Component::CurDir => {}
                c => components.push(c),
            }
        }
        components.iter().collect()
    }

    /// Check if a target path falls under a rule path (prefix match).
    fn path_matches(rule_path: &Path, target: &Path) -> bool {
        target.starts_with(rule_path)
    }

    /// Evaluate whether a file-tool operation should be allowed.
    fn evaluate(&self, tool_name: &str, args: &serde_json::Value) -> ExtensionHookResult {
        // Only check file tools: read, write, edit, bash.
        let is_file_tool = matches!(tool_name, "read" | "write" | "edit" | "bash");
        if !is_file_tool {
            return ExtensionHookResult::Continue;
        }

        let path_str = match self.extract_path(tool_name, args) {
            Some(p) => p,
            None => return ExtensionHookResult::Continue,
        };

        let target = self.normalize_path(&path_str);

        let (decision, reason) = match &self.policy {
            PathPolicy::AllowAll => ("allowed".to_string(), None),
            PathPolicy::DenyPaths(denied) => {
                let matched = denied.iter().any(|d| {
                    let rule = self.normalize_rule_path(d);
                    Self::path_matches(&rule, &target)
                });
                if matched {
                    (
                        "denied".to_string(),
                        Some(format!(
                            "protected-paths: path '{}' is denied by policy",
                            target.display()
                        )),
                    )
                } else {
                    ("allowed".to_string(), None)
                }
            }
            PathPolicy::AllowPaths(allowed) => {
                let matched = allowed.iter().any(|a| {
                    let rule = self.normalize_rule_path(a);
                    Self::path_matches(&rule, &target)
                });
                if matched {
                    ("allowed".to_string(), None)
                } else {
                    (
                        "denied".to_string(),
                        Some(format!(
                            "protected-paths: path '{}' is not in allowed paths",
                            target.display()
                        )),
                    )
                }
            }
        };

        self.audit_log.lock().unwrap().push(PathAuditEntry {
            tool_name: tool_name.to_string(),
            path: target.to_string_lossy().to_string(),
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

impl Extension for ProtectedPathsExtension {
    fn name(&self) -> &str {
        "protected-paths"
    }

    fn on_before_tool_call(
        &self,
        tool_name: &str,
        args: &serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = ExtensionHookResult> + Send>> {
        let result = self.evaluate(tool_name, args);
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
                    "path": e.path,
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
                log.push(PathAuditEntry {
                    tool_name: entry["tool_name"].as_str().unwrap_or("").to_string(),
                    path: entry["path"].as_str().unwrap_or("").to_string(),
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

/// Build a tool-call JSON argument string for a path-based tool.
fn path_args(path: &Path) -> String {
    serde_json::json!({"path": path.to_string_lossy().to_string()}).to_string()
}

/// Build a tool-call JSON argument string for a write tool.
fn write_args(path: &Path, content: &str) -> String {
    serde_json::json!({
        "path": path.to_string_lossy().to_string(),
        "content": content
    })
    .to_string()
}

/// Build a tool-call JSON argument string for a bash tool.
fn bash_args(command: &str) -> String {
    serde_json::json!({"command": command}).to_string()
}

/// Create a directory symlink. Returns Ok(()) on success, Err if symlinks are
/// not available on this platform (e.g., Windows without Developer Mode).
fn create_symlink_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(src, dst)
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_dir(src, dst)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (src, dst);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "symlinks not supported",
        ))
    }
}

// ---------------------------------------------------------------------------
// Tests: Allow
// ---------------------------------------------------------------------------

#[tokio::test]
async fn allow_all_permits_read_and_write() {
    let dir = TempDir::new().unwrap();
    let file_a = dir.path().join("a.txt");
    let file_b = dir.path().join("b.txt");
    std::fs::write(&file_a, "content-a").unwrap();

    let ext = ProtectedPathsExtension::new(PathPolicy::AllowAll, dir.path().to_path_buf());
    let audit = ext.audit_log.clone();

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    let provider = MockProvider::new(
        "mock",
        vec![
            tool_call_response("tc_1", "read", &path_args(&file_a)),
            tool_call_response("tc_2", "write", &write_args(&file_b, "content-b")),
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

    let result = agent.prompt("test").await.unwrap();
    let tool_text = extract_tool_result_text(&result);
    assert!(
        tool_text.contains("ok"),
        "both tools should execute, got: {tool_text}"
    );

    let log = audit.lock().unwrap();
    assert_eq!(log.len(), 2);
    assert_eq!(log[0].tool_name, "read");
    assert_eq!(log[0].decision, "allowed");
    assert_eq!(log[1].tool_name, "write");
    assert_eq!(log[1].decision, "allowed");
}

#[tokio::test]
async fn allow_paths_permits_listed_paths() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    let file = src.join("main.rs");
    std::fs::write(&file, "fn main() {}").unwrap();

    let ext = ProtectedPathsExtension::new(
        PathPolicy::AllowPaths(vec![src.clone()]),
        dir.path().to_path_buf(),
    );
    let audit = ext.audit_log.clone();

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    let provider = MockProvider::new(
        "mock",
        vec![
            tool_call_response("tc_1", "read", &path_args(&file)),
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
        "listed path should execute, got: {tool_text}"
    );

    let log = audit.lock().unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].decision, "allowed");
}

// ---------------------------------------------------------------------------
// Tests: Deny
// ---------------------------------------------------------------------------

#[tokio::test]
async fn deny_paths_blocks_matching_write() {
    let dir = TempDir::new().unwrap();
    let secret = dir.path().join("secret");
    std::fs::create_dir_all(&secret).unwrap();

    let ext = ProtectedPathsExtension::new(
        PathPolicy::DenyPaths(vec![secret.clone()]),
        dir.path().to_path_buf(),
    );
    let audit = ext.audit_log.clone();

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    let file_path = secret.join("key.pem");
    let provider = MockProvider::new(
        "mock",
        vec![
            tool_call_response("tc_1", "write", &write_args(&file_path, "secret")),
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
        tool_text.contains("protected-paths") && tool_text.contains("denied"),
        "write to denied path should be blocked, got: {tool_text}"
    );
    assert!(!tool_text.contains("ok"), "tool should NOT have executed");

    let log = audit.lock().unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].tool_name, "write");
    assert_eq!(log[0].decision, "denied");
}

#[tokio::test]
async fn deny_paths_allows_non_matching_path() {
    let dir = TempDir::new().unwrap();
    let secret = dir.path().join("secret");
    std::fs::create_dir_all(&secret).unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();

    let ext = ProtectedPathsExtension::new(
        PathPolicy::DenyPaths(vec![secret.clone()]),
        dir.path().to_path_buf(),
    );
    let audit = ext.audit_log.clone();

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    let file = src.join("main.rs");
    std::fs::write(&file, "fn main() {}").unwrap();
    let provider = MockProvider::new(
        "mock",
        vec![
            tool_call_response("tc_1", "read", &path_args(&file)),
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
        "non-denied path should execute, got: {tool_text}"
    );

    let log = audit.lock().unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].decision, "allowed");
}

// ---------------------------------------------------------------------------
// Tests: Edit tool
// ---------------------------------------------------------------------------

#[tokio::test]
async fn deny_paths_blocks_edit_on_protected_file() {
    let dir = TempDir::new().unwrap();
    let protected = dir.path().join("protected");
    std::fs::create_dir_all(&protected).unwrap();

    let ext = ProtectedPathsExtension::new(
        PathPolicy::DenyPaths(vec![protected.clone()]),
        dir.path().to_path_buf(),
    );

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    let file = protected.join("config.toml");
    std::fs::write(&file, "key = \"value\"").unwrap();
    let provider = MockProvider::new(
        "mock",
        vec![
            tool_call_response("tc_1", "edit", &path_args(&file)),
            text_response("Done"),
        ],
    );
    let hooks = registry.wrap_hooks(Box::new(TestHooks));
    let mut agent = opi_agent::Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        common::registrations_from(vec![Box::new(DummyTool::new("edit"))]),
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
        tool_text.contains("protected-paths"),
        "edit on protected path should be blocked, got: {tool_text}"
    );
}

// ---------------------------------------------------------------------------
// Tests: Bash cwd interaction
// ---------------------------------------------------------------------------

#[tokio::test]
async fn deny_paths_blocks_bash_when_workspace_root_is_denied() {
    let dir = TempDir::new().unwrap();

    // Deny the workspace root itself — bash's implicit cwd is denied.
    let ext = ProtectedPathsExtension::new(
        PathPolicy::DenyPaths(vec![dir.path().to_path_buf()]),
        dir.path().to_path_buf(),
    );

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    let provider = MockProvider::new(
        "mock",
        vec![
            tool_call_response("tc_1", "bash", &bash_args("ls")),
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
        tool_text.contains("protected-paths"),
        "bash should be blocked when workspace root is denied, got: {tool_text}"
    );
}

// ---------------------------------------------------------------------------
// Tests: Path normalization
// ---------------------------------------------------------------------------

#[tokio::test]
async fn parent_traversal_normalizes_to_workspace_root() {
    let dir = TempDir::new().unwrap();
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();

    // Place a secret file outside the workspace.
    let outside = dir.path().join("secret.txt");
    std::fs::write(&outside, "secret").unwrap();

    // Only allow the workspace directory.
    let ext = ProtectedPathsExtension::new(
        PathPolicy::AllowPaths(vec![workspace.clone()]),
        workspace.clone(),
    );

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    // Try to read ../secret.txt from within the workspace.
    let provider = MockProvider::new(
        "mock",
        vec![
            tool_call_response("tc_1", "read", r#"{"path":"../secret.txt"}"#),
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
        tool_text.contains("protected-paths"),
        "parent traversal should be blocked, got: {tool_text}"
    );
}

#[tokio::test]
async fn dot_dot_in_path_is_resolved() {
    let dir = TempDir::new().unwrap();
    let protected = dir.path().join("private");
    std::fs::create_dir_all(&protected).unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();

    let ext = ProtectedPathsExtension::new(
        PathPolicy::DenyPaths(vec![protected.clone()]),
        dir.path().to_path_buf(),
    );

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    // Access private/data via src/../private/data
    let traversal_path = src.join("..").join("private").join("data");
    let provider = MockProvider::new(
        "mock",
        vec![
            tool_call_response("tc_1", "write", &write_args(&traversal_path, "x")),
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
        tool_text.contains("protected-paths"),
        "path traversal through .. should be resolved and blocked, got: {tool_text}"
    );
}

#[tokio::test]
async fn absolute_path_outside_workspace_blocked() {
    let dir = TempDir::new().unwrap();
    let workspace = dir.path().join("project");
    std::fs::create_dir_all(&workspace).unwrap();

    let ext = ProtectedPathsExtension::new(
        PathPolicy::AllowPaths(vec![workspace.clone()]),
        workspace.clone(),
    );

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    // Absolute path outside the allowed workspace.
    let other = dir.path().join("other");
    std::fs::create_dir_all(&other).unwrap();
    let outside = other.join("file.txt");
    std::fs::write(&outside, "data").unwrap();
    let provider = MockProvider::new(
        "mock",
        vec![
            tool_call_response("tc_1", "read", &path_args(&outside)),
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
        tool_text.contains("protected-paths"),
        "absolute path outside workspace should be blocked, got: {tool_text}"
    );
}

// ---------------------------------------------------------------------------
// Tests: Symlink traversal
// ---------------------------------------------------------------------------

#[tokio::test]
async fn symlink_traversal_to_protected_path_blocked() {
    let dir = TempDir::new().unwrap();
    let protected = dir.path().join("secrets");
    std::fs::create_dir_all(&protected).unwrap();
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();

    // Create a symlink: workspace/link -> ../secrets
    let link = workspace.join("link");
    if create_symlink_dir(&protected, &link).is_err() {
        eprintln!("Skipping symlink test: symlink creation not available");
        return;
    }

    let ext = ProtectedPathsExtension::new(
        PathPolicy::DenyPaths(vec![protected.clone()]),
        workspace.clone(),
    );

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    // Access secrets through the symlink.
    let via_symlink = link.join("key.pem");
    let provider = MockProvider::new(
        "mock",
        vec![
            tool_call_response("tc_1", "read", &path_args(&via_symlink)),
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
        tool_text.contains("protected-paths"),
        "symlink traversal to protected path should be blocked, got: {tool_text}"
    );
}

// ---------------------------------------------------------------------------
// Tests: Non-file tools
// ---------------------------------------------------------------------------

#[tokio::test]
async fn non_file_tools_pass_through_unaffected() {
    let dir = TempDir::new().unwrap();

    let ext = ProtectedPathsExtension::new(
        PathPolicy::DenyPaths(vec![dir.path().join("everything")]),
        dir.path().to_path_buf(),
    );
    let audit = ext.audit_log.clone();

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    let provider = MockProvider::new(
        "mock",
        vec![
            tool_call_response("tc_1", "glob", r#"{"pattern":"**/*.rs"}"#),
            text_response("Done"),
        ],
    );
    let hooks = registry.wrap_hooks(Box::new(TestHooks));
    let mut agent = opi_agent::Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        common::registrations_from(vec![Box::new(DummyTool::new("glob"))]),
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
        "non-file tool should pass through, got: {tool_text}"
    );

    // Non-file tools should not generate audit entries.
    let log = audit.lock().unwrap();
    assert!(
        log.is_empty(),
        "non-file tools should not generate audit entries, got: {}",
        log.len()
    );
}

// ---------------------------------------------------------------------------
// Tests: Audit/event output
// ---------------------------------------------------------------------------

#[tokio::test]
async fn audit_log_records_allow_and_deny_across_turns() {
    let dir = TempDir::new().unwrap();
    let protected = dir.path().join("private");
    std::fs::create_dir_all(&protected).unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();

    let ext = ProtectedPathsExtension::new(
        PathPolicy::DenyPaths(vec![protected.clone()]),
        dir.path().to_path_buf(),
    );
    let audit = ext.audit_log.clone();

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    let allowed_file = src.join("main.rs");
    std::fs::write(&allowed_file, "fn main() {}").unwrap();
    let denied_file = protected.join("key.pem");
    std::fs::write(&denied_file, "secret").unwrap();

    let provider = MockProvider::new(
        "mock",
        vec![
            // Turn 1: read src/main.rs (allowed)
            tool_call_response("tc_1", "read", &path_args(&allowed_file)),
            // Turn 2: read private/key.pem (denied)
            tool_call_response("tc_2", "read", &path_args(&denied_file)),
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

    let _ = agent.prompt("test").await.unwrap();

    let log = audit.lock().unwrap();
    assert!(log.len() >= 2, "should have at least 2 audit entries");
    assert_eq!(log[0].tool_name, "read");
    assert_eq!(log[0].decision, "allowed");
    assert_eq!(log[1].tool_name, "read");
    assert_eq!(log[1].decision, "denied");
    assert!(log[1].reason.is_some());
}

#[tokio::test]
async fn extension_receives_agent_events() {
    let dir = TempDir::new().unwrap();
    let ext = ProtectedPathsExtension::new(PathPolicy::AllowAll, dir.path().to_path_buf());
    let events = ext.events_received.clone();

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(ext)).unwrap();

    let base_sink =
        Box::new(|_event: opi_agent::event::AgentEvent| {}) as opi_agent::event::AgentEventSink;
    let wrapped_sink = registry.wrap_event_sink(base_sink);

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
    let dir = TempDir::new().unwrap();
    let secrets = dir.path().join("secrets");
    std::fs::create_dir_all(&secrets).unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();

    let ext = ProtectedPathsExtension::new(
        PathPolicy::DenyPaths(vec![secrets.clone()]),
        dir.path().to_path_buf(),
    );

    // Populate the audit log through evaluate calls with real paths.
    let allowed_path = src.join("main.rs");
    std::fs::write(&allowed_path, "").unwrap();
    let denied_path = secrets.join("key.pem");
    std::fs::write(&denied_path, "").unwrap();

    ext.evaluate(
        "read",
        &serde_json::json!({"path": allowed_path.to_string_lossy()}),
    );
    ext.evaluate(
        "write",
        &serde_json::json!({"path": denied_path.to_string_lossy()}),
    );

    // Serialize.
    let state = ext.serialize_state().unwrap().unwrap();
    assert_eq!(state["audit_log"].as_array().unwrap().len(), 2);

    // Restore into a new extension.
    let ext2 = ProtectedPathsExtension::new(PathPolicy::AllowAll, dir.path().to_path_buf());
    ext2.restore_state(state).unwrap();

    let log = ext2.audit_log.lock().unwrap();
    assert_eq!(log.len(), 2);
    assert_eq!(log[0].tool_name, "read");
    assert_eq!(log[0].decision, "allowed");
    assert_eq!(log[1].tool_name, "write");
    assert_eq!(log[1].decision, "denied");
}
