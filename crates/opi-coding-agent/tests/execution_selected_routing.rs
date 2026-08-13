//! Phase 16 remediation: routed discovery validates only identities that can
//! participate in the configured Routing Strategy.

use std::path::{Path, PathBuf};

use opi_coding_agent::cli::PackageCommand;
use opi_coding_agent::config::{
    ExecutionRule, ExecutionRunMode, ExecutionStrategy, OpiConfig, PermissionDecision,
};
use opi_coding_agent::harness::CodingHarness;
use opi_coding_agent::package_activation::{
    ActivationRecord, PackageActivationStore, TrustConfirmer, TrustDisplay, host_opi_version,
    host_target_triple,
};
use opi_coding_agent::package_cli;
use opi_coding_agent::project_trust::TrustDecision;

const EXE_CONTENT: &[u8] = b"#!/bin/sh\necho hi\n";
const DRIFTED_EXE_CONTENT: &[u8] = b"#!/bin/sh\necho drifted\n";

fn sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(bytes))
}

fn make_executable(path: &Path) {
    let _ = path;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

struct TestPackage {
    _dir: tempfile::TempDir,
    root: PathBuf,
    executable: PathBuf,
}

fn package(adapter_id: &str) -> TestPackage {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("bin")).unwrap();
    let executable = dir.path().join("bin").join(adapter_id);
    std::fs::write(&executable, EXE_CONTENT).unwrap();
    make_executable(&executable);
    let opi_version = host_opi_version();
    let manifest = format!(
        "version = \"0.8.0\"\n\
         opi_version = \"={opi_version}\"\n\
         name = \"{adapter_id}\"\n\
         description = \"selected routing test package\"\n\
         \n\
         [[contributions.adapters]]\n\
         capability = \"command.execute\"\n\
         id = \"{adapter_id}\"\n\
         transport = \"process-jsonl\"\n\
         command = \"bin/{adapter_id}\"\n\
         args = [\"backend\", \"--stdio\"]\n\
         protocol = \"command-execution-jsonl-v1\"\n\
         target = \"{}\"\n\
         sha256 = \"{}\"\n\
         handshake_timeout_ms = 5000\n\
         adapter_config = {{}}\n",
        host_target_triple(),
        sha256(EXE_CONTENT),
    );
    std::fs::write(dir.path().join("package.toml"), manifest).unwrap();
    TestPackage {
        root: dir.path().to_path_buf(),
        executable,
        _dir: dir,
    }
}

struct GrantTrust;

impl TrustConfirmer for GrantTrust {
    fn confirm(&mut self, _display: &TrustDisplay) -> Result<(), String> {
        Ok(())
    }
}

fn install_and_enable(package: &TestPackage, workspace: &Path, user: &Path, name: &str) {
    let exit = package_cli::handle_package_command(
        &PackageCommand::Add {
            source: package.root.to_string_lossy().into_owned(),
            local: false,
        },
        workspace.to_path_buf(),
        user.to_path_buf(),
    );
    assert_eq!(exit, 0, "package add failed for {name}");
    PackageActivationStore::global(user.to_path_buf())
        .enable(
            name,
            host_target_triple(),
            host_opi_version(),
            &mut GrantTrust,
        )
        .unwrap();
}

fn record(user: &Path, name: &str) -> ActivationRecord {
    PackageActivationStore::global(user.to_path_buf())
        .read_records()
        .unwrap()
        .into_iter()
        .find(|record| record.name == name)
        .unwrap()
}

fn build_harness(workspace: &Path, user: &Path, config: OpiConfig) -> CodingHarness {
    let provider = opi_ai::test_support::MockProvider::new(
        "mock",
        vec![opi_ai::test_support::text_response("ok")],
    );
    build_harness_with_provider(workspace, user, config, provider)
}

fn build_harness_with_provider(
    workspace: &Path,
    user: &Path,
    config: OpiConfig,
    provider: opi_ai::test_support::MockProvider,
) -> CodingHarness {
    CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".to_string(),
        config,
        workspace.to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user.to_path_buf())
    .execution_mode(ExecutionRunMode::Interactive)
    .build()
}

