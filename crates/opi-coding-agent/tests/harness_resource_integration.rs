use std::path::{Path, PathBuf};

use opi_agent::Diagnostic;
use opi_agent::diagnostic::{SOURCE_ADAPTER, Severity, code};
use opi_agent::extension::ExtensionRegistry;
use opi_agent::session::{
    ExtensionStateEntry, MessageEntry, SessionEntry, SessionHeader, SessionWriter,
};
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::test_support::MockProvider;
use opi_coding_agent::adapter_extension::start_adapters_from_packages;
use opi_coding_agent::config::OpiConfig;
use opi_coding_agent::harness::{CodingHarness, DiscoveredResourceMetadata, ResumeInfo};
use opi_coding_agent::package_discovery::{
    AdapterManifest, PackageManifest, PackageResource, resolve_adapter_command,
};
use opi_coding_agent::package_resolver::local_lock_entry;
use opi_coding_agent::package_store::{PackageDeclaration, PackageStore};
use opi_coding_agent::policy::{RunMode, ToolRuntimeConfig, ToolSelection};

fn write_package_with_resources(pkg_dir: &Path) {
    std::fs::create_dir_all(pkg_dir).unwrap();
    std::fs::write(
        pkg_dir.join("package.toml"),
        r#"
name = "metadata-suite"
description = "Metadata package."
version = "1.2.3"
"#,
    )
    .unwrap();

    let ext_dir = pkg_dir.join("extensions").join("metadata-ext");
    std::fs::create_dir_all(&ext_dir).unwrap();
    std::fs::write(
        ext_dir.join("extension.toml"),
        r#"[extension]
name = "metadata-ext"
version = "0.1.0"
description = "Metadata extension."
"#,
    )
    .unwrap();

    let skill_dir = pkg_dir.join("skills").join("metadata-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: metadata-skill
description: Metadata skill.
---
FULL SKILL BODY SHOULD NOT LOAD
"#,
    )
    .unwrap();

    let fragment_dir = pkg_dir.join("fragments").join("metadata-fragment");
    std::fs::create_dir_all(&fragment_dir).unwrap();
    std::fs::write(
        fragment_dir.join("FRAGMENT.md"),
        r#"---
name: metadata-fragment
description: Metadata fragment.
arguments: text
---
FULL FRAGMENT BODY SHOULD NOT LOAD
"#,
    )
    .unwrap();

    let theme_dir = pkg_dir.join("themes").join("metadata-theme");
    std::fs::create_dir_all(&theme_dir).unwrap();
    std::fs::write(
        theme_dir.join("theme.toml"),
        r#"
name = "metadata-theme"
description = "Metadata theme."
"#,
    )
    .unwrap();
}

#[test]
fn harness_system_prompt_includes_configured_package_resource_metadata_only() {
    let workspace = tempfile::tempdir().unwrap();
    let global_config = tempfile::tempdir().unwrap();
    let package_dir = workspace.path().join("vendor").join("metadata-suite");
    write_package_with_resources(&package_dir);

    let mut config = OpiConfig::default();
    config.packages.paths = vec![package_dir.strip_prefix(workspace.path()).unwrap().into()];

    let provider = MockProvider::new("mock", Vec::new());
    let harness = CodingHarness::new_with_global_config_dir_tool_config(
        Box::new(provider),
        "mock:mock-model".into(),
        config,
        workspace.path().to_path_buf(),
        Box::new(opi_coding_agent::harness::CodingAgentHooks),
        None,
        Vec::new(),
        None,
        ToolRuntimeConfig {
            run_mode: RunMode::Interactive,
            active_tool_names: Vec::new(),
        },
        Some(global_config.path().to_path_buf()),
    );

    let prompt = harness.system_prompt();
    assert!(prompt.contains("metadata-suite"));
    assert!(prompt.contains("Metadata package."));
    assert!(prompt.contains("metadata-ext"));
    assert!(prompt.contains("Metadata extension."));
    assert!(prompt.contains("metadata-skill"));
    assert!(prompt.contains("Metadata skill."));
    assert!(prompt.contains("metadata-fragment"));
    assert!(prompt.contains("Metadata fragment."));
    assert!(prompt.contains("metadata-theme"));
    assert!(prompt.contains("Metadata theme."));
    assert!(!prompt.contains("FULL SKILL BODY SHOULD NOT LOAD"));
    assert!(!prompt.contains("FULL FRAGMENT BODY SHOULD NOT LOAD"));

    let metadata = harness.resource_metadata();
    assert_eq!(metadata.packages[0].name, "metadata-suite");
    assert_eq!(metadata.skills[0].name, "metadata-skill");

    let theme = harness
        .resolve_theme("metadata-theme")
        .expect("configured package theme should resolve");
    assert_eq!(theme.name, "metadata-theme");
}

