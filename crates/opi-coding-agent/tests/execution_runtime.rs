//! Task 16.8 acceptance: the production `ExecutionRuntime::build` assembly and
//! its routed backends (`RoutedBashOperations`, `ProcessCommandAdapter`).
//!
//! Most of the suite is feature-free: it proves the two assembly branches with
//! injected sentinels (a panic-on-call `IdentitySource` for branch 1, a recording
//! `BashOperations` for the no-fallback proof) and the routing/failure-code
//! surface. The mock-peer-driven `ProcessCommandAdapter` end-to-end test is
//! gated on `execution-backend-test-fixture` (it builds the 16.7 mock peer):
//!
//! ```text
//! cargo test -p opi-coding-agent --features execution-backend-test-fixture \
//!   --test execution_backend_mock --no-run   # build the mock peer first
//! cargo test -p opi-coding-agent --features execution-backend-test-fixture \
//!   --test execution_runtime
//! ```

use std::collections::BTreeMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use opi_agent::tool::{Tool, ToolResult};
use opi_coding_agent::config::{
    ExecutionConfig, ExecutionRule, ExecutionRunMode, ExecutionStrategy, PermissionDecision,
};
use opi_coding_agent::execution::permission::PermissionPolicy;
use opi_coding_agent::execution::{
    EnabledIdentity, ExecutionRuntime, IdentitySource, LOCAL_ADAPTER_ID, PermissionManager,
};
use opi_coding_agent::package_activation::{ActivatedContribution, ActivationError};
use opi_coding_agent::tool::{
    BashOpError, BashOperationContext, BashOperations, BashRequest, BashResult, BashTool,
};
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Mock seams
// ---------------------------------------------------------------------------

/// A `BashOperations` sentinel that records every `exec` call's command and
/// returns a canned in-band result. Used to prove the local backend is (or is
/// not) reached — never spawns a real process.
struct RecordingOps {
    calls: Mutex<Vec<String>>,
}

impl RecordingOps {
    fn new() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
        }
    }
    fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
}

impl BashOperations for RecordingOps {
    fn exec(
        &self,
        request: BashRequest,
    ) -> Pin<Box<dyn Future<Output = Result<BashResult, BashOpError>> + Send>> {
        self.calls.lock().unwrap().push(request.command.clone());
        Box::pin(async move {
            Ok(BashResult {
                stdout: b"local\n".to_vec(),
                stderr: Vec::new(),
                context: BashOperationContext::local(Some(0), None),
                diagnostics: Vec::new(),
            })
        })
    }
}

/// A `BashOperations` sentinel that always fails with a backend error — used to
/// prove a `local` selection under routing surfaces its failure (no magic).
struct AlwaysFailOps;
impl BashOperations for AlwaysFailOps {
    fn exec(
        &self,
        _request: BashRequest,
    ) -> Pin<Box<dyn Future<Output = Result<BashResult, BashOpError>> + Send>> {
        Box::pin(async move {
            Err(BashOpError::Other {
                message: "local sentinel failure".into(),
            })
        })
    }
}

/// What a `MockSource` returns from `activate`.
#[allow(dead_code)]
enum MockActivate {
    Untrusted,
    Disabled,
    NotInstalled,
    Canned(Box<ActivatedContribution>),
}

/// Injectable `IdentitySource` mock. Branch-1 tests use `PanicSource` instead
/// (below) to prove the store is never touched.
struct MockSource {
    outcome: MockActivate,
}

impl IdentitySource for MockSource {
    fn activate(
        &self,
        name: &str,
        _host_target: &str,
        _host_opi_version: &str,
    ) -> Result<ActivatedContribution, ActivationError> {
        match &self.outcome {
            MockActivate::Untrusted => Err(ActivationError::Untrusted {
                name: name.to_string(),
                detail: "lock drift".to_string(),
            }),
            MockActivate::Disabled => Err(ActivationError::Disabled(name.to_string())),
            MockActivate::NotInstalled => Err(ActivationError::NotInstalled(name.to_string())),
            MockActivate::Canned(contribution) => Ok((**contribution).clone()),
        }
    }
}

/// A store that panics if activated — the Minimal-Runtime branch-1 sentinel.
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

// ---------------------------------------------------------------------------
// Build helpers
// ---------------------------------------------------------------------------

