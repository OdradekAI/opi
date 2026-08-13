#![cfg(feature = "execution-backend-test-fixture")]
//! Task 16.9 SC16-07 (production path): a protocol-layer backend failure's
//! stable code survives the FULL production path — `ExecutionRuntime::build`
//! (wired by `CodingHarness::build_tools`) -> the production
//! `BashTool` -> `execute` -> `ToolResult.diagnostics`. The substrate suite
//! (`execution_runtime.rs`) proves the code reaches `BashOpError`; this proves
//! the `bash.rs` lift carries it into the agent `ToolResult`, end-to-end through
//! the real startup chokepoint.
//!
//! Build the mock peer first:
//!
//! ```text
//! cargo test -p opi-coding-agent --features execution-backend-test-fixture \
//!   --test execution_backend_mock --no-run
//! ```

mod common;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use opi_agent::event::AgentEvent;
use opi_agent::loop_types::{
    AgentLoopConfig, AgentLoopContext, InferenceConfig, ModelSelection, NextTurnState,
};
use opi_agent::message::AgentMessage;
use opi_agent::sdk::agent_event_to_value;
use opi_agent::session_event::AgentSessionEvent;
use opi_ai::test_support::{self, MockProvider, single_route_collection};
use opi_coding_agent::cli::PackageCommand;
use opi_coding_agent::config::{
    ExecutionConfig, ExecutionRunMode, ExecutionStrategy, OpiConfig, PermissionDecision,
};
use opi_coding_agent::doctor::{DoctorContext, DoctorScope, run_doctor};
use opi_coding_agent::execution::ValidatedExecutableContribution;
use opi_coding_agent::execution::permission::PermissionPolicy;
use opi_coding_agent::execution::{
    EnabledIdentity, IdentitySource, LOCAL_ADAPTER_ID, LockMaterial, PermissionManager,
};
use opi_coding_agent::harness::{CodingAgentHooks, CodingHarness, ExecutionWiring};
use opi_coding_agent::package_activation::{
    ActivatedContribution, ActivationError, PackageActivationStore, TrustConfirmer, TrustDisplay,
    host_opi_version, host_target_triple,
};
use opi_coding_agent::package_cli;
use opi_coding_agent::package_store::PackageLockEntry;
use opi_coding_agent::policy::{RunMode, ToolRuntimeConfig, ToolSelection};
use opi_protocol::execution::v1::WIRE_IDENTITY;
use tokio_util::sync::CancellationToken;

/// Locate the `execution_backend_mock` test binary in the same deps dir (mirrors
/// `execution_runtime.rs::mock_bin`).
fn mock_bin() -> PathBuf {
    let current = std::env::current_exe().expect("current exe path");
    let deps_dir = current.parent().expect("deps directory");
    let exact_name = if cfg!(windows) {
        "execution_backend_mock.exe"
    } else {
        "execution_backend_mock"
    };
    let exact_path = deps_dir.join(exact_name);
    if exact_path.exists() {
        return exact_path;
    }
    let exe_suffix = if cfg!(windows) { ".exe" } else { "" };
    let prefix = "execution_backend_mock-";
    if let Ok(entries) = std::fs::read_dir(deps_dir) {
        let newest = entries
            .flatten()
            .filter(|entry| {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                name.starts_with(prefix) && name.ends_with(exe_suffix) && !name.ends_with(".d")
            })
            .max_by_key(|entry| entry.metadata().and_then(|meta| meta.modified()).ok());
        if let Some(entry) = newest {
            return entry.path();
        }
    }
    panic!(
        "Could not find execution_backend_mock in {}. Build it first with \
         `cargo test --features execution-backend-test-fixture --test execution_backend_mock --no-run`",
        deps_dir.display()
    );
}

fn lock_material(adapter_id: &str) -> LockMaterial {
    LockMaterial {
        manifest_hash: "dummy".to_string(),
        executable_rel_path: "bin/mock".to_string(),
        executable_sha256: "dummy".to_string(),
        package_version: "mock-1.0.0".to_string(),
        target: "mock-target".to_string(),
        opi_range: ">=0.8,<0.9".to_string(),
        protocol: WIRE_IDENTITY.to_string(),
        adapter_id: adapter_id.to_string(),
    }
}

/// A canned activated contribution that launches the mock peer in `mode` (the
/// peer selects behavior by its first CLI arg).
fn canned(adapter_id: &str, pkg: &str, mode: &str) -> ActivatedContribution {
    canned_with_args(adapter_id, pkg, &[mode])
}

/// A canned activated contribution whose mock-peer CLI args are `[mode, extra]`
/// (the peer reads the failure code from the second arg for `failed_*` modes).
fn canned_with_args(adapter_id: &str, pkg: &str, mode_args: &[&str]) -> ActivatedContribution {
    ActivatedContribution {
        name: pkg.to_string(),
        source: pkg.to_string(),
        validated: vec![ValidatedExecutableContribution {
            capability: "command.execute".to_string(),
            id: adapter_id.to_string(),
            transport: "process-jsonl".to_string(),
            command: mock_bin(),
            executable: Arc::new(std::fs::File::open(mock_bin()).unwrap()),
            args: mode_args.iter().map(|s| (*s).to_string()).collect(),
            protocol: WIRE_IDENTITY.to_string(),
            target: "mock-target".to_string(),
            handshake_timeout_ms: 5000,
            adapter_config: serde_json::json!({}),
            lock: lock_material(adapter_id),
        }],
        lock: PackageLockEntry {
            identity_kind: "local".to_string(),
            identity_value: pkg.to_string(),
            source: pkg.to_string(),
            package_root: PathBuf::from("."),
            cache_path: None,
            git_commit: None,
            manifest_sha256: "dummy".to_string(),
            contributions: Vec::new(),
        },
    }
}

/// An `IdentitySource` that returns a fixed canned contribution on every
/// activation (the pre-spawn revalidation the routed backend performs).
struct MockSource {
    contribution: ActivatedContribution,
}
impl IdentitySource for MockSource {
    fn activate(
        &self,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<ActivatedContribution, ActivationError> {
        Ok(self.contribution.clone())
    }
}

/// An `IdentitySource` that panics if activated — correct for selection-time
/// failures (`no_eligible_adapter`, `adapter_not_selected`) that never reach a
/// package activation.
struct PanicSource;
impl IdentitySource for PanicSource {
    fn activate(
        &self,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<ActivatedContribution, ActivationError> {
        panic!("selection-failure tests must not activate any package");
    }
}

fn routed_wiring(contribution: ActivatedContribution) -> ExecutionWiring {
    ExecutionWiring {
        config: ExecutionConfig {
            strategy: ExecutionStrategy::Fixed,
            backend: "opi-sandbox".to_string(),
            ..ExecutionConfig::default()
        },
        enabled: vec![EnabledIdentity {
            adapter_id: "opi-sandbox".to_string(),
            package_name: "mock-pkg".to_string(),
        }],
        policy: PermissionPolicy::from_map(
            [("opi-sandbox".to_string(), PermissionDecision::Allow)]
                .into_iter()
                .collect(),
        ),
        store: Arc::new(MockSource { contribution }),
        mode: ExecutionRunMode::Interactive,
        host_target: host_target_triple().to_string(),
        host_opi_version: host_opi_version().to_string(),
        manager: Arc::new(PermissionManager::new()),
        broker: None,
    }
}

/// SC16-07: a protocol-layer backend failure (malformed frame -> ProtocolViolation)
/// reaches the agent `ToolResult.diagnostics` with its stable code intact,
/// through the FULL production path (build_tools -> BashTool ->
/// execute). The substrate suite proves the code reaches `BashOpError`; this
/// proves the `bash.rs` lift and that no failure retries through `local`.
#[tokio::test]
async fn protocol_violation_survives_into_tool_result_via_production_path() {
    let ws = tempfile::tempdir().unwrap();
    let wiring = routed_wiring(canned("opi-sandbox", "mock-pkg", "malformed_frame"));
    let tool_config =
        ToolRuntimeConfig::resolve(RunMode::Interactive, true, ToolSelection::Default)
            .expect("interactive tool config");
    let (mut tools, startup_diagnostics) =
        CodingHarness::build_tools(ws.path(), &tool_config, &wiring);
    assert!(
        startup_diagnostics.is_empty(),
        "routed allow must not warn at startup: {startup_diagnostics:?}"
    );
    let bash = tools
        .iter_mut()
        .find(|t| t.definition().name == "bash")
        .expect("the routed bash tool is present (build Ok, failure is at exec time)");

    let result = bash
        .execute(
            "test-call",
            serde_json::json!({"command": "echo hi", "timeout_secs": 5}),
            CancellationToken::new(),
            None,
        )
        .await
        .expect("bash tool executes");

    assert!(
        result.is_error,
        "a protocol violation must surface as a tool error (no degraded success)"
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "protocol_violation" || d.message.contains("protocol_violation")),
        "the stable protocol_violation code must survive into ToolResult.diagnostics: {:?}",
        result.diagnostics
    );
}

/// SC16-07 companion: an activation failure (package_untrusted) also lifts its
/// stable code into `ToolResult.diagnostics` via the production path, and does
/// NOT retry through `local` (the local sentinel is never reached).
#[tokio::test]
async fn activation_failure_survives_into_tool_result_via_production_path() {
    struct UntrustedSource;
    impl IdentitySource for UntrustedSource {
        fn activate(
            &self,
            name: &str,
            _: &str,
            _: &str,
        ) -> Result<ActivatedContribution, ActivationError> {
            Err(ActivationError::Untrusted {
                name: name.to_string(),
                detail: "lock drift".to_string(),
            })
        }
    }
    let ws = tempfile::tempdir().unwrap();
    let wiring = ExecutionWiring {
        config: ExecutionConfig {
            strategy: ExecutionStrategy::Fixed,
            backend: "opi-sandbox".to_string(),
            ..ExecutionConfig::default()
        },
        enabled: vec![EnabledIdentity {
            adapter_id: "opi-sandbox".to_string(),
            package_name: "mock-pkg".to_string(),
        }],
        policy: PermissionPolicy::from_map(
            [("opi-sandbox".to_string(), PermissionDecision::Allow)]
                .into_iter()
                .collect(),
        ),
        store: Arc::new(UntrustedSource),
        mode: ExecutionRunMode::Interactive,
        host_target: host_target_triple().to_string(),
        host_opi_version: host_opi_version().to_string(),
        manager: Arc::new(PermissionManager::new()),
        broker: None,
    };
    let tool_config =
        ToolRuntimeConfig::resolve(RunMode::Interactive, true, ToolSelection::Default)
            .expect("interactive tool config");
    let (mut tools, _diags) = CodingHarness::build_tools(ws.path(), &tool_config, &wiring);
    let bash = tools
        .iter_mut()
        .find(|t| t.definition().name == "bash")
        .expect("routed bash tool present");
    let result = bash
        .execute(
            "test-call",
            serde_json::json!({"command": "echo hi", "timeout_secs": 5}),
            CancellationToken::new(),
            None,
        )
        .await
        .expect("bash tool executes");
    assert!(result.is_error, "activation failure must be a tool error");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "package_untrusted"),
        "the stable package_untrusted code must survive into ToolResult.diagnostics: {:?}",
        result.diagnostics
    );
    assert_remediation(&result, "package_untrusted");
}