#[test]
fn harness_system_prompt_redacts_startup_diagnostics() {
    let workspace = tempfile::tempdir().unwrap();
    let secret = "sk-proj-1234567890abcdefghijklmnopqrstuv";
    let provider = MockProvider::new("mock", Vec::new());
    let harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
    )
    .startup_diagnostics(vec![
        Diagnostic::new(
            Severity::Warning,
            "package_diagnostic",
            "package",
            format!("failed at C:\\Users\\alice\\.config\\opi\\package.toml with {secret}"),
        )
        .action("inspect C:\\Users\\alice\\.config\\opi\\config.toml")
        .details(serde_json::json!({
            "package_error": format!("raw package body {secret}"),
            "adapter_command": "C:\\Users\\alice\\bin\\adapter.exe",
        })),
    ])
    .build();

    let prompt = harness.system_prompt();

    assert!(
        prompt.contains("Resource discovery diagnostics"),
        "system prompt should mention diagnostic summaries: {prompt}"
    );
    assert!(
        !prompt.contains(secret),
        "system prompt leaked secret: {prompt}"
    );
    assert!(
        !prompt.contains("alice"),
        "system prompt leaked absolute path component: {prompt}"
    );
    assert!(
        prompt.contains("[REDACTED]"),
        "system prompt should include redaction marker: {prompt}"
    );
}

#[test]
fn harness_system_prompt_includes_installed_project_package_without_config_paths() {
    let workspace = tempfile::tempdir().unwrap();
    let global_config = tempfile::tempdir().unwrap();
    let package_dir = workspace.path().join("vendor").join("metadata-suite");
    write_package_with_resources(&package_dir);

    let store = PackageStore::project(workspace.path().to_path_buf());
    store
        .write_declarations(&[PackageDeclaration {
            source: "./vendor/metadata-suite".into(),
            filters: Default::default(),
        }])
        .unwrap();
    store
        .write_lock(&[local_lock_entry("./vendor/metadata-suite".into(), &package_dir).unwrap()])
        .unwrap();

    let provider = MockProvider::new("mock", Vec::new());
    let harness = CodingHarness::new_with_global_config_dir_tool_config(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        Box::new(opi_coding_agent::harness::CodingAgentHooks),
        None,
        Vec::new(),
        None,
        ToolRuntimeConfig {
            run_mode: RunMode::Interactive,
            active_tool_names: Vec::new(),
        },
        Some(global_config.path().to_path_buf()),
    );

    let prompt = harness.system_prompt();
    assert!(prompt.contains("metadata-suite"));
    assert!(prompt.contains("Metadata package."));
    assert!(prompt.contains("metadata-skill"));

    let metadata = harness.resource_metadata();
    assert_eq!(metadata.packages[0].name, "metadata-suite");
    assert_eq!(metadata.skills[0].name, "metadata-skill");
}

// ---------------------------------------------------------------------------
// Adapter integration helpers
// ---------------------------------------------------------------------------

/// Locate the `adapter_host_mock` test binary, preferring the newest version.
fn mock_adapter_bin() -> PathBuf {
    let current = std::env::current_exe().expect("current exe path");
    let deps_dir = current.parent().expect("deps directory");

    let exact_name = if cfg!(windows) {
        "adapter_host_mock.exe"
    } else {
        "adapter_host_mock"
    };
    let exact_path = deps_dir.join(exact_name);
    if exact_path.exists() {
        return exact_path;
    }

    let exe_suffix = if cfg!(windows) { ".exe" } else { "" };
    let prefix = "adapter_host_mock-";
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
        .expect("Could not find adapter_host_mock binary in deps directory")
}

fn write_adapter_package_toml(pkg_dir: &Path, name: &str, adapter_command: &Path) {
    std::fs::create_dir_all(pkg_dir).unwrap();
    std::fs::write(
        pkg_dir.join("package.toml"),
        format!(
            "name = \"{name}\"\n\
             description = \"Adapter package.\"\n\
             version = \"0.1.0\"\n\
             [adapter]\n\
             kind = \"process-jsonl\"\n\
             command = \"{}\"\n\
             protocol = \"opi-extension-jsonl-v1\"\n",
            adapter_command.display().to_string().replace('\\', "\\\\")
        ),
    )
    .unwrap();
}

fn package_adapter_example_bin() -> PathBuf {
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

#[tokio::test]
async fn runtime_startup_starts_installed_project_package_adapter() {
    let workspace = tempfile::tempdir().unwrap();
    let global_config = tempfile::tempdir().unwrap();
    let package_dir = workspace.path().join("vendor").join("adapter-suite");
    write_adapter_package_toml(&package_dir, "adapter-suite", &mock_adapter_bin());

    let store = PackageStore::project(workspace.path().to_path_buf());
    store
        .write_declarations(&[PackageDeclaration {
            source: "./vendor/adapter-suite".into(),
            filters: Default::default(),
        }])
        .unwrap();
    store
        .write_lock(&[local_lock_entry("./vendor/adapter-suite".into(), &package_dir).unwrap()])
        .unwrap();

    let startup = opi_coding_agent::runtime_packages::start_installed_package_runtime(
        workspace.path(),
        global_config.path(),
    )
    .await;

    assert!(
        startup.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        startup.diagnostics
    );
    assert_eq!(startup.installed_packages.len(), 1);
    assert_eq!(startup.installed_packages[0].manifest.name, "adapter-suite");
    let tools = startup.extension_registry.collect_tools();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].definition().name, "test_tool");
}

