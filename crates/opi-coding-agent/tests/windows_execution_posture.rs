//! Task 16.14.2 acceptance (SC16-12a): the Windows unsupported execution
//! posture, and the platform-general execution-backend guarantees that flow
//! from it.
//!
//! These tests have NO top-level `cfg` gate by design (design-audit flagged
//! "cfg posture of windows_execution_posture.rs"): the clause-4 local guarantee
//! is `supervised` on EVERY host (spec Execution Backend table, design line
//! 146: `local -> supervised`), and the clause-5 package-selection fail-fast
//! paths are platform-neutral (the host target is an injectable `String`).
//! Leaving them ungated gives Linux/macOS CI free coverage of the same
//! invariants; the Windows-specific posture assertions (doctor unsupported,
//! pre-start `run` refusal) live in `crates/opi-sandbox/tests/cli_contract.rs`
//! alongside the existing `cfg!(target_os = "linux")` branch pattern.
//!
//! ## Clause 4: local reports `supervised`, never `restricted`
//!
//! Built-in LOCAL command execution reports its execution-backend guarantee as
//! `supervised` (placement `host`), never `restricted`. Per the spec Execution
//! Backend contract (design lines 144-154: `local -> supervised`; "Each
//! invocation reports its effective placement, guarantee, policy, and
//! limitations after setup has succeeded") and line 181 (L0 supervision "is
//! reported only as `supervised`"). The guarantee is a compile-time CONSTANT
//! for the local identity, NOT sourced from any sandbox/confinement state: the
//! execution-backend guarantee axis is distinct from the Phase 15 host-sandbox
//! restriction axis (seccomp+Landlock on Linux-Engaged), which is reported via
//! `CODE_PROCESS_TREE_DEGRADED`. The report medium is the in-band
//! `opi.operations.bash.operation_context` diagnostic on `BashResult` — the
//! local path cannot initialize protocol state (spec lines 195-197), so its
//! report intentionally does NOT flow to the agent `ToolResult` wire (the
//! diagnostic is filtered at `tool/bash.rs:append_backend_diagnostics`).
//!
//! ## Clause 5: package-selection fail-fast before command execution
//!
//! Selecting an absent opi-sandbox package fails with `package_not_installed`,
//! and a target-mismatched package fails with `package_untrusted`, both BEFORE
//! any command spawns and with NO fallback to the local backend. Both drive the
//! production `ProcessCommandAdapter::exec` through `ExecutionRuntime::build`
//! (the only seam, since the adapter's fields are private) and assert
//! `RecordingOps.call_count() == 0` — the observable no-fallback guarantee.
//! No-spawn is a structural proof by ordering (activate-`Err` returns at
//! `runtime.rs:503-507` before the protocol host is constructed), attributed in
//! the doc-comments rather than via an invented spawn sentinel. Mirrors
//! `tests/execution_runtime.rs::routed_external_activation_failure_does_not_fall_back_to_local`.

use std::collections::BTreeMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use opi_coding_agent::cli::PackageCommand;
use opi_coding_agent::config::{
    ExecutionConfig, ExecutionRunMode, ExecutionStrategy, PermissionDecision,
};
use opi_coding_agent::execution::permission::PermissionPolicy;
use opi_coding_agent::execution::{
    EnabledIdentity, ExecutionRuntime, IdentitySource, PermissionManager,
};
use opi_coding_agent::package_activation::{self, TrustConfirmer, TrustDisplay};
use opi_coding_agent::package_cli;
use opi_coding_agent::tool::{
    BashOpError, BashOperations, BashRequest, BashResult, LOCAL_BASH_OPERATION_DIAGNOSTIC,
    LocalBashOperations,
};
use tempfile::{TempDir, tempdir};
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Clause 4: local reports supervised
// ---------------------------------------------------------------------------

/// Built-in local execution reports guarantee=`supervised`, placement=`host`,
/// and never `restricted`. Drives the REAL `LocalBashOperations::exec` (Done
/// branch, a benign exit-0 target) and reads the in-band operation_context
/// diagnostic. The guarantee is a constant for the local identity (spec line
/// 146), not derived from the prepared/confinement state.
#[tokio::test]
async fn local_exec_reports_supervised_guarantee() {
    let workspace: TempDir = tempdir().expect("workspace temp dir");
    let ops = LocalBashOperations::new();
    let command = if cfg!(windows) { "exit 0" } else { "true" };
    let request = BashRequest {
        command: command.to_string(),
        cwd: workspace.path().to_path_buf(),
        timeout: Duration::from_secs(5),
        signal: CancellationToken::new(),
        env: Vec::new(),
        backend: None,
    };
    let result = ops
        .exec(request)
        .await
        .expect("benign local target must succeed");
    assert_eq!(
        result.exit_code,
        Some(0),
        "target must exit 0 so the Done-branch diagnostic is the one inspected"
    );
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|d| d.code == LOCAL_BASH_OPERATION_DIAGNOSTIC)
        .expect("the local operation_context diagnostic is emitted on every in-band BashResult");
    let details = diagnostic
        .details
        .as_ref()
        .expect("operation_context diagnostic carries a details payload");

    let guarantee = details.get("guarantee").and_then(|v| v.as_str());
    let placement = details.get("placement").and_then(|v| v.as_str());
    assert_eq!(
        guarantee,
        Some("supervised"),
        "local execution-backend guarantee is supervised (spec line 146; helper.rs:154-161)"
    );
    assert_eq!(
        placement,
        Some("host"),
        "local execution-backend placement is host (spec line 146)"
    );
    // The local identity never reports "restricted" (that is the opi-sandbox
    // adapter identity, spec line 147). The spec table assigns local only
    // placement+guarantee — no policy/limitations — so a future regression that
    // sources the guarantee from the Phase 15 host-sandbox state (and emits a
    // constant `policy="unrestricted"`) would be dishonest on Linux-Engaged.
    assert!(
        details.get("policy").is_none(),
        "local reports only placement+guarantee; no policy/limitations"
    );
}