fn setup_two_packages() -> (
    tempfile::TempDir,
    tempfile::TempDir,
    TestPackage,
    TestPackage,
) {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let selected = package("selected-adapter");
    let unrelated = package("unrelated-adapter");
    install_and_enable(&selected, workspace.path(), user.path(), "selected-adapter");
    install_and_enable(
        &unrelated,
        workspace.path(),
        user.path(),
        "unrelated-adapter",
    );
    std::fs::write(&unrelated.executable, DRIFTED_EXE_CONTENT).unwrap();
    (workspace, user, selected, unrelated)
}

fn rewrite_locked_adapter_id(
    package: &TestPackage,
    user: &Path,
    original_id: &str,
    selected_id: &str,
) {
    let manifest_path = package.root.join("package.toml");
    let manifest = std::fs::read_to_string(&manifest_path).unwrap();
    let original = format!("id = \"{original_id}\"");
    assert!(manifest.contains(&original));
    let manifest = manifest.replacen(&original, &format!("id = \"{selected_id}\""), 1);
    std::fs::write(&manifest_path, &manifest).unwrap();

    let activation = PackageActivationStore::global(user.to_path_buf());
    let mut locks = activation.store().read_lock().unwrap();
    let lock = locks
        .iter_mut()
        .find(|lock| {
            lock.contributions
                .iter()
                .any(|contribution| contribution.adapter_id == original_id)
        })
        .expect("installed package lock");
    lock.manifest_sha256 = sha256(manifest.as_bytes());
    lock.contributions
        .iter_mut()
        .find(|contribution| contribution.adapter_id == original_id)
        .expect("locked adapter")
        .adapter_id = selected_id.to_string();
    activation.store().write_lock(&locks).unwrap();
}

fn setup_duplicate_selected_packages() -> (
    tempfile::TempDir,
    tempfile::TempDir,
    TestPackage,
    TestPackage,
    TestPackage,
) {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let first = package("shared-adapter");
    let second = package("second-package");
    let unrelated = package("unrelated-adapter");
    install_and_enable(&first, workspace.path(), user.path(), "shared-adapter");
    install_and_enable(&second, workspace.path(), user.path(), "second-package");
    install_and_enable(
        &unrelated,
        workspace.path(),
        user.path(),
        "unrelated-adapter",
    );
    rewrite_locked_adapter_id(&second, user.path(), "second-package", "shared-adapter");

    for executable in [&first.executable, &second.executable, &unrelated.executable] {
        std::fs::write(executable, DRIFTED_EXE_CONTENT).unwrap();
    }
    (workspace, user, first, second, unrelated)
}

fn assert_unrelated_remains_enabled(user: &Path) {
    let record = record(user, "unrelated-adapter");
    assert!(record.trusted, "unrelated drift must not invalidate trust");
    assert!(
        record.enabled,
        "unrelated drift must not disable the package"
    );
}

fn assert_records_remain_enabled(user: &Path, names: &[&str]) {
    for name in names {
        let record = record(user, name);
        assert!(record.trusted, "{name} trust was mutated during discovery");
        assert!(record.enabled, "{name} was disabled during discovery");
    }
}

fn assert_collision_failure(harness: &CodingHarness) {
    let diagnostic = harness
        .resource_metadata()
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic
                .details
                .as_ref()
                .and_then(|details| details.get("code"))
                .and_then(serde_json::Value::as_str)
                == Some("adapter_unavailable")
        })
        .expect("duplicate selected adapter must fail closed");
    let details = diagnostic.details.as_ref().unwrap();
    assert_eq!(
        details
            .get("adapter_id")
            .and_then(serde_json::Value::as_str),
        Some("shared-adapter")
    );
    assert!(
        details
            .get("remediation")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .contains("collision"),
        "collision cause must survive the ActivationError mapping: {details}"
    );
}

