//! Task 16.10 SC16-05: the interactive permission broker outcomes via the
//! production `ExecutionRuntime::build` -> `RoutedBashOperations` -> `exec`
//! path, plus the headless fail-closed behavior and the redaction/no-persistence
//! contracts.
//!
//! A mock [`InteractivePermissionBroker`] stands in for the TUI; this proves the
//! keystone exec flow (a routed `ask` is intercepted, the broker is consulted,
//! the choice controls the outcome) adapter-agnostically. TUI rendering + key
//! handling are covered by `permission_prompt_snapshots` (opi-tui) and the widget
//! unit tests. (`interactive_mock.rs` covers the separate task-1.14 harness
//! wiring and is unchanged by this task.)

use std::collections::BTreeMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use opi_coding_agent::config::{
    ExecutionConfig, ExecutionRunMode, ExecutionStrategy, PermissionDecision,
};
use opi_coding_agent::execution::InteractivePermissionBroker;
use opi_coding_agent::execution::permission::PermissionPolicy;
use opi_coding_agent::execution::{
    EnabledIdentity, ExecutionRuntime, IdentitySource, LOCAL_ADAPTER_ID, PermissionManager,
};
use opi_coding_agent::harness::{CodingHarness, ExecutionWiring};
use opi_coding_agent::package_activation::{ActivatedContribution, ActivationError};
use opi_coding_agent::policy::{RunMode, ToolRuntimeConfig, ToolSelection};
use opi_coding_agent::tool::{BashOpError, BashOperations, BashRequest, BashResult};
use opi_tui::{PermissionChoice, PermissionSummary};
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Mock seams
// ---------------------------------------------------------------------------

/// A `BashOperations` sentinel recording every `exec` command and returning a
/// canned in-band result. Proves the local backend is (or is not) reached.
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

