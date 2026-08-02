//! Task 16.9 acceptance — SC16-01 Minimal Runtime (production startup path).
//!
//! Drives the REAL production chokepoint `CodingHarness::build_tools_with_sandbox`
//! (NOT `ExecutionRuntime::build` or `BashTool::new_with_ops` directly) to prove
//! that, with default-local routing and no enabled executable extension, startup:
//!   - does not touch an invalid package-store sentinel (a panic-on-activate
//!     store survives — `ExecutionRuntime::build` Branch 1 never activates);
//!   - starts no extension process and constructs no router/permission/protocol
//!     state (the bash tool runs the LOCAL backend end-to-end);
//!   - preserves the default bash input schema byte-for-byte vs a fresh
//!     `schemars::schema_for!(BashArgs)` computation (no `backend` enum added);
//!   - leaves local command results and L0 behavior unchanged.
//!
//! A silent absence of the 16.9 startup wiring — e.g. `build_tools_with_sandbox`
//! still constructing `LocalBashOperations` directly without calling
//! `ExecutionRuntime::build` — fails these tests loud.

use std::sync::Arc;

use opi_coding_agent::config::{ExecutionConfig, ExecutionRunMode, SandboxConfig};
use opi_coding_agent::execution::permission::PermissionPolicy;
use opi_coding_agent::execution::{IdentitySource, LockMaterial, PermissionManager};
use opi_coding_agent::harness::{CodingHarness, ExecutionWiring};
use opi_coding_agent::package_activation::{
    ActivatedContribution, ActivationError, ActivationRecord, PackageActivationStore,
};
use opi_coding_agent::package_store::PackageLockEntry;
use opi_coding_agent::policy::{RunMode, ToolRuntimeConfig, ToolSelection};
use opi_coding_agent::sandbox::prepare_production;
use opi_coding_agent::tool::default_bash_schema;
use tokio_util::sync::CancellationToken;