/// SC16-04 (production RUNTIME path, design §Model routing: "the model names a
/// backend and the router selects it"): the model-supplied `backend` value
/// survives the FULL production execute path — `BashTool::execute` ->
/// `BashCallArgs.backend` -> `BashRequest.backend` -> `RoutedBashOperations::exec`
/// -> `resolve_selection` — and controls which adapter runs. With
/// `backend="opi-sandbox"` the named EXTERNAL adapter runs (mock peer happy_path
/// reports "hello"); with `backend="local"` the LOCAL backend runs "echo hi". A
/// regression dropping the backend value (or hardcoding the adapter) fails one of
/// the two assertions.
#[tokio::test]
async fn model_supplied_backend_selects_named_adapter_through_execute() {
    let ws = tempfile::tempdir().unwrap();
    let wiring = ExecutionWiring {
        config: ExecutionConfig {
            strategy: ExecutionStrategy::Model,
            ..ExecutionConfig::default()
        },
        enabled: vec![EnabledIdentity {
            adapter_id: "opi-sandbox".to_string(),
            package_name: "mock-pkg".to_string(),
        }],
        policy: PermissionPolicy::from_map(
            [("opi-sandbox".to_string(), PermissionDecision::Allow)]
                .into_iter()
                .collect(),
        ),
        store: Arc::new(MockSource {
            contribution: canned("opi-sandbox", "mock-pkg", "happy_path"),
        }),
        mode: ExecutionRunMode::Interactive,
        host_target: host_target_triple().to_string(),
        host_opi_version: host_opi_version().to_string(),
        manager: Arc::new(PermissionManager::new()),
        broker: None,
    };
    let tool_config =
        ToolRuntimeConfig::resolve(RunMode::Interactive, true, ToolSelection::Default)
            .expect("interactive tool config");
    let (mut tools, _diags) = CodingHarness::build_tools(ws.path(), &tool_config, &wiring);
    let bash = tools
        .iter_mut()
        .find(|t| t.definition().name == "bash")
        .expect("bash tool present under routed model");

    // backend="opi-sandbox" -> the value reaches resolve_selection -> the named
    // EXTERNAL adapter runs (mock peer happy_path reports "hello"). Dropping the
    // backend value would yield adapter_not_selected (no run -> is_error).
    let result_external = bash
        .execute(
            "call-external",
            serde_json::json!({"command":"echo hi","backend":"opi-sandbox","timeout_secs":5}),
            CancellationToken::new(),
            None,
        )
        .await
        .expect("bash tool executes");
    assert!(
        !result_external.is_error,
        "model backend=opi-sandbox must run the named adapter: {:?}",
        result_external.content
    );
    let text_external = serde_json::to_string(&result_external.content).expect("outputs serialize");
    assert!(
        text_external.contains("hello"),
        "the mock peer ran via the supplied backend value: {text_external}"
    );

    // backend="local" -> the VALUE controls selection (not a hardcoded external):
    // the LOCAL backend runs "echo hi" -> "hi", NOT the peer's "hello".
    let result_local = bash
        .execute(
            "call-local",
            serde_json::json!({"command":"echo hi","backend":"local","timeout_secs":5}),
            CancellationToken::new(),
            None,
        )
        .await
        .expect("bash tool executes");
    assert!(
        !result_local.is_error,
        "model backend=local must run the local adapter: {:?}",
        result_local.content
    );
    let text_local = serde_json::to_string(&result_local.content).expect("outputs serialize");
    assert!(
        text_local.contains("hi") && !text_local.contains("hello"),
        "backend=local must select the LOCAL backend (echo hi -> 'hi', not the peer): {text_local}"
    );
}
/// `apply_execution_overrides(strategy=Model)` -> the resolved `ExecutionWiring`
/// -> `build_tools` -> the bash schema gains the model backend
/// field. Proves the CLI-override -> config -> harness link end-to-end. (Schema
/// only; no backend execution, so a panic store is correct.)
#[test]
fn cli_execution_overrides_reach_bash_tool() {
    struct PanicStore;
    impl IdentitySource for PanicStore {
        fn activate(
            &self,
            _: &str,
            _: &str,
            _: &str,
        ) -> Result<ActivatedContribution, ActivationError> {
            panic!("schema test must not activate any package");
        }
    }
    let mut config = OpiConfig::default();
    config.apply_execution_overrides(None, Some(ExecutionStrategy::Model));
    let ws = tempfile::tempdir().unwrap();
    let wiring = ExecutionWiring {
        config: config.execution.clone(),
        enabled: vec![EnabledIdentity {
            adapter_id: "opi-sandbox".to_string(),
            package_name: "pkg".to_string(),
        }],
        policy: PermissionPolicy::from_map(config.execution.permissions.clone()),
        store: Arc::new(PanicStore),
        mode: ExecutionRunMode::Interactive,
        host_target: host_target_triple().to_string(),
        host_opi_version: host_opi_version().to_string(),
        manager: Arc::new(PermissionManager::new()),
        broker: None,
    };
    let tool_config =
        ToolRuntimeConfig::resolve(RunMode::Interactive, true, ToolSelection::Default)
            .expect("interactive tool config");
    let (tools, diags) = CodingHarness::build_tools(ws.path(), &tool_config, &wiring);
    assert!(diags.is_empty(), "model override must not warn: {diags:?}");
    let bash = tools
        .iter()
        .find(|t| t.definition().name == "bash")
        .expect("bash tool present under routed model");
    let schema = bash.definition().input_schema;
    let const_vals: Vec<&str> = schema["properties"]["backend"]["oneOf"]
        .as_array()
        .expect("backend oneOf")
        .iter()
        .map(|v| v["const"].as_str().expect("const id"))
        .collect();
    assert!(
        const_vals.contains(&"local"),
        "the Model override reached the harness backend schema: {const_vals:?}"
    );
}