#[tokio::test]
async fn resumed_installed_adapter_state_restores_on_current_thread_runtime() {
    let workspace = tempfile::tempdir().unwrap();
    let global_config = tempfile::tempdir().unwrap();
    let package_dir = workspace.path().join("vendor").join("todo");
    write_adapter_package_toml(&package_dir, "todo", &package_adapter_example_bin());

    let store = PackageStore::project(workspace.path().to_path_buf());
    store
        .write_declarations(&[PackageDeclaration {
            source: "./vendor/todo".into(),
            filters: Default::default(),
        }])
        .unwrap();
    store
        .write_lock(&[local_lock_entry("./vendor/todo".into(), &package_dir).unwrap()])
        .unwrap();

    let session_path = workspace.path().join("session.jsonl");
    let header = SessionHeader::new(
        "sess-adapter-restore".into(),
        "2026-06-15T00:00:00Z".into(),
        workspace.path().display().to_string(),
        None,
    );
    let user = SessionEntry::Message(MessageEntry {
        id: "msg-1".into(),
        parent_id: None,
        timestamp: "2026-06-15T00:00:00Z".into(),
        message: Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "restore state".into(),
            }],
            timestamp_ms: 0,
        }),
    });
    let state = SessionEntry::ExtensionState(ExtensionStateEntry {
        id: "state-1".into(),
        parent_id: Some("msg-1".into()),
        timestamp: "2026-06-15T00:00:01Z".into(),
        state: serde_json::json!({
            "todo": {
                "items": [{
                    "id": "todo-1",
                    "title": "resume me",
                    "description": "state",
                    "status": "pending"
                }],
                "next_id": 2
            }
        }),
    });
    let mut writer = SessionWriter::create(&session_path, header).unwrap();
    writer.append(&user).unwrap();
    writer.append(&state).unwrap();
    drop(writer);
    let entries = vec![user, state];

    let startup = opi_coding_agent::runtime_packages::start_installed_package_runtime(
        workspace.path(),
        global_config.path(),
    )
    .await;
    assert!(
        startup.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        startup.diagnostics
    );

    let initial_messages = opi_coding_agent::session_cli::reconstruct_context(&entries);
    let provider = MockProvider::new(
        "mock",
        vec![opi_ai::test_support::text_response("restored")],
    );
    let resume = ResumeInfo {
        path: session_path,
        session_id: "sess-adapter-restore".into(),
        entries,
        original_cwd: workspace.path().to_path_buf(),
        diagnostics: Vec::new(),
        recorded_model: None,
        recorded_thinking: None,
    };
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
    )
    .initial_messages(initial_messages)
    .resume(resume)
    .extension_registry(startup.extension_registry)
    .installed_packages(startup.installed_packages)
    .startup_diagnostics(startup.diagnostics)
    .build();

    harness.prompt("trigger restore").await.unwrap();
    let list = harness
        .dispatch_extension_command("todo/list", None, serde_json::json!({}))
        .await
        .unwrap()
        .unwrap();

    let items = list["items"].as_array().expect("items");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["title"], "resume me");
}

/// Create a `PackageResource` with an adapter manifest pointing at the mock.
fn make_adapter_package(
    name: &str,
    adapter_command: PathBuf,
    precedence: u32,
    package_dir: PathBuf,
) -> PackageResource {
    let toml_path = package_dir.join("package.toml");
    PackageResource {
        manifest: PackageManifest {
            name: name.to_string(),
            description: format!("Test package {name}"),
            version: None,
            opi_version: None,
            adapter: Some(AdapterManifest {
                kind: "process-jsonl".to_string(),
                command: adapter_command.to_string_lossy().to_string(),
                args: vec![],
                protocol: "opi-extension-jsonl-v1".to_string(),
                timeout_ms: None,
            }),
            extensions: None,
            skills: None,
            fragments: None,
            themes: None,
            disabled: vec![],
        },
        path: package_dir,
        package_toml_path: toml_path,
        layer_precedence: precedence,
    }
}

/// Build a minimal config for harness tests.
fn adapter_test_config() -> OpiConfig {
    let mut config = OpiConfig::default();
    config.defaults.model = "anthropic:claude-sonnet-4-5-20250514".to_string();
    config
}

/// Build a mock provider for harness construction.
fn adapter_mock_provider() -> Box<dyn opi_ai::provider::Provider> {
    Box::new(MockProvider::new("anthropic", vec![]))
}

// ---------------------------------------------------------------------------
// 1. start_adapters_from_packages registers tools from mock adapter
// ---------------------------------------------------------------------------

#[tokio::test]
async fn adapter_startup_registers_tools_from_capabilities() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mock_bin = mock_adapter_bin();
    let package = make_adapter_package("test-pkg", mock_bin, 0, dir.path().to_path_buf());

    let registry = ExtensionRegistry::new();
    let (registry, diagnostics) =
        start_adapters_from_packages(&[package], dir.path(), registry).await;

    assert!(
        diagnostics.is_empty(),
        "unexpected diagnostics: {diagnostics:?}"
    );
    let tools = registry.collect_tools();
    assert_eq!(tools.len(), 1, "expected 1 tool from capabilities mock");
    assert_eq!(tools[0].definition().name, "test_tool");
}

// ---------------------------------------------------------------------------
// 2. Adapter startup failure produces diagnostic
// ---------------------------------------------------------------------------