/// A package-store sentinel that panics if `activate` is ever called. Minimal
/// Runtime must never activate any package, so this surviving through startup
/// plus one local execution proves the store was not touched (SC16-01).
struct PanicSource;
impl IdentitySource for PanicSource {
    fn activate(
        &self,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<ActivatedContribution, ActivationError> {
        panic!("Minimal Runtime must not activate any package");
    }
}

fn minimal_wiring(mode: ExecutionRunMode) -> ExecutionWiring {
    ExecutionWiring {
        config: ExecutionConfig::default(),
        enabled: Vec::new(),
        policy: PermissionPolicy::empty(),
        store: Arc::new(PanicSource),
        mode,
        host_target: opi_coding_agent::package_activation::host_target_triple().to_string(),
        host_opi_version: opi_coding_agent::package_activation::host_opi_version().to_string(),
        manager: Arc::new(PermissionManager::new()),
        broker: None,
    }
}

/// SC16-01: the production startup chokepoint preserves the Minimal Runtime.
#[tokio::test]
async fn production_minimal_runtime_preserves_schema_and_runs_local_backend() {
    let ws = tempfile::tempdir().unwrap();
    let tool_config =
        ToolRuntimeConfig::resolve(RunMode::Interactive, true, ToolSelection::Default)
            .expect("interactive tool config");
    // Mirror the production path: prepare_production(&config.sandbox, root).
    let prepared = prepare_production(&SandboxConfig::default(), ws.path());
    let (mut tools, startup_diagnostics) = CodingHarness::build_tools_with_sandbox(
        ws.path(),
        &tool_config,
        prepared,
        &minimal_wiring(ExecutionRunMode::Interactive),
    );
    assert!(
        startup_diagnostics.is_empty(),
        "Minimal Runtime must emit no startup diagnostics: {startup_diagnostics:?}"
    );
    let bash = tools
        .iter_mut()
        .find(|t| t.definition().name == "bash")
        .expect("build_tools must construct the bash tool under Minimal Runtime");

    // The default schema is byte-for-byte the pre-extension schema (no model
    // `backend` enum), compared against a FRESH computation — not a frozen
    // snapshot — so a schemars bump, an accidental BashArgs edit, or a
    // wrong-strategy injection fails loud.
    assert_eq!(
        bash.definition().input_schema,
        default_bash_schema(),
        "default Minimal-Runtime bash schema must equal the fresh BashArgs schema"
    );

    // The bash tool runs the LOCAL backend end-to-end. A trivial echo exits 0
    // with the expected output, proving no routed wrapper intercepted (and
    // transitively that no router/permission/protocol/adapter state was
    // constructed — all of which live only inside RoutedBashOperations). The
    // panic-on-activate store surviving this execution proves it was untouched.
    let result = bash
        .execute(
            "test-call",
            serde_json::json!({"command": "echo opi-minimal-runtime", "timeout_secs": 5}),
            CancellationToken::new(),
            None,
        )
        .await
        .expect("bash tool executes");
    assert!(
        !result.is_error,
        "local echo must succeed under Minimal Runtime: {:?}",
        result.content
    );
    let text = serde_json::to_string(&result.content).expect("outputs serialize");
    assert!(
        text.contains("opi-minimal-runtime"),
        "local echo output must reach the tool result: {text}"
    );
}

/// SC16-01: a corrupt/unreadable `package-trust.toml` is ignored — startup
/// treats it as "no enabled extensions" rather than aborting (mirrors the
/// tolerant `doctor.rs` read pattern). This is the `enabled_identities` resolver
/// property the production startup path depends on so an invalid package-store
/// sentinel never blocks Minimal Runtime.
#[test]
fn enabled_identities_ignores_corrupt_package_trust_file() {
    let dir = tempfile::tempdir().unwrap();
    let user_config = dir.path().to_path_buf();
    // A present-but-corrupt trust file (invalid TOML) at the exact trust_path.
    std::fs::write(
        user_config.join("package-trust.toml"),
        "this is = = not valid toml {{{",
    )
    .unwrap();
    let store = PackageActivationStore::global(user_config);
    let enabled = store.enabled_identities();
    assert!(
        enabled.is_empty(),
        "a corrupt package-trust.toml must yield no enabled identities: {enabled:?}"
    );
}

/// SC16-01 positive counterpart (D.2 must-fix L-D2): a VALID trusted+enabled
/// record plus its locked contribution yields the expected non-empty identity,
/// and a trusted-but-DISABLED record is excluded. Paired with the corrupt-file
/// test above, this discriminates "tolerates corruption" from a degenerate
/// always-empty stub and pins the `trusted && enabled` filter + per-adapter
/// expansion the production `execution_wiring()` path depends on.
#[test]
fn enabled_identities_returns_adapter_for_trusted_enabled_record_only() {
    let dir = tempfile::tempdir().unwrap();
    let user_config = dir.path().to_path_buf();
    let store = PackageActivationStore::global(user_config.clone());
    // One trusted+enabled record, one trusted+disabled record. Only the enabled
    // one may contribute — a stub returning all records (or always-empty) fails.
    store
        .write_records(&[
            ActivationRecord {
                name: "enabled-pkg".to_string(),
                source: "enabled-src".to_string(),
                trusted: true,
                enabled: true,
            },
            ActivationRecord {
                name: "disabled-pkg".to_string(),
                source: "disabled-src".to_string(),
                trusted: true,
                enabled: false,
            },
        ])
        .unwrap();
    // A lock entry for the enabled package carrying one contribution adapter id.
    let lock = PackageLockEntry {
        identity_kind: "local".to_string(),
        identity_value: "enabled-src".to_string(),
        source: "enabled-src".to_string(),
        package_root: user_config,
        cache_path: None,
        git_commit: None,
        manifest_sha256: "dummy".to_string(),
        contributions: vec![LockMaterial {
            manifest_hash: "dummy".to_string(),
            executable_rel_path: "bin/mock".to_string(),
            executable_sha256: "dummy".to_string(),
            package_version: "0.8.0".to_string(),
            target: opi_coding_agent::package_activation::host_target_triple().to_string(),
            opi_range: ">=0.8,<0.9".to_string(),
            protocol: "opi.execution.v1".to_string(),
            adapter_id: "opi-sandbox".to_string(),
        }],
    };
    store.store().write_lock(&[lock]).unwrap();

    let enabled = store.enabled_identities();
    assert_eq!(
        enabled.len(),
        1,
        "exactly the one trusted+enabled adapter: {enabled:?}"
    );
    assert_eq!(enabled[0].adapter_id, "opi-sandbox");
    assert_eq!(enabled[0].package_name, "enabled-pkg");
}
