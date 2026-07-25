//! Phase 15.5.1 integration tests: strict-sandbox policy enforcement and
//! production dispatch.
//!
//! These prove the cross-platform policy end-to-end against the real
//! production symbols — [`LocalBashOperations::exec`] enforcement,
//! [`CodingHarness::build_tools_with_sandbox`] wiring, and
//! [`prepare_production`]/[`prepare`] resolution — with **no host kernel
//! dependency**: backends are capability-injected via [`StrictBackend`] fakes,
//! and `prepare_production` is asserted only on host-independent invariants
//! (15.5.1 ships no engaged backend, so strict never claims engagement).
//!
//! DoD scenarios covered:
//! - `phase15-sandbox-fallback-policy`: `unavailable_layer_fail_open_and_fail_closed`
//!   (fail-open runs at L0 with one degraded diagnostic; fail-closed returns
//!   `SandboxUnavailable` before spawn) and `permanent_gap_diagnostic_is_once_per_startup`
//!   (permanent gaps surface once at startup, never per command).
//! - `phase15-sandbox-config-production-path`: `production_build_tools_wires_strict_policy_into_bash`
//!   drives the production `build_tools_with_sandbox` choke point that
//!   interactive/non-interactive/RPC startup all reach via
//!   `CodingHarness::new_with_build_options`.

use std::sync::Arc;
use std::time::Duration;

use opi_coding_agent::config::{SandboxConfig, SandboxMode};
use opi_coding_agent::diagnostics::{CODE_SANDBOX_DEGRADED, CODE_SANDBOX_UNAVAILABLE};
use opi_coding_agent::sandbox::{
    LayerAvailability, PreparedSandbox, SandboxLayer, StrictBackend, StrictOutcome, prepare,
    prepare_production,
};
use opi_coding_agent::tool::{BashOpError, BashOperations, BashRequest, LocalBashOperations};
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Capability-injected fake backend
// ---------------------------------------------------------------------------

struct FakeBackend<F: Fn(SandboxLayer) -> LayerAvailability + Send + Sync>(F);

impl<F> StrictBackend for FakeBackend<F>
where
    F: Fn(SandboxLayer) -> LayerAvailability + Send + Sync,
{
    fn availability(&self, layer: SandboxLayer) -> LayerAvailability {
        (self.0)(layer)
    }
}

fn fake_backend<F>(f: F) -> Arc<FakeBackend<F>>
where
    F: Fn(SandboxLayer) -> LayerAvailability + Send + Sync,
{
    Arc::new(FakeBackend(f))
}

fn strict_config(require: bool) -> SandboxConfig {
    SandboxConfig {
        mode: SandboxMode::Strict,
        require,
        fs: None,
        network: None,
        syscalls: None,
    }
}

fn bash_request(cwd: &std::path::Path) -> BashRequest {
    BashRequest {
        command: "echo hi".to_string(),
        cwd: cwd.to_path_buf(),
        timeout: Duration::from_secs(5),
        signal: CancellationToken::new(),
        env: vec![],
    }
}

// ---------------------------------------------------------------------------
// DoD: unavailable_layer_fail_open_and_fail_closed
// ---------------------------------------------------------------------------