// ---------------------------------------------------------------------------
// SC16-13 vertical slice: install-to-execute through the REAL package lifecycle
// ---------------------------------------------------------------------------
//
// The DoD requires that "a packaged adapter reaches a real bash tool turn
// through package CLI dispatch, PackageActivationStore, ExecutionRuntime,
// routing, permission, the production protocol host, and BashTool". The
// substrate suites prove each layer in isolation (and with canned in-process
// contributions); these tests prove the WHOLE chain with a REAL packaged
// archive: the mock peer binary is copied into a package directory, the package
// is added through the production `package add` CLI dispatch, enabled through
// the real `PackageActivationStore`, and then a real `BashTool::execute` call
// runs the packaged backend process end-to-end.

/// sha256 hex of the packaged executable (the validator requires an exact
/// match between the declared hash and the bytes under `bin/`).
fn t_sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(bytes))
}

/// chmod +x on Unix (the packaged executable must pass `is_executable`).
fn make_executable(path: &std::path::Path) {
    let _ = path;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

/// The packaged executable name inside `bin/`. On Windows the copy keeps the
/// `.exe` suffix so the OS can launch it; elsewhere it is bare.
fn packaged_exe_name() -> &'static str {
    if cfg!(windows) {
        "mock-peer.exe"
    } else {
        "mock-peer"
    }
}

/// A confirmer that grants trust deterministically (the real store's enable
/// path requires explicit confirmation on first enablement).
struct GrantingConfirmer;
impl TrustConfirmer for GrantingConfirmer {
    fn confirm(&mut self, _display: &TrustDisplay) -> Result<(), String> {
        Ok(())
    }
}

/// Build a real package directory whose `bin/` holds a copy of the
/// `execution_backend_mock` peer and whose `package.toml` declares it as a
/// `command.execute` contribution. Returns the tempdir (kept alive by the
/// caller) and its root path.
fn packaged_mock_peer(adapter_id: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("bin")).unwrap();
    let exe = dir.path().join("bin").join(packaged_exe_name());
    std::fs::copy(mock_bin(), &exe).expect("copy mock peer into package bin");
    make_executable(&exe);
    let sha = t_sha256(&std::fs::read(&exe).expect("read packaged exe"));
    let target = host_target_triple();
    let toml = format!(
        "version = \"0.8.0\"\n\
         opi_version = \">=0.7,<0.8\"\n\
         name = \"{adapter_id}\"\n\
         description = \"packaged execution backend (16.16.2)\"\n\
         \n\
         [[contributions.adapters]]\n\
         capability = \"command.execute\"\n\
         id = \"{adapter_id}\"\n\
         transport = \"process-jsonl\"\n\
         command = \"bin/{}\"\n\
         args = [\"happy_path\", \"{adapter_id}\", \"0.8.0\", \"{target}\"]\n\
         protocol = \"command-execution-jsonl-v1\"\n\
         target = \"{target}\"\n\
         sha256 = \"{sha}\"\n\
         handshake_timeout_ms = 5000\n\
         adapter_config = {{}}\n",
        packaged_exe_name()
    );
    std::fs::write(dir.path().join("package.toml"), toml).unwrap();
    let root = dir.path().to_path_buf();
    (dir, root)
}

#[test]
fn doctor_does_not_synthesize_untrusted_state_when_activation_store_is_corrupt() {
    let (_package, root) = packaged_mock_peer("opi-sandbox");
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    assert_eq!(
        package_cli::handle_package_command(
            &PackageCommand::Add {
                source: root.to_str().unwrap().to_string(),
                local: false,
            },
            workspace.path().to_path_buf(),
            user.path().to_path_buf(),
        ),
        0
    );
    let store = PackageActivationStore::global(user.path().to_path_buf());
    std::fs::write(store.store().trust_path(), "not = [valid toml").unwrap();
    let config = OpiConfig::default();
    let env_var = |_name: &str| None;
    let store_probe = HashMap::new();
    let context = DoctorContext {
        config: &config,
        config_error: None,
        workspace_root: workspace.path(),
        user_config_dir: user.path(),
        sessions_dir: workspace.path(),
        term: None,
        term_program: None,
        term_features: None,
        no_color: false,
        colorterm: None,
        env_var: &env_var,
        store_probe: &store_probe,
    };

    let report = run_doctor(&[DoctorScope::Package], &context);
    assert!(
        report
            .entries
            .iter()
            .any(|entry| entry.diagnostic.code == "adapter_unavailable")
    );
    assert!(
        report
            .entries
            .iter()
            .all(|entry| entry.diagnostic.code != "package_untrusted"),
        "corrupt store must not be presented as an untrusted package: {:?}",
        report.entries
    );
}

/// The production wiring shape for a real installed+enabled package: the real
/// `PackageActivationStore` is the `IdentitySource`, and `enabled` comes from
/// `PackageActivationStore::usable_enabled_identities` after production host
/// compatibility filtering.
fn real_store_wiring(
    user_dir: &std::path::Path,
    backend: &str,
    mode: ExecutionRunMode,
) -> (ExecutionWiring, PackageActivationStore) {
    let store = PackageActivationStore::global(user_dir.to_path_buf());
    let enabled = store.usable_enabled_identities(host_target_triple(), host_opi_version());
    let wiring = ExecutionWiring {
        config: ExecutionConfig {
            strategy: ExecutionStrategy::Fixed,
            backend: backend.to_string(),
            ..ExecutionConfig::default()
        },
        enabled,
        policy: PermissionPolicy::from_map(
            [(backend.to_string(), PermissionDecision::Allow)]
                .into_iter()
                .collect(),
        ),
        store: Arc::new(store.clone()),
        mode,
        host_target: host_target_triple().to_string(),
        host_opi_version: host_opi_version().to_string(),
        manager: Arc::new(PermissionManager::new()),
        broker: None,
    };
    (wiring, store)
}

/// SC16-13 (DoD): a packaged archive is added through the production
/// `package add` CLI dispatch, trusted+enabled through the real
/// `PackageActivationStore`, and then drives a REAL bash tool turn through
/// `ExecutionRuntime::build` -> routing -> permission -> `ExecutionProtocolHost`
/// -> `BashTool::execute`. The packaged mock peer reports "hello" so the
/// assertion proves the whole chain ran the packaged backend, not a canned
/// in-process contribution.
#[tokio::test]
async fn packaged_adapter_reaches_bash_turn_through_real_package_lifecycle() {
    let (_pkg, root) = packaged_mock_peer("opi-sandbox");
    let (_mismatched_pkg, mismatched_root) = packaged_mock_peer("target-mismatch");
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();

    // 1. package CLI dispatch: `package add` (global scope).
    let exit = package_cli::handle_package_command(
        &PackageCommand::Add {
            source: root.to_str().unwrap().to_string(),
            local: false,
        },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );
    assert_eq!(exit, 0, "package add must install the archive");
    let exit = package_cli::handle_package_command(
        &PackageCommand::Add {
            source: mismatched_root.to_str().unwrap().to_string(),
            local: false,
        },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );
    assert_eq!(exit, 0, "package add must install the mismatched archive");

    // 2. PackageActivationStore: explicit trust + enable (first enablement
    // requires interactive confirmation; the granting confirmer stands in for
    // the TUI).
    let store = PackageActivationStore::global(user.path().to_path_buf());
    store
        .enable(
            "opi-sandbox",
            host_target_triple(),
            host_opi_version(),
            &mut GrantingConfirmer,
        )
        .expect("enable must grant trust + enablement");
    store
        .enable(
            "target-mismatch",
            host_target_triple(),
            host_opi_version(),
            &mut GrantingConfirmer,
        )
        .expect("the second package is initially compatible and enabled");

    // Simulate machine-owned lock metadata copied from another target after
    // enablement. The trust record remains really enabled, while startup must
    // exclude this now-incompatible locked contribution.
    let mut locks = store.store().read_lock().expect("read package lock");
    let mismatched = locks
        .iter_mut()
        .flat_map(|entry| entry.contributions.iter_mut())
        .find(|contribution| contribution.adapter_id == "target-mismatch")
        .expect("target-mismatch locked contribution");
    assert_eq!(mismatched.target, host_target_triple());
    mismatched.target = "mismatched-target".to_string();
    store
        .store()
        .write_lock(&locks)
        .expect("persist mismatched locked target");
    assert_eq!(
        store.enabled_identities().len(),
        2,
        "both the compatible and target-mismatched package identities are really enabled"
    );

    // 3. Production wiring + build_tools chokepoint.
    let (wiring, _store) =
        real_store_wiring(user.path(), "opi-sandbox", ExecutionRunMode::Interactive);
    assert_eq!(
        wiring.enabled.len(),
        1,
        "startup compatibility filtering must remove the enabled target mismatch"
    );
    assert_eq!(wiring.enabled[0].adapter_id, "opi-sandbox");
    let tool_config =
        ToolRuntimeConfig::resolve(RunMode::Interactive, true, ToolSelection::Default)
            .expect("interactive tool config");
    let (mut tools, startup_diagnostics) =
        CodingHarness::build_tools(workspace.path(), &tool_config, &wiring);
    assert!(
        startup_diagnostics.is_empty(),
        "a trusted+enabled+allowed backend must not warn at startup: {startup_diagnostics:?}"
    );

    // 4. Real bash tool turn through the packaged adapter.
    let bash = tools
        .iter_mut()
        .find(|t| t.definition().name == "bash")
        .expect("the packaged adapter built a bash tool");
    let result = bash
        .execute(
            "sc16-13-call",
            serde_json::json!({"command": "echo hi", "backend": "opi-sandbox", "timeout_secs": 5}),
            CancellationToken::new(),
            None,
        )
        .await
        .expect("bash tool executes");
    assert!(
        !result.is_error,
        "the packaged adapter must run (no degraded success): {:?}",
        result.content
    );
    let text = serde_json::to_string(&result.content).expect("outputs serialize");
    assert!(
        text.contains("hello"),
        "the PACKAGED mock peer ran end-to-end (its happy_path reports 'hello'): {text}"
    );
    let details = result.details.as_ref().expect("bash result details");
    assert_eq!(details["adapter_id"], "opi-sandbox");
    assert_eq!(details["implementation_version"], "0.8.0");
    assert_eq!(details["target"], host_target_triple());
    assert_eq!(details["protocol"], WIRE_IDENTITY);
    assert_eq!(details["placement"], "host");
    assert_eq!(details["guarantee"], "supervised");
    assert_eq!(details["policy"], "none");
    assert_eq!(details["limitations"], serde_json::json!([]));

    // NDJSON wraps this exact public event in AgentSessionEvent; RPC applies
    // agent_event_to_value. Both must retain the same effective contract.
    let event = AgentEvent::ToolExecutionEnd {
        tool_call_id: "sc16-13-call".into(),
        tool_name: "bash".into(),
        result: serde_json::json!(&result.content),
        details: result.details.clone(),
        is_error: result.is_error,
        truncated: result.truncated,
        diagnostics: result.diagnostics.clone(),
    };
    let ndjson = serde_json::to_value(AgentSessionEvent::Agent {
        event: event.clone(),
    })
    .unwrap();
    let rpc = agent_event_to_value(&event);
    for (surface, value) in [("ndjson", ndjson), ("rpc", rpc)] {
        let encoded = value.to_string();
        for expected in [
            "adapter_id",
            "implementation_version",
            "target",
            "protocol",
            "placement",
            "guarantee",
            "policy",
            "limitations",
        ] {
            assert!(
                encoded.contains(expected),
                "{surface} dropped effective contract field {expected}: {encoded}"
            );
        }
    }
}

