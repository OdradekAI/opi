//! Trust-gated project resource discovery — task 15.7.
//!
//! Drives the production two-stage config path, `CodingHarness::discover_resources`,
//! `context_files` injection, the `read` tool, and `start_installed_package_runtime`
//! through the trust gate. An untrusted project's config layer, project
//! skills/fragments/themes/extensions, project `.opi/packages.toml` adapter
//! declarations, and project `AGENTS.md`/`CLAUDE.md` are skipped; user-global
//! resources and explicit `--config`/`--system-prompt` inputs remain available;
//! tool execution (read) is ungated. A trusted project retains the existing
//! resource layers.

use std::fs;
use std::path::Path;
use std::process::Command;

use opi_agent::tool::{Tool, ToolResult};
use opi_ai::test_support::MockProvider;
use opi_coding_agent::config::{
    ConfigSource, OpiConfig, merge_project_config, resolve_pre_trust_config,
};
use opi_coding_agent::context_files;
use opi_coding_agent::harness::CodingHarness;
use opi_coding_agent::package_resolver::local_lock_entry;
use opi_coding_agent::package_store::{PackageDeclaration, PackageStore};
use opi_coding_agent::project_trust::{
    ProjectTrustStore, TrustDecision, resolve_project_trust_decision,
};
use opi_coding_agent::resource::{DiscoveryLayer, DiscoveryLayerKind, ResourceDiscoveryLayers};
use opi_coding_agent::runtime_packages::start_installed_package_runtime_with_trust;
use opi_coding_agent::tool::ReadTool;
use serde_json::json;
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn init_git(dir: &Path) {
    // A `.git` marker bounds the context-file walk so it does not escape the
    // temp workspace into real parent directories.
    fs::create_dir_all(dir.join(".git")).expect("create .git marker");
}

fn write_project_resources(workspace: &Path) {
    // Project `.opi/config.toml` — the T6 bedrock-profile vector.
    fs::create_dir_all(workspace.join(".opi")).expect("create .opi");
    fs::write(
        workspace.join(".opi").join("config.toml"),
        "[providers.bedrock]\nprofile = \"proj-aws-profile\"\n",
    )
    .expect("write project config");

    // Project skill.
    let skill_dir = workspace.join(".opi").join("skills").join("proj-skill");
    fs::create_dir_all(&skill_dir).expect("create proj skill dir");
    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: proj-skill\ndescription: Project-only skill.\n---\nPROJ SKILL BODY\n",
    )
    .expect("write proj skill");

    // Project fragment.
    let frag_dir = workspace.join(".opi").join("fragments").join("proj-frag");
    fs::create_dir_all(&frag_dir).expect("create proj frag dir");
    fs::write(
        frag_dir.join("FRAGMENT.md"),
        "---\nname: proj-frag\ndescription: Project-only fragment.\narguments: text\n---\nPROJ FRAG BODY\n",
    )
    .expect("write proj frag");

    // Project theme.
    let theme_dir = workspace.join(".opi").join("themes").join("proj-theme");
    fs::create_dir_all(&theme_dir).expect("create proj theme dir");
    fs::write(
        theme_dir.join("theme.toml"),
        "name = \"proj-theme\"\ndescription = \"Project-only theme.\"\n",
    )
    .expect("write proj theme");

    // Project extension.
    let extension_dir = workspace
        .join(".opi")
        .join("extensions")
        .join("proj-extension");
    fs::create_dir_all(&extension_dir).expect("create proj extension dir");
    fs::write(
        extension_dir.join("extension.toml"),
        "[extension]\nname = \"proj-extension\"\nversion = \"0.1.0\"\ndescription = \"Project-only extension.\"\n",
    )
    .expect("write proj extension");

    // Project context files (prompt-injection channel for an untrusted project).
    fs::write(workspace.join("AGENTS.md"), "PROJECT AGENTS INSTRUCTIONS")
        .expect("write project AGENTS.md");
    fs::write(workspace.join("CLAUDE.md"), "PROJECT CLAUDE INSTRUCTIONS")
        .expect("write project CLAUDE.md");
}