/// A store that panics if activated. The local-ask + deny paths never dispatch
/// to an external adapter, so activation is never reached.
struct PanicSource;
impl IdentitySource for PanicSource {
    fn activate(
        &self,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<ActivatedContribution, ActivationError> {
        panic!("permission exec-flow tests must not activate any package");
    }
}

/// A recording [`InteractivePermissionBroker`]: captures every [`PermissionSummary`]
/// it is asked about and returns a fixed [`PermissionChoice`].
struct RecordingBroker {
    choice: PermissionChoice,
    seen: Mutex<Vec<PermissionSummary>>,
}
impl RecordingBroker {
    fn new(choice: PermissionChoice) -> Arc<Self> {
        Arc::new(Self {
            choice,
            seen: Mutex::new(Vec::new()),
        })
    }
    fn seen_summaries(&self) -> Vec<PermissionSummary> {
        self.seen.lock().unwrap().clone()
    }
}
impl InteractivePermissionBroker for RecordingBroker {
    fn resolve_ask(
        &self,
        summary: PermissionSummary,
    ) -> Pin<Box<dyn Future<Output = PermissionChoice> + Send + '_>> {
        self.seen.lock().unwrap().push(summary);
        let choice = self.choice;
        Box::pin(async move { choice })
    }
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

/// fixed backend=`local` with `local = ask`, plus one dummy enabled identity so
/// `ExecutionRuntime::build` takes Branch 2 (routed) — the only branch that
/// constructs the broker-backed `RoutedBashOperations`. Routing still selects
/// `local` (fixed), so the broker fires for the `local` adapter and dispatches
/// directly to the local backend.
fn local_ask_routed() -> (ExecutionConfig, Vec<EnabledIdentity>, PermissionPolicy) {
    let mut perms = BTreeMap::new();
    perms.insert(LOCAL_ADAPTER_ID.to_string(), PermissionDecision::Ask);
    let config = ExecutionConfig {
        strategy: ExecutionStrategy::Fixed,
        backend: LOCAL_ADAPTER_ID.to_string(),
        permissions: perms.clone(),
        ..ExecutionConfig::default()
    };
    let enabled = vec![EnabledIdentity {
        adapter_id: "dummy".to_string(),
        package_name: "dummy-pkg".to_string(),
    }];
    let policy = PermissionPolicy::from_map(perms);
    (config, enabled, policy)
}

async fn exec_code(ops: Arc<dyn BashOperations>, command: &str) -> Option<String> {
    match ops.exec(request(command)).await {
        Ok(_) => None,
        Err(error) => error.diagnostics().first().map(|d| d.code.clone()),
    }
}

// ---------------------------------------------------------------------------
// SC16-05: allow-once / allow-session / deny outcomes
// ---------------------------------------------------------------------------

/// AllowOnce dispatches to the local backend exactly once and records NO
/// session grant, so the next call re-prompts.
#[tokio::test]
async fn allow_once_dispatches_local_without_session_grant() {
    let (config, enabled, policy) = local_ask_routed();
    let manager = Arc::new(PermissionManager::new());
    let broker = RecordingBroker::new(PermissionChoice::AllowOnce);
    let local_ops_recorder: Arc<RecordingOps> = Arc::new(RecordingOps::new());
    let local_ops: Arc<dyn BashOperations> = local_ops_recorder.clone();
    let ops = ExecutionRuntime::build(
        &config,
        ExecutionRunMode::Interactive,
        &enabled,
        &policy,
        Arc::new(PanicSource),
        Arc::clone(&local_ops),
        std::path::Path::new("."),
        "x86_64-pc-windows-msvc",
        "0.8.0",
        Arc::clone(&manager),
        Some(broker.clone()),
    )
    .expect("routed build");

    assert!(ops.exec(request("echo hi")).await.is_ok());
    assert_eq!(
        local_ops_recorder.call_count(),
        1,
        "AllowOnce must dispatch to the local backend once"
    );
    assert!(
        !manager.has_session_grant(LOCAL_ADAPTER_ID),
        "AllowOnce records no session grant"
    );
}

/// AllowSession dispatches AND records a session grant, so a second call skips
/// the broker entirely (no re-prompt).
#[tokio::test]
async fn allow_session_dispatches_and_suppresses_re_prompt() {
    let (config, enabled, policy) = local_ask_routed();
    let manager = Arc::new(PermissionManager::new());
    let broker = RecordingBroker::new(PermissionChoice::AllowSession);
    let local_ops_recorder: Arc<RecordingOps> = Arc::new(RecordingOps::new());
    let local_ops: Arc<dyn BashOperations> = local_ops_recorder.clone();
    let ops = ExecutionRuntime::build(
        &config,
        ExecutionRunMode::Interactive,
        &enabled,
        &policy,
        Arc::new(PanicSource),
        Arc::clone(&local_ops),
        std::path::Path::new("."),
        "x86_64-pc-windows-msvc",
        "0.8.0",
        Arc::clone(&manager),
        Some(broker.clone()),
    )
    .expect("routed build");

    assert!(ops.exec(request("echo one")).await.is_ok());
    assert_eq!(broker.seen_summaries().len(), 1, "first call prompts once");
    assert_eq!(local_ops_recorder.call_count(), 1);

    // Second call: the session grant suppresses the broker.
    assert!(ops.exec(request("echo two")).await.is_ok());
    assert_eq!(
        broker.seen_summaries().len(),
        1,
        "session grant suppresses re-prompting"
    );
    assert_eq!(
        local_ops_recorder.call_count(),
        2,
        "second call still dispatches local"
    );
    assert!(manager.has_session_grant(LOCAL_ADAPTER_ID));
}

/// Deny yields the stable `permission_denied` code and dispatches NOTHING (no
/// `local` fallback).
#[tokio::test]
async fn deny_yields_permission_denied_and_no_dispatch() {
    let (config, enabled, policy) = local_ask_routed();
    let manager = Arc::new(PermissionManager::new());
    let broker = RecordingBroker::new(PermissionChoice::Deny);
    let local_ops_recorder: Arc<RecordingOps> = Arc::new(RecordingOps::new());
    let local_ops: Arc<dyn BashOperations> = local_ops_recorder.clone();
    let ops = ExecutionRuntime::build(
        &config,
        ExecutionRunMode::Interactive,
        &enabled,
        &policy,
        Arc::new(PanicSource),
        Arc::clone(&local_ops),
        std::path::Path::new("."),
        "x86_64-pc-windows-msvc",
        "0.8.0",
        Arc::clone(&manager),
        Some(broker.clone()),
    )
    .expect("routed build");

    let code = exec_code(ops, "echo hi").await;
    assert_eq!(code.as_deref(), Some("permission_denied"));
    assert_eq!(
        local_ops_recorder.call_count(),
        0,
        "Deny must not dispatch (no local fallback)"
    );
}

/// SC16-14 interactive surface at the PRODUCTION harness chokepoint: a routed
/// external with `ask` policy, Interactive mode, and a denying broker drives the
/// FULL `CodingHarness::build_tools` -> `BashTool::execute` -> `ToolResult`
/// path (the same wiring the interactive startup installs). The stable
/// `permission_denied` code survives into `ToolResult.diagnostics` — proving the
/// interactive/TUI surface carries the same stable redacted code as NDJSON/RPC.
#[tokio::test]
async fn interactive_harness_chokepoint_surfaces_permission_denied() {
    let workspace = tempfile::tempdir().unwrap();
    let mut perms = BTreeMap::new();
    perms.insert("opi-sandbox".to_string(), PermissionDecision::Ask);
    let wiring = ExecutionWiring {
        config: ExecutionConfig {
            strategy: ExecutionStrategy::Fixed,
            backend: "opi-sandbox".to_string(),
            permissions: perms.clone(),
            ..ExecutionConfig::default()
        },
        enabled: vec![EnabledIdentity {
            adapter_id: "opi-sandbox".to_string(),
            package_name: "mock-pkg".to_string(),
        }],
        policy: PermissionPolicy::from_map(perms),
        store: Arc::new(PanicSource),
        mode: ExecutionRunMode::Interactive,
        host_target: opi_coding_agent::package_activation::host_target_triple().to_string(),
        host_opi_version: opi_coding_agent::package_activation::host_opi_version().to_string(),
        manager: Arc::new(PermissionManager::new()),
        broker: Some(RecordingBroker::new(PermissionChoice::Deny)),
    };
    let tool_config =
        ToolRuntimeConfig::resolve(RunMode::Interactive, true, ToolSelection::Default)
            .expect("interactive tool config");
    let (mut tools, startup_diagnostics) =
        CodingHarness::build_tools(workspace.path(), &tool_config, &wiring);
    assert!(
        startup_diagnostics.is_empty(),
        "routed ask must not warn at startup: {startup_diagnostics:?}"
    );
    let bash = tools
        .iter_mut()
        .find(|t| t.definition().name == "bash")
        .expect("routed bash tool present");
    let result = bash
        .execute(
            "interactive-deny",
            serde_json::json!({"command": "echo hi", "timeout_secs": 5}),
            CancellationToken::new(),
            None,
        )
        .await
        .expect("bash tool executes");
    assert!(result.is_error, "deny must fail the tool turn");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "permission_denied"),
        "the stable permission_denied code must reach ToolResult.diagnostics: {:?}",
        result.diagnostics
    );
    // Remediation must ride along and stay command-text-free.
    let pd = result
        .diagnostics
        .iter()
        .find(|d| d.code == "permission_denied")
        .expect("permission_denied diagnostic");
    let remediation = pd
        .context
        .get("remediation")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    assert!(
        !remediation.is_empty(),
        "permission_denied must carry actionable remediation: {pd:?}"
    );
    assert!(
        !remediation.contains("echo"),
        "permission_denied remediation must not leak command text: {remediation}"
    );
}

