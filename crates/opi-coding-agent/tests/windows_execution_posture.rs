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
//! `CODE_PROCESS_TREE_DEGRADED`. The typed operation context on `BashResult`
//! carries the contract; `BashTool` publishes its redaction-safe fields in
//! `ToolResult::details` for every public surface.
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

use opi_agent::tool::Tool as _;
use opi_coding_agent::cli::PackageCommand;
use opi_coding_agent::config::{
    ExecutionConfig, ExecutionRunMode, ExecutionStrategy, PermissionDecision,
};
use opi_coding_agent::execution::permission::PermissionPolicy;
use opi_coding_agent::execution::{
    ContributionValidationError, EnabledIdentity, ExecutionRuntime, IdentitySource, PackageSource,
    PermissionManager, validate_executable_contributions,
};
use opi_coding_agent::package_activation::{self, TrustConfirmer, TrustDisplay};
use opi_coding_agent::package_cli;
use opi_coding_agent::package_discovery::PackageManifest;
use opi_coding_agent::tool::{
    BashOpError, BashOperationContext, BashOperations, BashRequest, BashResult, BashTool,
    LocalBashOperations,
};
use tempfile::{TempDir, tempdir};
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Clause 4: local reports supervised
// ---------------------------------------------------------------------------

/// Built-in local execution reports guarantee=`supervised`, placement=`host`,
/// and never `restricted`. Drives the REAL `LocalBashOperations::exec` (Done
/// branch, a benign exit-0 target) and reads the typed operation context. The
/// guarantee is a constant for the local identity (spec line 146), not derived
/// from the prepared/confinement state.
#[tokio::test]
async fn local_exec_reports_supervised_guarantee() {
    let workspace: TempDir = tempdir().expect("workspace temp dir");
    let ops = Arc::new(LocalBashOperations::new());
    let command = if cfg!(windows) { "exit 0" } else { "true" };
    let request = BashRequest {
        command: command.to_string(),
        cwd: workspace.path().to_path_buf(),
        timeout: Duration::from_secs(5),
        signal: CancellationToken::new(),
        env: Vec::new(),
        backend: None,
        authorized_backend: None,
    };
    let result = ops
        .exec(request)
        .await
        .expect("benign local target must succeed");
    assert_eq!(
        result.context.exit_code,
        Some(0),
        "target must exit 0 so the Done-branch context is the one inspected"
    );
    assert_eq!(
        result.context.contract.guarantee, "supervised",
        "local execution-backend guarantee is supervised (spec line 146; helper.rs:154-161)"
    );
    assert_eq!(
        result.context.contract.placement, "host",
        "local execution-backend placement is host (spec line 146)"
    );
    // The local identity never reports "restricted" (that is the opi-sandbox
    // adapter identity, spec line 147). The spec table assigns local only
    // placement+guarantee — no policy/limitations — so a future regression that
    // sources the guarantee from the Phase 15 host-sandbox state (and emits a
    // constant `policy="unrestricted"`) would be dishonest on Linux-Engaged.
    assert!(
        result.context.contract.policy.is_none(),
        "local reports only placement+guarantee; no policy/limitations"
    );

    let tool = BashTool::new_with_ops(workspace.path().to_path_buf(), ops);
    let public = tool
        .execute(
            "local-contract",
            serde_json::json!({"command": command}),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    let public_details = public.details.expect("public ToolResult details");
    assert_eq!(public_details["placement"], "host");
    assert_eq!(public_details["guarantee"], "supervised");
}

// ---------------------------------------------------------------------------
// Shared seams for clause 5 (mirrored per-file from execution_runtime.rs and
// execution_package_lifecycle.rs; NOT factored into tests/common by the
// established repo convention across the execution_*.rs test binaries).
// ---------------------------------------------------------------------------

const EXE_CONTENT: &[u8] = b"#!/bin/sh\necho hi\n";

fn major_minor(version: &str) -> (u64, u64) {
    let mut parts = version.split('.');
    let major: u64 = parts.next().expect("major").parse().expect("numeric major");
    let minor: u64 = parts.next().expect("minor").parse().expect("numeric minor");
    (major, minor)
}

fn compatible_minor_range(version: &str) -> String {
    let (major, minor) = major_minor(version);
    format!(">={major}.{minor}.0-0,<{major}.{}.0-0", minor + 1)
}

fn incompatible_adjacent_minor_range(version: &str) -> String {
    let (major, minor) = major_minor(version);
    format!(">={major}.{}.0-0,<{major}.{}.0-0", minor + 1, minor + 2)
}

#[test]
fn generated_minor_range_accepts_prerelease_host_versions() {
    let host = "0.8.0-rc.1+build.17";
    let range = compatible_minor_range(host);
    let diagnostic = opi_coding_agent::package_discovery::OpiVersionDiagnostic::check(&range, host);
    assert!(
        diagnostic.is_none(),
        "generated range {range:?} must include prerelease host {host:?}: {diagnostic:?}"
    );
}

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
                context: BashOperationContext::local(Some(0), None),
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
        package_activation::host_target_triple(),
        package_activation::host_opi_version(),
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
        authorized_backend: None,
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
    let opi_range = compatible_minor_range(package_activation::host_opi_version());
    let toml = format!(
        "version = \"0.8.0\"\n\
         opi_version = \"{opi_range}\"\n\
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

    let raw = std::fs::read(&manifest).expect("read target-tampered manifest");
    let parsed = PackageManifest::from_toml(&String::from_utf8_lossy(&raw), &manifest)
        .expect("target-tampered manifest remains syntactically valid");
    let validation = validate_executable_contributions(
        &parsed,
        &raw,
        &root,
        PackageSource::Global,
        &host_target,
        package_activation::host_opi_version(),
    )
    .expect_err("target-tampered contribution must fail validation");
    match validation {
        ContributionValidationError::IncompatibleTarget { wanted, got } => {
            assert_eq!(wanted, host_target, "validator must name the injected host");
            assert_eq!(
                got, foreign_target,
                "validator must name the manifest target"
            );
        }
        other => panic!("target tamper must reach IncompatibleTarget, got {other:?}"),
    }

    // Prove the real activation path reaches the target gate before checking
    // the public redacted failure code below. Both incompatible-version and
    // incompatible-target failures intentionally map to package_untrusted, so
    // the public code alone cannot distinguish which validation gate ran.
    let activation = package_activation::PackageActivationStore::global(user.path().to_path_buf());
    let internal = activation
        .activate(
            "opi-sandbox",
            package_activation::host_target_triple(),
            package_activation::host_opi_version(),
        )
        .expect_err("the tampered target must fail pre-spawn revalidation");
    let detail = match internal {
        package_activation::ActivationError::Untrusted { detail, .. } => detail,
        other => panic!("expected target mismatch to invalidate trust, got {other}"),
    };
    assert!(
        detail.contains("manifest target"),
        "activation must fail at IncompatibleTarget, got: {detail}"
    );
    assert!(
        !detail.contains("opi version range"),
        "the version gate must not mask the target gate: {detail}"
    );

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

/// An incompatible Opi range reaches the version gate without being masked by
/// target validation. This is intentionally adjacent to the target-mismatch
/// regression because both internal failures redact to `package_untrusted` on
/// the public tool surface.
#[tokio::test]
async fn version_mismatched_package_fails_untrusted_without_target_masking() {
    let (_pkg, root) = make_execution_package("opi-sandbox");
    let workspace = tempdir().expect("workspace dir");
    let user = tempdir().expect("user dir");
    assert_eq!(
        add_global(root.to_str().unwrap(), workspace.path(), user.path()),
        0,
        "global add with the compatible generated range must succeed"
    );
    let mut confirmer = TestConfirmer { grant: true };
    package_activation::PackageActivationStore::global(user.path().to_path_buf())
        .enable(
            "opi-sandbox",
            package_activation::host_target_triple(),
            package_activation::host_opi_version(),
            &mut confirmer,
        )
        .expect("granting confirmer enables");

    let manifest = root.join("package.toml");
    let content = std::fs::read_to_string(&manifest).unwrap();
    let compatible = compatible_minor_range(package_activation::host_opi_version());
    let from_line = format!("opi_version = \"{compatible}\"");
    let incompatible = incompatible_adjacent_minor_range(package_activation::host_opi_version());
    let to_line = format!("opi_version = \"{incompatible}\"");
    assert!(
        content.contains(&from_line),
        "manifest must carry the generated compatible range before tamper"
    );
    std::fs::write(&manifest, content.replacen(&from_line, &to_line, 1)).unwrap();

    let raw = std::fs::read(&manifest).expect("read version-tampered manifest");
    let parsed = PackageManifest::from_toml(&String::from_utf8_lossy(&raw), &manifest)
        .expect("version-tampered manifest remains syntactically valid");
    let host = package_activation::host_opi_version();
    let validation = validate_executable_contributions(
        &parsed,
        &raw,
        &root,
        PackageSource::Global,
        package_activation::host_target_triple(),
        host,
    )
    .expect_err("version-tampered contribution must fail validation");
    match validation {
        ContributionValidationError::IncompatibleOpiRange { range, host: got } => {
            assert_eq!(
                range, incompatible,
                "validator must name the manifest range"
            );
            assert_eq!(got, host, "validator must name the injected host version");
        }
        other => panic!("version tamper must reach IncompatibleOpiRange, got {other:?}"),
    }

    let activation = package_activation::PackageActivationStore::global(user.path().to_path_buf());
    let internal = activation
        .activate(
            "opi-sandbox",
            package_activation::host_target_triple(),
            package_activation::host_opi_version(),
        )
        .expect_err("the tampered Opi range must fail pre-spawn revalidation");
    let detail = match internal {
        package_activation::ActivationError::Untrusted { detail, .. } => detail,
        other => panic!("expected version mismatch to invalidate trust, got {other}"),
    };
    assert!(
        detail.contains("opi version range"),
        "activation must fail at IncompatibleOpiRange, got: {detail}"
    );
    assert!(
        !detail.contains("manifest target"),
        "the target gate must not mask the version gate: {detail}"
    );

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
        "version-mismatched package must redact to package_untrusted"
    );
    assert_eq!(
        local_handle.call_count(),
        0,
        "version-mismatched-package failure must NOT fall back to local"
    );
}