fn write_global_resources(global: &Path) {
    // Global skill.
    let skill_dir = global.join("skills").join("global-skill");
    fs::create_dir_all(&skill_dir).expect("create global skill dir");
    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: global-skill\ndescription: User-global skill.\n---\nGLOBAL SKILL BODY\n",
    )
    .expect("write global skill");

    // Global theme.
    let theme_dir = global.join("themes").join("global-theme");
    fs::create_dir_all(&theme_dir).expect("create global theme dir");
    fs::write(
        theme_dir.join("theme.toml"),
        "name = \"global-theme\"\ndescription = \"User-global theme.\"\n",
    )
    .expect("write global theme");

    // Global extension.
    let extension_dir = global.join("extensions").join("global-extension");
    fs::create_dir_all(&extension_dir).expect("create global extension dir");
    fs::write(
        extension_dir.join("extension.toml"),
        "[extension]\nname = \"global-extension\"\nversion = \"0.1.0\"\ndescription = \"User-global extension.\"\n",
    )
    .expect("write global extension");

    // Global context file.
    fs::write(global.join("AGENTS.md"), "GLOBAL AGENTS INSTRUCTIONS")
        .expect("write global AGENTS.md");
}

fn write_package(pkg_dir: &Path, name: &str) {
    fs::create_dir_all(pkg_dir).expect("create package dir");
    fs::write(
        pkg_dir.join("package.toml"),
        format!("\nname = \"{name}\"\ndescription = \"{name} package.\"\nversion = \"0.1.0\"\n"),
    )
    .expect("write package.toml");
}

fn compile_marker_adapter(base: &Path) -> std::path::PathBuf {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("support")
        .join("marker_adapter_fixture.rs");
    let binary = base.join(if cfg!(windows) {
        "marker-adapter-fixture.exe"
    } else {
        "marker-adapter-fixture"
    });
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let status = Command::new(rustc)
        .arg("--edition=2024")
        .arg(source)
        .arg("-o")
        .arg(&binary)
        .status()
        .expect("run rustc for marker adapter fixture");
    assert!(status.success(), "compile marker adapter fixture");
    binary
}

fn install_marker_adapter(
    store: &PackageStore,
    base: &Path,
    package_name: &str,
    marker: &Path,
    command: &Path,
) {
    let package_dir = base.join("vendor").join(package_name);
    fs::create_dir_all(&package_dir).unwrap();
    let command = command.display().to_string().replace('\\', "\\\\");
    let marker = marker.display().to_string().replace('\\', "\\\\");
    fs::write(
        package_dir.join("package.toml"),
        format!(
            "name = \"{package_name}\"\n\
             description = \"Marker adapter.\"\n\
             version = \"0.1.0\"\n\
             [adapter]\n\
             kind = \"process-jsonl\"\n\
             command = \"{command}\"\n\
             args = [\"startup_marker\", \"{marker}\"]\n\
             protocol = \"opi-extension-jsonl-v1\"\n"
        ),
    )
    .unwrap();
    let source = format!("./vendor/{package_name}");
    store
        .write_declarations(&[PackageDeclaration {
            source: source.clone(),
            filters: Default::default(),
        }])
        .unwrap();
    store
        .write_lock(&[local_lock_entry(source, &package_dir).unwrap()])
        .unwrap();
}