#[tokio::test]
async fn adapter_startup_failure_produces_diagnostic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bad_bin = dir.path().join("nonexistent_adapter_binary_12345");
    let package = make_adapter_package("bad-pkg", bad_bin, 0, dir.path().to_path_buf());

    let registry = ExtensionRegistry::new();
    let (registry, diagnostics) =
        start_adapters_from_packages(&[package], dir.path(), registry).await;

    assert!(
        !diagnostics.is_empty(),
        "expected diagnostics for failed adapter"
    );
    assert_eq!(diagnostics[0].code, code::CODE_ADAPTER_STARTUP_FAILED);
    assert_eq!(diagnostics[0].source, SOURCE_ADAPTER);
    assert_eq!(diagnostics[0].severity, Severity::Error);
    assert_eq!(
        diagnostics[0].details.as_ref().unwrap()["package_name"],
        "bad-pkg"
    );
    assert!(
        registry.collect_tools().is_empty(),
        "no tools from failed adapter"
    );
}

// ---------------------------------------------------------------------------
// 3. Unsupported protocol produces diagnostic
// ---------------------------------------------------------------------------

#[tokio::test]
async fn unsupported_adapter_protocol_produces_diagnostic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut package =
        make_adapter_package("proto-pkg", mock_adapter_bin(), 0, dir.path().to_path_buf());
    package.manifest.adapter.as_mut().unwrap().protocol = "unknown-protocol".to_string();

    let registry = ExtensionRegistry::new();
    let (_registry, diagnostics) =
        start_adapters_from_packages(&[package], dir.path(), registry).await;

    assert_eq!(diagnostics.len(), 1, "expected exactly one diagnostic");
    assert_eq!(diagnostics[0].code, code::CODE_ADAPTER_PROTOCOL_UNSUPPORTED);
    assert_eq!(diagnostics[0].source, SOURCE_ADAPTER);
    assert_eq!(
        diagnostics[0].details.as_ref().unwrap()["actual_protocol"],
        "unknown-protocol"
    );
}

// ---------------------------------------------------------------------------
// 4. Unsupported kind produces diagnostic
// ---------------------------------------------------------------------------

#[tokio::test]
async fn unsupported_adapter_kind_produces_diagnostic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut package =
        make_adapter_package("kind-pkg", mock_adapter_bin(), 0, dir.path().to_path_buf());
    package.manifest.adapter.as_mut().unwrap().kind = "websocket".to_string();

    let registry = ExtensionRegistry::new();
    let (_registry, diagnostics) =
        start_adapters_from_packages(&[package], dir.path(), registry).await;

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, code::CODE_ADAPTER_KIND_UNSUPPORTED);
    assert_eq!(diagnostics[0].source, SOURCE_ADAPTER);
    assert_eq!(
        diagnostics[0].details.as_ref().unwrap()["actual_kind"],
        "websocket"
    );
}

// ---------------------------------------------------------------------------
// 5. Deterministic startup order: precedence then name
// ---------------------------------------------------------------------------

#[tokio::test]
async fn adapters_start_in_deterministic_order() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mock_bin = mock_adapter_bin();

    let pkg_low = make_adapter_package("z-low", mock_bin.clone(), 0, dir.path().join("low"));
    let pkg_high = make_adapter_package("a-high", mock_bin.clone(), 1, dir.path().join("high"));

    // Pass in reverse order to verify sorting
    let registry = ExtensionRegistry::new();
    let (registry, diagnostics) =
        start_adapters_from_packages(&[pkg_high, pkg_low], dir.path(), registry).await;

    assert!(
        diagnostics.is_empty(),
        "unexpected diagnostics: {diagnostics:?}"
    );
    let names = registry.names();
    assert_eq!(names.len(), 2, "both adapters should be registered");
    assert!(names.contains(&"z-low"));
    assert!(names.contains(&"a-high"));
}

// ---------------------------------------------------------------------------
// 6. Non-adapter packages are silently skipped
// ---------------------------------------------------------------------------

#[tokio::test]
async fn non_adapter_packages_are_skipped() {
    let dir = tempfile::tempdir().expect("tempdir");
    let plain = PackageResource {
        manifest: PackageManifest {
            name: "plain-pkg".to_string(),
            description: "No adapter".to_string(),
            version: None,
            opi_version: None,
            adapter: None,
            extensions: None,
            skills: None,
            fragments: None,
            themes: None,
            disabled: vec![],
        },
        path: dir.path().to_path_buf(),
        package_toml_path: dir.path().join("package.toml"),
        layer_precedence: 0,
    };

    let registry = ExtensionRegistry::new();
    let (registry, diagnostics) =
        start_adapters_from_packages(&[plain], dir.path(), registry).await;

    assert!(diagnostics.is_empty(), "no diagnostics for plain packages");
    assert!(
        registry.names().is_empty(),
        "no adapters from plain packages"
    );
}

// ---------------------------------------------------------------------------
// 7. Harness includes adapter tools alongside builtins
// ---------------------------------------------------------------------------