/// SC16-14 companion: the same REAL store + real packaged backend, but the
/// package is enabled then DISABLED through the store. The selected external
/// adapter fails closed with `contribution_disabled` — it does NOT fall back to
/// `local`, and the tool turn is an error (no degraded success).
#[tokio::test]
async fn disabled_packaged_adapter_is_contribution_disabled_without_fallback() {
    let (_pkg, root) = packaged_mock_peer("opi-sandbox");
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();

    let exit = package_cli::handle_package_command(
        &PackageCommand::Add {
            source: root.to_str().unwrap().to_string(),
            local: false,
        },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );
    assert_eq!(exit, 0);

    let store = PackageActivationStore::global(user.path().to_path_buf());
    store
        .enable(
            "opi-sandbox",
            host_target_triple(),
            host_opi_version(),
            &mut GrantingConfirmer,
        )
        .expect("enable");

    // Wire the harness while the package is enabled, then disable the store
    // before the turn (the startup-time enabled set is a snapshot).
    let (wiring, store) =
        real_store_wiring(user.path(), "opi-sandbox", ExecutionRunMode::Interactive);
    store.disable("opi-sandbox").expect("disable");

    let tool_config =
        ToolRuntimeConfig::resolve(RunMode::Interactive, true, ToolSelection::Default)
            .expect("interactive tool config");
    let (mut tools, _diags) = CodingHarness::build_tools(workspace.path(), &tool_config, &wiring);
    let bash = tools
        .iter_mut()
        .find(|t| t.definition().name == "bash")
        .expect("routed bash tool present");
    let result = bash
        .execute(
            "sc16-14-disable",
            serde_json::json!({"command": "echo hi", "backend": "opi-sandbox", "timeout_secs": 5}),
            CancellationToken::new(),
            None,
        )
        .await
        .expect("bash tool executes");
    assert!(
        result.is_error,
        "a disabled selected external must fail the turn (no degraded success)"
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "contribution_disabled"),
        "the stable contribution_disabled code must surface: {:?}",
        result.diagnostics
    );
    assert_remediation(&result, "contribution_disabled");
    // No-fallback is proven by `is_error` + the contribution_disabled code above
    // (a local fallback would SUCCEED and produce no error). This last assertion
    // additionally confirms the packaged peer's happy_path did NOT run.
    let text = serde_json::to_string(&result.content).expect("outputs serialize");
    assert!(
        !text.contains("hello"),
        "the disabled packaged backend must NOT run: {text}"
    );
}

// ---------------------------------------------------------------------------
// SC16-14: all 14 stable codes reach ToolResult.diagnostics via the production
// path
// ---------------------------------------------------------------------------
//
// The DoD requires the full 14-code set to reach the surfaces with the same
// stable code + remediation. The substrate suites prove the codes at the
// `BashOpError` layer; these tests prove each code survives the FULL production
// path (`CodingHarness::build_tools` -> `BashTool::execute` ->
// `ToolResult.diagnostics`), the shared envelope NDJSON/RPC/interactive lift.

/// Drive a routed bash tool turn through the production chokepoint with the
/// given contribution + policy, returning the `ToolResult` diagnostics.
async fn routed_tool_result(
    contribution: ActivatedContribution,
    permissions: &[(&str, PermissionDecision)],
    enabled: &[(&str, &str)],
    mode: ExecutionRunMode,
) -> opi_agent::tool::ToolResult {
    routed_tool_result_with_command(contribution, permissions, enabled, mode, "echo hi").await
}

async fn routed_tool_result_with_command(
    contribution: ActivatedContribution,
    permissions: &[(&str, PermissionDecision)],
    enabled: &[(&str, &str)],
    mode: ExecutionRunMode,
    command: &str,
) -> opi_agent::tool::ToolResult {
    let ws = tempfile::tempdir().unwrap();
    let wiring = ExecutionWiring {
        config: ExecutionConfig {
            strategy: ExecutionStrategy::Fixed,
            backend: "opi-sandbox".to_string(),
            ..ExecutionConfig::default()
        },
        enabled: enabled
            .iter()
            .map(|(id, pkg)| EnabledIdentity {
                adapter_id: (*id).to_string(),
                package_name: (*pkg).to_string(),
            })
            .collect(),
        policy: PermissionPolicy::from_map(
            permissions
                .iter()
                .map(|(id, decision)| ((*id).to_string(), *decision))
                .collect(),
        ),
        store: Arc::new(MockSource { contribution }),
        mode,
        host_target: host_target_triple().to_string(),
        host_opi_version: host_opi_version().to_string(),
        manager: Arc::new(PermissionManager::new()),
        broker: None,
    };
    let tool_config =
        ToolRuntimeConfig::resolve(RunMode::Interactive, true, ToolSelection::Default)
            .expect("interactive tool config");
    let (tools, diags) = CodingHarness::build_tools(ws.path(), &tool_config, &wiring);
    assert!(diags.is_empty(), "routed allow must not warn: {diags:?}");
    let bash = tools
        .into_iter()
        .find(|t| t.definition().name == "bash")
        .expect("routed bash tool present");
    bash.execute(
        "code-matrix",
        serde_json::json!({"command": command, "backend": "opi-sandbox", "timeout_secs": 5}),
        CancellationToken::new(),
        None,
    )
    .await
    .expect("bash tool executes")
}

/// Assert the stable code's diagnostic carries non-empty, command-text-free
/// remediation in `context` (the `bash.rs` lift maps the execution-failure
/// `details.remediation` into `ToolResult.diagnostics[].context.remediation`).
fn assert_remediation(result: &opi_agent::tool::ToolResult, expected_code: &str) {
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|d| d.code == expected_code)
        .unwrap_or_else(|| {
            panic!(
                "expected diagnostic code {expected_code}: {:?}",
                result.diagnostics
            )
        });
    let remediation = diagnostic
        .context
        .get("remediation")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    assert!(
        !remediation.is_empty(),
        "stable code `{expected_code}` must carry actionable remediation: {diagnostic:?}"
    );
    assert!(
        !remediation.contains("echo"),
        "remediation for `{expected_code}` must not leak command text: {remediation}"
    );
}