fn tool_result_text(result: &ToolResult) -> String {
    result
        .content
        .iter()
        .filter_map(|c| match c {
            opi_ai::message::OutputContent::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

fn config_source(workspace: &Path, global: &Path) -> ConfigSource {
    ConfigSource {
        cli_model: None,
        config_path: None,
        env_model: None,
        project_dir: Some(workspace.to_path_buf()),
        user_config_path: Some(global.join("config.toml")),
    }
}

// ---------------------------------------------------------------------------
// resolve_project_trust_decision
// ---------------------------------------------------------------------------

#[test]
fn no_resource_project_is_trusted() {
    let user_config = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    init_git(workspace.path());
    // No .opi resources at all.
    let store = ProjectTrustStore::load(user_config.path()).expect("load store");
    assert_eq!(
        resolve_project_trust_decision(&store, workspace.path()),
        TrustDecision::Trusted
    );
}

#[test]
fn resource_project_with_no_store_entry_is_fail_closed() {
    let user_config = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    init_git(workspace.path());
    write_project_resources(workspace.path());
    let store = ProjectTrustStore::load(user_config.path()).expect("load store");
    assert_eq!(
        resolve_project_trust_decision(&store, workspace.path()),
        TrustDecision::Untrusted
    );
}

#[test]
fn resource_project_with_deny_store_entry_is_untrusted() {
    let user_config = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    init_git(workspace.path());
    write_project_resources(workspace.path());
    let store = ProjectTrustStore::load(user_config.path()).expect("load store");
    store.record(workspace.path(), false).expect("record deny");
    assert_eq!(
        resolve_project_trust_decision(&store, workspace.path()),
        TrustDecision::Untrusted
    );
}

#[test]
fn resource_project_with_allow_store_entry_is_trusted() {
    let user_config = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    init_git(workspace.path());
    write_project_resources(workspace.path());
    let store = ProjectTrustStore::load(user_config.path()).expect("load store");
    store.record(workspace.path(), true).expect("record allow");
    assert_eq!(
        resolve_project_trust_decision(&store, workspace.path()),
        TrustDecision::Trusted
    );
}

// ---------------------------------------------------------------------------
// Two-stage config: resolve_pre_trust_config + merge_project_config
// ---------------------------------------------------------------------------

#[test]
fn pre_trust_config_omits_project_bedrock_profile() {
    let global = tempfile::tempdir().unwrap();
    fs::write(global.path().join("config.toml"), "").expect("empty global config");
    let workspace = tempfile::tempdir().unwrap();
    init_git(workspace.path());
    fs::create_dir_all(workspace.path().join(".opi")).expect("create .opi");
    fs::write(
        workspace.path().join(".opi").join("config.toml"),
        "[providers.bedrock]\nprofile = \"proj-aws-profile\"\n",
    )
    .expect("write project config");

    let pre = resolve_pre_trust_config(config_source(workspace.path(), global.path()))
        .expect("pre-trust resolve");
    assert_eq!(
        pre.providers.bedrock.profile, None,
        "project .opi/config.toml must NOT be merged at the pre-trust stage"
    );
}

#[test]
fn merge_project_config_loads_project_bedrock_profile() {
    let global = tempfile::tempdir().unwrap();
    fs::write(global.path().join("config.toml"), "").expect("empty global config");
    let workspace = tempfile::tempdir().unwrap();
    init_git(workspace.path());
    fs::create_dir_all(workspace.path().join(".opi")).expect("create .opi");
    fs::write(
        workspace.path().join(".opi").join("config.toml"),
        "[providers.bedrock]\nprofile = \"proj-aws-profile\"\n",
    )
    .expect("write project config");

    let source = config_source(workspace.path(), global.path());
    let pre = resolve_pre_trust_config(source).expect("pre-trust resolve");
    assert_eq!(pre.providers.bedrock.profile, None);
    let full = merge_project_config(pre, workspace.path()).expect("merge project config");
    assert_eq!(
        full.providers.bedrock.profile.as_deref(),
        Some("proj-aws-profile"),
        "trusted project's .opi/config.toml must be merged"
    );
}

// ---------------------------------------------------------------------------
// discover_context_files_with_trust
// ---------------------------------------------------------------------------

#[test]
fn untrusted_context_skips_project_keeps_global() {
    let workspace = tempfile::tempdir().unwrap();
    init_git(workspace.path());
    fs::write(workspace.path().join("AGENTS.md"), "PROJECT AGENTS")
        .expect("write project AGENTS.md");
    let global = tempfile::tempdir().unwrap();
    fs::write(global.path().join("AGENTS.md"), "GLOBAL AGENTS").expect("write global AGENTS.md");

    let trusted = context_files::discover_context_files_with_trust(
        workspace.path(),
        Some(global.path()),
        true,
    );
    assert!(trusted.content.contains("PROJECT AGENTS"));
    assert!(trusted.content.contains("GLOBAL AGENTS"));

    let untrusted = context_files::discover_context_files_with_trust(
        workspace.path(),
        Some(global.path()),
        false,
    );
    assert!(
        !untrusted.content.contains("PROJECT AGENTS"),
        "untrusted project context must not be discovered"
    );
    assert!(
        untrusted.content.contains("GLOBAL AGENTS"),
        "user-global context must still load"
    );
}

// ---------------------------------------------------------------------------
// Acceptance scenario 1: untrusted skips every gated layer
// ---------------------------------------------------------------------------

#[test]
fn untrusted_project_skips_every_gated_layer() {
    let workspace = tempfile::tempdir().unwrap();
    init_git(workspace.path());
    write_project_resources(workspace.path());
    let global = tempfile::tempdir().unwrap();
    write_global_resources(global.path());

    let provider = MockProvider::new("mock", Vec::new());
    let harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Untrusted,
    )
    .global_config_dir(global.path().to_path_buf())
    .build();

    let prompt = harness.system_prompt();

    // Project skill/fragment/context are gated out.
    assert!(
        !prompt.contains("proj-skill"),
        "project skill leaked: {prompt}"
    );
    assert!(!prompt.contains("PROJ SKILL BODY"));
    assert!(
        !prompt.contains("proj-frag"),
        "project fragment leaked: {prompt}"
    );
    assert!(
        !prompt.contains("PROJECT AGENTS INSTRUCTIONS"),
        "project context leaked: {prompt}"
    );
    assert!(
        !prompt.contains("PROJECT CLAUDE INSTRUCTIONS"),
        "project context leaked: {prompt}"
    );

    // User-global resources still load.
    assert!(
        prompt.contains("global-skill"),
        "global skill should load: {prompt}"
    );
    assert!(
        prompt.contains("GLOBAL AGENTS INSTRUCTIONS"),
        "global context should load: {prompt}"
    );

    let metadata = harness.resource_metadata();
    assert!(
        !metadata.skills.iter().any(|s| s.name == "proj-skill"),
        "project skill must not be discovered"
    );
    assert!(
        metadata.skills.iter().any(|s| s.name == "global-skill"),
        "global skill must be discovered"
    );
    assert!(
        !metadata.fragments.iter().any(|f| f.name == "proj-frag"),
        "project fragment must not be discovered"
    );
    assert!(
        !metadata
            .themes
            .iter()
            .any(|theme| theme.name == "proj-theme"),
        "project theme must not be discovered"
    );
    assert!(
        !metadata
            .extensions
            .iter()
            .any(|extension| extension.name == "proj-extension"),
        "project extension must not be discovered"
    );
    assert!(
        metadata
            .themes
            .iter()
            .any(|theme| theme.name == "global-theme"),
        "global theme must still be discovered"
    );
    assert!(
        metadata
            .extensions
            .iter()
            .any(|extension| extension.name == "global-extension"),
        "global extension must still be discovered"
    );
}

// ---------------------------------------------------------------------------
// Acceptance scenario 4: trusted retains existing resource layers
// ---------------------------------------------------------------------------

#[test]
fn trusted_project_retains_existing_resource_layers() {
    let workspace = tempfile::tempdir().unwrap();
    init_git(workspace.path());
    write_project_resources(workspace.path());
    let global = tempfile::tempdir().unwrap();
    write_global_resources(global.path());

    let provider = MockProvider::new("mock", Vec::new());
    let harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(global.path().to_path_buf())
    .build();

    let prompt = harness.system_prompt();
    assert!(
        prompt.contains("proj-skill"),
        "project skill should load: {prompt}"
    );
    assert!(prompt.contains("global-skill"));
    assert!(
        prompt.contains("PROJECT AGENTS INSTRUCTIONS"),
        "project context should load when trusted: {prompt}"
    );

    let metadata = harness.resource_metadata();
    assert!(metadata.skills.iter().any(|s| s.name == "proj-skill"));
    assert!(metadata.skills.iter().any(|s| s.name == "global-skill"));
    assert!(metadata.fragments.iter().any(|f| f.name == "proj-frag"));
    assert!(
        metadata
            .themes
            .iter()
            .any(|theme| theme.name == "proj-theme")
    );
    assert!(
        metadata
            .themes
            .iter()
            .any(|theme| theme.name == "global-theme")
    );
    assert!(
        metadata
            .extensions
            .iter()
            .any(|extension| extension.name == "proj-extension")
    );
    assert!(
        metadata
            .extensions
            .iter()
            .any(|extension| extension.name == "global-extension")
    );
}

#[test]
fn undecided_public_harness_state_is_fail_closed() {
    let workspace = tempfile::tempdir().unwrap();
    init_git(workspace.path());
    write_project_resources(workspace.path());
    let global = tempfile::tempdir().unwrap();
    write_global_resources(global.path());

    let harness = CodingHarness::builder(
        Box::new(MockProvider::new("mock", Vec::new())),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Undecided,
    )
    .global_config_dir(global.path().to_path_buf())
    .build();

    let prompt = harness.system_prompt();
    assert!(
        !prompt.contains("proj-skill"),
        "undecided leaked project skill"
    );
    assert!(
        !prompt.contains("PROJECT AGENTS INSTRUCTIONS"),
        "undecided leaked project context"
    );
    assert!(prompt.contains("global-skill"));
    assert!(prompt.contains("GLOBAL AGENTS INSTRUCTIONS"));
}

#[test]
fn untrusted_filter_uses_structural_layer_kind() {
    let workspace = tempfile::tempdir().unwrap();
    init_git(workspace.path());
    let global = tempfile::tempdir().unwrap();

    let custom_project = workspace.path().join("custom-project-skills");
    let project_skill = custom_project.join("project-custom");
    fs::create_dir_all(&project_skill).unwrap();
    fs::write(
        project_skill.join("SKILL.md"),
        "---\nname: project-custom\ndescription: project custom.\n---\nbody\n",
    )
    .unwrap();

    let lookalike = workspace.path().join(".opi-backup").join("skills");
    let explicit_skill = lookalike.join("explicit-lookalike");
    fs::create_dir_all(&explicit_skill).unwrap();
    fs::write(
        explicit_skill.join("SKILL.md"),
        "---\nname: explicit-lookalike\ndescription: explicit lookalike.\n---\nbody\n",
    )
    .unwrap();

    let layers = ResourceDiscoveryLayers {
        skills: vec![
            DiscoveryLayer {
                kind: DiscoveryLayerKind::Project,
                root: custom_project,
                subdirectory: None,
                precedence: 1,
            },
            DiscoveryLayer {
                kind: DiscoveryLayerKind::Explicit,
                root: lookalike,
                subdirectory: None,
                precedence: 2,
            },
        ],
        ..Default::default()
    };
    let harness = CodingHarness::builder(
        Box::new(MockProvider::new("mock", Vec::new())),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Untrusted,
    )
    .global_config_dir(global.path().to_path_buf())
    .resource_layers(layers)
    .build();

    assert!(
        !harness
            .resource_metadata()
            .skills
            .iter()
            .any(|skill| skill.name == "project-custom"),
        "a structurally project-owned custom layer must be trust-gated"
    );
    assert!(
        harness
            .resource_metadata()
            .skills
            .iter()
            .any(|skill| skill.name == "explicit-lookalike"),
        "an explicit .opi-backup lookalike must not be filtered by its name"
    );
}

// ---------------------------------------------------------------------------
// Acceptance scenario 3: untrusted context readable but not injected
// ---------------------------------------------------------------------------

#[tokio::test]
async fn untrusted_context_files_are_readable_but_not_injected() {
    let workspace = tempfile::tempdir().unwrap();
    init_git(workspace.path());
    write_project_resources(workspace.path());
    let global = tempfile::tempdir().unwrap();
    write_global_resources(global.path());

    let provider = MockProvider::new("mock", Vec::new());
    let harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Untrusted,
    )
    .global_config_dir(global.path().to_path_buf())
    .build();

    let prompt = harness.system_prompt();
    assert!(
        !prompt.contains("PROJECT AGENTS INSTRUCTIONS"),
        "untrusted project context must not be injected: {prompt}"
    );
    assert!(
        !prompt.contains("PROJECT CLAUDE INSTRUCTIONS"),
        "untrusted project context must not be injected: {prompt}"
    );
    assert!(
        prompt.contains("GLOBAL AGENTS INSTRUCTIONS"),
        "user-global context must be injected: {prompt}"
    );

    // The read tool is ungated: it can still read the untrusted project file.
    let read = ReadTool::new(workspace.path().to_path_buf());
    let result = read
        .execute(
            "c1",
            json!({ "path": "AGENTS.md" }),
            CancellationToken::new(),
            None,
        )
        .await
        .expect("read executes");
    assert!(
        !result.is_error,
        "read should succeed: {}",
        tool_result_text(&result)
    );
    let text = tool_result_text(&result);
    assert!(
        text.contains("PROJECT AGENTS INSTRUCTIONS"),
        "read tool must read untrusted project context: {text}"
    );
}

// ---------------------------------------------------------------------------
// Acceptance scenario 2: untrusted adapter declaration never spawns
// ---------------------------------------------------------------------------

#[tokio::test]
async fn untrusted_project_adapter_declaration_never_spawns() {
    let workspace = tempfile::tempdir().unwrap();
    init_git(workspace.path());
    let global = tempfile::tempdir().unwrap();

    // Project package declaration.
    let proj_pkg = workspace.path().join("vendor").join("proj-pkg");
    write_package(&proj_pkg, "proj-pkg");
    let proj_store = PackageStore::project(workspace.path().to_path_buf());
    proj_store
        .write_declarations(&[PackageDeclaration {
            source: "./vendor/proj-pkg".into(),
            filters: Default::default(),
        }])
        .expect("write project declarations");
    proj_store
        .write_lock(&[local_lock_entry("./vendor/proj-pkg".into(), &proj_pkg).unwrap()])
        .expect("write project lock");

    // Global package declaration.
    let global_pkg = global.path().join("vendor").join("global-pkg");
    write_package(&global_pkg, "global-pkg");
    let global_store = PackageStore::global(global.path().to_path_buf());
    global_store
        .write_declarations(&[PackageDeclaration {
            source: "./vendor/global-pkg".into(),
            filters: Default::default(),
        }])
        .expect("write global declarations");
    global_store
        .write_lock(&[local_lock_entry("./vendor/global-pkg".into(), &global_pkg).unwrap()])
        .expect("write global lock");

    // Untrusted: project package filtered out of the installed-package set that
    // feeds `start_adapters_from_packages` -> `AdapterHost::start` (the only
    // child-spawn site). A filtered declaration cannot reach spawn.
    let untrusted = start_installed_package_runtime_with_trust(
        workspace.path(),
        global.path(),
        TrustDecision::Untrusted,
    )
    .await;
    assert!(
        !untrusted
            .installed_packages
            .iter()
            .any(|p| p.manifest.name == "proj-pkg"),
        "untrusted project adapter declaration must be filtered before spawn"
    );
    assert!(
        untrusted
            .installed_packages
            .iter()
            .any(|p| p.manifest.name == "global-pkg"),
        "user-global adapter declaration must still load"
    );

    // Trusted: both scopes load (existing behavior).
    let trusted = start_installed_package_runtime_with_trust(
        workspace.path(),
        global.path(),
        TrustDecision::Trusted,
    )
    .await;
    assert!(
        trusted
            .installed_packages
            .iter()
            .any(|p| p.manifest.name == "proj-pkg"),
        "trusted project adapter declaration should load"
    );
    assert!(
        trusted
            .installed_packages
            .iter()
            .any(|p| p.manifest.name == "global-pkg"),
        "global adapter declaration should load"
    );
}

#[tokio::test]
async fn untrusted_package_runtime_never_reads_project_store() {
    let workspace = tempfile::tempdir().unwrap();
    init_git(workspace.path());
    let global = tempfile::tempdir().unwrap();

    let global_pkg = global.path().join("vendor").join("shared");
    write_package(&global_pkg, "shared");
    let global_store = PackageStore::global(global.path().to_path_buf());
    global_store
        .write_declarations(&[PackageDeclaration {
            source: "./vendor/shared".into(),
            filters: Default::default(),
        }])
        .unwrap();
    global_store
        .write_lock(&[local_lock_entry("./vendor/shared".into(), &global_pkg).unwrap()])
        .unwrap();

    let project_store_dir = workspace.path().join(".opi");
    fs::create_dir_all(&project_store_dir).unwrap();
    fs::write(
        project_store_dir.join("packages.toml"),
        "not valid toml = [",
    )
    .unwrap();

    let startup = start_installed_package_runtime_with_trust(
        workspace.path(),
        global.path(),
        TrustDecision::Untrusted,
    )
    .await;

    assert_eq!(startup.installed_packages.len(), 1);
    assert_eq!(startup.installed_packages[0].manifest.name, "shared");
    assert!(
        startup
            .diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.message.contains("Project package declarations")),
        "untrusted startup must not parse or diagnose the project store"
    );

    let harness = CodingHarness::builder(
        Box::new(MockProvider::new("mock", Vec::new())),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Untrusted,
    )
    .global_config_dir(global.path().to_path_buf())
    .build();
    assert!(
        harness
            .resource_metadata()
            .packages
            .iter()
            .any(|package| package.name == "shared"),
        "harness fallback must retain the valid global package"
    );
}