#[tokio::test]
async fn harness_includes_adapter_tools_alongside_builtins() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mock_bin = mock_adapter_bin();
    let package = make_adapter_package("harness-pkg", mock_bin, 0, dir.path().to_path_buf());

    let registry = ExtensionRegistry::new();
    let (registry, diagnostics) =
        start_adapters_from_packages(&[package], dir.path(), registry).await;
    assert!(
        diagnostics.is_empty(),
        "unexpected diagnostics: {diagnostics:?}"
    );

    let metadata = DiscoveredResourceMetadata::default();
    let harness = CodingHarness::builder(
        adapter_mock_provider(),
        "claude-sonnet-4-5-20250514".to_string(),
        adapter_test_config(),
        dir.path().to_path_buf(),
    )
    .extension_registry(registry)
    .resource_metadata(metadata)
    .build();

    let prompt = harness.system_prompt();
    assert!(
        prompt.contains("- read:"),
        "builtin read tool missing from prompt"
    );
    assert!(
        prompt.contains("- test_tool:"),
        "adapter test_tool missing from prompt"
    );
}

// ---------------------------------------------------------------------------
// 8. ToolSelection::Disabled filters adapter tools
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tool_selection_disabled_filters_adapter_tools() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mock_bin = mock_adapter_bin();
    let package = make_adapter_package("disabled-pkg", mock_bin, 0, dir.path().to_path_buf());

    let registry = ExtensionRegistry::new();
    let (registry, diagnostics) =
        start_adapters_from_packages(&[package], dir.path(), registry).await;
    assert!(
        diagnostics.is_empty(),
        "unexpected diagnostics: {diagnostics:?}"
    );

    let metadata = DiscoveredResourceMetadata::default();
    let harness = CodingHarness::builder(
        adapter_mock_provider(),
        "claude-sonnet-4-5-20250514".to_string(),
        adapter_test_config(),
        dir.path().to_path_buf(),
    )
    .extension_registry(registry)
    .resource_metadata(metadata)
    .tool_selection(ToolSelection::Disabled)
    .build();

    let prompt = harness.system_prompt();
    assert!(
        !prompt.contains("- read:"),
        "expected no tools with ToolSelection::Disabled"
    );
    assert!(
        !prompt.contains("- test_tool:"),
        "expected no adapter tools with ToolSelection::Disabled"
    );
}

// ---------------------------------------------------------------------------
// 9. ToolSelection::NoBuiltin keeps adapter tools, removes builtins
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tool_selection_no_builtin_keeps_adapter_tools() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mock_bin = mock_adapter_bin();
    let package = make_adapter_package("nobuiltin-pkg", mock_bin, 0, dir.path().to_path_buf());

    let registry = ExtensionRegistry::new();
    let (registry, diagnostics) =
        start_adapters_from_packages(&[package], dir.path(), registry).await;
    assert!(
        diagnostics.is_empty(),
        "unexpected diagnostics: {diagnostics:?}"
    );

    let metadata = DiscoveredResourceMetadata::default();
    let harness = CodingHarness::builder(
        adapter_mock_provider(),
        "claude-sonnet-4-5-20250514".to_string(),
        adapter_test_config(),
        dir.path().to_path_buf(),
    )
    .extension_registry(registry)
    .resource_metadata(metadata)
    .tool_selection(ToolSelection::NoBuiltin)
    .build();

    let prompt = harness.system_prompt();
    assert!(
        !prompt.contains("- read:"),
        "builtin read should be absent with NoBuiltin"
    );
    assert!(
        prompt.contains("- test_tool:"),
        "adapter test_tool should be present with NoBuiltin"
    );
}

// ---------------------------------------------------------------------------
// 10. Adapter diagnostics flow through resource metadata
// ---------------------------------------------------------------------------

#[tokio::test]
async fn adapter_diagnostics_in_resource_metadata() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bad_bin = dir.path().join("does_not_exist_99999");
    let package = make_adapter_package("diag-pkg", bad_bin, 0, dir.path().to_path_buf());

    let registry = ExtensionRegistry::new();
    let (_registry, diagnostics) =
        start_adapters_from_packages(&[package], dir.path(), registry).await;

    let mut metadata = DiscoveredResourceMetadata::default();
    metadata.diagnostics.extend(diagnostics);

    assert!(
        !metadata.diagnostics.is_empty(),
        "metadata should contain adapter diagnostics"
    );
    let rpc_json = metadata.to_rpc_json();
    let diag_arr = rpc_json["diagnostics"]
        .as_array()
        .expect("diagnostics array");
    assert!(!diag_arr.is_empty(), "RPC JSON should contain diagnostics");
    assert_eq!(
        diag_arr[0]["code"],
        code::CODE_ADAPTER_STARTUP_FAILED,
        "RPC diagnostics should be structured payloads"
    );
}

// ---------------------------------------------------------------------------
// 11. Command resolution: absolute, relative, bare name
// ---------------------------------------------------------------------------

fn make_adapter_manifest(command: &str) -> AdapterManifest {
    AdapterManifest {
        kind: "process-jsonl".to_string(),
        command: command.to_string(),
        args: vec![],
        protocol: "opi-extension-jsonl-v1".to_string(),
        timeout_ms: None,
    }
}

#[test]
fn resolve_adapter_command_absolute_path() {
    let pkg_dir = PathBuf::from(if cfg!(windows) {
        r"C:\opt\my-package"
    } else {
        "/opt/my-package"
    });
    let abs = if cfg!(windows) {
        r"C:\usr\bin\adapter.exe"
    } else {
        "/usr/bin/adapter"
    };
    let manifest = make_adapter_manifest(abs);
    let resolved = resolve_adapter_command(&manifest, &pkg_dir);
    assert_eq!(resolved, PathBuf::from(abs));
}

