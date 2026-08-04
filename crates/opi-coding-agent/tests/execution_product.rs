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

use std::path::PathBuf;
use std::sync::Arc;

use opi_coding_agent::config::{
    ExecutionConfig, ExecutionRunMode, ExecutionStrategy, OpiConfig, PermissionDecision,
};
use opi_coding_agent::execution::ValidatedExecutableContribution;
use opi_coding_agent::execution::permission::PermissionPolicy;
use opi_coding_agent::execution::{
    EnabledIdentity, IdentitySource, LockMaterial, PermissionManager,
};
use opi_coding_agent::harness::{CodingHarness, ExecutionWiring};
use opi_coding_agent::package_activation::{
    ActivatedContribution, ActivationError, host_opi_version, host_target_triple,
};
use opi_coding_agent::package_store::PackageLockEntry;
use opi_coding_agent::policy::{RunMode, ToolRuntimeConfig, ToolSelection};
use opi_protocol::execution::v1::WIRE_IDENTITY;
use tokio_util::sync::CancellationToken;

const HOST_TARGET: &str = if cfg!(windows) {
    "x86_64-pc-windows-msvc"
} else if cfg!(target_os = "linux") {
    "x86_64-unknown-linux-gnu"
} else {
    "x86_64-apple-darwin"
};
const HOST_OPI_VERSION: &str = "0.8.0";

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
        package_version: HOST_OPI_VERSION.to_string(),
        target: HOST_TARGET.to_string(),
        opi_range: ">=0.8,<0.9".to_string(),
        protocol: WIRE_IDENTITY.to_string(),
        adapter_id: adapter_id.to_string(),
    }
}

/// A canned activated contribution that launches the mock peer in `mode` (the
/// peer selects behavior by its first CLI arg).
fn canned(adapter_id: &str, pkg: &str, mode: &str) -> ActivatedContribution {
    ActivatedContribution {
        name: pkg.to_string(),
        source: pkg.to_string(),
        validated: vec![ValidatedExecutableContribution {
            capability: "command.execute".to_string(),
            id: adapter_id.to_string(),
            transport: "process-jsonl".to_string(),
            command: mock_bin(),
            args: vec![mode.to_string()],
            protocol: WIRE_IDENTITY.to_string(),
            target: HOST_TARGET.to_string(),
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
            .any(|d| d.code == "protocol_violation"),
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