const HOST_TARGET: &str = if cfg!(windows) {
    "x86_64-pc-windows-msvc"
} else if cfg!(target_os = "linux") {
    "x86_64-unknown-linux-gnu"
} else {
    "x86_64-apple-darwin"
};
const HOST_OPI_VERSION: &str = "0.8.0";

fn empty_policy() -> PermissionPolicy {
    PermissionPolicy::empty()
}

fn policy(pairs: &[(&str, PermissionDecision)]) -> PermissionPolicy {
    let map: BTreeMap<String, PermissionDecision> =
        pairs.iter().map(|(k, v)| ((*k).to_string(), *v)).collect();
    PermissionPolicy::from_map(map)
}

fn fixed(backend: &str) -> ExecutionConfig {
    ExecutionConfig {
        strategy: ExecutionStrategy::Fixed,
        backend: backend.to_string(),
        ..ExecutionConfig::default()
    }
}

fn model() -> ExecutionConfig {
    ExecutionConfig {
        strategy: ExecutionStrategy::Model,
        ..ExecutionConfig::default()
    }
}

fn identity(adapter_id: &str, pkg: &str) -> EnabledIdentity {
    EnabledIdentity {
        adapter_id: adapter_id.to_string(),
        package_name: pkg.to_string(),
    }
}

fn build(
    config: &ExecutionConfig,
    enabled: &[EnabledIdentity],
    policy: &PermissionPolicy,
    store: Arc<dyn IdentitySource>,
    local_ops: Arc<dyn BashOperations>,
) -> Arc<dyn BashOperations> {
    ExecutionRuntime::build(
        config,
        ExecutionRunMode::Interactive,
        enabled,
        policy,
        store,
        local_ops,
        Path::new("."),
        HOST_TARGET,
        HOST_OPI_VERSION,
        Arc::new(PermissionManager::new()),
        None,
    )
    .expect("build succeeds for these inputs")
}

fn request(command: &str) -> BashRequest {
    BashRequest {
        authorized_backend: None,
        command: command.to_string(),
        cwd: PathBuf::from("."),
        timeout: Duration::from_secs(5),
        signal: CancellationToken::new(),
        env: Vec::new(),
        backend: None,
    }
}

async fn exec_code(ops: Arc<dyn BashOperations>, command: &str) -> Option<String> {
    // Returns the stable code on Err, or None on Ok (routing failure surface).
    match ops.exec(request(command)).await {
        Ok(_) => None,
        Err(error) => error.diagnostics().first().map(|d| d.code.clone()),
    }
}

// ---------------------------------------------------------------------------
// Branch 1: Minimal Runtime
// ---------------------------------------------------------------------------

#[tokio::test]
async fn minimal_runtime_returns_local_ops_directly_and_skips_store() {
    // Default-local + no enabled identity: build returns the SAME local Arc
    // (pointer-identity), proving no RoutedBashOperations wrapper — and
    // transitively no eligibility/router/permission/protocol/adapter state, all
    // of which live only inside RoutedBashOperations. The panic-on-call store
    // survives, proving build never touched it.
    let local_ops: Arc<dyn BashOperations> = Arc::new(RecordingOps::new());
    let store: Arc<dyn IdentitySource> = Arc::new(PanicSource);
    let returned = build(
        &ExecutionConfig::default(),
        &[],
        &empty_policy(),
        store,
        Arc::clone(&local_ops),
    );
    assert!(
        Arc::ptr_eq(&returned, &local_ops),
        "Minimal Runtime must return the local backend by pointer-identity (no wrapper)"
    );
}

#[test]
fn minimal_runtime_local_ask_is_permission_required() {
    // MF2: branch 1 consults the policy — explicit local ask must fail even with
    // no enabled identity (consistent with the routed branch).
    let local_ops: Arc<dyn BashOperations> = Arc::new(RecordingOps::new());
    let store: Arc<dyn IdentitySource> = Arc::new(PanicSource);
    let err = match ExecutionRuntime::build(
        &ExecutionConfig::default(),
        ExecutionRunMode::Interactive,
        &[],
        &policy(&[(LOCAL_ADAPTER_ID, PermissionDecision::Ask)]),
        store,
        local_ops,
        Path::new("."),
        HOST_TARGET,
        HOST_OPI_VERSION,
        Arc::new(PermissionManager::new()),
        None,
    ) {
        Err(e) => e,
        Ok(_) => panic!("local ask must fail, but build returned Ok"),
    };
    assert_eq!(err.code(), "permission_required");
}