/// SC16-14: the mock-peer protocol/execution failure modes lift their stable
/// codes into `ToolResult.diagnostics` through the production path — the same
/// envelope NDJSON/RPC/interactive surfaces serialize. Covers the codes the
/// packaged adapter itself can report.
#[tokio::test]
async fn mock_peer_failure_modes_surface_stable_codes_via_production_path() {
    // (mode, extra, expected stable code). The mock peer reads the failure code
    // from its second CLI arg for the failed_* modes. A pre-start generic
    // failure maps to `execution_failed`; pre-start `unavailable` preserves the
    // selected adapter identity and is covered separately below.
    let cases: &[(&str, &str, &str)] = &[
        ("failed_pre_started", "failed", "execution_failed"),
        (
            "failed_post_started",
            "execution_failed",
            "execution_failed",
        ),
        (
            "failed_post_started",
            "execution_timed_out",
            "execution_timed_out",
        ),
        (
            "failed_post_started",
            "cleanup_unconfirmed",
            "cleanup_unconfirmed",
        ),
        (
            "failed_pre_started",
            "protocol_incompatible",
            "protocol_incompatible",
        ),
        (
            "failed_post_started",
            "protocol_violation",
            "protocol_violation",
        ),
    ];
    for (mode, extra, expected) in cases {
        let result = routed_tool_result(
            canned_with_args("opi-sandbox", "mock-pkg", &[mode, extra]),
            &[("opi-sandbox", PermissionDecision::Allow)],
            &[("opi-sandbox", "mock-pkg")],
            ExecutionRunMode::Interactive,
        )
        .await;
        assert!(
            result.is_error,
            "mode {mode} {extra} must fail the turn (no degraded success)"
        );
        assert!(
            result.diagnostics.iter().any(|d| &d.code == expected),
            "mode {mode} {extra} must surface stable code `{expected}`: {:?}",
            result.diagnostics
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.code == "opi.execution.backend_diagnostic"),
            "mode {mode} {extra} must surface redacted failed-terminal diagnostics: {:?}",
            result.diagnostics
        );
        for diagnostic in &result.diagnostics {
            if &diagnostic.code == expected {
                let remediation = diagnostic
                    .context
                    .get("remediation")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                assert!(
                    !remediation.is_empty(),
                    "stable code `{expected}` must carry actionable remediation: {diagnostic:?}"
                );
                assert!(
                    !remediation.contains("echo"),
                    "remediation must not leak command text: {remediation}"
                );
            }
        }
    }

    // `protocol_incompatible` is a handshake-only distress code. Once the
    // backend has published Started, the same wire code is an invalid terminal
    // combination and the host must normalize it to `protocol_violation`
    // without retaining diagnostics from the rejected terminal frame.
    let invalid_post_started = routed_tool_result(
        canned_with_args(
            "opi-sandbox",
            "mock-pkg",
            &["failed_post_started", "protocol_incompatible"],
        ),
        &[("opi-sandbox", PermissionDecision::Allow)],
        &[("opi-sandbox", "mock-pkg")],
        ExecutionRunMode::Interactive,
    )
    .await;
    assert!(invalid_post_started.is_error);
    assert!(
        invalid_post_started
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "protocol_violation"
                || diagnostic.message.contains("protocol_violation")),
        "post-start protocol_incompatible must normalize to protocol_violation: {:?}",
        invalid_post_started.diagnostics
    );
    assert!(
        invalid_post_started
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "opi.execution.backend_diagnostic"),
        "diagnostics on a rejected terminal frame must not be retained: {:?}",
        invalid_post_started.diagnostics
    );
}

/// SC16-14: remediation text is DISTINCT per stable code (not one generic
/// string), so a regression collapsing `ExecutionFailure::remediation()` to a
/// single non-'echo' phrase fails the matrix. Drives three different codes
/// through the production chokepoint and asserts each remediation carries a
/// code-specific actionable fragment and the fragments differ.
#[tokio::test]
async fn remediation_is_distinct_per_stable_code() {
    let policy_denied = routed_tool_result(
        canned("opi-sandbox", "mock-pkg", "happy_path"),
        &[("opi-sandbox", PermissionDecision::Deny)],
        &[("opi-sandbox", "mock-pkg")],
        ExecutionRunMode::Interactive,
    )
    .await;
    // `execution_failed` (mock peer) and `no_eligible_adapter` (fixed backend
    // naming an uninstalled adapter) are distinct codes reachable through the
    // same production chokepoint with no store seam.
    let execution_failed = routed_tool_result(
        canned_with_args("opi-sandbox", "mock-pkg", &["failed_pre_started", "failed"]),
        &[("opi-sandbox", PermissionDecision::Allow)],
        &[("opi-sandbox", "mock-pkg")],
        ExecutionRunMode::Interactive,
    )
    .await;
    let no_eligible = routed_tool_result(
        canned("opi-sandbox", "mock-pkg", "happy_path"),
        &[("opi-sandbox", PermissionDecision::Allow)],
        &[],
        ExecutionRunMode::Interactive,
    )
    .await;

    let remediation_for = |result: &opi_agent::tool::ToolResult, code: &str| -> String {
        result
            .diagnostics
            .iter()
            .find(|d| d.code == code)
            .and_then(|d| d.context.get("remediation"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    // These three produce distinct codes at the chokepoint.
    let pd = remediation_for(&policy_denied, "policy_denied");
    let ef = remediation_for(&execution_failed, "execution_failed");
    let ne = remediation_for(&no_eligible, "no_eligible_adapter");
    assert!(
        !pd.is_empty() && !ef.is_empty() && !ne.is_empty(),
        "each stable code must carry actionable remediation: policy_denied={pd:?} execution_failed={ef:?} no_eligible_adapter={ne:?}"
    );
    assert!(
        pd.contains("execution permission")
            && ef.contains("redacted diagnostics")
            && ne.contains("No eligible command.execute adapter"),
        "remediation must be code-specific, not one generic phrase: policy_denied={pd:?} execution_failed={ef:?} no_eligible_adapter={ne:?}"
    );
    assert!(
        pd != ef && ef != ne && pd != ne,
        "remediation must differ across codes: policy_denied={pd:?} execution_failed={ef:?} no_eligible_adapter={ne:?}"
    );
}

/// SC16-14 "no degraded success state exists": a backend reporting an in-band
/// `Completed{timed_out: true, exit: Some(0)}` must surface as an ERROR (the
/// timeout, not the clean exit code, is the effective contract) — matching the
/// local backend's `exit_code=None -> is_error=true` semantics and the design's
/// "The adapter either reports its effective contract or the command fails."
#[tokio::test]
async fn timed_out_in_band_completed_is_not_a_success() {
    let result = routed_tool_result(
        canned("opi-sandbox", "mock-pkg", "completed_timed_out"),
        &[("opi-sandbox", PermissionDecision::Allow)],
        &[("opi-sandbox", "mock-pkg")],
        ExecutionRunMode::Interactive,
    )
    .await;
    let content_text = serde_json::to_string(&result.content).unwrap_or_default();
    assert!(
        result.is_error,
        "a timed-out completed frame must fail the turn (no degraded success): {content_text}"
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.context.get("timed_out") == Some(&serde_json::json!(true))),
        "the timed-out operation-context flag must ride along: {:?}",
        result.diagnostics
    );
    assert_remediation(&result, "execution_timed_out");
    let event = AgentEvent::ToolExecutionEnd {
        tool_call_id: "timed-out-completed".into(),
        tool_name: "bash".into(),
        result: serde_json::json!(&result.content),
        details: result.details.clone(),
        is_error: result.is_error,
        truncated: result.truncated,
        diagnostics: result.diagnostics.clone(),
    };
    let event = event.redacted_for_public();
    for surface in [
        serde_json::to_string(&AgentSessionEvent::Agent {
            event: event.clone(),
        })
        .unwrap(),
        agent_event_to_value(&event).to_string(),
    ] {
        assert!(
            surface.contains("execution_timed_out"),
            "public event lost stable timeout identity: {surface}"
        );
    }
}

#[tokio::test]
async fn adapter_text_cannot_echo_request_canary_to_public_surfaces() {
    let canary = "PUBLIC_REQUEST_CANARY_7f43d1";
    let command = format!("echo {canary}");
    let result = routed_tool_result_with_command(
        canned("opi-sandbox", "mock-pkg", "public_text_echo"),
        &[("opi-sandbox", PermissionDecision::Allow)],
        &[("opi-sandbox", "mock-pkg")],
        ExecutionRunMode::Interactive,
        &command,
    )
    .await;
    assert!(
        !result.is_error,
        "redaction must preserve the successful outcome"
    );
    let event = AgentEvent::ToolExecutionEnd {
        tool_call_id: "public-text-echo".into(),
        tool_name: "bash".into(),
        result: serde_json::json!(&result.content),
        details: result.details.clone(),
        is_error: result.is_error,
        truncated: result.truncated,
        diagnostics: result.diagnostics.clone(),
    };
    let event = event.redacted_for_public();
    let details = result.details.as_ref().expect("effective contract details");
    let contract = serde_json::json!({
        "placement": details["placement"],
        "guarantee": details["guarantee"],
        "policy": details["policy"],
        "limitations": details["limitations"],
    });
    let surfaces = [
        contract.to_string(),
        serde_json::to_string(&result.diagnostics).unwrap(),
        serde_json::to_string(&AgentSessionEvent::Agent {
            event: event.clone(),
        })
        .unwrap(),
        agent_event_to_value(&event).to_string(),
    ];
    for surface in surfaces {
        assert!(
            !surface.contains(canary),
            "adapter-controlled text leaked the request canary: {surface}"
        );
    }
    assert_eq!(
        result.diagnostics[0].message,
        "backend reported a diagnostic"
    );
    assert!(details["placement"].as_str().unwrap().contains("host"));
    assert!(
        details["guarantee"]
            .as_str()
            .unwrap()
            .contains("supervised")
    );
}

/// SC16-14 "no degraded success state exists" (cancelled leg): a backend
/// reporting an in-band `Completed{cancelled: true, exit: Some(0)}` must surface
/// as an ERROR even though the exit code is clean — matching the timed_out leg
/// and the local backend's cancellation semantics.
#[tokio::test]
async fn cancelled_in_band_completed_is_not_a_success() {
    let result = routed_tool_result(
        canned("opi-sandbox", "mock-pkg", "completed_cancelled"),
        &[("opi-sandbox", PermissionDecision::Allow)],
        &[("opi-sandbox", "mock-pkg")],
        ExecutionRunMode::Interactive,
    )
    .await;
    let content_text = serde_json::to_string(&result.content).unwrap_or_default();
    assert!(
        result.is_error,
        "a cancelled completed frame must fail the turn (no degraded success): {content_text}"
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.context.get("cancelled") == Some(&serde_json::json!(true))),
        "the cancelled operation-context flag must ride along: {:?}",
        result.diagnostics
    );
}

#[tokio::test]
async fn wire_unavailable_preserves_selected_adapter_in_public_diagnostic() {
    let result = routed_tool_result(
        canned_with_args(
            "opi-sandbox",
            "mock-pkg",
            &["failed_pre_started", "unavailable"],
        ),
        &[("opi-sandbox", PermissionDecision::Allow)],
        &[("opi-sandbox", "mock-pkg")],
        ExecutionRunMode::Interactive,
    )
    .await;
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "adapter_unavailable")
        .expect("wire unavailable carries the stable diagnostic");
    assert_eq!(diagnostic.context["adapter_id"], "opi-sandbox");
    assert!(
        diagnostic.context["remediation"]
            .as_str()
            .unwrap_or_default()
            .contains("opi-sandbox")
    );

    let event = AgentEvent::ToolExecutionEnd {
        tool_call_id: "wire-unavailable".into(),
        tool_name: "bash".into(),
        result: serde_json::json!(&result.content),
        details: result.details.clone(),
        is_error: result.is_error,
        truncated: result.truncated,
        diagnostics: result.diagnostics.clone(),
    };
    for value in [
        serde_json::to_value(AgentSessionEvent::Agent {
            event: event.clone(),
        })
        .unwrap(),
        agent_event_to_value(&event),
    ] {
        assert!(value.to_string().contains("opi-sandbox"));
    }
}