// ---------------------------------------------------------------------------
// Shared seams for clause 5 (mirrored per-file from execution_runtime.rs and
// execution_package_lifecycle.rs; NOT factored into tests/common by the
// established repo convention across the execution_*.rs test binaries).
// ---------------------------------------------------------------------------

const HOST_TARGET: &str = if cfg!(windows) {
    "x86_64-pc-windows-msvc"
} else if cfg!(target_os = "linux") {
    "x86_64-unknown-linux-gnu"
} else {
    "x86_64-apple-darwin"
};
const HOST_OPI_VERSION: &str = "0.8.0";
const EXE_CONTENT: &[u8] = b"#!/bin/sh\necho hi\n";

/// A `BashOperations` sentinel that records every `exec` call's command and
/// returns a canned in-band result. Proves the local backend is (or is not)
/// reached — never spawns a real process.
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
                exit_code: Some(0),
                signal: None,
                diagnostics: Vec::new(),
            })
        })
    }
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
        command: command.to_string(),
        cwd: PathBuf::from("."),
        timeout: Duration::from_secs(5),
        signal: CancellationToken::new(),
        env: Vec::new(),
        backend: None,
    }
}

/// Returns the stable failure code on `Err`, or `None` on `Ok`.
async fn exec_code(ops: Arc<dyn BashOperations>, command: &str) -> Option<String> {
    match ops.exec(request(command)).await {
        Ok(_) => None,
        Err(error) => error.diagnostics().first().map(|d| d.code.clone()),
    }
}

fn t_sha256(bytes: &[u8]) -> String {
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

/// Build a package dir whose `package.toml` declares one executable
/// `command.execute` contribution targeting the running host. (Mirrored from
/// `execution_package_lifecycle.rs`.) The store references this source dir, so a
/// post-install manifest tamper is seen at `revalidate_lock` time.
fn make_execution_package(adapter_id: &str) -> (TempDir, PathBuf) {
    let dir = tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("bin")).unwrap();
    let exe = dir.path().join("bin").join(adapter_id);
    std::fs::write(&exe, EXE_CONTENT).unwrap();
    make_executable(&exe);
    let sha = t_sha256(EXE_CONTENT);
    let target = package_activation::host_target_triple();
    let toml = format!(
        "version = \"0.8.0\"\n\
         opi_version = \">=0.7,<0.8\"\n\
         name = \"{adapter_id}\"\n\
         description = \"test execution backend\"\n\
         \n\
         [[contributions.adapters]]\n\
         capability = \"command.execute\"\n\
         id = \"{adapter_id}\"\n\
         transport = \"process-jsonl\"\n\
         command = \"bin/{adapter_id}\"\n\
         args = [\"backend\", \"--stdio\"]\n\
         protocol = \"command-execution-jsonl-v1\"\n\
         target = \"{target}\"\n\
         sha256 = \"{sha}\"\n\
         handshake_timeout_ms = 5000\n\
         adapter_config = {{}}\n"
    );
    std::fs::write(dir.path().join("package.toml"), toml).unwrap();
    let root = dir.path().to_path_buf();
    (dir, root)
}

/// A confirmer that grants or refuses deterministically.
struct TestConfirmer {
    grant: bool,
}
impl TrustConfirmer for TestConfirmer {
    fn confirm(&mut self, _display: &TrustDisplay) -> Result<(), String> {
        if self.grant {
            Ok(())
        } else {
            Err("test confirmer refused".into())
        }
    }
}

fn add_global(source: &str, workspace: &Path, user: &Path) -> i32 {
    package_cli::handle_package_command(
        &PackageCommand::Add {
            source: source.to_string(),
            local: false,
        },
        workspace.to_path_buf(),
        user.to_path_buf(),
    )
}