// ---------------------------------------------------------------------------
// Branch 2: routed assembly
// ---------------------------------------------------------------------------

#[tokio::test]
async fn routed_branch_wraps_local_ops() {
    // A non-default config wraps local_ops in RoutedBashOperations: the returned
    // Arc is a DIFFERENT allocation (no pointer-identity with the local backend).
    let local_ops: Arc<dyn BashOperations> = Arc::new(RecordingOps::new());
    let store: Arc<dyn IdentitySource> = Arc::new(MockSource {
        outcome: MockActivate::NotInstalled,
    });
    let returned = build(
        &fixed("opi-sandbox"),
        &[identity("opi-sandbox", "mock-pkg")],
        &empty_policy(),
        store,
        Arc::clone(&local_ops),
    );
    assert!(
        !Arc::ptr_eq(&returned, &local_ops),
        "routed branch must wrap local_ops (RoutedBashOperations)"
    );
}

#[tokio::test]
async fn routed_external_activation_failure_does_not_fall_back_to_local() {
    // fixed external + the package is untrusted at activate time: the routed
    // backend fails with package_untrusted and the LOCAL backend is never called
    // (no fallback). This is the load-bearing no-fallback guarantee.
    let local_ops = Arc::new(RecordingOps::new());
    let local_handle = Arc::clone(&local_ops);
    let store: Arc<dyn IdentitySource> = Arc::new(MockSource {
        outcome: MockActivate::Untrusted,
    });
    let ops: Arc<dyn BashOperations> = local_ops as Arc<dyn BashOperations>;
    let returned = build(
        &fixed("opi-sandbox"),
        &[identity("opi-sandbox", "mock-pkg")],
        &policy(&[("opi-sandbox", PermissionDecision::Allow)]),
        store,
        ops,
    );
    let code = exec_code(returned, "echo hi").await;
    assert_eq!(code.as_deref(), Some("package_untrusted"));
    assert_eq!(
        local_handle.call_count(),
        0,
        "external failure must NOT fall back to the local backend"
    );
}

#[tokio::test]
async fn routed_local_failure_surfaces_directly() {
    // A `local` selection under routing executes the local backend; its failure
    // surfaces as a normal backend error (the routing layer does not swallow it).
    let store: Arc<dyn IdentitySource> = Arc::new(MockSource {
        outcome: MockActivate::NotInstalled,
    });
    let local_ops: Arc<dyn BashOperations> = Arc::new(AlwaysFailOps);
    let returned = build(
        &fixed("local"),
        &[identity("opi-sandbox", "mock-pkg")],
        &empty_policy(),
        store,
        local_ops,
    );
    let result = returned.exec(request("echo hi")).await;
    assert!(result.is_err(), "local sentinel failure must surface");
}

// ---------------------------------------------------------------------------
// Routing failure codes
// ---------------------------------------------------------------------------

#[tokio::test]
async fn fixed_unknown_backend_is_no_eligible_adapter() {
    let store: Arc<dyn IdentitySource> = Arc::new(MockSource {
        outcome: MockActivate::NotInstalled,
    });
    let returned = build(
        &fixed("ghost"),
        &[identity("opi-sandbox", "mock-pkg")],
        &empty_policy(),
        store,
        Arc::new(RecordingOps::new()),
    );
    assert_eq!(
        exec_code(returned, "echo hi").await.as_deref(),
        Some("no_eligible_adapter")
    );
}

#[tokio::test]
async fn fixed_denied_backend_is_policy_denied() {
    let store: Arc<dyn IdentitySource> = Arc::new(MockSource {
        outcome: MockActivate::NotInstalled,
    });
    let returned = build(
        &fixed("opi-sandbox"),
        &[identity("opi-sandbox", "mock-pkg")],
        &policy(&[("opi-sandbox", PermissionDecision::Deny)]),
        store,
        Arc::new(RecordingOps::new()),
    );
    assert_eq!(
        exec_code(returned, "echo hi").await.as_deref(),
        Some("policy_denied")
    );
}