// ---------------------------------------------------------------------------
// Headless / no-broker fail-closed (DoD: "headless ask returns permission_required")
// ---------------------------------------------------------------------------

/// NonInteractive `ask` never consults the broker and surfaces
/// `permission_required`.
#[tokio::test]
async fn headless_noninteractive_yields_permission_required_no_broker_call() {
    let (config, enabled, policy) = local_ask_routed();
    let manager = Arc::new(PermissionManager::new());
    let broker = RecordingBroker::new(PermissionChoice::AllowSession);
    let local_ops_recorder: Arc<RecordingOps> = Arc::new(RecordingOps::new());
    let local_ops: Arc<dyn BashOperations> = local_ops_recorder.clone();
    let ops = ExecutionRuntime::build(
        &config,
        ExecutionRunMode::NonInteractive,
        &enabled,
        &policy,
        Arc::new(PanicSource),
        Arc::clone(&local_ops),
        std::path::Path::new("."),
        "x86_64-pc-windows-msvc",
        "0.8.0",
        Arc::clone(&manager),
        Some(broker.clone()),
    )
    .expect("routed build");

    let code = exec_code(ops, "echo hi").await;
    assert_eq!(code.as_deref(), Some("permission_required"));
    assert!(
        broker.seen_summaries().is_empty(),
        "headless must not prompt"
    );
    assert_eq!(local_ops_recorder.call_count(), 0);
}