#[tokio::test]
async fn untrusted_same_name_project_package_cannot_suppress_global_package() {
    let workspace = tempfile::tempdir().unwrap();
    init_git(workspace.path());
    let global = tempfile::tempdir().unwrap();
    let global_pkg = global.path().join("vendor").join("global-shared");
    let project_pkg = workspace.path().join("vendor").join("project-shared");
    write_package(&global_pkg, "shared");
    write_package(&project_pkg, "shared");

    let global_store = PackageStore::global(global.path().to_path_buf());
    global_store
        .write_declarations(&[PackageDeclaration {
            source: "./vendor/global-shared".into(),
            filters: Default::default(),
        }])
        .unwrap();
    global_store
        .write_lock(&[local_lock_entry("./vendor/global-shared".into(), &global_pkg).unwrap()])
        .unwrap();

    let project_store = PackageStore::project(workspace.path().to_path_buf());
    project_store
        .write_declarations(&[PackageDeclaration {
            source: "./vendor/project-shared".into(),
            filters: Default::default(),
        }])
        .unwrap();
    project_store
        .write_lock(&[local_lock_entry("./vendor/project-shared".into(), &project_pkg).unwrap()])
        .unwrap();

    let startup = start_installed_package_runtime_with_trust(
        workspace.path(),
        global.path(),
        TrustDecision::Untrusted,
    )
    .await;
    assert_eq!(startup.installed_packages.len(), 1);
    assert_eq!(
        startup.installed_packages[0].path,
        global_pkg.canonicalize().unwrap()
    );
}