#[tokio::test]
async fn fixed_ask_backend_is_permission_required() {
    let store: Arc<dyn IdentitySource> = Arc::new(MockSource {
        outcome: MockActivate::NotInstalled,
    });
    let returned = build(
        &fixed("opi-sandbox"),
        &[identity("opi-sandbox", "mock-pkg")],
        &policy(&[("opi-sandbox", PermissionDecision::Ask)]),
        store,
        Arc::new(RecordingOps::new()),
    );
    assert_eq!(
        exec_code(returned, "echo hi").await.as_deref(),
        Some("permission_required")
    );
}

#[tokio::test]
async fn model_strategy_without_backend_field_is_adapter_not_selected() {
    // FL12: model strategy + no per-invocation backend (deferred to 16.9's
    // bash-schema field) yields adapter_not_selected, NOT a silent local run.
    let store: Arc<dyn IdentitySource> = Arc::new(MockSource {
        outcome: MockActivate::NotInstalled,
    });
    let returned = build(
        &model(),
        &[identity("opi-sandbox", "mock-pkg")],
        &empty_policy(),
        store,
        Arc::new(RecordingOps::new()),
    );
    assert_eq!(
        exec_code(returned, "echo hi").await.as_deref(),
        Some("adapter_not_selected")
    );
}

#[tokio::test]
async fn rules_first_match_wins_and_local_catch_all_routes_to_local() {
    // rules: non-interactive -> opi-sandbox (but it is untrusted -> fail closed);
    //        interactive -> local catch-all -> local backend runs.
    let cfg = ExecutionConfig {
        strategy: ExecutionStrategy::Rules,
        rules: vec![
            ExecutionRule {
                modes: Some(vec![ExecutionRunMode::NonInteractive]),
                backend: "opi-sandbox".to_string(),
            },
            ExecutionRule {
                modes: None,
                backend: "local".to_string(),
            },
        ],
        ..ExecutionConfig::default()
    };
    let local_ops = Arc::new(RecordingOps::new());
    let local_handle = Arc::clone(&local_ops);
    let store: Arc<dyn IdentitySource> = Arc::new(MockSource {
        outcome: MockActivate::Untrusted,
    });
    let returned = build(
        &cfg,
        &[identity("opi-sandbox", "mock-pkg")],
        &empty_policy(),
        store,
        local_ops,
    );
    let outcome = returned
        .exec(request("echo hi"))
        .await
        .expect("local routes to local");
    assert_eq!(outcome.context.exit_code, Some(0));
    assert_eq!(local_handle.call_count(), 1);
}

// ---------------------------------------------------------------------------
// FL7: the stable ExecutionFailure code lifts through BashTool into the
// agent ToolResult (not just the BashOpError mapping).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stable_failure_code_lifts_into_tool_result_diagnostics() {
    // build a routed backend whose external adapter fails at activate time
    // (package_untrusted), wire it through the REAL BashTool, and assert the
    // stable code reaches ToolResult.diagnostics via bash.rs's lift path.
    let store: Arc<dyn IdentitySource> = Arc::new(MockSource {
        outcome: MockActivate::Untrusted,
    });
    let ops: Arc<dyn BashOperations> = build(
        &fixed("opi-sandbox"),
        &[identity("opi-sandbox", "mock-pkg")],
        &policy(&[("opi-sandbox", PermissionDecision::Allow)]),
        store,
        Arc::new(RecordingOps::new()),
    );
    let tool = BashTool::new_with_ops(PathBuf::from("."), ops);
    let result_json = serde_json::json!({ "command": "echo hi" });
    let outcome = tool
        .execute("call-1", result_json, CancellationToken::new(), None)
        .await
        .expect("BashTool::execute returns ToolResult");
    let ToolResult { diagnostics, .. } = outcome;
    assert!(
        diagnostics.iter().any(|d| d.code == "package_untrusted"),
        "stable code must lift into ToolResult diagnostics: {diagnostics:?}"
    );
}

// ---------------------------------------------------------------------------
// ProcessCommandAdapter end-to-end via the 16.7 mock peer (feature-gated)
// ---------------------------------------------------------------------------

