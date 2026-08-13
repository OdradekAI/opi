use std::path::Path;
use std::sync::Arc;
use std::sync::OnceLock;

use opi_ai::test_support::{self, MockProvider, text_response};
use opi_coding_agent::cli::PackageCommand;
use opi_coding_agent::config::{
    ExecutionRunMode, ExecutionStrategy, OpiConfig, PermissionDecision,
};
use opi_coding_agent::credential_store::{FakeKeyringBackend, KeychainCredentialStore};
use opi_coding_agent::harness::CodingHarness;
use opi_coding_agent::interactive::{install_interactive_tui_test_driver, run_interactive_tui};
use opi_coding_agent::package_activation::{
    PackageActivationStore, TrustConfirmer, TrustDisplay, host_opi_version, host_target_triple,
};
use opi_coding_agent::package_cli;
use opi_coding_agent::project_trust::TrustDecision;
use opi_tui::Keybindings;

const COMMAND_CANARY: &str = "bin/private-command-canary";
const ENV_CANARY: &str = "OPI_INTERACTIVE_SECRET_ENV=env-secret-canary";
const CREDENTIAL_CANARY: &str = "sk-interactive-credential-canary";
const ABSOLUTE_PATH_CANARY: &str = "C:/private/interactive/adapter.exe";
const EXE_CONTENT: &[u8] = b"#!/bin/sh\necho initial\n";
const DRIFTED_EXE_CONTENT: &[u8] = b"#!/bin/sh\necho drifted\n";

static TEST_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

async fn test_lock() -> tokio::sync::MutexGuard<'static, ()> {
    TEST_LOCK
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

fn expected_message(code: &str, remediation: &str) -> String {
    format!(
        "[error] adapter::{code}: execution backend unavailable at startup (action: {remediation})"
    )
}

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

struct GrantTrust;

impl TrustConfirmer for GrantTrust {
    fn confirm(&mut self, _display: &TrustDisplay) -> Result<(), String> {
        Ok(())
    }
}

fn install_drifted_external_package(workspace: &Path, user: &Path) -> tempfile::TempDir {
    let package = tempfile::tempdir().unwrap();
    let command = package.path().join(COMMAND_CANARY);
    std::fs::create_dir_all(command.parent().unwrap()).unwrap();
    std::fs::write(&command, EXE_CONTENT).unwrap();
    make_executable(&command);
    let opi_version = host_opi_version();
    let manifest = format!(
        "version = \"0.8.0\"\n\
         opi_version = \"={opi_version}\"\n\
         name = \"fixed-external\"\n\
         description = \"interactive startup diagnostic fixture\"\n\
         \n\
         [[contributions.adapters]]\n\
         capability = \"command.execute\"\n\
         id = \"fixed-external\"\n\
         transport = \"process-jsonl\"\n\
         command = \"{COMMAND_CANARY}\"\n\
         args = [\"{ENV_CANARY}\", \"{ABSOLUTE_PATH_CANARY}\"]\n\
         protocol = \"command-execution-jsonl-v1\"\n\
         target = \"{}\"\n\
         sha256 = \"{}\"\n\
         handshake_timeout_ms = 5000\n\
         adapter_config = {{ credential = \"{CREDENTIAL_CANARY}\" }}\n",
        host_target_triple(),
        sha256(EXE_CONTENT),
    );
    std::fs::write(package.path().join("package.toml"), manifest).unwrap();
    let exit = package_cli::handle_package_command(
        &PackageCommand::Add {
            source: package.path().to_string_lossy().into_owned(),
            local: false,
        },
        workspace.to_path_buf(),
        user.to_path_buf(),
    );
    assert_eq!(exit, 0);
    PackageActivationStore::global(user.to_path_buf())
        .enable(
            "fixed-external",
            host_target_triple(),
            host_opi_version(),
            &mut GrantTrust,
        )
        .unwrap();
    std::fs::write(command, DRIFTED_EXE_CONTENT).unwrap();
    package
}