fn assert_diagnostic_code(harness: &CodingHarness, code: &str) {
    assert!(
        harness
            .resource_metadata()
            .diagnostics
            .iter()
            .any(|diagnostic| {
                diagnostic
                    .details
                    .as_ref()
                    .and_then(|details| details.get("code"))
                    .and_then(serde_json::Value::as_str)
                    == Some(code)
            }),
        "expected {code}, got {:?}",
        harness.resource_metadata().diagnostics
    );
}

fn write_corrupt_package_state(user: &Path) {
    std::fs::write(user.join("package-trust.toml"), "[[record]\ninvalid = [\n").unwrap();
}

#[test]
fn fixed_local_startup_ignores_corrupt_unrelated_package_state() {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    write_corrupt_package_state(user.path());

    let harness = build_harness(workspace.path(), user.path(), OpiConfig::default());

    assert!(
        harness.resource_metadata().diagnostics.is_empty(),
        "fixed-local startup must not inspect unrelated package state: {:?}",
        harness.resource_metadata().diagnostics
    );
}

#[test]
fn rules_without_a_matching_rule_ignore_corrupt_unrelated_package_state() {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    write_corrupt_package_state(user.path());
    let mut config = OpiConfig::default();
    config.execution.strategy = ExecutionStrategy::Rules;
    config.execution.rules = vec![ExecutionRule {
        modes: Some(vec![ExecutionRunMode::Rpc]),
        backend: "unused-adapter".into(),
    }];

    let harness = build_harness(workspace.path(), user.path(), config);

    assert!(
        harness.resource_metadata().diagnostics.is_empty(),
        "rules startup without a matching rule must defer routing without reading package state: {:?}",
        harness.resource_metadata().diagnostics
    );
}

#[test]
fn rules_local_startup_ignores_corrupt_unrelated_package_state() {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    write_corrupt_package_state(user.path());
    let mut config = OpiConfig::default();
    config.execution.strategy = ExecutionStrategy::Rules;
    config.execution.rules = vec![ExecutionRule {
        modes: Some(vec![ExecutionRunMode::Interactive]),
        backend: "local".into(),
    }];

    let harness = build_harness(workspace.path(), user.path(), config);

    assert!(
        harness.resource_metadata().diagnostics.is_empty(),
        "rules-local startup must not inspect unrelated package state: {:?}",
        harness.resource_metadata().diagnostics
    );
}

#[test]
fn fixed_startup_does_not_activate_or_mutate_unrelated_drifted_package() {
    let (workspace, user, _selected, _unrelated) = setup_two_packages();
    let mut config = OpiConfig::default();
    config.execution.backend = "selected-adapter".into();
    config
        .execution
        .permissions
        .insert("selected-adapter".into(), PermissionDecision::Allow);

    let harness = build_harness(workspace.path(), user.path(), config);

    assert_unrelated_remains_enabled(user.path());
    assert!(
        harness
            .resource_metadata()
            .diagnostics
            .iter()
            .all(|diagnostic| {
                diagnostic
                    .details
                    .as_ref()
                    .and_then(|details| details.get("code"))
                    .and_then(serde_json::Value::as_str)
                    != Some("package_untrusted")
            })
    );
}

#[test]
fn rules_startup_validates_only_the_first_matching_adapter() {
    let (workspace, user, _selected, _unrelated) = setup_two_packages();
    let mut config = OpiConfig::default();
    config.execution.strategy = ExecutionStrategy::Rules;
    config.execution.rules = vec![
        ExecutionRule {
            modes: Some(vec![ExecutionRunMode::Interactive]),
            backend: "selected-adapter".into(),
        },
        ExecutionRule {
            modes: None,
            backend: "unrelated-adapter".into(),
        },
    ];
    config
        .execution
        .permissions
        .insert("selected-adapter".into(), PermissionDecision::Allow);

    let _harness = build_harness(workspace.path(), user.path(), config);

    assert_unrelated_remains_enabled(user.path());
}