#[tokio::test]
async fn routed_signal_is_public_and_signal_specific() {
    let result = routed_tool_result(
        canned("opi-sandbox", "mock-pkg", "signal_in_band"),
        &[("opi-sandbox", PermissionDecision::Allow)],
        &[("opi-sandbox", "mock-pkg")],
        ExecutionRunMode::Interactive,
    )
    .await;

    assert!(result.is_error);
    assert_eq!(result.details.as_ref().unwrap()["signal"], 9);
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "tool_execution_failed")
        .expect("signal termination carries the public operation diagnostic");
    assert_eq!(diagnostic.message, "command terminated by signal 9");
    assert_eq!(diagnostic.context["signal"], 9);
    match &result.content[0] {
        opi_ai::message::OutputContent::Text { text } => {
            assert_eq!(text, "command terminated by signal 9")
        }
        other => panic!("expected text output, got {other:?}"),
    }
}

/// SC16-14: `permission_required` (headless ask on a selected external) lifts its
/// stable code into `ToolResult.diagnostics` through the production path in a
/// headless (broker-less) run mode. (`policy_denied` is pinned separately in
/// [`policy_denied_surfaces_via_production_path`].)
#[tokio::test]
async fn permission_required_surfaces_via_production_path() {
    // Headless `ask` -> permission_required, no broker.
    let ask_result = routed_tool_result(
        canned("opi-sandbox", "mock-pkg", "happy_path"),
        &[("opi-sandbox", PermissionDecision::Ask)],
        &[("opi-sandbox", "mock-pkg")],
        ExecutionRunMode::NonInteractive,
    )
    .await;
    assert!(
        ask_result.is_error,
        "headless ask must fail the turn (no degraded success)"
    );
    assert!(
        ask_result
            .diagnostics
            .iter()
            .any(|d| d.code == "permission_required"),
        "headless ask must surface permission_required: {:?}",
        ask_result.diagnostics
    );
    assert_remediation(&ask_result, "permission_required");
}

/// SC16-14: `policy_denied` ALSO lifts into `ToolResult.diagnostics` through the
/// production path when the selected backend is denied by the permission policy
/// (the routed exec gates the selection at call time). This pins the policy_denied
/// leg the test name promises.
#[tokio::test]
async fn policy_denied_surfaces_via_production_path() {
    let denied = routed_tool_result(
        canned("opi-sandbox", "mock-pkg", "happy_path"),
        &[("opi-sandbox", PermissionDecision::Deny)],
        &[("opi-sandbox", "mock-pkg")],
        ExecutionRunMode::Interactive,
    )
    .await;
    assert!(
        denied.is_error,
        "a denied selected backend must fail the turn (no degraded success)"
    );
    assert!(
        denied.diagnostics.iter().any(|d| d.code == "policy_denied"),
        "a denied selected backend must surface policy_denied: {:?}",
        denied.diagnostics
    );
    assert_remediation(&denied, "policy_denied");
}

