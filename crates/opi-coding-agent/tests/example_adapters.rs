//! Integration tests for runnable example adapter packages (task 5.8).
//!
//! Verifies that the todo, permission-gate, and protected-paths example packages
//! declare process adapters and can be exercised through the full
//! discover-start-capabilities-command-hook pipeline without Node, npm, or
//! live providers.

use std::path::PathBuf;
use std::time::Duration;

use opi_agent::extension::{Extension, ExtensionCommand, ExtensionHookResult, ExtensionRegistry};
use opi_coding_agent::adapter_extension::start_adapters_from_packages;
use opi_coding_agent::adapter_host::{AdapterHost, AdapterProcessConfig};
use opi_coding_agent::package_discovery::{AdapterManifest, PackageManifest, PackageResource};

// ---------------------------------------------------------------------------
// Binary discovery
// ---------------------------------------------------------------------------

/// Locate the `package_adapter_example` test binary, preferring the newest version.
fn example_adapter_bin() -> PathBuf {
    let current = std::env::current_exe().expect("current exe path");
    let deps_dir = current.parent().expect("deps directory");

    let exact_name = if cfg!(windows) {
        "package_adapter_example.exe"
    } else {
        "package_adapter_example"
    };
    let exact_path = deps_dir.join(exact_name);
    if exact_path.exists() {
        return exact_path;
    }

    let exe_suffix = if cfg!(windows) { ".exe" } else { "" };
    let prefix = "package_adapter_example-";
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    if let Ok(entries) = std::fs::read_dir(deps_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with(prefix)
                && name_str.ends_with(exe_suffix)
                && !name_str.ends_with(".d")
                && let Ok(meta) = entry.metadata()
                && let Ok(modified) = meta.modified()
                && best.as_ref().is_none_or(|(t, _)| modified > *t)
            {
                best = Some((modified, entry.path()));
            }
        }
    }

    best.map(|(_, p)| p)
        .expect("Could not find package_adapter_example binary in deps directory")
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_example_package(name: &str, mode: &str, precedence: u32) -> PackageResource {
    let dir = tempfile::tempdir().expect("tempdir for package");
    let toml_path = dir.path().join("package.toml");

    // Keep the tempdir alive for the test duration by leaking it
    let dir_path = dir.keep();

    PackageResource {
        manifest: PackageManifest {
            name: name.to_string(),
            description: format!("Example {name} adapter package"),
            version: Some("0.1.0".to_string()),
            opi_version: None,
            adapter: Some(AdapterManifest {
                kind: "process-jsonl".to_string(),
                command: example_adapter_bin().to_string_lossy().to_string(),
                args: vec![mode.to_string()],
                protocol: "opi-extension-jsonl-v1".to_string(),
                timeout_ms: None,
            }),
            extensions: None,
            skills: None,
            fragments: None,
            themes: None,
            disabled: vec![],
            adapter_contributions: vec![],
        },
        path: dir_path,
        package_toml_path: toml_path,
        layer_precedence: precedence,
    }
}

async fn start_example(mode: &str) -> (std::sync::Arc<AdapterHost>, Box<dyn Extension>) {
    let bin = example_adapter_bin();
    let config = AdapterProcessConfig {
        command: bin,
        args: vec![mode.to_string()],
        working_dir: std::env::current_dir().expect("cwd"),
        env: vec![],
    };

    let host = AdapterHost::start(mode, config, Duration::from_secs(10))
        .await
        .expect("start adapter");

    let caps = host.capabilities().clone();
    let arc_host = std::sync::Arc::new(host);
    let adapter = opi_coding_agent::adapter_extension::ProcessAdapter::from_host(
        mode,
        arc_host.clone(),
        caps,
    );

    (arc_host, Box::new(adapter))
}

/// Start adapter directly (bypassing start_adapters_from_packages) for
/// unit-level testing of the adapter binary's behavior.
async fn start_host(mode: &str) -> std::sync::Arc<AdapterHost> {
    let bin = example_adapter_bin();
    let config = AdapterProcessConfig {
        command: bin,
        args: vec![mode.to_string()],
        working_dir: std::env::current_dir().expect("cwd"),
        env: vec![],
    };
    let host = AdapterHost::start(mode, config, Duration::from_secs(10))
        .await
        .expect("start adapter");
    std::sync::Arc::new(host)
}

