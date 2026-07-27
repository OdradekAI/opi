//! Phase 15 task 15.8.1 integration tests: RPC mode never prompts for trust.
//!
//! RPC startup resolves trust headlessly in `main` (via `prepare_project_startup`
//! and `HeadlessPreTrustUi`) before the RPC runner exists, so the runner has no
//! trust-prompt surface and emits no RPC UI request. These tests prove the
//! headless UI used during RPC startup never emits a UI request, and an RPC run
//! over the production JSONL transport produces no trust prompt.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use opi_ai::test_support::{MockProvider, text_response};
use opi_coding_agent::config::OpiConfig;
use opi_coding_agent::policy::ToolSelection;
use opi_coding_agent::project_trust::{
    PreTrustUi, PreTrustUiError, ProjectTrustCli, ProjectTrustResolverRegistry, TrustDecision,
    prepare_project_startup,
};
use opi_coding_agent::rpc::{RpcCommand, RpcRunner};

/// Recording adapter that replicates the [`HeadlessPreTrustUi`] behavior
/// (select/confirm/input -> Unavailable, notify no-op) while counting the
/// request-producing method calls. Proves the headless UI emits no interactive
/// request during preflight: select/input (prompt surfaces) are never called and
/// the ask confirm returns Unavailable rather than issuing a request.
struct RecordingHeadlessUi {
    select_calls: Arc<AtomicUsize>,
    confirm_calls: Arc<AtomicUsize>,
    input_calls: Arc<AtomicUsize>,
}

impl PreTrustUi for RecordingHeadlessUi {
    fn select(
        &self,
        _: &str,
        _: &[&str],
    ) -> Pin<Box<dyn Future<Output = Result<usize, PreTrustUiError>> + Send + '_>> {
        self.select_calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Err(PreTrustUiError::Unavailable) })
    }
    fn confirm(
        &self,
        _: &str,
    ) -> Pin<Box<dyn Future<Output = Result<bool, PreTrustUiError>> + Send + '_>> {
        self.confirm_calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Err(PreTrustUiError::Unavailable) })
    }
    fn input(
        &self,
        _: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, PreTrustUiError>> + Send + '_>> {
        self.input_calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Err(PreTrustUiError::Unavailable) })
    }
    fn notify(&self, _: &str) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(async {})
    }
}

fn init_git(dir: &std::path::Path) {
    std::fs::create_dir_all(dir.join(".git")).expect("create .git marker");
}

/// Collect every value emitted on the RPC output channel until it closes.
async fn collect_output(
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<serde_json::Value>,
) -> Vec<serde_json::Value> {
    let mut out = Vec::new();
    while let Some(value) = rx.recv().await {
        let is_quit = value.get("type").and_then(|v| v.as_str()) == Some("response")
            && value.get("command").and_then(|v| v.as_str()) == Some("quit");
        out.push(value);
        if is_quit {
            break;
        }
    }
    out
}

#[tokio::test]
async fn headless_pre_trust_ui_emits_no_rpc_ui_request() {
    // The headless UI used during RPC startup returns Unavailable / no-op and
    // has no RPC wire, so prepare_project_startup emits no UI request. A
    // resource project falls through to the headless ask -> Undecided.
    let user_config = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    init_git(workspace.path());
    let skill_dir = workspace
        .path()
        .join(".opi")
        .join("skills")
        .join("rpc-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: rpc-skill\ndescription: project skill.\n---\nbody\n",
    )
    .unwrap();

    let mut registry = ProjectTrustResolverRegistry::new();
    let select_calls = Arc::new(AtomicUsize::new(0));
    let confirm_calls = Arc::new(AtomicUsize::new(0));
    let input_calls = Arc::new(AtomicUsize::new(0));
    let ui = RecordingHeadlessUi {
        select_calls: select_calls.clone(),
        confirm_calls: confirm_calls.clone(),
        input_calls: input_calls.clone(),
    };
    let plan = prepare_project_startup(
        ProjectTrustCli::default(),
        &mut registry,
        user_config.path(),
        workspace.path(),
        TrustDecision::Undecided,
        &ui,
    )
    .await
    .unwrap();
    // Unresolved ask -> headless denies project resources.
    assert_eq!(plan.headless_decision(), TrustDecision::Untrusted);
    // No interactive RPC UI request is emitted: the prompt surfaces (select,
    // input) are never called, and the single ask confirm returns Unavailable
    // rather than issuing a request. The wire-level guarantee is covered by
    // `rpc_never_emits_trust_prompt` (which captures the RPC output channel).
    assert_eq!(
        select_calls.load(Ordering::SeqCst),
        0,
        "headless UI never issues a select request"
    );
    assert_eq!(
        input_calls.load(Ordering::SeqCst),
        0,
        "headless UI never issues an input request"
    );
    assert_eq!(
        confirm_calls.load(Ordering::SeqCst),
        1,
        "headless ask reaches confirm exactly once (returning Unavailable)"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_never_emits_trust_prompt() {
    // A resource project run through the production RPC transport produces no
    // trust prompt / UI request: trust was resolved headlessly before the runner
    // was constructed, and the runner owns no trust-UI surface.
    let workspace = tempfile::tempdir().unwrap();
    init_git(workspace.path());
    let skill_dir = workspace
        .path()
        .join(".opi")
        .join("skills")
        .join("rpc-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: rpc-skill\ndescription: project skill.\n---\nbody\n",
    )
    .unwrap();

    let provider = MockProvider::new("mock", vec![text_response("ok")]);
    let mut runner = RpcRunner::new(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        false,
        ToolSelection::Disabled,
        None,
        Vec::new(),
        TrustDecision::Untrusted,
    )
    .expect("rpc runner should construct");

    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (output_tx, mut output_rx) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::spawn(async move { runner.run_with_channels(command_rx, output_tx).await });

    // Drive one prompt turn, then quit.
    command_tx
        .send(RpcCommand::prompt {
            id: None,
            message: "hi".into(),
        })
        .unwrap();
    // Let the turn produce events.
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    command_tx.send(RpcCommand::quit { id: None }).unwrap();

    let output = collect_output(&mut output_rx).await;
    assert_eq!(task.await.unwrap(), 0, "rpc runner exits 0 on quit");

    // No emitted value is a trust prompt or a UI request.
    for value in &output {
        let ty = value.get("type").and_then(|v| v.as_str()).unwrap_or("");
        assert!(
            !ty.eq_ignore_ascii_case("ui_request"),
            "RPC must not emit a UI request: {value}"
        );
        let serialized = value.to_string().to_lowercase();
        assert!(
            !serialized.contains("trust this project"),
            "RPC must not emit a trust prompt: {value}"
        );
    }
}