/// SC16-14 (DoD "fixed/rules/model ... separation"): the `rules` strategy's
/// first-match backend runs through the production chokepoint, and a denied
/// selected backend fails the turn closed WITHOUT falling through to the
/// catch-all. Pins the rules leg at `CodingHarness::build_tools` ->
/// `BashTool::execute`, symmetric with the fixed and model legs.
#[tokio::test]
async fn rules_strategy_runs_selected_backend_and_fails_denied_closed() {
    use opi_coding_agent::config::ExecutionRule;

    let ws = tempfile::tempdir().unwrap();
    let rules_wiring = ExecutionWiring {
        config: ExecutionConfig {
            strategy: ExecutionStrategy::Rules,
            rules: vec![
                ExecutionRule {
                    modes: Some(vec![ExecutionRunMode::Interactive]),
                    backend: "opi-sandbox".to_string(),
                },
                ExecutionRule {
                    modes: None, // catch-all -> local
                    backend: LOCAL_ADAPTER_ID.to_string(),
                },
            ],
            ..ExecutionConfig::default()
        },
        enabled: vec![EnabledIdentity {
            adapter_id: "opi-sandbox".to_string(),
            package_name: "mock-pkg".to_string(),
        }],
        policy: PermissionPolicy::from_map(
            [("opi-sandbox".to_string(), PermissionDecision::Allow)]
                .into_iter()
                .collect(),
        ),
        store: Arc::new(MockSource {
            contribution: canned("opi-sandbox", "mock-pkg", "happy_path"),
        }),
        mode: ExecutionRunMode::Interactive,
        host_target: host_target_triple().to_string(),
        host_opi_version: host_opi_version().to_string(),
        manager: Arc::new(PermissionManager::new()),
        broker: None,
    };
    let tool_config =
        ToolRuntimeConfig::resolve(RunMode::Interactive, true, ToolSelection::Default)
            .expect("interactive tool config");
    let (mut tools, diags) = CodingHarness::build_tools(ws.path(), &tool_config, &rules_wiring);
    assert!(diags.is_empty(), "rules allow must not warn: {diags:?}");
    let bash = tools
        .iter_mut()
        .find(|t| t.definition().name == "bash")
        .expect("routed bash tool present");
    let result = bash
        .execute(
            "rules-first-match",
            serde_json::json!({"command": "echo hi", "backend": "opi-sandbox", "timeout_secs": 5}),
            CancellationToken::new(),
            None,
        )
        .await
        .expect("bash tool executes");
    let result_text = serde_json::to_string(&result.content).expect("outputs serialize");
    assert!(
        !result.is_error,
        "rules first-match backend must run: {result_text}"
    );
    assert!(
        result_text.contains("hello"),
        "the rules first-match backend (mock peer happy_path) ran: {result_text}"
    );

    // Deny the first-match backend; the turn must fail closed with policy_denied
    // and NOT fall through to the catch-all local backend (which would succeed).
    let denied_wiring = ExecutionWiring {
        policy: PermissionPolicy::from_map(
            [("opi-sandbox".to_string(), PermissionDecision::Deny)]
                .into_iter()
                .collect(),
        ),
        ..rules_wiring
    };
    let (mut tools, _diags) = CodingHarness::build_tools(ws.path(), &tool_config, &denied_wiring);
    let bash = tools
        .iter_mut()
        .find(|t| t.definition().name == "bash")
        .expect("routed bash tool present");
    let result = bash
        .execute(
            "rules-denied",
            serde_json::json!({"command": "echo hi", "backend": "opi-sandbox", "timeout_secs": 5}),
            CancellationToken::new(),
            None,
        )
        .await
        .expect("bash tool executes");
    assert!(result.is_error, "denied rules backend must fail the turn");
    assert!(
        result.diagnostics.iter().any(|d| d.code == "policy_denied"),
        "rules denied backend must surface policy_denied (no catch-all fallthrough): {:?}",
        result.diagnostics
    );
}

/// SC16-14: `no_eligible_adapter` surfaces through the production path when the
/// fixed strategy names a backend that is NOT in the eligibility set (the routed
/// exec resolves the selection at call time, not at build).
#[tokio::test]
async fn no_eligible_adapter_surfaces_via_production_path() {
    let ws = tempfile::tempdir().unwrap();
    let wiring = ExecutionWiring {
        config: ExecutionConfig {
            strategy: ExecutionStrategy::Fixed,
            backend: "missing-adapter".to_string(),
            ..ExecutionConfig::default()
        },
        enabled: Vec::new(),
        policy: PermissionPolicy::from_map(
            [("opi-sandbox".to_string(), PermissionDecision::Allow)]
                .into_iter()
                .collect(),
        ),
        store: Arc::new(PanicSource),
        mode: ExecutionRunMode::Interactive,
        host_target: host_target_triple().to_string(),
        host_opi_version: host_opi_version().to_string(),
        manager: Arc::new(PermissionManager::new()),
        broker: None,
    };
    let tool_config =
        ToolRuntimeConfig::resolve(RunMode::Interactive, true, ToolSelection::Default)
            .expect("interactive tool config");
    let (mut tools, _diags) = CodingHarness::build_tools(ws.path(), &tool_config, &wiring);
    let bash = tools
        .iter_mut()
        .find(|t| t.definition().name == "bash")
        .expect("routed bash tool present");
    let result = bash
        .execute(
            "no-eligible",
            serde_json::json!({"command": "echo hi", "backend": "missing-adapter", "timeout_secs": 5}),
            CancellationToken::new(),
            None,
        )
        .await
        .expect("bash tool executes");
    assert!(result.is_error, "no eligible adapter must fail the turn");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "no_eligible_adapter"),
        "no_eligible_adapter must reach ToolResult.diagnostics: {:?}",
        result.diagnostics
    );
    assert_remediation(&result, "no_eligible_adapter");
}

/// SC16-14: `adapter_not_selected` surfaces through the production path when the
/// model strategy omits the required `backend` field (a selection-time error,
/// never a silent local fallback).
#[tokio::test]
async fn adapter_not_selected_surfaces_via_production_path() {
    let ws = tempfile::tempdir().unwrap();
    let wiring = ExecutionWiring {
        config: ExecutionConfig {
            strategy: ExecutionStrategy::Model,
            ..ExecutionConfig::default()
        },
        enabled: vec![EnabledIdentity {
            adapter_id: "opi-sandbox".to_string(),
            package_name: "mock-pkg".to_string(),
        }],
        policy: PermissionPolicy::from_map(
            [("opi-sandbox".to_string(), PermissionDecision::Allow)]
                .into_iter()
                .collect(),
        ),
        store: Arc::new(PanicSource),
        mode: ExecutionRunMode::Interactive,
        host_target: host_target_triple().to_string(),
        host_opi_version: host_opi_version().to_string(),
        manager: Arc::new(PermissionManager::new()),
        broker: None,
    };
    let tool_config =
        ToolRuntimeConfig::resolve(RunMode::Interactive, true, ToolSelection::Default)
            .expect("interactive tool config");
    let (mut tools, _diags) = CodingHarness::build_tools(ws.path(), &tool_config, &wiring);
    let bash = tools
        .iter_mut()
        .find(|t| t.definition().name == "bash")
        .expect("routed bash tool present");
    // The model omits `backend` entirely -> resolve_selection yields
    // adapter_not_selected at exec time.
    let result = bash
        .execute(
            "no-backend",
            serde_json::json!({"command": "echo hi", "timeout_secs": 5}),
            CancellationToken::new(),
            None,
        )
        .await
        .expect("bash tool executes");
    assert!(result.is_error, "missing model backend must fail the turn");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "adapter_not_selected"),
        "adapter_not_selected must reach ToolResult.diagnostics: {:?}",
        result.diagnostics
    );
    assert_remediation(&result, "adapter_not_selected");
}

#[tokio::test]
async fn hostile_model_backend_is_redacted_on_all_public_surfaces() {
    let canary = r#"C:\private\HOSTILE sk-proj-012345678901234567890123456789"#;
    let ws = tempfile::tempdir().unwrap();
    let wiring = ExecutionWiring {
        config: ExecutionConfig {
            strategy: ExecutionStrategy::Model,
            ..ExecutionConfig::default()
        },
        enabled: vec![EnabledIdentity {
            adapter_id: "opi-sandbox".to_string(),
            package_name: "mock-pkg".to_string(),
        }],
        policy: PermissionPolicy::from_map(
            [("opi-sandbox".to_string(), PermissionDecision::Allow)]
                .into_iter()
                .collect(),
        ),
        store: Arc::new(PanicSource),
        mode: ExecutionRunMode::Interactive,
        host_target: host_target_triple().to_string(),
        host_opi_version: host_opi_version().to_string(),
        manager: Arc::new(PermissionManager::new()),
        broker: None,
    };
    let tool_config =
        ToolRuntimeConfig::resolve(RunMode::Interactive, true, ToolSelection::Default)
            .expect("interactive tool config");
    let (tools, _diags) = CodingHarness::build_tools(ws.path(), &tool_config, &wiring);
    let bash = tools
        .into_iter()
        .find(|tool| tool.definition().name == "bash")
        .expect("routed bash tool present");
    let result = bash
        .execute(
            "hostile-backend",
            serde_json::json!({"command": "echo hi", "backend": canary, "timeout_secs": 5}),
            CancellationToken::new(),
            None,
        )
        .await
        .expect("bash tool executes");
    assert!(result.is_error);

    let event = AgentEvent::ToolExecutionEnd {
        tool_call_id: "hostile-backend".into(),
        tool_name: "bash".into(),
        result: serde_json::json!(&result.content),
        details: result.details.clone(),
        is_error: result.is_error,
        truncated: result.truncated,
        diagnostics: result.diagnostics.clone(),
    };
    let public_surfaces = [
        serde_json::to_string(&result.content).unwrap(),
        serde_json::to_string(&result.diagnostics).unwrap(),
        serde_json::to_string(&AgentSessionEvent::Agent {
            event: event.clone(),
        })
        .unwrap(),
        agent_event_to_value(&event).to_string(),
    ];
    for surface in public_surfaces {
        assert!(
            !surface.contains(canary),
            "hostile backend leaked: {surface}"
        );
        assert!(
            surface.contains("<unavailable>") || surface.contains("adapter_not_selected"),
            "public surface lost safe failure context: {surface}"
        );
    }
}