// ---------------------------------------------------------------------------
// Clause 5: package-selection fail-fast through ProcessCommandAdapter::exec
// ---------------------------------------------------------------------------

/// An ABSENT opi-sandbox package fails with `package_not_installed` BEFORE any
/// command spawns, with NO fallback to the local backend. Drives the REAL
/// `PackageActivationStore` over an empty user dir (so `find_lock_by_source`
/// returns `None` -> `NotInstalled`) through `ProcessCommandAdapter::exec`.
/// `call_count() == 0` is the observable no-fallback guarantee; no-spawn is
/// structural (activate-`Err` returns before the protocol host is constructed).
#[tokio::test]
async fn absent_package_fails_not_installed_before_command_with_no_fallback() {
    let local_ops = Arc::new(RecordingOps::new());
    let local_handle = Arc::clone(&local_ops);
    // Real store over an EMPTY user dir: no lock entry -> NotInstalled through
    // the real activate path (not a mock short-circuit).
    let user = tempdir().expect("user dir");
    let store: Arc<dyn IdentitySource> = Arc::new(
        package_activation::PackageActivationStore::global(user.path().to_path_buf()),
    );
    let ops: Arc<dyn BashOperations> = build(
        &fixed("opi-sandbox"),
        &[identity("opi-sandbox", "opi-sandbox")],
        &policy(&[("opi-sandbox", PermissionDecision::Allow)]),
        store,
        local_ops as Arc<dyn BashOperations>,
    );
    let code = exec_code(ops, "echo hi").await;
    assert_eq!(
        code.as_deref(),
        Some("package_not_installed"),
        "absent package must fail fast with package_not_installed"
    );
    assert_eq!(
        local_handle.call_count(),
        0,
        "absent-package failure must NOT fall back to the local backend"
    );
}

/// A TARGET-MISMATCHED opi-sandbox package fails with `package_untrusted`
/// BEFORE any command spawns, with NO fallback to the local backend. Installs +
/// enables a package at the HOST target (so install succeeds and trust is
/// granted), then tampers the manifest's `target` line to a foreign triple; at
/// exec `revalidate_lock` -> `validate_executable_contributions` hits the REAL
/// `IncompatibleTarget` path -> `package_untrusted` (design-audit MF-3: a mock
/// would short-circuit before revalidate and be vacuous for this case).
/// `call_count() == 0` is the observable no-fallback guarantee.
#[tokio::test]
async fn target_mismatched_package_fails_untrusted_before_command_with_no_fallback() {
    let (_pkg, root) = make_execution_package("opi-sandbox");
    let workspace = tempdir().expect("workspace dir");
    let user = tempdir().expect("user dir");
    assert_eq!(
        add_global(root.to_str().unwrap(), workspace.path(), user.path()),
        0,
        "global add at the host target must succeed"
    );
    // Grant trust + enable (matching the host target the package was built for).
    let mut confirmer = TestConfirmer { grant: true };
    package_activation::PackageActivationStore::global(user.path().to_path_buf())
        .enable(
            "opi-sandbox",
            package_activation::host_target_triple(),
            package_activation::host_opi_version(),
            &mut confirmer,
        )
        .expect("granting confirmer enables");

    // Tamper the manifest target to a FOREIGN triple. The store references this
    // source dir, so revalidate_lock at exec time reads the tampered target.
    let host_target = package_activation::host_target_triple().to_string();
    let foreign_target = if host_target.contains("windows") {
        "x86_64-unknown-linux-gnu"
    } else {
        "x86_64-pc-windows-msvc"
    };
    assert_ne!(
        host_target, foreign_target,
        "foreign target must differ from the host target"
    );
    let manifest = root.join("package.toml");
    let content = std::fs::read_to_string(&manifest).unwrap();
    let from_line = format!("target = \"{host_target}\"");
    let to_line = format!("target = \"{foreign_target}\"");
    assert!(
        content.contains(&from_line),
        "manifest must carry the host target before tamper"
    );
    std::fs::write(&manifest, content.replacen(&from_line, &to_line, 1)).unwrap();

    let local_ops = Arc::new(RecordingOps::new());
    let local_handle = Arc::clone(&local_ops);
    let store: Arc<dyn IdentitySource> = Arc::new(
        package_activation::PackageActivationStore::global(user.path().to_path_buf()),
    );
    let ops: Arc<dyn BashOperations> = build(
        &fixed("opi-sandbox"),
        &[identity("opi-sandbox", "opi-sandbox")],
        &policy(&[("opi-sandbox", PermissionDecision::Allow)]),
        store,
        local_ops as Arc<dyn BashOperations>,
    );
    let code = exec_code(ops, "echo hi").await;
    assert_eq!(
        code.as_deref(),
        Some("package_untrusted"),
        "target-mismatched package must fail fast with package_untrusted via IncompatibleTarget"
    );
    assert_eq!(
        local_handle.call_count(),
        0,
        "target-mismatched-package failure must NOT fall back to the local backend"
    );
}