/// `phase15-sandbox-fallback-policy`: a requested-but-unavailable layer under
/// `require = false` proceeds at the L0 baseline with one `CODE_SANDBOX_DEGRADED`
/// diagnostic per command; under `require = true` the same layer returns a named
/// `SandboxUnavailable` error before any spawn side effect.
#[tokio::test]
async fn unavailable_layer_fail_open_and_fail_closed() {
    let dir = tempfile::tempdir().unwrap();

    // Fail-open (require=false) on a TEMPORARY gap: command runs, degraded diag.
    let backend = fake_backend(|_| LayerAvailability::TemporarilyUnavailable {
        reason: "fake temporary".to_string(),
    });
    let prepared = prepare(&strict_config(false), backend.as_ref());
    let ops = LocalBashOperations::with_prepared(prepared);
    let result = ops.exec(bash_request(dir.path())).await.unwrap();
    assert!(
        result.exit_code.is_some(),
        "fail-open must still run the command, got exit {:?}",
        result.exit_code
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == CODE_SANDBOX_DEGRADED),
        "fail-open must emit one CODE_SANDBOX_DEGRADED diagnostic per command"
    );

    // Fail-closed (require=true) on a PERMANENT gap: SandboxUnavailable, no spawn.
    let backend = fake_backend(|_| LayerAvailability::PermanentlyUnavailable {
        reason: "fake permanent".to_string(),
    });
    let prepared = prepare(&strict_config(true), backend.as_ref());
    let ops = LocalBashOperations::with_prepared(prepared);
    let err = ops.exec(bash_request(dir.path())).await.unwrap_err();
    match err {
        BashOpError::SandboxUnavailable { message } => {
            assert!(
                message.contains("fs")
                    && message.contains("network")
                    && message.contains("syscalls"),
                "fail-closed reason should name the unavailable layers, got: {message}"
            );
        }
        other => panic!("expected SandboxUnavailable, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// DoD: permanent_gap_diagnostic_is_once_per_startup
// ---------------------------------------------------------------------------

/// `phase15-sandbox-fallback-policy`: a PERMANENT platform gap surfaces exactly
/// once per startup (via `startup_diagnostics`, emitted through the harness
/// startup channel) and is never re-emitted per command. A FailOpen decision for
/// permanent-only gaps carries no per-command temporaries, so repeated execs add
/// zero sandbox diagnostics after the single startup emission.
#[tokio::test]
async fn permanent_gap_diagnostic_is_once_per_startup() {
    let dir = tempfile::tempdir().unwrap();

    let backend = fake_backend(|_| LayerAvailability::PermanentlyUnavailable {
        reason: "fake permanent".to_string(),
    });
    let prepared = prepare(&strict_config(false), backend.as_ref());

    // Once at startup: fs + network + syscalls = three permanent diagnostics.
    let startup = prepared.startup_diagnostics();
    assert_eq!(
        startup.len(),
        3,
        "one permanent diagnostic per layer at startup"
    );
    assert!(
        startup.iter().all(|d| d.code == CODE_SANDBOX_UNAVAILABLE),
        "permanent gaps use CODE_SANDBOX_UNAVAILABLE"
    );

    // The FailOpen decision for permanent-only gaps has NO per-command temporaries.
    if let PreparedSandbox::Strict(decision) = &prepared
        && let StrictOutcome::FailOpen {
            per_command_temporary,
        } = &decision.outcome
    {
        assert!(
            per_command_temporary.is_empty(),
            "permanent gaps must not be re-emitted per command"
        );
    } else {
        panic!("expected Strict FailOpen for permanent-only gaps");
    }

    // Repeated execs emit ZERO sandbox diagnostics (the permanent ones were
    // already emitted once at startup; the per-command channel stays empty).
    let ops = LocalBashOperations::with_prepared(prepared.clone());
    for i in 0..3 {
        let result = ops.exec(bash_request(dir.path())).await.unwrap();
        let sandbox_diags: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.code == CODE_SANDBOX_UNAVAILABLE || d.code == CODE_SANDBOX_DEGRADED)
            .collect();
        assert!(
            sandbox_diags.is_empty(),
            "command {i} must not re-emit a permanent gap diagnostic, got {sandbox_diags:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// DoD: production dispatch reaches build_tools + exec (the shared choke point)
// ---------------------------------------------------------------------------

/// `phase15-sandbox-config-production-path`: `prepare_production` on the current
/// host never claims engagement (15.5.1 ships no platform backend), so a
/// strict+require request must fail closed through the production symbols that
/// `CodingHarness::new_with_build_options` wires — on EVERY host, with no kernel
/// feature required.
#[tokio::test]
async fn production_strict_require_fails_closed_on_every_platform() {
    let dir = tempfile::tempdir().unwrap();
    let prepared = prepare_production(&strict_config(true));
    let ops = LocalBashOperations::with_prepared(prepared);
    let err = ops.exec(bash_request(dir.path())).await.unwrap_err();
    assert!(
        matches!(err, BashOpError::SandboxUnavailable { .. }),
        "production strict+require must fail closed on every platform, got {err:?}"
    );
}

/// `phase15-sandbox-config-production-path`: the production
/// [`CodingHarness::build_tools_with_sandbox`] choke point — the function
/// `new_with_build_options` calls for interactive, non-interactive, and RPC
/// startup — constructs a `BashTool` whose execution enforces the resolved
/// policy. With strict+require, executing the bash tool surfaces the fail-closed
/// error result end-to-end.
#[tokio::test]
async fn production_build_tools_wires_strict_policy_into_bash() {
    use opi_coding_agent::harness::CodingHarness;
    use opi_coding_agent::policy::{RunMode, ToolRuntimeConfig, ToolSelection};

    let ws = tempfile::tempdir().unwrap();
    let tool_config =
        ToolRuntimeConfig::resolve(RunMode::Interactive, true, ToolSelection::Default)
            .expect("interactive tool config");
    // The production path resolves prepare_production(&config.sandbox) inside
    // new_with_build_options; mirror that exactly here.
    let prepared = prepare_production(&strict_config(true));
    let (mut tools, _startup_diagnostics) =
        CodingHarness::build_tools_with_sandbox(ws.path(), &tool_config, prepared);
    let bash = tools
        .iter_mut()
        .find(|t| t.definition().name == "bash")
        .expect("build_tools must construct the bash tool");

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
        "strict+require bash must surface fail-closed as an error result"
    );
    let text = serde_json::to_string(&result.content).expect("outputs serialize");
    assert!(
        text.contains("sandbox required but unavailable"),
        "fail-closed error message must reach the tool result, got: {text}"
    );
}

/// Companion: with `mode = off` (the default), `build_tools_with_sandbox`
/// constructs a bash tool that runs normally — the always-on L0 baseline is
/// preserved and no sandbox diagnostic is emitted. Guards against the policy
/// accidentally engaging for the default off configuration.
#[tokio::test]
async fn production_off_mode_runs_command_without_sandbox_diagnostic() {
    use opi_coding_agent::harness::CodingHarness;
    use opi_coding_agent::policy::{RunMode, ToolRuntimeConfig, ToolSelection};

    let ws = tempfile::tempdir().unwrap();
    let tool_config =
        ToolRuntimeConfig::resolve(RunMode::Interactive, true, ToolSelection::Default)
            .expect("interactive tool config");
    let off_config = SandboxConfig::default();
    let prepared = prepare_production(&off_config);
    let (mut tools, startup_diagnostics) =
        CodingHarness::build_tools_with_sandbox(ws.path(), &tool_config, prepared);
    assert!(
        startup_diagnostics.is_empty(),
        "off mode must produce no startup diagnostics"
    );
    let bash = tools
        .iter_mut()
        .find(|t| t.definition().name == "bash")
        .expect("bash tool present");
    let result = bash
        .execute(
            "test-call",
            serde_json::json!({"command": "echo hello-odradek", "timeout_secs": 5}),
            CancellationToken::new(),
            None,
        )
        .await
        .expect("bash tool executes");
    assert!(!result.is_error, "off-mode echo must succeed");
    let text = serde_json::to_string(&result.content).expect("outputs serialize");
    assert!(
        text.contains("hello-odradek"),
        "off-mode command must run and produce output, got: {text}"
    );
}

// ---------------------------------------------------------------------------
// Phase 15 task 15.5.5: Windows strict capability fallback (native product tests)
// ---------------------------------------------------------------------------
//
// These are the mandatory native Windows product tests. They are `#[cfg(windows)]`
// and exercise the REAL production Windows backend (L0-only) through the public
// dispatch surface. On a Windows runner they MUST report at least one passed
// test with zero failures and zero ignored/skipped tests; on any other host they
// compile out (a wrong-host zero-test run is NOT acceptance evidence and leaves
// the task failing there). 15.5.1 shipped the inline Windows backend and the
// fail-open / fail-closed / once-per-startup policy; 15.5.5 extracts that backend
// into `sandbox/windows.rs` on the production dispatch path — these tests pin the
// observable Windows behavior across that behavior-preserving refactor and prove
// the production dispatcher reaches it.

/// DoD gate `windows_strict_reports_l0_only` (15.5.5): on a native Windows
/// runner, the production Windows backend classifies every strict layer as a
/// PERMANENT gap, surfaces exactly one redacted `CODE_SANDBOX_UNAVAILABLE`
/// diagnostic per layer ONCE at startup (never per command), runs the command at
/// the L0 baseline under `require = false`, and returns `SandboxUnavailable`
/// before spawn under `require = true`.
#[cfg(windows)]
#[tokio::test]
async fn windows_strict_reports_l0_only() {
    let dir = tempfile::tempdir().unwrap();

    // require=false: fail-open at L0; three permanent startup diagnostics.
    let prepared = prepare_production(&strict_config(false));
    let startup = prepared.startup_diagnostics();
    assert_eq!(
        startup.len(),
        3,
        "one permanent CODE_SANDBOX_UNAVAILABLE per strict layer at startup"
    );
    assert!(
        startup.iter().all(|d| d.code == CODE_SANDBOX_UNAVAILABLE),
        "Windows permanent gaps use CODE_SANDBOX_UNAVAILABLE"
    );
    // Redacted details: exactly {layer, reason}, nothing else leaks.
    for d in &startup {
        let obj = d
            .details
            .as_ref()
            .and_then(|v| v.as_object())
            .expect("startup diagnostic carries structured details");
        assert_eq!(obj.len(), 2, "details must carry only layer and reason");
        assert!(obj.contains_key("layer"));
        assert!(obj.contains_key("reason"));
    }

    // Fail-open runs the command at the L0 baseline; the permanent gaps are NOT
    // re-emitted per command (they were already emitted once at startup above).
    let ops = LocalBashOperations::with_prepared(prepared.clone());
    let result = ops.exec(bash_request(dir.path())).await.unwrap();
    assert!(
        result.exit_code.is_some(),
        "require=false must execute at the L0 baseline, got exit {:?}",
        result.exit_code
    );
    let per_command: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == CODE_SANDBOX_UNAVAILABLE || d.code == CODE_SANDBOX_DEGRADED)
        .collect();
    assert!(
        per_command.is_empty(),
        "permanent gaps must not be re-emitted per command, got {per_command:?}"
    );

    // require=true: fail-closed BEFORE process creation, but the permanent gaps
    // still surface once at startup.
    let prepared_req = prepare_production(&strict_config(true));
    assert_eq!(
        prepared_req.startup_diagnostics().len(),
        3,
        "require=true still surfaces the permanent gaps once at startup"
    );
    let ops_req = LocalBashOperations::with_prepared(prepared_req);
    let err = ops_req.exec(bash_request(dir.path())).await.unwrap_err();
    match err {
        BashOpError::SandboxUnavailable { message } => {
            assert!(
                message.contains("fs")
                    && message.contains("network")
                    && message.contains("syscalls"),
                "fail-closed reason must name all three strict layers, got: {message}"
            );
        }
        other => panic!("require=true must fail closed with SandboxUnavailable, got {other:?}"),
    }
}

/// DoD gate `windows_strict_production_dispatch_reports_l0_only` (15.5.5): a
/// factory-built `BashTool` resolved through the production dispatcher
/// (`CodingHarness::build_tools_with_sandbox` with `prepare_production`, which on
/// Windows routes through `sandbox::windows::prepare`) surfaces the Windows
/// L0-only truth end-to-end — the permanent gaps reach the harness startup
/// channel, `require = true` fail-closes the bash tool, and `require = false`
/// still runs the command at L0.
#[cfg(windows)]
#[tokio::test]
async fn windows_strict_production_dispatch_reports_l0_only() {
    use opi_coding_agent::harness::CodingHarness;
    use opi_coding_agent::policy::{RunMode, ToolRuntimeConfig, ToolSelection};

    let ws = tempfile::tempdir().unwrap();
    let tool_config =
        ToolRuntimeConfig::resolve(RunMode::Interactive, true, ToolSelection::Default)
            .expect("interactive tool config");

    // require=true through the production dispatcher: the harness startup channel
    // carries the three Windows permanent gaps and the bash tool fail-closes.
    let prepared_req = prepare_production(&strict_config(true));
    assert_eq!(
        prepared_req.startup_diagnostics().len(),
        3,
        "production dispatch surfaces all three Windows permanent gaps at startup"
    );
    let (mut tools, startup_diagnostics) =
        CodingHarness::build_tools_with_sandbox(ws.path(), &tool_config, prepared_req);
    assert_eq!(
        startup_diagnostics.len(),
        3,
        "harness startup channel receives the three CODE_SANDBOX_UNAVAILABLE diagnostics"
    );
    assert!(
        startup_diagnostics
            .iter()
            .all(|d| d.code == CODE_SANDBOX_UNAVAILABLE),
    );
    let bash = tools
        .iter_mut()
        .find(|t| t.definition().name == "bash")
        .expect("build_tools constructs the bash tool");
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
        "require=true bash must fail-closed through the production dispatcher"
    );
    let text = serde_json::to_string(&result.content).expect("outputs serialize");
    assert!(
        text.contains("sandbox required but unavailable"),
        "fail-closed error must reach the tool result, got: {text}"
    );

    // require=false through the production dispatcher: startup channel still
    // carries the three gaps, but the bash tool runs at the L0 baseline.
    let prepared_open = prepare_production(&strict_config(false));
    let (mut tools2, startup2) =
        CodingHarness::build_tools_with_sandbox(ws.path(), &tool_config, prepared_open);
    assert_eq!(
        startup2.len(),
        3,
        "require=false still reports the gaps at startup"
    );
    let bash2 = tools2
        .iter_mut()
        .find(|t| t.definition().name == "bash")
        .expect("bash tool present");
    let result2 = bash2
        .execute(
            "test-call",
            serde_json::json!({"command": "echo hello-odradek", "timeout_secs": 5}),
            CancellationToken::new(),
            None,
        )
        .await
        .expect("bash tool executes");
    assert!(
        !result2.is_error,
        "require=false bash must run at the L0 baseline"
    );
    let text2 = serde_json::to_string(&result2.content).expect("outputs serialize");
    assert!(
        text2.contains("hello-odradek"),
        "require=false command must produce output, got: {text2}"
    );
}