/// Rpc `ask` likewise surfaces `permission_required` and never prompts.
#[tokio::test]
async fn headless_rpc_yields_permission_required_no_broker_call() {
    let (config, enabled, policy) = local_ask_routed();
    let manager = Arc::new(PermissionManager::new());
    let broker = RecordingBroker::new(PermissionChoice::AllowSession);
    let local_ops_recorder: Arc<RecordingOps> = Arc::new(RecordingOps::new());
    let local_ops: Arc<dyn BashOperations> = local_ops_recorder.clone();
    let ops = ExecutionRuntime::build(
        &config,
        ExecutionRunMode::Rpc,
        &enabled,
        &policy,
        Arc::new(PanicSource),
        Arc::clone(&local_ops),
        std::path::Path::new("."),
        "x86_64-pc-windows-msvc",
        "0.8.0",
        Arc::clone(&manager),
        Some(broker.clone()),
    )
    .expect("routed build");

    let code = exec_code(ops, "echo hi").await;
    assert_eq!(code.as_deref(), Some("permission_required"));
    assert!(broker.seen_summaries().is_empty());
}

/// An Interactive `ask` with NO broker installed is fail-closed:
/// `permission_required`, never a silent dispatch or local fallback.
#[tokio::test]
async fn no_broker_interactive_is_fail_closed_permission_required() {
    let (config, enabled, policy) = local_ask_routed();
    let manager = Arc::new(PermissionManager::new());
    let local_ops_recorder: Arc<RecordingOps> = Arc::new(RecordingOps::new());
    let local_ops: Arc<dyn BashOperations> = local_ops_recorder.clone();
    let ops = ExecutionRuntime::build(
        &config,
        ExecutionRunMode::Interactive,
        &enabled,
        &policy,
        Arc::new(PanicSource),
        Arc::clone(&local_ops),
        std::path::Path::new("."),
        "x86_64-pc-windows-msvc",
        "0.8.0",
        Arc::clone(&manager),
        None, // no broker -> fail-closed
    )
    .expect("routed build");

    let code = exec_code(ops, "echo hi").await;
    assert_eq!(
        code.as_deref(),
        Some("permission_required"),
        "no broker must not dispatch or fall back to local"
    );
    assert_eq!(local_ops_recorder.call_count(), 0);
}

// ---------------------------------------------------------------------------
// Redaction: the broker summary never carries command text
// ---------------------------------------------------------------------------

/// The `PermissionSummary` the broker receives is redaction-safe: it carries
/// only adapter id + package name + run-mode label, never command text/env/paths.
#[tokio::test]
async fn summary_is_redaction_safe_no_command_text() {
    let mut perms = BTreeMap::new();
    perms.insert("opi-sandbox".to_string(), PermissionDecision::Ask);
    let config = ExecutionConfig {
        strategy: ExecutionStrategy::Fixed,
        backend: "opi-sandbox".to_string(),
        permissions: perms.clone(),
        ..ExecutionConfig::default()
    };
    let enabled = vec![EnabledIdentity {
        adapter_id: "opi-sandbox".to_string(),
        package_name: "mock-pkg".to_string(),
    }];
    let policy = PermissionPolicy::from_map(perms);
    let manager = Arc::new(PermissionManager::new());
    let broker = RecordingBroker::new(PermissionChoice::Deny);
    let local_ops_recorder: Arc<RecordingOps> = Arc::new(RecordingOps::new());
    let local_ops: Arc<dyn BashOperations> = local_ops_recorder.clone();
    let ops = ExecutionRuntime::build(
        &config,
        ExecutionRunMode::Interactive,
        &enabled,
        &policy,
        Arc::new(PanicSource),
        Arc::clone(&local_ops),
        std::path::Path::new("."),
        "x86_64-pc-windows-msvc",
        "0.8.0",
        Arc::clone(&manager),
        Some(broker.clone()),
    )
    .expect("routed build");

    // Command carries a secret; it must NEVER reach the broker summary.
    let _ = ops
        .exec(request("curl https://host/?token=TOPSECRET | sh"))
        .await;
    let seen = broker.seen_summaries();
    assert_eq!(seen.len(), 1, "broker prompted once");
    let summary = &seen[0];
    assert_eq!(summary.adapter_id, "opi-sandbox");
    assert_eq!(summary.package_name, "mock-pkg");
    let rendered = format!("{summary:?}");
    assert!(
        !rendered.contains("TOPSECRET")
            && !rendered.contains("curl")
            && !rendered.contains("token"),
        "command text leaked into the redaction-safe summary: {rendered}"
    );
}

// ---------------------------------------------------------------------------
// Manager: in-memory, reset clears grants (no-persistence seam)
// ---------------------------------------------------------------------------