#[test]
fn resolve_adapter_command_relative_path() {
    let pkg_dir = if cfg!(windows) {
        PathBuf::from(r"C:\opt\my-package")
    } else {
        PathBuf::from("/opt/my-package")
    };
    let manifest = make_adapter_manifest("./bin/adapter");
    let resolved = resolve_adapter_command(&manifest, &pkg_dir);
    assert_eq!(resolved, pkg_dir.join("./bin/adapter"));
}

#[test]
fn resolve_adapter_command_bare_name() {
    let pkg_dir = if cfg!(windows) {
        PathBuf::from(r"C:\opt\my-package")
    } else {
        PathBuf::from("/opt/my-package")
    };
    let manifest = make_adapter_manifest("my-adapter");
    let resolved = resolve_adapter_command(&manifest, &pkg_dir);
    // Bare name — should NOT be resolved against package dir
    assert_eq!(resolved, PathBuf::from("my-adapter"));
}

// ---------------------------------------------------------------------------
// 12. Existing registry extensions preserved when starting adapters
// ---------------------------------------------------------------------------

/// Minimal Extension implementation for testing pre-existing registrations.
struct ManualExtension;

impl opi_agent::extension::Extension for ManualExtension {
    fn name(&self) -> &str {
        "manual"
    }
}

#[tokio::test]
async fn existing_registry_preserved_when_starting_adapters() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mock_bin = mock_adapter_bin();
    let package = make_adapter_package("new-pkg", mock_bin, 0, dir.path().to_path_buf());

    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(ManualExtension))
        .expect("register manual");

    let (registry, diagnostics) =
        start_adapters_from_packages(&[package], dir.path(), registry).await;

    assert!(
        diagnostics.is_empty(),
        "unexpected diagnostics: {diagnostics:?}"
    );
    let names = registry.names();
    assert_eq!(names.len(), 2, "should have both manual + adapter");
    assert!(names.contains(&"manual"));
    assert!(names.contains(&"new-pkg"));
}

// ---------------------------------------------------------------------------
// 13. Harness metadata includes adapter extension names
// ---------------------------------------------------------------------------

#[tokio::test]
async fn harness_metadata_includes_adapter_extensions() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mock_bin = mock_adapter_bin();
    let package = make_adapter_package("meta-pkg", mock_bin, 0, dir.path().to_path_buf());

    let registry = ExtensionRegistry::new();
    let (registry, diagnostics) =
        start_adapters_from_packages(&[package], dir.path(), registry).await;
    assert!(
        diagnostics.is_empty(),
        "unexpected diagnostics: {diagnostics:?}"
    );

    let metadata = DiscoveredResourceMetadata::default();
    let harness = CodingHarness::builder(
        adapter_mock_provider(),
        "claude-sonnet-4-5-20250514".to_string(),
        adapter_test_config(),
        dir.path().to_path_buf(),
    )
    .extension_registry(registry)
    .resource_metadata(metadata)
    .build();

    let json = harness.resource_metadata_json();
    let extensions = json["extensions"].as_array().expect("extensions array");
    let ext_names: Vec<&str> = extensions.iter().filter_map(|v| v.as_str()).collect();
    assert!(
        ext_names.contains(&"meta-pkg"),
        "adapter name should appear in extensions: {ext_names:?}"
    );
}

// ---------------------------------------------------------------------------
// Phase 10 task 10.4: CodingHarness product-wrapper boundary.
//
// CodingHarness is the coding-agent product wrapper over the generic opi-agent
// runtime seams (Agent, AgentHooks, ExtensionRegistry, session storage,
// ProviderCollection, CompactionEngine). Generic turn lifecycle / phase /
// save-point / pending-write semantics live in opi_agent::harness. These two
// contract tests pin (a) that the wrapper composes product inputs over the
// generic seams at runtime, and (b) that product/CLI/package policy never leaks
// into opi-agent. See docs/superpowers/specs/2026-06-24-phase10-core-architecture-deepening-design.md
// Workstream 10.2 + Phase 10 Success Criterion 4.
// ---------------------------------------------------------------------------

/// Strip Rust line and (nested) block comments while preserving string/char
/// literal contents, so a source-structure guard scans code, not doc prose.
/// Product types legitimately appear in opi-agent doc comments explaining the
/// boundary; they must NOT appear in opi-agent code.
fn strip_rust_comments(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0usize;
    let mut block_depth = 0i32;
    while i < bytes.len() {
        let two = if i + 1 < bytes.len() {
            Some((bytes[i], bytes[i + 1]))
        } else {
            None
        };
        if block_depth > 0 {
            match two {
                Some((b'/', b'*')) => {
                    block_depth += 1;
                    i += 2;
                    continue;
                }
                Some((b'*', b'/')) => {
                    block_depth -= 1;
                    i += 2;
                    continue;
                }
                _ => {
                    if bytes[i] == b'\n' {
                        out.push('\n');
                    }
                    i += 1;
                    continue;
                }
            }
        }
        match two {
            Some((b'/', b'/')) => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            Some((b'/', b'*')) => {
                block_depth += 1;
                i += 2;
                continue;
            }
            _ => {
                let c = bytes[i];
                if c == b'"' || c == b'\'' {
                    let quote = c;
                    out.push(c as char);
                    i += 1;
                    while i < bytes.len() {
                        let cc = bytes[i];
                        out.push(cc as char);
                        if cc == b'\\' && i + 1 < bytes.len() {
                            out.push(bytes[i + 1] as char);
                            i += 2;
                            continue;
                        }
                        i += 1;
                        if cc == quote {
                            break;
                        }
                    }
                    continue;
                }
                out.push(c as char);
                i += 1;
            }
        }
    }
    out
}