#[tokio::test]
async fn untrusted_runtime_starts_global_marker_adapter_but_not_project_marker_adapter() {
    let workspace = tempfile::tempdir().unwrap();
    init_git(workspace.path());
    let global = tempfile::tempdir().unwrap();
    let project_marker = workspace.path().join("project-started.marker");
    let global_marker = global.path().join("global-started.marker");
    let marker_adapter = compile_marker_adapter(global.path());

    install_marker_adapter(
        &PackageStore::project(workspace.path().to_path_buf()),
        workspace.path(),
        "project-marker",
        &project_marker,
        &marker_adapter,
    );
    install_marker_adapter(
        &PackageStore::global(global.path().to_path_buf()),
        global.path(),
        "global-marker",
        &global_marker,
        &marker_adapter,
    );

    let startup = start_installed_package_runtime_with_trust(
        workspace.path(),
        global.path(),
        TrustDecision::Untrusted,
    )
    .await;

    assert!(
        global_marker.is_file(),
        "the authorized global adapter must actually start"
    );
    assert!(
        !project_marker.exists(),
        "the untrusted project adapter must not execute any side effect"
    );
    assert_eq!(startup.installed_packages.len(), 1);
    assert_eq!(startup.installed_packages[0].manifest.name, "global-marker");

    let harness = CodingHarness::builder(
        Box::new(MockProvider::new("mock", Vec::new())),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Untrusted,
    )
    .global_config_dir(global.path().to_path_buf())
    .build();
    assert!(
        harness
            .resource_metadata()
            .packages
            .iter()
            .any(|package| package.name == "global-marker")
    );
    assert!(
        harness
            .resource_metadata()
            .packages
            .iter()
            .all(|package| package.name != "project-marker")
    );
}