#[test]
fn fixed_duplicate_selected_adapter_fails_before_any_package_activation() {
    let (workspace, user, _first, _second, _unrelated) = setup_duplicate_selected_packages();
    let mut config = OpiConfig::default();
    config.execution.backend = "shared-adapter".into();
    config
        .execution
        .permissions
        .insert("shared-adapter".into(), PermissionDecision::Allow);

    let harness = build_harness(workspace.path(), user.path(), config);

    assert_collision_failure(&harness);
    assert_records_remain_enabled(
        user.path(),
        &["shared-adapter", "second-package", "unrelated-adapter"],
    );
}

#[test]
fn rules_duplicate_first_match_fails_before_any_package_activation() {
    let (workspace, user, _first, _second, _unrelated) = setup_duplicate_selected_packages();
    let mut config = OpiConfig::default();
    config.execution.strategy = ExecutionStrategy::Rules;
    config.execution.rules = vec![
        ExecutionRule {
            modes: Some(vec![ExecutionRunMode::Interactive]),
            backend: "shared-adapter".into(),
        },
        ExecutionRule {
            modes: None,
            backend: "unrelated-adapter".into(),
        },
    ];
    config
        .execution
        .permissions
        .insert("shared-adapter".into(), PermissionDecision::Allow);

    let harness = build_harness(workspace.path(), user.path(), config);

    assert_collision_failure(&harness);
    assert_records_remain_enabled(
        user.path(),
        &["shared-adapter", "second-package", "unrelated-adapter"],
    );
}

#[test]
fn selected_drifted_fixed_package_surfaces_package_untrusted() {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let selected = package("selected-adapter");
    install_and_enable(&selected, workspace.path(), user.path(), "selected-adapter");
    std::fs::write(&selected.executable, DRIFTED_EXE_CONTENT).unwrap();
    let mut config = OpiConfig::default();
    config.execution.backend = "selected-adapter".into();
    config
        .execution
        .permissions
        .insert("selected-adapter".into(), PermissionDecision::Allow);

    let harness = build_harness(workspace.path(), user.path(), config);

    assert!(
        harness
            .resource_metadata()
            .diagnostics
            .iter()
            .any(|diagnostic| {
                diagnostic
                    .details
                    .as_ref()
                    .and_then(|details| details.get("code"))
                    .and_then(serde_json::Value::as_str)
                    == Some("package_untrusted")
            })
    );
}

#[test]
fn fixed_selected_untrusted_package_surfaces_package_untrusted() {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let selected = package("selected-adapter");
    let exit = package_cli::handle_package_command(
        &PackageCommand::Add {
            source: selected.root.to_string_lossy().into_owned(),
            local: false,
        },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );
    assert_eq!(exit, 0);
    let mut config = OpiConfig::default();
    config.execution.backend = "selected-adapter".into();
    config
        .execution
        .permissions
        .insert("selected-adapter".into(), PermissionDecision::Allow);

    let harness = build_harness(workspace.path(), user.path(), config);

    assert_diagnostic_code(&harness, "package_untrusted");
}

#[test]
fn fixed_unknown_external_surfaces_package_not_installed() {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let mut config = OpiConfig::default();
    config.execution.backend = "missing-adapter".into();
    config
        .execution
        .permissions
        .insert("missing-adapter".into(), PermissionDecision::Allow);

    let harness = build_harness(workspace.path(), user.path(), config);

    assert_diagnostic_code(&harness, "package_not_installed");
}

#[test]
fn rules_selected_disabled_package_surfaces_contribution_disabled() {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let selected = package("selected-adapter");
    install_and_enable(&selected, workspace.path(), user.path(), "selected-adapter");
    PackageActivationStore::global(user.path().to_path_buf())
        .disable("selected-adapter")
        .unwrap();
    let mut config = OpiConfig::default();
    config.execution.strategy = ExecutionStrategy::Rules;
    config.execution.rules = vec![ExecutionRule {
        modes: Some(vec![ExecutionRunMode::Interactive]),
        backend: "selected-adapter".into(),
    }];
    config
        .execution
        .permissions
        .insert("selected-adapter".into(), PermissionDecision::Allow);

    let harness = build_harness(workspace.path(), user.path(), config);

    assert_diagnostic_code(&harness, "contribution_disabled");
}