/// Collect every `.rs` file under `dir` recursively.
fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[tokio::test]
async fn coding_harness_composes_generic_opi_agent_seams() {
    // The product wrapper must drive a turn through the generic opi-agent
    // runtime seams: the Agent (provider/tool loop), the AgentHooks, the
    // AgentControl / CancellationToken control surface, and the generic
    // SessionWriter/SessionReader durable storage. Build via the production
    // builder on a MockProvider and prove each seam is observable end-to-end.
    let workspace = tempfile::tempdir().unwrap();
    let global_config = tempfile::tempdir().unwrap();

    // Pre-create a session file the harness adopts (ResumeInfo), so the test
    // never touches the real user session dir or OPI_SESSIONS_DIR.
    let session_path = workspace.path().join("compose-session.jsonl");
    let header = SessionHeader::new(
        "sess-compose".into(),
        "2026-06-26T00:00:00Z".into(),
        workspace.path().display().to_string(),
        None,
    );
    let seed = SessionEntry::Message(MessageEntry {
        id: "msg-seed".into(),
        parent_id: None,
        timestamp: "2026-06-26T00:00:00Z".into(),
        message: Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "seed".into(),
            }],
            timestamp_ms: 0,
        }),
    });
    {
        let mut writer = SessionWriter::create(&session_path, header).unwrap();
        writer.append(&seed).unwrap();
    }
    let seed_entries = vec![seed];
    let initial_messages = opi_coding_agent::session_cli::reconstruct_context(&seed_entries);

    let provider = MockProvider::new(
        "mock",
        vec![opi_ai::test_support::text_response("composed-turn")],
    );
    let resume = ResumeInfo {
        path: session_path.clone(),
        session_id: "sess-compose".into(),
        entries: seed_entries,
        original_cwd: workspace.path().to_path_buf(),
        diagnostics: Vec::new(),
        recorded_model: None,
        recorded_thinking: None,
    };
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
    )
    .hooks(Box::new(opi_coding_agent::harness::CodingAgentHooks))
    .global_config_dir(global_config.path().to_path_buf())
    .initial_messages(initial_messages)
    .resume(resume)
    .build();

    // Control surface delegates to the generic opi-agent Agent seam.
    let _control: opi_agent::agent::AgentControl = harness.control_handle();
    let cancel_token: tokio_util::sync::CancellationToken = harness.cancel_token();
    assert!(!cancel_token.is_cancelled(), "pre-abort token must be live");

    // Drive a turn through the generic Agent seam (provider stream consumed).
    let messages: Vec<opi_agent::message::AgentMessage> =
        harness.prompt("drive the seam").await.unwrap();
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, opi_agent::message::AgentMessage::Llm(Message::Assistant(a))
                if a.content.iter().any(|block| matches!(block,
                    opi_ai::message::AssistantContent::Text { text } if text.contains("composed-turn"))))),
        "assistant turn must come back through the generic AgentMessage seam"
    );

    // The wrapper persisted the new turn through the generic session-storage
    // seam: SessionReader::read_all now holds more Message entries than the
    // single seed entry written before the turn.
    let (_hdr, entries) = opi_agent::session::SessionReader::read_all(&session_path)
        .expect("session readable via generic SessionReader seam");
    let message_count = entries
        .iter()
        .filter(|e| matches!(e, SessionEntry::Message(_)))
        .count();
    assert!(
        message_count > 1,
        "wrapper must persist the new turn through the generic session seam; got {message_count} message entries"
    );
}