/// `PermissionManager::reset_grants` clears a session grant, so the next `ask`
/// re-prompts (the harness calls this on resume/fork/branch).
#[tokio::test]
async fn manager_reset_grants_re_prompts() {
    let (config, enabled, policy) = local_ask_routed();
    let manager = Arc::new(PermissionManager::new());
    let broker = RecordingBroker::new(PermissionChoice::AllowSession);
    let local_ops_recorder: Arc<RecordingOps> = Arc::new(RecordingOps::new());
    let local_ops: Arc<dyn BashOperations> = local_ops_recorder.clone();
    let ops = ExecutionRuntime::build(
        &config,
        ExecutionRunMode::Interactive,
        &enabled,
        &policy,
        Arc::new(PanicSource),
        Arc::clone(&local_ops),
        std::path::Path::new("."),
        "x86_64-pc-windows-msvc",
        "0.8.0",
        Arc::clone(&manager),
        Some(broker.clone()),
    )
    .expect("routed build");

    assert!(ops.exec(request("echo one")).await.is_ok());
    assert_eq!(broker.seen_summaries().len(), 1);

    // Simulate an in-process session switch: the harness resets grants.
    manager.reset_grants();
    assert!(!manager.has_session_grant(LOCAL_ADAPTER_ID));

    assert!(ops.exec(request("echo two")).await.is_ok());
    assert_eq!(
        broker.seen_summaries().len(),
        2,
        "after reset, the next ask re-prompts"
    );
}

/// Two independently-built managers do not share grants (fresh-per-harness).
#[test]
fn two_managers_do_not_share_grants() {
    let a = PermissionManager::new();
    let b = PermissionManager::new();
    a.grant_session("opi-sandbox");
    assert!(a.has_session_grant("opi-sandbox"));
    assert!(
        !b.has_session_grant("opi-sandbox"),
        "separate managers must not share grants"
    );
}

// ---------------------------------------------------------------------------
// TuiPermissionBroker: relay + cancellation/drop safety (audit must-fix #5)
// ---------------------------------------------------------------------------

use opi_coding_agent::interactive::{PermissionPromptRequest, TuiPermissionBroker};

fn redaction_safe_summary() -> PermissionSummary {
    PermissionSummary {
        adapter_id: "opi-sandbox".to_string(),
        package_name: "mock-pkg".to_string(),
        run_mode_label: "interactive".to_string(),
    }
}

/// The broker relays the event loop's choice back to the waiting tool call and
/// forwards the redaction-safe summary to the loop.
#[tokio::test]
async fn tui_broker_relays_the_loop_choice_and_summary() {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<PermissionPromptRequest>(8);
    let broker = Arc::new(TuiPermissionBroker::new(tx));
    let b = broker.clone();
    let handle = tokio::spawn(async move { b.resolve_ask(redaction_safe_summary()).await });

    // Event-loop side: receive the request, send AllowSession.
    let req = rx.recv().await.expect("prompt request arrived");
    assert_eq!(req.summary.adapter_id, "opi-sandbox");
    req.responder.send(PermissionChoice::AllowSession).unwrap();

    assert_eq!(handle.await.unwrap(), PermissionChoice::AllowSession);
}

/// A dropped receiver (the loop is gone — terminal close / shutdown) resolves
/// the broker to `Deny`, so the tool call fails closed with `permission_denied`
/// instead of hanging or panicking.
#[tokio::test]
async fn tui_broker_dropped_receiver_resolves_to_deny() {
    let (tx, rx) = tokio::sync::mpsc::channel::<PermissionPromptRequest>(8);
    let broker = TuiPermissionBroker::new(tx);
    drop(rx); // loop gone
    let choice = broker.resolve_ask(redaction_safe_summary()).await;
    assert_eq!(choice, PermissionChoice::Deny);
}

/// A dropped responder (the loop accepted the request then closed before
/// answering) also resolves to `Deny`.
#[tokio::test]
async fn tui_broker_dropped_responder_resolves_to_deny() {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<PermissionPromptRequest>(8);
    let broker = Arc::new(TuiPermissionBroker::new(tx));
    let b = broker.clone();
    let handle = tokio::spawn(async move { b.resolve_ask(redaction_safe_summary()).await });

    let req = rx.recv().await.expect("prompt request arrived");
    drop(req.responder); // loop closed without answering
    assert_eq!(handle.await.unwrap(), PermissionChoice::Deny);
}