async fn run_refused_interactive(
    config: OpiConfig,
    workspace: &Path,
    user: &Path,
) -> (Vec<String>, Vec<Vec<String>>) {
    let provider = MockProvider::new("mock", vec![text_response("ok")]);
    let call_log = provider.call_log_handle();
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".into(),
        config,
        workspace.to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user.to_path_buf())
    .execution_mode(ExecutionRunMode::Interactive)
    .build();
    harness.credential_store = Some(Arc::new(KeychainCredentialStore::new(
        Box::new(FakeKeyringBackend::new()),
        user.to_path_buf(),
    )));

    let driver = install_interactive_tui_test_driver(["inspect", "exit"]).unwrap();
    run_interactive_tui(
        harness,
        "mock:mock-model".into(),
        "default",
        Keybindings::default(),
    )
    .await
    .unwrap();

    let capture = driver.capture();
    let calls = call_log
        .lock()
        .unwrap()
        .iter()
        .map(|request| request.tools.iter().map(|tool| tool.name.clone()).collect())
        .collect();
    (capture.system_messages, calls)
}

fn assert_refusal_surface(messages: &[String], calls: &[Vec<String>], expected: &str) {
    assert_eq!(
        messages,
        [expected],
        "the startup refusal must be inserted once and retain its initial ordering"
    );
    assert_eq!(calls.len(), 1);
    assert!(
        calls[0].iter().all(|tool| tool != "bash"),
        "a refused execution backend must omit bash: {:?}",
        calls[0]
    );

    let rendered = messages.join("\n");
    for canary in [
        COMMAND_CANARY,
        ENV_CANARY,
        CREDENTIAL_CANARY,
        ABSOLUTE_PATH_CANARY,
    ] {
        assert!(!rendered.contains(canary), "startup TUI leaked {canary:?}");
    }
    assert!(
        !rendered.contains(&std::process::id().to_string()),
        "startup TUI leaked the process id: {rendered}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn interactive_local_deny_shows_stable_startup_refusal_once() {
    let _lock = test_lock().await;
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let mut config = OpiConfig::default();
    config.execution.strategy = ExecutionStrategy::Fixed;
    config.execution.backend = "local".into();
    config
        .execution
        .permissions
        .insert("local".into(), PermissionDecision::Deny);

    let (messages, calls) = run_refused_interactive(config, workspace.path(), user.path()).await;
    let remediation = "Adapter \"local\" is denied by your execution permission policy. To allow it, set `[execution.permissions]` in your USER config (project permission sections are not honored).";
    assert_refusal_surface(
        &messages,
        &calls,
        &expected_message("policy_denied", remediation),
    );
}

#[tokio::test(flavor = "current_thread")]
async fn interactive_unavailable_fixed_external_shows_stable_startup_refusal_once() {
    let _lock = test_lock().await;
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let _package = install_drifted_external_package(workspace.path(), user.path());
    let mut config = OpiConfig::default();
    config.execution.strategy = ExecutionStrategy::Fixed;
    config.execution.backend = "fixed-external".into();
    config
        .execution
        .permissions
        .insert("fixed-external".into(), PermissionDecision::Allow);

    let (messages, calls) = run_refused_interactive(config, workspace.path(), user.path()).await;
    let remediation = "Adapter package \"fixed-external\" is not trusted (never confirmed or its manifest/lock/executable drifted). Review it with `opi package doctor`, then re-confirm trust with `opi package enable fixed-external`.";
    assert_refusal_surface(
        &messages,
        &calls,
        &expected_message("package_untrusted", remediation),
    );
}

#[tokio::test(flavor = "current_thread")]
async fn interactive_tool_failure_diagnostic_survives_provider_recovery() {
    let _lock = test_lock().await;
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let command = if cfg!(windows) { "exit /B 7" } else { "exit 7" };
    let provider = MockProvider::new(
        "mock",
        vec![
            test_support::tool_call_response(
                "local-failure",
                "bash",
                &serde_json::json!({"command": command}).to_string(),
            ),
            text_response("recovered"),
        ],
    );
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user.path().to_path_buf())
    .execution_mode(ExecutionRunMode::Interactive)
    .build();
    harness.credential_store = Some(Arc::new(KeychainCredentialStore::new(
        Box::new(FakeKeyringBackend::new()),
        user.path().to_path_buf(),
    )));

    let driver = install_interactive_tui_test_driver(["run a failing command", "exit"]).unwrap();
    run_interactive_tui(
        harness,
        "mock:mock-model".into(),
        "default",
        Keybindings::default(),
    )
    .await
    .unwrap();

    let rendered = driver.capture().system_messages.join("\n");
    assert!(
        rendered.contains("tool::tool_execution_failed"),
        "tool failure diagnostic must remain visible after provider recovery: {rendered:?}"
    );
    assert!(
        rendered.contains("command exited non-zero"),
        "the TUI diagnostic must preserve its public message: {rendered:?}"
    );
}
