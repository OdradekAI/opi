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
// BashResult is referenced only by the Linux/macOS engaged-product test helper
// (`assert_probe_exit`), so the import is gated to match its cfg-gated use;
// importing it unguarded trips `unused_imports` (and `-D warnings`) on hosts
// that compile out the engaged tests.
#[cfg(any(target_os = "linux", target_os = "macos"))]
use opi_coding_agent::tool::BashResult;
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

/// `phase15-sandbox-config-production-path`: production strict+require is
/// resolved through the symbols `CodingHarness::new_with_build_options` wires.
/// On a capable Linux kernel (15.5.3) every strict layer engages, so the command
/// runs confined; on Windows (L0-only), macOS (not-yet-wired), or an old Linux
/// kernel, `require = true` fail-closes with `SandboxUnavailable` before spawn.
#[tokio::test]
async fn production_strict_require_runs_confined_or_fail_closed() {
    let dir = tempfile::tempdir().unwrap();
    let prepared = prepare_production(&strict_config(true), dir.path());
    let ops = LocalBashOperations::with_prepared(prepared);
    match ops.exec(bash_request(dir.path())).await {
        Ok(result) => {
            // The capable-Linux engaged path: the (confined) command still runs.
            #[cfg(target_os = "linux")]
            assert!(
                result.exit_code.is_some(),
                "engaged strict+require must run the confined command, got {:?}",
                result.exit_code
            );
            #[cfg(not(target_os = "linux"))]
            panic!("non-Linux strict+require must fail-closed, not run: {result:?}");
        }
        Err(BashOpError::SandboxUnavailable { .. }) => {
            // Fail-closed path: Windows / macOS / old Linux kernel.
            #[cfg(target_os = "linux")]
            {}
        }
        Err(other) => panic!("expected Ok (engaged) or SandboxUnavailable, got {other:?}"),
    }
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
    let prepared = prepare_production(&strict_config(true), ws.path());
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
    // 15.5.3: on a capable Linux kernel every strict layer engages, so the
    // confined "echo hi" runs (no network, no outside-write); on Windows
    // (L0-only) / macOS (not-yet-wired), strict+require fail-closes.
    #[cfg(target_os = "linux")]
    {
        assert!(
            !result.is_error,
            "engaged Linux strict must run the confined echo, got: {:?}",
            result.content
        );
        let text = serde_json::to_string(&result.content).expect("outputs serialize");
        assert!(
            text.contains("hi"),
            "engaged confined echo must produce output, got: {text}"
        );
    }
    #[cfg(not(target_os = "linux"))]
    {
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
    let prepared = prepare_production(&off_config, ws.path());
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
/// diagnostic for the aggregate L1-L3 platform gap ONCE at startup (never per
/// command), runs the command at
/// the L0 baseline under `require = false`, and returns `SandboxUnavailable`
/// before spawn under `require = true`.
#[cfg(windows)]
#[tokio::test]
async fn windows_strict_reports_l0_only() {
    let dir = tempfile::tempdir().unwrap();

    // require=false: fail-open at L0; one aggregate permanent diagnostic.
    let prepared = prepare_production(&strict_config(false), dir.path());
    let startup = prepared.startup_diagnostics();
    assert_eq!(
        startup.len(),
        1,
        "Windows L1-L3 must be one permanent platform capability gap"
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
    let prepared_req = prepare_production(&strict_config(true), dir.path());
    assert_eq!(
        prepared_req.startup_diagnostics().len(),
        1,
        "require=true still surfaces the aggregate permanent gap once"
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
    let prepared_req = prepare_production(&strict_config(true), ws.path());
    assert_eq!(
        prepared_req.startup_diagnostics().len(),
        1,
        "production dispatch surfaces one aggregate Windows permanent gap"
    );
    let (mut tools, startup_diagnostics) =
        CodingHarness::build_tools_with_sandbox(ws.path(), &tool_config, prepared_req);
    assert_eq!(
        startup_diagnostics.len(),
        1,
        "harness startup channel receives one CODE_SANDBOX_UNAVAILABLE diagnostic"
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
    let prepared_open = prepare_production(&strict_config(false), ws.path());
    let (mut tools2, startup2) =
        CodingHarness::build_tools_with_sandbox(ws.path(), &tool_config, prepared_open);
    assert_eq!(
        startup2.len(),
        1,
        "require=false still reports the aggregate gap at startup"
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

// ---------------------------------------------------------------------------
// Phase 15 task 15.5.4 — macOS sandbox-exec strict backend (host-independent
// substrate). The profile/capability/argv model is pure Rust (no macOS kernel,
// no elevated privileges); the runtime (sandbox-exec probe, Confinement launcher
// integration, dispatcher wiring) plus the three native engaged product
// assertions run on a macOS runner only. This test proves the substrate
// invariants on every host.
// ---------------------------------------------------------------------------

/// The macOS strict substrate: the seatbelt deny-overlay profile (deterministic
/// and escaped, with fs/network toggles), the production launcher confinement,
/// and the per-layer capability matrix (L3/syscalls permanently
/// unavailable; L1 fs and L2 network engaged when `sandbox-exec` is usable, with
/// exact missing/unusable reasons). Pure host-independent coverage of the DoD
/// profile tests: path escaping, fs/network toggles, unavailable-tool behavior,
/// and launcher construction.
#[test]
fn macos_profile_and_capability_matrix() {
    use opi_coding_agent::sandbox::LayerAvailability;
    use opi_coding_agent::sandbox::macos;

    // --- capability matrix ---
    // sandbox-exec available -> fs + network engage; syscalls ALWAYS permanently
    // unavailable (sandbox-exec is L1/L2 only).
    let cap_avail = macos::macos_strict_capability(&macos::SandboxExecStatus::Available);
    assert_eq!(
        cap_avail.fs,
        LayerAvailability::Engaged,
        "fs engages when sandbox-exec is available"
    );
    assert_eq!(
        cap_avail.network,
        LayerAvailability::Engaged,
        "network engages when sandbox-exec is available"
    );
    match &cap_avail.syscalls {
        LayerAvailability::PermanentlyUnavailable { reason } => {
            assert!(
                reason.contains("L1/L2"),
                "L3 reason must state macOS is L1/L2-only, got: {reason}"
            );
        }
        other => panic!("syscalls must be PermanentlyUnavailable on macOS, got {other:?}"),
    }

    // sandbox-exec missing -> fs + network temporarily unavailable with the
    // EXACT missing reason; syscalls still permanently unavailable (independent
    // of the helper).
    let cap_miss = macos::macos_strict_capability(&macos::SandboxExecStatus::Missing);
    match &cap_miss.fs {
        LayerAvailability::TemporarilyUnavailable { reason } => {
            assert_eq!(reason, macos::SANDBOX_EXEC_MISSING_REASON);
        }
        other => panic!("missing helper: fs must be TemporarilyUnavailable, got {other:?}"),
    }
    match &cap_miss.network {
        LayerAvailability::TemporarilyUnavailable { reason } => {
            assert_eq!(reason, macos::SANDBOX_EXEC_MISSING_REASON);
        }
        other => panic!("missing helper: network must be TemporarilyUnavailable, got {other:?}"),
    }
    assert!(
        matches!(
            cap_miss.syscalls,
            LayerAvailability::PermanentlyUnavailable { .. }
        ),
        "syscalls stays permanently unavailable regardless of sandbox-exec"
    );

    // sandbox-exec unusable -> stable redacted reason. Probe stderr and I/O
    // display may carry paths/secrets and must never enter diagnostics.
    let canary = "token=TOP-SECRET path=/Users/private/project";
    let cap_unus =
        macos::macos_strict_capability(&macos::SandboxExecStatus::Unusable(canary.to_string()));
    match &cap_unus.fs {
        LayerAvailability::TemporarilyUnavailable { reason } => {
            assert!(
                reason.starts_with(macos::SANDBOX_EXEC_UNUSABLE_PREFIX),
                "unusable reason must use the stable prefix, got: {reason}"
            );
            assert!(
                !reason.contains(canary)
                    && !reason.contains("TOP-SECRET")
                    && !reason.contains("/Users/private"),
                "probe failure details must be sanitized, got: {reason}"
            );
        }
        other => panic!("unusable helper: fs must be TemporarilyUnavailable, got {other:?}"),
    }

    // --- profile rendering: deny-overlay + toggles ---
    // Both layers on -> full deny-overlay (root deny + workspace/temp exceptions)
    // plus the network deny.
    let p_both = macos::render_profile("/Users/a/ws", "/tmp", true, true);
    assert!(
        p_both.contains("(version 1)"),
        "profile has the seatbelt version header"
    );
    assert!(
        p_both.contains("(allow default)"),
        "profile must carry an (allow default) base — seatbelt's default is DENY, so without it the confined child cannot exec or read system files"
    );
    // Seatbelt is last-match-wins: the root deny MUST precede the
    // workspace/temp exceptions so the later, narrower allows punch through.
    let ws_idx = p_both
        .find("(allow file-write* (subpath \"/Users/a/ws\"))")
        .expect("workspace exception present");
    let deny_idx = p_both
        .find("(deny file-write* (subpath \"/\"))")
        .expect("root deny present");
    assert!(
        deny_idx < ws_idx,
        "root deny must precede the workspace exception (seatbelt is last-match-wins)"
    );
    assert!(
        p_both.contains("(deny file-write* (subpath \"/\"))"),
        "deny-overlay root must be present when fs engaged"
    );
    assert!(
        p_both.contains("(allow file-write* (subpath \"/Users/a/ws\"))"),
        "workspace write exception must be present"
    );
    assert!(
        p_both.contains("(allow file-write* (subpath \"/tmp\"))"),
        "temp write exception must be present"
    );
    assert!(
        p_both.contains("(deny network*)"),
        "network deny must be present when network engaged"
    );

    // fs disabled -> no file-write deny overlay (network still denied).
    let p_no_fs = macos::render_profile("/Users/a/ws", "/tmp", false, true);
    assert!(
        !p_no_fs.contains("file-write*"),
        "fs disabled must omit the write overlay"
    );
    assert!(
        p_no_fs.contains("(deny network*)"),
        "network independent of fs"
    );

    // network disabled -> no network deny (fs overlay still present).
    let p_no_net = macos::render_profile("/Users/a/ws", "/tmp", true, false);
    assert!(
        p_no_net.contains("file-write*"),
        "fs overlay independent of network"
    );
    assert!(
        !p_no_net.contains("network*"),
        "network disabled must omit the network deny"
    );

    // --- profile rendering: escaping ---
    // Special chars in the workspace path are backslash-escaped so the raw path
    // never appears verbatim and seatbelt `${var}` expansion is neutralized.
    let nasty = "/Users/a/we\"rd$\\${EVIL}";
    let p_escaped = macos::render_profile(nasty, "/tmp", true, false);
    assert!(
        !p_escaped.contains(nasty),
        "raw special-char workspace must not appear verbatim (escaping applied)"
    );
    assert!(
        p_escaped.contains("\\\""),
        "double-quote must be backslash-escaped"
    );
    assert!(
        p_escaped.contains("\\\\"),
        "backslash must be backslash-escaped"
    );
    assert!(
        p_escaped.contains("\\$"),
        "dollar must be backslash-escaped (neutralizes seatbelt ${{var}} expansion)"
    );

    // --- launcher plan ---
    // Test the actual Confinement representation consumed by the production
    // spawn composer rather than a dead wrapper-argv model.
    let confinement = macos::build_macos_confinement(
        std::path::Path::new("/Users/a/ws"),
        &macos::SandboxExecStatus::Available,
        &[SandboxLayer::Fs, SandboxLayer::Network],
    )
    .expect("available helper with enabled layers builds a launcher");
    let (program, prefix) = confinement
        .launcher_prefix()
        .expect("macOS confinement is a launcher");
    assert_eq!(program, "sandbox-exec");
    assert_eq!(prefix.first().map(String::as_str), Some("-p"));
    let profile = prefix.get(1).expect("profile follows -p");
    assert!(profile.contains("file-write*"));
    assert!(profile.contains("(deny network*)"));

    let fs_only = macos::build_macos_confinement(
        std::path::Path::new("/Users/a/ws"),
        &macos::SandboxExecStatus::Available,
        &[SandboxLayer::Fs],
    )
    .expect("fs-only launcher");
    let fs_profile = &fs_only.launcher_prefix().unwrap().1[1];
    assert!(fs_profile.contains("file-write*"));
    assert!(!fs_profile.contains("network*"));

    let network_only = macos::build_macos_confinement(
        std::path::Path::new("/Users/a/ws"),
        &macos::SandboxExecStatus::Available,
        &[SandboxLayer::Network],
    )
    .expect("network-only launcher");
    let network_profile = &network_only.launcher_prefix().unwrap().1[1];
    assert!(!network_profile.contains("file-write*"));
    assert!(network_profile.contains("(deny network*)"));

    assert!(
        macos::build_macos_confinement(
            std::path::Path::new("/Users/a/ws"),
            &macos::SandboxExecStatus::Available,
            &[],
        )
        .is_none(),
        "no enabled L1/L2 layer must not launch sandbox-exec"
    );
}

// ---------------------------------------------------------------------------
// Phase 15 task 15.5.3 — Linux strict backend capability matrix
// ---------------------------------------------------------------------------

/// The Linux strict backend reports per-layer availability from the OBSERVED
/// Landlock ABI (not a release string). Injecting ABI values covers the
/// release/ABI-mismatch branches (the DoD requires injected mismatch coverage).
/// seccomp (syscalls + socket gate) is ABI-independent; landlock fs needs ABI>=1;
/// landlock TCP bind/connect needs ABI>=4.
#[cfg(target_os = "linux")]
#[test]
fn linux_strict_backend_capability_matrix() {
    use opi_coding_agent::sandbox::linux::{LinuxStrictBackend, linux_strict_capability};
    use opi_coding_agent::sandbox::{LayerAvailability, SandboxLayer};
    use std::path::Path;
    use std::sync::Arc;

    let ws: Arc<Path> = Arc::from(Path::new("/"));
    let all_layers = [
        SandboxLayer::Fs,
        SandboxLayer::Network,
        SandboxLayer::Syscalls,
    ];

    // ABI V4 (Linux 6.7+): every strict layer engages.
    let v4 = LinuxStrictBackend::with_observed_abi(ws.clone(), landlock::ABI::V4);
    for layer in all_layers {
        assert!(
            matches!(v4.availability(layer), LayerAvailability::Engaged),
            "ABI V4: {layer:?} must engage"
        );
    }

    // ABI V3 (Linux 6.2, pre-network): fs + syscalls engage, network does not.
    let v3 = LinuxStrictBackend::with_observed_abi(ws.clone(), landlock::ABI::V3);
    assert!(matches!(
        v3.availability(SandboxLayer::Fs),
        LayerAvailability::Engaged
    ));
    assert!(matches!(
        v3.availability(SandboxLayer::Syscalls),
        LayerAvailability::Engaged
    ));
    match v3.availability(SandboxLayer::Network) {
        LayerAvailability::TemporarilyUnavailable { reason } => {
            assert!(
                reason.contains("ABI"),
                "V3 network gap should reference the ABI, got: {reason}"
            );
        }
        other => panic!("ABI V3 network must be temporarily unavailable, got {other:?}"),
    }

    // ABI Unsupported: fs + network temp-unavailable; syscalls still engages
    // (seccomp is ABI-independent of Landlock).
    let none = LinuxStrictBackend::with_observed_abi(ws, landlock::ABI::Unsupported);
    assert!(matches!(
        none.availability(SandboxLayer::Syscalls),
        LayerAvailability::Engaged
    ));
    assert!(matches!(
        none.availability(SandboxLayer::Fs),
        LayerAvailability::TemporarilyUnavailable { .. }
    ));
    assert!(matches!(
        none.availability(SandboxLayer::Network),
        LayerAvailability::TemporarilyUnavailable { .. }
    ));

    // The capability report distinguishes the seccomp socket-creation layer
    // from the landlock TCP bind/connect layer (two separate fields).
    let cap = linux_strict_capability(landlock::ABI::V4);
    assert!(
        !cap.seccomp_socket_creation.denied_families.is_empty(),
        "seccomp socket-creation layer carries the denied families"
    );
    assert!(
        cap.landlock_tcp_bind_connect.tcp_bind_connect,
        "ABI V4 enables landlock TCP bind/connect"
    );
}

// ---------------------------------------------------------------------------
// Phase 15 task 15.5.3 — engaged product tests (real seccomp + Landlock on a
// capable Linux kernel). A tiny C probe is compiled at test time with `cc`
// (always present where opi builds) and run confined through the production
// `prepare_production` -> `LocalBashOperations::exec` path.
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
mod linux_engaged {
    use super::{
        BashRequest, CancellationToken, Duration, LocalBashOperations, prepare_production,
        strict_config,
    };
    use std::path::{Path, PathBuf};
    use std::process::Command;

    /// Minimal async-signal-safe probe: each subcommand performs one confined
    /// operation and exits 0 (allowed) or 1 (denied with an errno report).
    const PROBE_SRC: &str = r#"
#include <sys/socket.h>
#include <netinet/in.h>
#include <sys/un.h>
#include <unistd.h>
#include <string.h>
#include <stdio.h>
#include <errno.h>
#include <fcntl.h>
#include <stdlib.h>

static int denied(const char* what, int e) { fprintf(stderr, "%s DENIED errno=%d\n", what, e); return 1; }
static int allowed(const char* what) { fprintf(stderr, "%s ALLOWED\n", what); return 0; }

int main(int argc, char** argv) {
    if (argc < 2) { fprintf(stderr, "usage: probe <op> [arg]\n"); return 2; }
    const char* op = argv[1];
    int s;
    if (!strcmp(op, "inet")) {
        s = socket(AF_INET, SOCK_STREAM, 0);
        if (s < 0) return denied("inet", errno);
        close(s); return allowed("inet");
    }
    if (!strcmp(op, "inet6")) {
        s = socket(AF_INET6, SOCK_STREAM, 0);
        if (s < 0) return denied("inet6", errno);
        close(s); return allowed("inet6");
    }
    if (!strcmp(op, "netlink")) {
        s = socket(AF_NETLINK, SOCK_RAW, 0);
        if (s < 0) return denied("netlink", errno);
        close(s); return allowed("netlink");
    }
    if (!strcmp(op, "unix")) {
        // create + bind + listen on a named AF_UNIX SOCK_STREAM server socket,
        // then connect + accept + send + receive (the full IPC path).
        if (argc < 3) return denied("unix-needs-path", ENOENT);
        int srv = socket(AF_UNIX, SOCK_STREAM, 0);
        if (srv < 0) return denied("unix-create", errno);
        struct sockaddr_un addr;
        memset(&addr, 0, sizeof(addr));
        addr.sun_family = AF_UNIX;
        snprintf(addr.sun_path, sizeof(addr.sun_path), "%s", argv[2]);
        unlink(addr.sun_path);
        if (bind(srv, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
            close(srv);
            return denied("unix-bind", errno);
        }
        if (listen(srv, 1) < 0) {
            close(srv);
            return denied("unix-listen", errno);
        }
        int cli = socket(AF_UNIX, SOCK_STREAM, 0);
        if (cli < 0) {
            close(srv);
            return denied("unix-client-create", errno);
        }
        if (connect(cli, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
            close(srv);
            close(cli);
            return denied("unix-connect", errno);
        }
        int acc = accept(srv, NULL, NULL);
        if (acc < 0) {
            close(srv);
            close(cli);
            return denied("unix-accept", errno);
        }
        const char m = 'x';
        char b = 0;
        if (write(cli, &m, 1) != 1) {
            close(srv);
            close(cli);
            close(acc);
            return denied("unix-send", errno);
        }
        if (read(acc, &b, 1) != 1) {
            close(srv);
            close(cli);
            close(acc);
            return denied("unix-recv", errno);
        }
        close(srv);
        close(cli);
        close(acc);
        return allowed("unix");
    }
    if (!strcmp(op, "tcp-bind-fd")) {
        if (argc < 3) return 2;
        int fd = atoi(argv[2]);
        struct sockaddr_in addr; memset(&addr, 0, sizeof(addr));
        addr.sin_family = AF_INET;
        addr.sin_port = 0;
        addr.sin_addr.s_addr = htonl(0x7f000001); /* 127.0.0.1 */
        if (bind(fd, (struct sockaddr*)&addr, sizeof(addr)) < 0) return denied("tcp-bind-fd", errno);
        return allowed("tcp-bind-fd");
    }
    if (!strcmp(op, "write-file")) {
        if (argc < 3) return 2;
        int fd = open(argv[2], O_WRONLY|O_CREAT|O_TRUNC, 0644);
        if (fd < 0) return denied("write", errno);
        write(fd, "x", 1); close(fd); return allowed("write");
    }
    fprintf(stderr, "unknown op: %s\n", op);
    return 2;
}
"#;

    /// Compile the probe into `workspace` and return its binary path.
    pub fn build_probe(workspace: &Path) -> PathBuf {
        let src = workspace.join("sandbox_probe.c");
        std::fs::write(&src, PROBE_SRC).expect("write probe source");
        let bin = workspace.join("sandbox_probe");
        let out = Command::new("cc")
            .arg("-o")
            .arg(&bin)
            .arg(&src)
            .arg("-Wall")
            .arg("-O2")
            .output()
            .expect("cc runs (build-essential is required to build opi)");
        assert!(
            out.status.success(),
            "cc failed to compile the probe: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        bin
    }

    /// A BashRequest that runs the probe with `args` (op [+ operand]) under the
    /// confined production path, cwd = workspace.
    pub fn probe_request(workspace: &Path, probe: &Path, args: &str) -> BashRequest {
        BashRequest {
            command: format!("{} {}", probe.display(), args),
            cwd: workspace.to_path_buf(),
            timeout: Duration::from_secs(15),
            signal: CancellationToken::new(),
            env: vec![],
        }
    }

    /// Build the engaged strict decision for `workspace` and wrap it in the ops.
    pub fn engaged_ops(workspace: &Path) -> LocalBashOperations {
        LocalBashOperations::with_prepared(prepare_production(&strict_config(false), workspace))
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn assert_probe_exit(result: &BashResult, expected: i32, ctx: &str) {
    assert_eq!(
        result.exit_code,
        Some(expected),
        "{ctx}: expected probe exit {expected}, got {:?}",
        result.exit_code
    );
}

/// seccomp L2: new `socket(AF_INET | AF_INET6 | AF_NETLINK)` is denied with a
/// stable errno (EPERM) while AF_UNIX is preserved (next test).
#[cfg(target_os = "linux")]
#[tokio::test]
async fn linux_new_inet_inet6_netlink_sockets_are_denied() {
    let workspace = tempfile::tempdir().unwrap();
    let probe = linux_engaged::build_probe(workspace.path());
    let ops = linux_engaged::engaged_ops(workspace.path());
    for op in ["inet", "inet6", "netlink"] {
        let result = ops
            .exec(linux_engaged::probe_request(workspace.path(), &probe, op))
            .await
            .expect("exec runs");
        assert_probe_exit(&result, 1, op);
    }
}

/// AF_UNIX stream socket create + bind to a workspace path survives the
/// socket-creation gate (IPC remains usable).
#[cfg(target_os = "linux")]
#[tokio::test]
async fn linux_af_unix_survives_socket_creation_gate() {
    let workspace = tempfile::tempdir().unwrap();
    let probe = linux_engaged::build_probe(workspace.path());
    let ops = linux_engaged::engaged_ops(workspace.path());
    let sock_path = workspace.path().join("unix.sock");
    let result = ops
        .exec(linux_engaged::probe_request(
            workspace.path(),
            &probe,
            &format!("unix {}", sock_path.display()),
        ))
        .await
        .expect("exec runs");
    assert_probe_exit(&result, 0, "unix");
}

/// Landlock ABI-4 TCP bind is denied. seccomp denies fresh `socket(AF_INET)`
/// first, so this is exercised through an INHERITED TCP descriptor opened in the
/// (unconfined) parent and passed to the confined child: the child's `bind()` is
/// allowed by seccomp (bind is not gated) but denied by Landlock (no allow-port
/// rules). This isolates the Landlock TCP layer directly.
#[cfg(target_os = "linux")]
#[tokio::test]
async fn linux_landlock_abi4_denies_tcp_bind_connect() {
    use opi_coding_agent::sandbox::linux::LinuxStrictBackend;
    use std::sync::Arc;

    let workspace = tempfile::tempdir().unwrap();

    // Capability guard: the engaged Landlock TCP layer is armed only at ABI >= 4.
    let backend = LinuxStrictBackend::new(Arc::from(workspace.path()));
    let abi = backend.observed_abi();
    assert!(
        matches!(
            abi,
            landlock::ABI::V4 | landlock::ABI::V5 | landlock::ABI::V6 | landlock::ABI::V7
        ),
        "this engaged test requires observed Landlock ABI >= 4 (got {abi:?}); run on a capable kernel"
    );

    // Open a TCP socket in the unconfined parent and clear CLOEXEC so the
    // confined child inherits it across exec.
    let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0) };
    assert!(
        fd >= 0,
        "parent socket() failed: {}",
        std::io::Error::last_os_error()
    );
    unsafe { libc::fcntl(fd, libc::F_SETFD, 0) }; // clear CLOEXEC
    let probe = linux_engaged::build_probe(workspace.path());
    let ops = linux_engaged::engaged_ops(workspace.path());
    let result = ops
        .exec(linux_engaged::probe_request(
            workspace.path(),
            &probe,
            &format!("tcp-bind-fd {fd}"),
        ))
        .await
        .expect("exec runs");
    unsafe { libc::close(fd) };
    // Landlock denies the TCP bind -> probe exits 1.
    assert_probe_exit(&result, 1, "tcp-bind-fd");
}

/// Landlock L1 fs: a write OUTSIDE the configured workspace/temp paths is denied,
/// while a write INSIDE the workspace is allowed. ("Permits only configured
/// workspace/temp writes.")
#[cfg(target_os = "linux")]
#[tokio::test]
async fn linux_engaged_subprocess_denies_requested_access() {
    let workspace = tempfile::tempdir().unwrap();
    // /var/tmp is a world-writable dir that is NOT std::env::temp_dir() (/tmp)
    // and not the workspace, so Landlock (which grants workspace + /tmp) denies
    // writes there.
    let outside = tempfile::Builder::new()
        .prefix("opi-outside-")
        .tempdir_in("/var/tmp")
        .expect("/var/tmp must exist for the outside-write denial test");
    let probe = linux_engaged::build_probe(workspace.path());
    let ops = linux_engaged::engaged_ops(workspace.path());

    // Outside write -> denied (exit 1).
    let outside_target = outside.path().join("denied.txt");
    let result = ops
        .exec(linux_engaged::probe_request(
            workspace.path(),
            &probe,
            &format!("write-file {}", outside_target.display()),
        ))
        .await
        .expect("exec runs");
    assert_probe_exit(&result, 1, "write-outside");

    // Workspace write -> allowed (exit 0).
    let inside_target = workspace.path().join("allowed.txt");
    let result = ops
        .exec(linux_engaged::probe_request(
            workspace.path(),
            &probe,
            &format!("write-file {}", inside_target.display()),
        ))
        .await
        .expect("exec runs");
    assert_probe_exit(&result, 0, "write-workspace");
}

// ---------------------------------------------------------------------------
// Phase 15 task 15.5.4 — macOS engaged product tests (real sandbox-exec on a
// native macOS runner). A tiny C probe is compiled at test time with `cc`
// (clang on macOS) and run confined through the production `prepare_production`
// -> `LocalBashOperations::exec` path under a `sandbox-exec -p <profile>`
// launcher. The three DoD contracts: outside-write deny, network deny,
// workspace+temp write allow. These are `#[cfg(target_os = "macos")]`; a
// wrong-host run compiles them out and is NOT acceptance evidence.
// ---------------------------------------------------------------------------

fn default_macos_strict_config() -> SandboxConfig {
    SandboxConfig {
        mode: SandboxMode::Strict,
        ..SandboxConfig::default()
    }
}

#[test]
fn default_macos_acceptance_config_keeps_all_layer_defaults() {
    let config = default_macos_strict_config();
    assert_eq!(config.mode, SandboxMode::Strict);
    assert!(!config.require);
    assert_eq!(
        (config.fs, config.network, config.syscalls),
        (None, None, None)
    );
}

#[cfg(target_os = "macos")]
mod macos_engaged {
    use super::{
        BashRequest, CancellationToken, Duration, LayerAvailability, LocalBashOperations,
        SandboxLayer, StrictBackend, default_macos_strict_config, prepare_production,
    };
    use opi_coding_agent::sandbox::macos::MacosStrictBackend;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::Arc;

    /// Minimal probe: each op performs one confined operation and exits 0
    /// (allowed) or 1 (denied with an errno report). POSIX-only headers so it
    /// compiles with macOS clang.
    const PROBE_SRC: &str = r#"
#include <sys/socket.h>
#include <netinet/in.h>
#include <unistd.h>
#include <string.h>
#include <stdio.h>
#include <errno.h>
#include <fcntl.h>

int main(int argc, char** argv) {
    if (argc < 2) { fprintf(stderr, "usage: probe <op> [arg]\n"); return 2; }
    const char* op = argv[1];
    int s;
    if (!strcmp(op, "inet")) {
        s = socket(AF_INET, SOCK_STREAM, 0);
        if (s < 0) { fprintf(stderr, "inet-socket DENIED errno=%d\n", errno); return 1; }
        struct sockaddr_in addr;
        memset(&addr, 0, sizeof(addr));
        addr.sin_family = AF_INET;
        addr.sin_port = 0;
        addr.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
        /* sandbox-exec (deny network*) blocks bind/connect, not socket() itself,
           so the probe binds: under the profile bind is denied (EPERM). */
        if (bind(s, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
            fprintf(stderr, "inet-bind DENIED errno=%d\n", errno); close(s); return 1;
        }
        close(s); fprintf(stderr, "inet OK\n"); return 0;
    }
    if (!strcmp(op, "write-file")) {
        if (argc < 3) return 2;
        int fd = open(argv[2], O_WRONLY|O_CREAT|O_TRUNC, 0644);
        if (fd < 0) { fprintf(stderr, "write DENIED errno=%d\n", errno); return 1; }
        write(fd, "x", 1); close(fd); fprintf(stderr, "write OK\n"); return 0;
    }
    fprintf(stderr, "unknown op: %s\n", op);
    return 2;
}
"#;

    /// Compile the probe into `workspace` and return its binary path.
    pub fn build_probe(workspace: &Path) -> PathBuf {
        let src = workspace.join("macos_sandbox_probe.c");
        std::fs::write(&src, PROBE_SRC).expect("write probe source");
        let bin = workspace.join("macos_sandbox_probe");
        let out = Command::new("cc")
            .arg("-o")
            .arg(&bin)
            .arg(&src)
            .arg("-Wall")
            .arg("-O2")
            .output()
            .expect("cc runs (clang is required to build opi on macOS)");
        assert!(
            out.status.success(),
            "cc failed to compile the probe: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        bin
    }

    /// A BashRequest that runs the probe with `args` under the confined
    /// production path, cwd = workspace.
    pub fn probe_request(workspace: &Path, probe: &Path, args: &str) -> BashRequest {
        BashRequest {
            command: format!("{} {}", probe.display(), args),
            cwd: workspace.to_path_buf(),
            timeout: Duration::from_secs(15),
            signal: CancellationToken::new(),
            env: vec![],
        }
    }

    /// Default macOS strict reports the permanent L3 gap as fail-open while
    /// retaining and applying the independently engaged L1/L2 launcher.
    pub fn default_strict_ops(workspace: &Path) -> LocalBashOperations {
        LocalBashOperations::with_prepared(prepare_production(
            &default_macos_strict_config(),
            workspace,
        ))
    }

    /// Capability guard: the engaged tests require sandbox-exec to be usable so
    /// fs+network engage. Panics with the probe status if it did not, so a GHA
    /// failure (MDM block, missing helper) is debuggable instead of a mystery
    /// exit code from the probe.
    pub fn assert_fs_network_engaged(workspace: &Path) {
        let backend = MacosStrictBackend::new(Arc::from(workspace));
        assert!(
            matches!(
                backend.availability(SandboxLayer::Fs),
                LayerAvailability::Engaged
            ),
            "macOS fs must engage (sandbox-exec usable); probe status: {:?}",
            backend.status()
        );
        assert!(
            matches!(
                backend.availability(SandboxLayer::Network),
                LayerAvailability::Engaged
            ),
            "macOS network must engage (sandbox-exec usable); probe status: {:?}",
            backend.status()
        );
    }
}

/// sandbox-exec L1 fs: a write OUTSIDE the configured workspace/temp paths is
/// denied by the seatbelt deny-overlay (`(deny file-write* (subpath "/"))` with
/// workspace+temp exceptions). `/var/tmp` is outside `$TMPDIR` (macOS temp_dir)
/// and outside the workspace, so it is denied.
#[cfg(target_os = "macos")]
#[tokio::test]
async fn macos_engaged_subprocess_denies_outside_write() {
    let workspace = tempfile::tempdir().unwrap();
    macos_engaged::assert_fs_network_engaged(workspace.path());
    let probe = macos_engaged::build_probe(workspace.path());
    let ops = macos_engaged::default_strict_ops(workspace.path());
    let outside = tempfile::Builder::new()
        .prefix("opi-outside-")
        .tempdir_in("/var/tmp")
        .expect("/var/tmp must exist for the outside-write denial test");
    let target = outside.path().join("denied.txt");
    let result = ops
        .exec(macos_engaged::probe_request(
            workspace.path(),
            &probe,
            &format!("write-file {}", target.display()),
        ))
        .await
        .expect("exec runs");
    assert_probe_exit(&result, 1, "write-outside");
}

/// sandbox-exec L2 network: `socket(AF_INET)` may be created, but `bind(2)` is
/// denied by `(deny network*)`.
#[cfg(target_os = "macos")]
#[tokio::test]
async fn macos_engaged_subprocess_denies_network() {
    let workspace = tempfile::tempdir().unwrap();
    macos_engaged::assert_fs_network_engaged(workspace.path());
    let probe = macos_engaged::build_probe(workspace.path());
    let ops = macos_engaged::default_strict_ops(workspace.path());
    let result = ops
        .exec(macos_engaged::probe_request(
            workspace.path(),
            &probe,
            "inet",
        ))
        .await
        .expect("exec runs");
    assert_probe_exit(&result, 1, "inet");
}

/// sandbox-exec L1 fs: writes INSIDE the configured workspace and temp dir are
/// allowed by the deny-overlay exceptions.
#[cfg(target_os = "macos")]
#[tokio::test]
async fn macos_engaged_subprocess_allows_workspace_and_temp_writes() {
    let workspace = tempfile::tempdir().unwrap();
    macos_engaged::assert_fs_network_engaged(workspace.path());
    let probe = macos_engaged::build_probe(workspace.path());
    let ops = macos_engaged::default_strict_ops(workspace.path());

    // Workspace write -> allowed (exit 0).
    let ws_target = workspace.path().join("allowed.txt");
    let r1 = ops
        .exec(macos_engaged::probe_request(
            workspace.path(),
            &probe,
            &format!("write-file {}", ws_target.display()),
        ))
        .await
        .expect("exec runs");
    assert_probe_exit(&r1, 0, "write-workspace");

    // Temp ($TMPDIR) write -> allowed (exit 0).
    let tmp_target = std::env::temp_dir().join("opi_macos_temp_allowed.txt");
    let r2 = ops
        .exec(macos_engaged::probe_request(
            workspace.path(),
            &probe,
            &format!("write-file {}", tmp_target.display()),
        ))
        .await
        .expect("exec runs");
    assert_probe_exit(&r2, 0, "write-temp");
    let _ = std::fs::remove_file(&tmp_target);
}