// ===========================================================================
// todo adapter tests
// ===========================================================================

// ---------------------------------------------------------------------------
// 1. Todo adapter starts and advertises commands
// ---------------------------------------------------------------------------

#[tokio::test]
async fn example_adapter_todo_advertises_commands() {
    let host = start_host("todo").await;
    let caps = host.capabilities();

    assert!(caps.tools.is_empty(), "todo adapter has no tools");
    assert_eq!(caps.commands.len(), 4, "todo adapter has 4 commands");

    let cmd_names: Vec<&str> = caps.commands.iter().map(|c| c.name.as_str()).collect();
    assert!(cmd_names.contains(&"todo/add"), "commands: {cmd_names:?}");
    assert!(cmd_names.contains(&"todo/list"), "commands: {cmd_names:?}");
    assert!(
        cmd_names.contains(&"todo/update"),
        "commands: {cmd_names:?}"
    );
    assert!(
        cmd_names.contains(&"todo/complete"),
        "commands: {cmd_names:?}"
    );
    assert!(caps.hooks.contains(&"event".to_string()));
}

// ---------------------------------------------------------------------------
// 2. Todo add and list command dispatch
// ---------------------------------------------------------------------------

#[tokio::test]
async fn example_adapter_todo_add_and_list() {
    let (_host, adapter) = start_example("todo").await;

    let add_cmd = ExtensionCommand::new(
        "todo/add",
        serde_json::json!({"title": "write tests", "description": "write adapter tests"}),
    );
    let result = adapter.on_command(&add_cmd).await.expect("add command");
    assert!(result.is_some());
    let item = result.unwrap();
    assert_eq!(item["title"], "write tests");
    assert_eq!(item["status"], "pending");

    let list_cmd = ExtensionCommand::new("todo/list", serde_json::json!({}));
    let result = adapter.on_command(&list_cmd).await.expect("list command");
    assert!(result.is_some());
    let data = result.unwrap();
    let items = data["items"].as_array().expect("items array");
    assert_eq!(items.len(), 1);
}

// ---------------------------------------------------------------------------
// 3. Todo complete command dispatch
// ---------------------------------------------------------------------------

#[tokio::test]
async fn example_adapter_todo_complete() {
    let (_host, adapter) = start_example("todo").await;

    // Add an item first
    let add_cmd = ExtensionCommand::new("todo/add", serde_json::json!({"title": "complete me"}));
    let result = adapter.on_command(&add_cmd).await.expect("add");
    let item_id = result.unwrap()["id"].as_str().unwrap().to_string();

    // Complete it
    let complete_cmd = ExtensionCommand::new("todo/complete", serde_json::json!({"id": item_id}));
    let result = adapter.on_command(&complete_cmd).await.expect("complete");
    assert!(result.is_some());
    assert_eq!(result.unwrap()["status"], "completed");
}

// ---------------------------------------------------------------------------
// 4. Todo state serialize/restore round-trip
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn example_adapter_todo_state_round_trip() {
    let (_host, adapter) = start_example("todo").await;

    // Add an item to create state
    let add_cmd = ExtensionCommand::new("todo/add", serde_json::json!({"title": "persist me"}));
    let _ = adapter.on_command(&add_cmd).await;

    // Serialize
    let state = adapter.serialize_state().expect("serialize");
    assert!(state.is_some());
    let state = state.unwrap();
    assert!(state["items"].is_array());
    assert_eq!(state["items"].as_array().unwrap().len(), 1);
    assert!(state["next_id"].is_number());

    // Restore into a fresh adapter (same process)
    adapter.restore_state(state).expect("restore");
}

// ===========================================================================
// permission-gate adapter tests
// ===========================================================================

// ---------------------------------------------------------------------------
// 5. Permission-gate adapter starts and advertises hooks
// ---------------------------------------------------------------------------

#[tokio::test]
async fn example_adapter_permission_gate_advertises_hooks() {
    let host = start_host("permission-gate").await;
    let caps = host.capabilities();

    assert!(caps.tools.is_empty());
    assert!(caps.commands.is_empty());
    assert!(
        caps.hooks.contains(&"before_tool_call".to_string()),
        "hooks: {:?}",
        caps.hooks
    );
}