#[cfg(feature = "execution-backend-test-fixture")]
mod fixture {
    use super::*;
    use opi_coding_agent::execution::{
        InteractivePermissionBroker, LockMaterial, ValidatedExecutableContribution,
    };
    use opi_coding_agent::package_store::PackageLockEntry;
    use opi_protocol::execution::v1::WIRE_IDENTITY;
    use opi_tui::{PermissionChoice, PermissionSummary};

    /// Locate the `execution_backend_mock` test binary in the same deps dir
    /// (mirrors `execution_protocol_host.rs::mock_bin`).
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

    fn canned(adapter_id: &str, pkg: &str, mode: &str) -> ActivatedContribution {
        // The mock peer selects behavior by first CLI arg. Happy-path launches
        // also pin the expected adapter identity, package version, and target.
        let args = if mode == "happy_path" {
            vec![
                mode.to_string(),
                adapter_id.to_string(),
                "mock-1.0.0".to_string(),
                "mock-target".to_string(),
            ]
        } else {
            vec![mode.to_string()]
        };
        ActivatedContribution {
            name: pkg.to_string(),
            source: pkg.to_string(),
            validated: vec![ValidatedExecutableContribution {
                capability: "command.execute".to_string(),
                id: adapter_id.to_string(),
                transport: "process-jsonl".to_string(),
                command: mock_bin(),
                executable: Arc::new(std::fs::File::open(mock_bin()).unwrap()),
                args,
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

    fn routed_with(canned_contribution: ActivatedContribution) -> Arc<dyn BashOperations> {
        let store: Arc<dyn IdentitySource> = Arc::new(MockSource {
            outcome: MockActivate::Canned(Box::new(canned_contribution)),
        });
        build(
            &fixed("opi-sandbox"),
            &[identity("opi-sandbox", "mock-pkg")],
            &policy(&[("opi-sandbox", PermissionDecision::Allow)]),
            store,
            Arc::new(RecordingOps::new()),
        )
    }

    /// A multi-package activation seam for total-dispatch tests. Resolving by
    /// requested package makes invocation-time activation observable, so a
    /// candidate/dispatch index mismatch selects the wrong marker or fails.
    struct CatalogSource {
        contributions: BTreeMap<String, ActivatedContribution>,
        activated: Mutex<Vec<String>>,
    }

    impl CatalogSource {
        fn new(entries: impl IntoIterator<Item = (String, ActivatedContribution)>) -> Arc<Self> {
            Arc::new(Self {
                contributions: entries.into_iter().collect(),
                activated: Mutex::new(Vec::new()),
            })
        }

        fn activated(&self) -> Vec<String> {
            self.activated.lock().unwrap().clone()
        }
    }

    impl IdentitySource for CatalogSource {
        fn activate(
            &self,
            name: &str,
            _host_target: &str,
            _host_opi_version: &str,
        ) -> Result<ActivatedContribution, ActivationError> {
            self.activated.lock().unwrap().push(name.to_string());
            self.contributions
                .get(name)
                .cloned()
                .ok_or_else(|| ActivationError::NotInstalled(name.to_string()))
        }
    }

    struct AllowOnceBroker {
        seen: Mutex<Vec<PermissionSummary>>,
    }

    impl AllowOnceBroker {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                seen: Mutex::new(Vec::new()),
            })
        }

        fn seen(&self) -> Vec<PermissionSummary> {
            self.seen.lock().unwrap().clone()
        }
    }

    impl InteractivePermissionBroker for AllowOnceBroker {
        fn resolve_ask(
            &self,
            summary: PermissionSummary,
        ) -> Pin<Box<dyn Future<Output = PermissionChoice> + Send + '_>> {
            self.seen.lock().unwrap().push(summary);
            Box::pin(async { PermissionChoice::AllowOnce })
        }
    }

    fn two_candidate_source() -> Arc<CatalogSource> {
        CatalogSource::new([
            (
                "pkg-a".to_string(),
                canned("opi-sandbox", "pkg-a", "nonzero_exit"),
            ),
            (
                "pkg-b".to_string(),
                canned("second-adapter", "pkg-b", "happy_path"),
            ),
        ])
    }

    fn model_request(backend: &str) -> BashRequest {
        BashRequest {
            backend: Some(backend.to_string()),
            ..request("echo selected")
        }
    }

