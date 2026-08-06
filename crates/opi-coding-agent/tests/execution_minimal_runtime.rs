//! Task 16.9 acceptance — SC16-01 Minimal Runtime runtime seam.
//!
//! Drives `CodingHarness::build_tools` and `ExecutionRuntime::build` to prove
//! that fixed-local effective allow:
//!   - does not touch an invalid package-store sentinel (a panic-on-activate
//!     store survives — `ExecutionRuntime::build` Branch 1 never activates);
//!   - ignores enabled-but-unselected external identities and constructs no
//!     routed adapter/protocol state (the bash tool runs local end-to-end);
//!   - preserves the default bash input schema byte-for-byte vs a fresh
//!     `schemars::schema_for!(BashArgs)` computation (no `backend` enum added);
//!   - leaves local command results and L0 behavior unchanged.
//!
//! The private harness unit tests separately drive the real constructor with a
//! panic-on-open activation-store probe and construction counters, because that
//! early classifier intentionally bypasses `ExecutionWiring` altogether.

use std::sync::Arc;

use opi_coding_agent::config::{ExecutionConfig, ExecutionRunMode};
use opi_coding_agent::execution::permission::PermissionPolicy;
use opi_coding_agent::execution::{
    EnabledIdentity, ExecutionRuntime, IdentitySource, LockMaterial, PermissionManager,
};
use opi_coding_agent::harness::{CodingHarness, ExecutionWiring};
use opi_coding_agent::package_activation::{
    ActivatedContribution, ActivationError, ActivationRecord, PackageActivationStore,
};
use opi_coding_agent::package_store::PackageLockEntry;
use opi_coding_agent::policy::{RunMode, ToolRuntimeConfig, ToolSelection};
use opi_coding_agent::tool::{BashOperations, LocalBashOperations, default_bash_schema};
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

#[test]
fn fixed_local_effective_allow_is_direct_even_with_enabled_external_identity() {
    let config = ExecutionConfig::default();
    let policy = PermissionPolicy::from_map(config.permissions.clone());
    let local_ops: Arc<dyn BashOperations> = Arc::new(LocalBashOperations::new());
    let selected = ExecutionRuntime::build(
        &config,
        ExecutionRunMode::Interactive,
        &[EnabledIdentity {
            adapter_id: "opi-sandbox".to_string(),
            package_name: "enabled-but-unselected".to_string(),
        }],
        &policy,
        Arc::new(PanicSource),
        Arc::clone(&local_ops),
        std::path::Path::new("."),
        opi_coding_agent::package_activation::host_target_triple(),
        opi_coding_agent::package_activation::host_opi_version(),
        Arc::new(PermissionManager::new()),
        None,
    )
    .expect("fixed local allow must select the direct local backend");

    assert!(
        Arc::ptr_eq(&selected, &local_ops),
        "resolved fixed-local allow must not construct the routed runtime"
    );
}

/// SC16-01: the tool-assembly seam preserves the Minimal Runtime.
#[tokio::test]
async fn tool_assembly_minimal_runtime_preserves_schema_and_runs_local_backend() {
    let ws = tempfile::tempdir().unwrap();
    let tool_config =
        ToolRuntimeConfig::resolve(RunMode::Interactive, true, ToolSelection::Default)
            .expect("interactive tool config");
    let (mut tools, startup_diagnostics) = CodingHarness::build_tools(
        ws.path(),
        &tool_config,
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
    // with the expected output, proving no routed wrapper intercepted. The
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

/// Independent activation-store tolerance coverage: a corrupt/unreadable
/// `package-trust.toml` is treated as "no enabled extensions" rather than
/// aborting (mirrors the tolerant `doctor.rs` read pattern). Production Minimal
/// Runtime does not depend on this fallback because it never opens the store.
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
/// expansion the production routed `execution_wiring()` path depends on.
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