#[test]
fn model_startup_validates_only_candidates_exposed_by_user_policy() {
    let (workspace, user, _selected, unrelated) = setup_two_packages();
    let exposed_drifted = package("exposed-drifted");
    install_and_enable(
        &exposed_drifted,
        workspace.path(),
        user.path(),
        "exposed-drifted",
    );
    std::fs::write(&exposed_drifted.executable, DRIFTED_EXE_CONTENT).unwrap();

    let mut config = OpiConfig::default();
    config.execution.strategy = ExecutionStrategy::Model;
    config
        .execution
        .permissions
        .insert("selected-adapter".into(), PermissionDecision::Allow);
    config
        .execution
        .permissions
        .insert("unrelated-adapter".into(), PermissionDecision::Deny);
    config
        .execution
        .permissions
        .insert("exposed-drifted".into(), PermissionDecision::Allow);

    let _harness = build_harness(workspace.path(), user.path(), config);

    let denied = record(user.path(), "unrelated-adapter");
    assert!(
        denied.trusted && denied.enabled,
        "denied package was activated"
    );
    let exposed = record(user.path(), "exposed-drifted");
    assert!(
        !exposed.trusted && !exposed.enabled,
        "model-visible package was not revalidated"
    );
    assert_eq!(
        std::fs::read(&unrelated.executable).unwrap(),
        DRIFTED_EXE_CONTENT
    );
}

#[tokio::test]
async fn model_discovery_validates_allow_and_ask_candidates_but_not_denied() {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let allowed = package("allowed-adapter");
    let ask = package("ask-adapter");
    let denied = package("denied-adapter");
    install_and_enable(&allowed, workspace.path(), user.path(), "allowed-adapter");
    install_and_enable(&ask, workspace.path(), user.path(), "ask-adapter");
    install_and_enable(&denied, workspace.path(), user.path(), "denied-adapter");
    std::fs::write(&denied.executable, DRIFTED_EXE_CONTENT).unwrap();

    let mut config = OpiConfig::default();
    config.execution.strategy = ExecutionStrategy::Model;
    config
        .execution
        .permissions
        .insert("allowed-adapter".into(), PermissionDecision::Allow);
    config
        .execution
        .permissions
        .insert("ask-adapter".into(), PermissionDecision::Ask);
    config
        .execution
        .permissions
        .insert("denied-adapter".into(), PermissionDecision::Deny);
    let provider = opi_ai::test_support::MockProvider::new(
        "mock",
        vec![opi_ai::test_support::text_response("ok")],
    );
    let call_log = provider.call_log_handle();
    let mut harness = build_harness_with_provider(workspace.path(), user.path(), config, provider);

    harness.prompt("inspect model routing").await.unwrap();

    let calls = call_log.lock().unwrap();
    let bash = calls[0]
        .tools
        .iter()
        .find(|tool| tool.name == "bash")
        .expect("model routing exposes bash");
    let variants = bash.input_schema["properties"]["backend"]["oneOf"]
        .as_array()
        .expect("bounded backend variants");
    let ids = variants
        .iter()
        .map(|variant| variant["const"].as_str().expect("backend const"))
        .collect::<Vec<_>>();
    assert!(ids.contains(&"allowed-adapter"));
    assert!(ids.contains(&"ask-adapter"));
    assert!(!ids.contains(&"denied-adapter"));
    let ask_variant = variants
        .iter()
        .find(|variant| variant["const"] == "ask-adapter")
        .expect("ask candidate is model-visible");
    assert!(
        ask_variant["description"]
            .as_str()
            .unwrap_or_default()
            .contains("approval"),
        "ask candidate must retain its permission status in the model schema"
    );
    drop(calls);

    assert!(record(user.path(), "allowed-adapter").trusted);
    assert!(record(user.path(), "ask-adapter").trusted);
    let denied_record = record(user.path(), "denied-adapter");
    assert!(
        denied_record.trusted && denied_record.enabled,
        "denied-only drift must not be activated or mutate Package Trust"
    );
}
