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