#[tokio::test]
async fn agent_loop_redacts_hostile_backend_rejected_by_model_schema() {
    let canary = "HOSTILE_BACKEND_VALIDATION_CANARY";
    let hostile_backend = format!(r#"C:\private\{canary} sk-proj-012345678901234567890123456789"#);
    let ws = tempfile::tempdir().unwrap();
    let wiring = ExecutionWiring {
        config: ExecutionConfig {
            strategy: ExecutionStrategy::Model,
            ..ExecutionConfig::default()
        },
        enabled: vec![EnabledIdentity {
            adapter_id: "opi-sandbox".to_string(),
            package_name: "mock-pkg".to_string(),
        }],
        policy: PermissionPolicy::from_map(
            [("opi-sandbox".to_string(), PermissionDecision::Allow)]
                .into_iter()
                .collect(),
        ),
        store: Arc::new(PanicSource),
        mode: ExecutionRunMode::Interactive,
        host_target: host_target_triple().to_string(),
        host_opi_version: host_opi_version().to_string(),
        manager: Arc::new(PermissionManager::new()),
        broker: None,
    };
    let tool_config =
        ToolRuntimeConfig::resolve(RunMode::Interactive, true, ToolSelection::Default)
            .expect("interactive tool config");
    let (tools, diagnostics) = CodingHarness::build_tools(ws.path(), &tool_config, &wiring);
    assert!(diagnostics.is_empty());
    let bash_schema = tools
        .iter()
        .find(|tool| tool.definition().name == "bash")
        .expect("model-routed bash tool")
        .definition()
        .input_schema;
    assert!(bash_schema["properties"]["backend"]["oneOf"].is_array());

    let args = serde_json::json!({
        "command": "echo validation must reject before execution",
        "backend": hostile_backend,
    })
    .to_string();
    let provider = MockProvider::new(
        "mock",
        vec![
            test_support::tool_call_response("hostile-validation", "bash", &args),
            test_support::text_response("done"),
        ],
    );
    let events = Arc::new(Mutex::new(Vec::new()));
    let event_sink = Box::new({
        let events = events.clone();
        move |event: AgentEvent| {
            events.lock().unwrap().push(agent_event_to_value(&event));
        }
    });
    let model_spec = "mock:mock-model".to_string();
    let context = AgentLoopContext {
        collection: Arc::new(single_route_collection(Box::new(provider))),
        registry: std::sync::Arc::new(
            opi_agent::authority::ToolRegistry::from_tools(common::registrations_from(tools))
                .expect("distinct test tool names"),
        ),
        authorizer: Some(common::permissive_authorizer()),
        evidence_health: opi_agent::evidence::EvidenceHealth::healthy(),
        state: NextTurnState::new(
            Vec::new(),
            ModelSelection::parse_spec(&model_spec).unwrap(),
            InferenceConfig::default(),
        ),
        system: None,
        steering_queue: None,
        follow_up_queue: None,
        diagnostic_sink: None,
        trace: None,
        session_id: None,
    };

    let messages = opi_agent::agent_loop(
        context,
        AgentLoopConfig::default(),
        &CodingAgentHooks,
        event_sink,
        CancellationToken::new(),
    )
    .await
    .expect("schema rejection is a normal tool result");
    let public_tool_results = messages
        .context
        .iter()
        .filter_map(|message| match message {
            AgentMessage::Llm(opi_ai::message::Message::ToolResult(result)) => Some(result),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(public_tool_results.len(), 1);
    let public_tool_result_json = serde_json::to_string(&public_tool_results).unwrap();
    assert!(public_tool_result_json.contains("schema validation failed"));

    for surface in [
        public_tool_result_json,
        serde_json::to_string(&*events.lock().unwrap()).unwrap(),
    ] {
        assert!(
            !surface.contains(canary),
            "hostile backend leaked: {surface}"
        );
    }
}

/// SC16-14: activation-failure codes (`package_not_installed`) reach
/// `ToolResult.diagnostics` through the production path when the pre-spawn
/// revalidation fails closed.
#[tokio::test]
async fn package_not_installed_surfaces_via_production_path() {
    struct NotInstalledSource;
    impl IdentitySource for NotInstalledSource {
        fn activate(
            &self,
            name: &str,
            _: &str,
            _: &str,
        ) -> Result<ActivatedContribution, ActivationError> {
            Err(ActivationError::NotInstalled(name.to_string()))
        }
    }
    let ws = tempfile::tempdir().unwrap();
    let wiring = ExecutionWiring {
        config: ExecutionConfig {
            strategy: ExecutionStrategy::Fixed,
            backend: "opi-sandbox".to_string(),
            ..ExecutionConfig::default()
        },
        enabled: vec![EnabledIdentity {
            adapter_id: "opi-sandbox".to_string(),
            package_name: "mock-pkg".to_string(),
        }],
        policy: PermissionPolicy::from_map(
            [("opi-sandbox".to_string(), PermissionDecision::Allow)]
                .into_iter()
                .collect(),
        ),
        store: Arc::new(NotInstalledSource),
        mode: ExecutionRunMode::Interactive,
        host_target: host_target_triple().to_string(),
        host_opi_version: host_opi_version().to_string(),
        manager: Arc::new(PermissionManager::new()),
        broker: None,
    };
    let tool_config =
        ToolRuntimeConfig::resolve(RunMode::Interactive, true, ToolSelection::Default)
            .expect("interactive tool config");
    let (tools, _diags) = CodingHarness::build_tools(ws.path(), &tool_config, &wiring);
    let bash = tools
        .into_iter()
        .find(|t| t.definition().name == "bash")
        .expect("routed bash tool present");
    let result = bash
        .execute(
            "not-installed",
            serde_json::json!({"command": "echo hi", "backend": "opi-sandbox", "timeout_secs": 5}),
            CancellationToken::new(),
            None,
        )
        .await
        .expect("bash tool executes");
    assert!(result.is_error, "not-installed must fail the turn");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "package_not_installed"),
        "package_not_installed must reach ToolResult.diagnostics: {:?}",
        result.diagnostics
    );
    assert_remediation(&result, "package_not_installed");
}

/// SC16-14: `adapter_unavailable` is produced at the ACTIVATION layer (an
/// adapter-id collision / store error surfaces as `ActivationError::Store`, which
/// maps to `adapter_unavailable`) and lifts into `ToolResult.diagnostics` through
/// the production path.
#[tokio::test]
async fn adapter_unavailable_surfaces_via_production_path() {
    struct StoreErrorSource;
    impl IdentitySource for StoreErrorSource {
        fn activate(
            &self,
            _: &str,
            _: &str,
            _: &str,
        ) -> Result<ActivatedContribution, ActivationError> {
            Err(ActivationError::Store(
                opi_coding_agent::package_store::PackageStoreError::Package(
                    "adapter-unavailable fixture".into(),
                ),
            ))
        }
    }
    let ws = tempfile::tempdir().unwrap();
    let wiring = ExecutionWiring {
        config: ExecutionConfig {
            strategy: ExecutionStrategy::Fixed,
            backend: "opi-sandbox".to_string(),
            ..ExecutionConfig::default()
        },
        enabled: vec![EnabledIdentity {
            adapter_id: "opi-sandbox".to_string(),
            package_name: "mock-pkg".to_string(),
        }],
        policy: PermissionPolicy::from_map(
            [("opi-sandbox".to_string(), PermissionDecision::Allow)]
                .into_iter()
                .collect(),
        ),
        store: Arc::new(StoreErrorSource),
        mode: ExecutionRunMode::Interactive,
        host_target: host_target_triple().to_string(),
        host_opi_version: host_opi_version().to_string(),
        manager: Arc::new(PermissionManager::new()),
        broker: None,
    };
    let tool_config =
        ToolRuntimeConfig::resolve(RunMode::Interactive, true, ToolSelection::Default)
            .expect("interactive tool config");
    let (tools, _diags) = CodingHarness::build_tools(ws.path(), &tool_config, &wiring);
    let bash = tools
        .into_iter()
        .find(|t| t.definition().name == "bash")
        .expect("routed bash tool present");
    let result = bash
        .execute(
            "adapter-unavailable",
            serde_json::json!({"command": "echo hi", "backend": "opi-sandbox", "timeout_secs": 5}),
            CancellationToken::new(),
            None,
        )
        .await
        .expect("bash tool executes");
    assert!(result.is_error, "activation failure must fail the turn");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "adapter_unavailable"),
        "adapter_unavailable must reach ToolResult.diagnostics: {:?}",
        result.diagnostics
    );
    assert_remediation(&result, "adapter_unavailable");
}