// ---------------------------------------------------------------------------
// 6. Permission-gate blocks mutating tools
// ---------------------------------------------------------------------------

#[tokio::test]
async fn example_adapter_permission_gate_blocks_mutating_tools() {
    let (_host, adapter) = start_example("permission-gate").await;

    for tool in &["bash", "write", "edit"] {
        let result = adapter
            .on_before_tool_call(tool, &serde_json::json!({}))
            .await;
        assert!(
            matches!(result, ExtensionHookResult::Block { .. }),
            "expected Block for tool '{tool}', got {result:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// 7. Permission-gate allows read-only tools
// ---------------------------------------------------------------------------

#[tokio::test]
async fn example_adapter_permission_gate_allows_readonly_tools() {
    let (_host, adapter) = start_example("permission-gate").await;

    for tool in &["read", "glob", "grep"] {
        let result = adapter
            .on_before_tool_call(tool, &serde_json::json!({}))
            .await;
        assert!(
            matches!(result, ExtensionHookResult::Continue),
            "expected Continue for tool '{tool}', got {result:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// 8. Permission-gate state round-trip
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn example_adapter_permission_gate_state_round_trip() {
    let (_host, adapter) = start_example("permission-gate").await;

    // Trigger a hook to populate audit log
    let _ = adapter
        .on_before_tool_call("bash", &serde_json::json!({}))
        .await;
    let _ = adapter
        .on_before_tool_call("read", &serde_json::json!({}))
        .await;

    let state = adapter.serialize_state().expect("serialize");
    assert!(state.is_some());
    let state = state.unwrap();
    assert!(state["audit_log"].is_array());

    adapter.restore_state(state).expect("restore");
}

// ===========================================================================
// protected-paths adapter tests
// ===========================================================================

// ---------------------------------------------------------------------------
// 9. Protected-paths adapter starts and advertises hooks
// ---------------------------------------------------------------------------

#[tokio::test]
async fn example_adapter_protected_paths_advertises_hooks() {
    let host = start_host("protected-paths").await;
    let caps = host.capabilities();

    assert!(caps.tools.is_empty());
    assert!(caps.commands.is_empty());
    assert!(
        caps.hooks.contains(&"before_tool_call".to_string()),
        "hooks: {:?}",
        caps.hooks
    );
}

// ---------------------------------------------------------------------------
// 10. Protected-paths blocks /etc/passwd
// ---------------------------------------------------------------------------

#[tokio::test]
async fn example_adapter_protected_paths_blocks_etc() {
    let (_host, adapter) = start_example("protected-paths").await;

    let result = adapter
        .on_before_tool_call("read", &serde_json::json!({"path": "/etc/passwd"}))
        .await;
    assert!(
        matches!(result, ExtensionHookResult::Block { .. }),
        "expected Block for /etc/passwd, got {result:?}"
    );
}

// ---------------------------------------------------------------------------
// 11. Protected-paths allows safe paths
// ---------------------------------------------------------------------------

#[tokio::test]
async fn example_adapter_protected_paths_allows_safe_paths() {
    let (_host, adapter) = start_example("protected-paths").await;

    let result = adapter
        .on_before_tool_call(
            "read",
            &serde_json::json!({"path": "/home/user/project/src/main.rs"}),
        )
        .await;
    assert!(
        matches!(result, ExtensionHookResult::Continue),
        "expected Continue for safe path, got {result:?}"
    );
}

// ---------------------------------------------------------------------------
// 12. Protected-paths state round-trip
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn example_adapter_protected_paths_state_round_trip() {
    let (_host, adapter) = start_example("protected-paths").await;

    // Trigger hooks to populate audit log
    let _ = adapter
        .on_before_tool_call("read", &serde_json::json!({"path": "/etc/hosts"}))
        .await;
    let _ = adapter
        .on_before_tool_call("read", &serde_json::json!({"path": "/tmp/safe"}))
        .await;

    let state = adapter.serialize_state().expect("serialize");
    assert!(state.is_some());
    let state = state.unwrap();
    assert!(state["audit_log"].is_array());

    adapter.restore_state(state).expect("restore");
}

// ===========================================================================
// Full pipeline: start_adapters_from_packages with example packages
// ===========================================================================

// ---------------------------------------------------------------------------
// 13. All three example adapters start from packages
// ---------------------------------------------------------------------------

#[tokio::test]
async fn example_adapters_all_start_from_packages() {
    let dir = tempfile::tempdir().expect("tempdir");
    let packages = vec![
        make_example_package("todo", "todo", 0),
        make_example_package("permission-gate", "permission-gate", 1),
        make_example_package("protected-paths", "protected-paths", 2),
    ];

    let registry = ExtensionRegistry::new();
    let (registry, diagnostics) =
        start_adapters_from_packages(&packages, dir.path(), registry).await;

    assert!(
        diagnostics.is_empty(),
        "unexpected diagnostics: {diagnostics:?}"
    );
    let names = registry.names();
    assert_eq!(names.len(), 3, "all three adapters should be registered");
    assert!(names.contains(&"todo"));
    assert!(names.contains(&"permission-gate"));
    assert!(names.contains(&"protected-paths"));
}

// ---------------------------------------------------------------------------
// 14. Todo adapter from package dispatches commands through registry
// ---------------------------------------------------------------------------

#[tokio::test]
async fn example_adapter_todo_from_package_dispatches_command() {
    let dir = tempfile::tempdir().expect("tempdir");
    let packages = vec![make_example_package("todo", "todo", 0)];

    let registry = ExtensionRegistry::new();
    let (registry, diagnostics) =
        start_adapters_from_packages(&packages, dir.path(), registry).await;
    assert!(
        diagnostics.is_empty(),
        "unexpected diagnostics: {diagnostics:?}"
    );

    let cmd = ExtensionCommand::new("todo/list", serde_json::json!({}));
    let result = registry.dispatch_command(&cmd).await.expect("dispatch");
    assert!(result.is_some(), "todo adapter should handle todo/list");
    assert!(result.unwrap()["items"].is_array());
}

// ---------------------------------------------------------------------------
// 15. Permission-gate from package blocks through registry hooks
// ---------------------------------------------------------------------------

#[tokio::test]
async fn example_adapter_permission_gate_blocks_through_registry() {
    let dir = tempfile::tempdir().expect("tempdir");
    let packages = vec![make_example_package(
        "permission-gate",
        "permission-gate",
        0,
    )];

    let registry = ExtensionRegistry::new();
    let (registry, diagnostics) =
        start_adapters_from_packages(&packages, dir.path(), registry).await;
    assert!(
        diagnostics.is_empty(),
        "unexpected diagnostics: {diagnostics:?}"
    );

    let hooks = registry.wrap_hooks(Box::new(opi_coding_agent::harness::CodingAgentHooks));

    let ctx = opi_agent::hooks::BeforeToolCallContext {
        tool_call_id: "call-1".into(),
        tool_name: "bash".into(),
        args: serde_json::json!({"command": "rm -rf /"}),
        messages: vec![],
    };

    let result = hooks.before_tool_call(ctx).await;
    assert!(
        matches!(result, opi_agent::hooks::BeforeToolCallResult::Deny { .. }),
        "permission-gate should deny bash through composite hooks"
    );
}

// ---------------------------------------------------------------------------
// 16. Protected-paths from package allows safe paths through registry
// ---------------------------------------------------------------------------

#[tokio::test]
async fn example_adapter_protected_paths_allows_through_registry() {
    let dir = tempfile::tempdir().expect("tempdir");
    let packages = vec![make_example_package(
        "protected-paths",
        "protected-paths",
        0,
    )];

    let registry = ExtensionRegistry::new();
    let (registry, diagnostics) =
        start_adapters_from_packages(&packages, dir.path(), registry).await;
    assert!(
        diagnostics.is_empty(),
        "unexpected diagnostics: {diagnostics:?}"
    );

    let hooks = registry.wrap_hooks(Box::new(opi_coding_agent::harness::CodingAgentHooks));

    let ctx = opi_agent::hooks::BeforeToolCallContext {
        tool_call_id: "call-2".into(),
        tool_name: "read".into(),
        args: serde_json::json!({"path": "/home/user/code/main.rs"}),
        messages: vec![],
    };

    let result = hooks.before_tool_call(ctx).await;
    assert!(
        matches!(result, opi_agent::hooks::BeforeToolCallResult::Allow),
        "protected-paths should allow safe read through composite hooks"
    );
}