    fn model_runtime(
        permission: PermissionDecision,
        source: Arc<CatalogSource>,
        broker: Option<Arc<dyn InteractivePermissionBroker>>,
    ) -> Arc<dyn BashOperations> {
        ExecutionRuntime::build(
            &model(),
            ExecutionRunMode::Interactive,
            &[
                identity("opi-sandbox", "pkg-a"),
                identity("second-adapter", "pkg-b"),
            ],
            &policy(&[
                ("opi-sandbox", PermissionDecision::Allow),
                ("second-adapter", permission),
            ]),
            source,
            Arc::new(RecordingOps::new()),
            Path::new("."),
            HOST_TARGET,
            HOST_OPI_VERSION,
            Arc::new(PermissionManager::new()),
            broker,
        )
        .expect("model runtime builds")
    }

    #[tokio::test]
    async fn model_allow_dispatches_the_selected_non_first_external_candidate() {
        let source = two_candidate_source();
        let ops = model_runtime(PermissionDecision::Allow, source.clone(), None);

        let outcome = ops
            .exec(model_request("second-adapter"))
            .await
            .expect("selected second adapter executes");

        assert_eq!(outcome.context.exit_code, Some(0));
        assert_eq!(outcome.stdout, b"hello\n");
        assert_eq!(
            source.activated(),
            ["pkg-b"],
            "dispatch must activate exactly the package paired with the selected candidate"
        );
    }

    #[tokio::test]
    async fn model_ask_keeps_the_selected_non_first_target_through_approval() {
        let refused_source = two_candidate_source();
        let refused = model_runtime(PermissionDecision::Ask, refused_source.clone(), None);
        let error = refused
            .exec(model_request("second-adapter"))
            .await
            .expect_err("ask without a broker fails closed");
        assert!(
            error
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == "permission_required")
        );
        assert!(
            refused_source.activated().is_empty(),
            "permission-required must not activate an adapter"
        );

        let source = two_candidate_source();
        let broker = AllowOnceBroker::new();
        let broker_trait: Arc<dyn InteractivePermissionBroker> = broker.clone();
        let approved = model_runtime(PermissionDecision::Ask, source.clone(), Some(broker_trait));
        let outcome = approved
            .exec(model_request("second-adapter"))
            .await
            .expect("approved ask dispatches");

        assert_eq!(outcome.context.exit_code, Some(0));
        assert_eq!(outcome.stdout, b"hello\n");
        assert_eq!(source.activated(), ["pkg-b"]);
        let seen = broker.seen();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].adapter_id, "second-adapter");
        assert_eq!(seen[0].package_name, "pkg-b");
    }

    #[tokio::test]
    async fn process_command_adapter_drives_mock_peer_happy_path() {
        let ops = routed_with(canned("opi-sandbox", "mock-pkg", "happy_path"));
        let outcome = ops
            .exec(request("echo hi"))
            .await
            .expect("happy path is Ok");
        assert_eq!(outcome.context.exit_code, Some(0));
        assert_eq!(outcome.stdout, b"hello\n");
        // The routed success path carries the typed operation context that
        // bash.rs consumes directly.
        assert_eq!(
            outcome.context.contract.adapter_id.as_deref(),
            Some("opi-sandbox")
        );
        assert_eq!(outcome.context.contract.guarantee, "supervised");
    }

    #[tokio::test]
    async fn process_command_adapter_preserves_nonzero_exit_in_band() {
        let ops = routed_with(canned("opi-sandbox", "mock-pkg", "nonzero_exit"));
        let outcome = ops
            .exec(request("echo hi"))
            .await
            .expect("nonzero exit is in-band Ok");
        assert_eq!(outcome.context.exit_code, Some(2));
    }

    #[tokio::test]
    async fn process_command_adapter_protocol_violation_lifts_stable_code() {
        // The mock peer produces a malformed frame -> host returns
        // ProtocolViolation -> ProcessCommandAdapter maps it to a BashOpError
        // whose diagnostic carries the stable code.
        let ops = routed_with(canned("opi-sandbox", "mock-pkg", "malformed_frame"));
        let err = ops.exec(request("echo hi")).await.unwrap_err();
        assert_eq!(
            err.root_cause().to_string(),
            "bash backend error: protocol_violation"
        );
        assert!(
            err.diagnostics()
                .iter()
                .any(|d| d.code == "protocol_violation")
        );
    }
}