#[test]
fn coding_harness_wrapper_keeps_product_policy_out_of_opi_agent() {
    // Mechanical proof of the acceptance-scenario boundary: "without moving
    // package or CLI policy into opi-agent." None of these product-policy
    // tokens may appear in opi-agent CODE (comments are stripped, since the
    // generic seam legitimately names product types in doc prose that explains
    // the boundary). opi-agent cannot even depend on opi-coding-agent, so any
    // code occurrence would be a policy leak or a naming collision.
    let opi_agent_src = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("opi-agent")
        .join("src");

    let product_policy_tokens = [
        "CodingHarness",
        "CodingHarnessBuilder",
        "CodingAgentHooks",
        "InteractiveCodingHooks",
        "SessionCoordinator",
        "OpiConfig",
        "ToolRuntimeConfig",
        "ToolSelection",
        "DiscoveredResourceMetadata",
        "HarnessResources",
        "PackageStore",
        "PackageResource",
        "PackageManifest",
        "RuntimePackageStartup",
        "NonInteractiveRunner",
        "RpcRunner",
        "run_interactive_tui",
        "provider_factory",
        "assemble_harness_collection",
        "build_provider",
        "start_installed_package_runtime",
        "resolve_installed_packages",
    ];

    let mut files = Vec::new();
    collect_rs_files(&opi_agent_src, &mut files);
    assert!(!files.is_empty(), "opi-agent/src must contain .rs files");

    let mut violations: Vec<String> = Vec::new();
    let mut generic_seam_seen = false;
    for file in &files {
        let rel = file
            .strip_prefix(&opi_agent_src)
            .unwrap_or(file)
            .to_string_lossy()
            .replace('\\', "/");
        let raw = std::fs::read_to_string(file).unwrap_or_default();
        let code = strip_rust_comments(&raw);
        if code.contains("AgentHarness") {
            generic_seam_seen = true;
        }
        for token in product_policy_tokens {
            if code.contains(token) {
                violations.push(format!(
                    "product token `{token}` leaked into opi-agent `{rel}`"
                ));
            }
        }
    }
    // Sanity / non-vacuousness: the generic harness seam must be present in
    // opi-agent code (proves the scan is live and the boundary is "generic
    // seam present, product policy absent").
    assert!(
        generic_seam_seen,
        "expected opi-agent to own the generic AgentHarness seam"
    );
    assert!(
        violations.is_empty(),
        "product/CLI/package policy leaked into opi-agent:\n{}",
        violations.join("\n")
    );

    // Phase 8 API-surface classification: the generic harness types must stay
    // module-path only (no crate-root `pub use harness::` re-export).
    let lib_rs = std::fs::read_to_string(opi_agent_src.join("lib.rs")).unwrap_or_default();
    let lib_code = strip_rust_comments(&lib_rs);
    for line in lib_code.lines() {
        let trimmed = line.trim();
        assert!(
            !trimmed.starts_with("pub use harness::")
                && !trimmed.starts_with("pub use crate::harness::"),
            "crate-root harness re-export breaks Phase 8 module-path-only classification: {line}"
        );
    }
}

// ---------------------------------------------------------------------------
// 14/15. Phase 6 (task 6.4) startup diagnostic contract.
//
// The design Error Handling section requires startup diagnostics to identify
// the package name, adapter command when relevant, the expected/actual
// protocol/kind, the timeout surface / exit code (carried by the wrapped
// AdapterHostError Display, variant-tested in adapter_host.rs), and whether the
// package is disabled at runtime. These tests pin the start_adapters_from_packages
// diagnostic FORMATTING for the protocol gate and the shared startup-failure arm
// through the production path. The InitializeTimeout / AdapterExited error
// variants themselves (timeout value, exit code) are produced by AdapterHost::start
// and ride through `{e}` unchanged; their content is covered by adapter_host.rs.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn adapter_unsupported_protocol_diagnostic_includes_command_and_disabled_state() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mock_bin = mock_adapter_bin();
    let cmd_str = mock_bin.to_string_lossy().to_string();
    let mut package = make_adapter_package("proto-cmd-pkg", mock_bin, 0, dir.path().to_path_buf());
    package.manifest.adapter.as_mut().unwrap().protocol = "unknown-protocol".to_string();

    let registry = ExtensionRegistry::new();
    let (registry, diagnostics) =
        start_adapters_from_packages(&[package], dir.path(), registry).await;

    assert_eq!(diagnostics.len(), 1, "expected exactly one diagnostic");
    let d = &diagnostics[0];
    assert_eq!(d.code, code::CODE_ADAPTER_PROTOCOL_UNSUPPORTED);
    assert_eq!(d.message, "unsupported adapter protocol");
    let details = d.details.as_ref().expect("adapter details");
    assert_eq!(details["package_name"], "proto-cmd-pkg");
    assert_eq!(details["actual_protocol"], "unknown-protocol");
    assert_eq!(details["expected_protocol"], "opi-extension-jsonl-v1");
    assert_eq!(details["adapter_command"], cmd_str);
    assert_eq!(details["disabled_at_runtime"], true);
    assert!(
        registry.collect_tools().is_empty(),
        "unsupported-protocol package must not register an adapter"
    );
}

#[tokio::test]
async fn adapter_startup_failure_diagnostic_includes_command_and_disabled_state() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bad_bin = dir.path().join("nonexistent_adapter_binary_67890");
    let cmd_str = bad_bin.to_string_lossy().to_string();
    // SpawnFailed is one variant of the shared startup-failure arm; the same
    // formatting wraps InitializeTimeout (timeout surface) and AdapterExited
    // (exit code), whose variant content is variant-tested in adapter_host.rs.
    let package = make_adapter_package("bad-cmd-pkg", bad_bin, 0, dir.path().to_path_buf());

    let registry = ExtensionRegistry::new();
    let (registry, diagnostics) =
        start_adapters_from_packages(&[package], dir.path(), registry).await;

    assert_eq!(diagnostics.len(), 1, "expected exactly one diagnostic");
    let d = &diagnostics[0];
    assert_eq!(d.code, code::CODE_ADAPTER_STARTUP_FAILED);
    assert_eq!(d.message, "adapter startup failed");
    let details = d.details.as_ref().expect("adapter details");
    assert_eq!(details["package_name"], "bad-cmd-pkg");
    assert_eq!(details["adapter_command"], cmd_str);
    assert_eq!(details["disabled_at_runtime"], true);
    assert!(details.get("adapter_error").is_some());
    assert!(
        registry.collect_tools().is_empty(),
        "failed-startup package must not register an adapter"
    );
}
