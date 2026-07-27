//! Phase 15 task 15.8.2 integration tests: the interactive trust prompt fires
//! only for an undecided project with trust-requiring resources, the guarded
//! startup operations do not run before the prompt resolves, and every
//! pre-decided source bypasses the prompt.
//!
//! These drive the real `prepare_project_startup`, `resolve_interactive_trust_decision`,
//! `apply_ui_choice`, `ProjectTrustStore`, and `CodingHarnessBuilder::trust_decision`
//! symbols; only the terminal-bound prompt is faked (`FakePrompt`/`PanicPrompt`),
//! since the real TUI needs a TTY. No host kernel feature is required; no
//! provider is contacted.

use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;

use opi_ai::test_support::MockProvider;
use opi_coding_agent::config::{OpiConfig, merge_project_config};
use opi_coding_agent::harness::CodingHarness;
use opi_coding_agent::interactive::{InteractiveTrustPrompt, resolve_interactive_trust_decision};
use opi_coding_agent::project_trust::{
    HeadlessPreTrustUi, PreTrustUi, PreTrustUiError, ProjectTrustCli, ProjectTrustResolver,
    ProjectTrustResolverRegistry, ProjectTrustStore, TrustContext, TrustDecision, TrustError,
    TrustVote, prepare_project_startup,
};
use opi_coding_agent::runtime_packages::start_installed_package_runtime_with_trust;
use opi_tui::TrustChoice;
use tokio::sync::oneshot;

// ---------------------------------------------------------------------------
// Fakes
// ---------------------------------------------------------------------------

/// Recording fake resolver: returns a configured vote (15.8.1 pattern).
struct FakeResolver {
    vote: TrustVote,
}
impl ProjectTrustResolver for FakeResolver {
    fn resolve(
        &self,
        _ctx: &TrustContext,
        _ui: &dyn PreTrustUi,
    ) -> Pin<Box<dyn Future<Output = TrustVote> + Send + '_>> {
        let vote = self.vote;
        Box::pin(async move { vote })
    }
}

/// Prompt that resolves from a oneshot the test controls. `ask` taking the
/// receiver models the production await on the user's choice; dropping the
/// sender models a cancelled/closed prompt (`None`).
struct FakePrompt {
    rx: Option<oneshot::Receiver<TrustChoice>>,
}
impl InteractiveTrustPrompt for FakePrompt {
    fn ask(
        &mut self,
        _project_path: &Path,
    ) -> Pin<Box<dyn Future<Output = Option<TrustChoice>> + Send + '_>> {
        let rx = self.rx.take().expect("FakePrompt.ask called exactly once");
        Box::pin(async move { rx.await.ok() })
    }
}

/// Prompt whose `ask` panics. Used to prove a bypass path never renders it.
struct PanicPrompt;
impl InteractiveTrustPrompt for PanicPrompt {
    fn ask(
        &mut self,
        _project_path: &Path,
    ) -> Pin<Box<dyn Future<Output = Option<TrustChoice>> + Send + '_>> {
        panic!("trust prompt must not be reached on a bypass path");
    }
}

fn init_git(dir: &Path) {
    std::fs::create_dir_all(dir.join(".git")).expect("create .git marker");
}

fn write_project_skill(workspace: &Path) {
    let skill_dir = workspace.join(".opi").join("skills").join("proj-skill");
    std::fs::create_dir_all(&skill_dir).expect("create proj skill dir");
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: proj-skill\ndescription: Project-only skill.\n---\nPROJ SKILL BODY\n",
    )
    .expect("write proj skill");
}

/// Run `prepare_project_startup` with an empty (standard-CLI) registry and the
/// headless UI (resolvers are empty, so the UI is never invoked).
async fn plan_with_empty_registry(
    cli: ProjectTrustCli,
    user_config: &Path,
    workspace: &Path,
    global_default: TrustDecision,
) -> opi_coding_agent::project_trust::ProjectStartupPlan {
    let mut registry = ProjectTrustResolverRegistry::new();
    prepare_project_startup(
        cli,
        &mut registry,
        user_config,
        workspace,
        global_default,
        &HeadlessPreTrustUi,
    )
    .await
    .expect("plan")
}

// ---------------------------------------------------------------------------
// Bypass: pre-decided sources never render the prompt
// ---------------------------------------------------------------------------

#[tokio::test]
async fn interactive_predecided_paths_bypass_prompt() {
    let user_config = tempfile::tempdir().unwrap();

    // CLI --trust -> Trusted, no prompt.
    {
        let workspace = tempfile::tempdir().unwrap();
        init_git(workspace.path());
        write_project_skill(workspace.path());
        let plan = plan_with_empty_registry(
            ProjectTrustCli {
                trust: true,
                no_trust: false,
            },
            user_config.path(),
            workspace.path(),
            TrustDecision::Undecided,
        )
        .await;
        let mut prompt = PanicPrompt;
        let decision = resolve_interactive_trust_decision(
            &plan,
            user_config.path(),
            workspace.path(),
            &mut prompt,
        )
        .await
        .unwrap();
        assert_eq!(decision, TrustDecision::Trusted);
    }

    // CLI --no-trust -> Untrusted, no prompt.
    {
        let workspace = tempfile::tempdir().unwrap();
        init_git(workspace.path());
        write_project_skill(workspace.path());
        let plan = plan_with_empty_registry(
            ProjectTrustCli {
                trust: false,
                no_trust: true,
            },
            user_config.path(),
            workspace.path(),
            TrustDecision::Undecided,
        )
        .await;
        let mut prompt = PanicPrompt;
        let decision = resolve_interactive_trust_decision(
            &plan,
            user_config.path(),
            workspace.path(),
            &mut prompt,
        )
        .await
        .unwrap();
        assert_eq!(decision, TrustDecision::Untrusted);
    }

    // Embedder resolver Trust -> Trusted, no prompt.
    {
        let workspace = tempfile::tempdir().unwrap();
        init_git(workspace.path());
        write_project_skill(workspace.path());
        let mut registry = ProjectTrustResolverRegistry::new();
        registry
            .register(Arc::new(FakeResolver {
                vote: TrustVote::Trust,
            }))
            .unwrap();
        let plan = prepare_project_startup(
            ProjectTrustCli::default(),
            &mut registry,
            user_config.path(),
            workspace.path(),
            TrustDecision::Undecided,
            &HeadlessPreTrustUi,
        )
        .await
        .unwrap();
        let mut prompt = PanicPrompt;
        let decision = resolve_interactive_trust_decision(
            &plan,
            user_config.path(),
            workspace.path(),
            &mut prompt,
        )
        .await
        .unwrap();
        assert_eq!(decision, TrustDecision::Trusted);
    }

    // Embedder resolver Deny -> Untrusted, no prompt.
    {
        let workspace = tempfile::tempdir().unwrap();
        init_git(workspace.path());
        write_project_skill(workspace.path());
        let mut registry = ProjectTrustResolverRegistry::new();
        registry
            .register(Arc::new(FakeResolver {
                vote: TrustVote::Deny,
            }))
            .unwrap();
        let plan = prepare_project_startup(
            ProjectTrustCli::default(),
            &mut registry,
            user_config.path(),
            workspace.path(),
            TrustDecision::Undecided,
            &HeadlessPreTrustUi,
        )
        .await
        .unwrap();
        let mut prompt = PanicPrompt;
        let decision = resolve_interactive_trust_decision(
            &plan,
            user_config.path(),
            workspace.path(),
            &mut prompt,
        )
        .await
        .unwrap();
        assert_eq!(decision, TrustDecision::Untrusted);
    }

    // Stored allow -> Trusted, no prompt.
    {
        let workspace = tempfile::tempdir().unwrap();
        init_git(workspace.path());
        write_project_skill(workspace.path());
        ProjectTrustStore::load(user_config.path())
            .unwrap()
            .record(workspace.path(), true)
            .unwrap();
        let plan = plan_with_empty_registry(
            ProjectTrustCli::default(),
            user_config.path(),
            workspace.path(),
            TrustDecision::Undecided,
        )
        .await;
        let mut prompt = PanicPrompt;
        let decision = resolve_interactive_trust_decision(
            &plan,
            user_config.path(),
            workspace.path(),
            &mut prompt,
        )
        .await
        .unwrap();
        assert_eq!(decision, TrustDecision::Trusted);
    }

    // Stored deny -> Untrusted, no prompt.
    {
        let workspace = tempfile::tempdir().unwrap();
        init_git(workspace.path());
        write_project_skill(workspace.path());
        ProjectTrustStore::load(user_config.path())
            .unwrap()
            .record(workspace.path(), false)
            .unwrap();
        let plan = plan_with_empty_registry(
            ProjectTrustCli::default(),
            user_config.path(),
            workspace.path(),
            TrustDecision::Undecided,
        )
        .await;
        let mut prompt = PanicPrompt;
        let decision = resolve_interactive_trust_decision(
            &plan,
            user_config.path(),
            workspace.path(),
            &mut prompt,
        )
        .await
        .unwrap();
        assert_eq!(decision, TrustDecision::Untrusted);
    }

    // [defaults] default_project_trust = always -> Trusted, no prompt.
    {
        let workspace = tempfile::tempdir().unwrap();
        init_git(workspace.path());
        write_project_skill(workspace.path());
        let plan = plan_with_empty_registry(
            ProjectTrustCli::default(),
            user_config.path(),
            workspace.path(),
            TrustDecision::Trusted,
        )
        .await;
        let mut prompt = PanicPrompt;
        let decision = resolve_interactive_trust_decision(
            &plan,
            user_config.path(),
            workspace.path(),
            &mut prompt,
        )
        .await
        .unwrap();
        assert_eq!(decision, TrustDecision::Trusted);
    }

    // [defaults] default_project_trust = never -> Untrusted, no prompt.
    {
        let workspace = tempfile::tempdir().unwrap();
        init_git(workspace.path());
        write_project_skill(workspace.path());
        let plan = plan_with_empty_registry(
            ProjectTrustCli::default(),
            user_config.path(),
            workspace.path(),
            TrustDecision::Untrusted,
        )
        .await;
        let mut prompt = PanicPrompt;
        let decision = resolve_interactive_trust_decision(
            &plan,
            user_config.path(),
            workspace.path(),
            &mut prompt,
        )
        .await
        .unwrap();
        assert_eq!(decision, TrustDecision::Untrusted);
    }

    // No trust-requiring resources -> Trusted, no prompt (no gate fires).
    {
        let workspace = tempfile::tempdir().unwrap();
        init_git(workspace.path());
        // bare workspace, no .opi resources
        let plan = plan_with_empty_registry(
            ProjectTrustCli::default(),
            user_config.path(),
            workspace.path(),
            TrustDecision::Undecided,
        )
        .await;
        let mut prompt = PanicPrompt;
        let decision = resolve_interactive_trust_decision(
            &plan,
            user_config.path(),
            workspace.path(),
            &mut prompt,
        )
        .await
        .unwrap();
        assert_eq!(decision, TrustDecision::Trusted);
    }
}

// ---------------------------------------------------------------------------
// Ordering: build does not precede the prompt's oneshot; cancel denies safely
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn interactive_prompt_precedes_project_startup_side_effects() {
    let user_config = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    init_git(workspace.path());
    write_project_skill(workspace.path());

    // Undecided plan (ask default, empty registry, no store entry, resources present).
    let plan = plan_with_empty_registry(
        ProjectTrustCli::default(),
        user_config.path(),
        workspace.path(),
        TrustDecision::Undecided,
    )
    .await;
    assert_eq!(
        plan.decision,
        TrustDecision::Undecided,
        "ask position reached"
    );

    let (choice_tx, choice_rx) = oneshot::channel::<TrustChoice>();
    let prompt = FakePrompt {
        rx: Some(choice_rx),
    };
    // Cell filled with the REAL harness build outcome `(decision, project skill
    // loaded?)` once the build chain completes. `None` until then.
    let built: Arc<Mutex<Option<(TrustDecision, bool)>>> = Arc::new(Mutex::new(None));

    // What this test proves: (1) `resolve_interactive_trust_decision` awaits
    // `prompt.ask()` for an undecided plan; (2) the resolved decision drives a
    // REAL CodingHarness build whose `discover_resources` gate loads/gates the
    // project skill accordingly; (3) the build consumes the decision (a data
    // dependency), so it structurally cannot run before the prompt resolves. The
    // build composed here is the same trust-gated sequence `run_interactive_core`
    // performs (decision-gated config merge + package/adapter runtime +
    // CodingHarnessBuilder::build); `build_provider_bundle` is substituted with a
    // MockProvider because it is provider construction (not a trust-gated
    // resource load) and needs provider auth this test does not have.
    // `run_interactive_core` itself is bin-private and hardcodes TuiTrustPrompt
    // (no injection seam), so it cannot be driven from this integration test; the
    // prompt-before-build ordering in main.rs is additionally guaranteed by two
    // sequential `rt.block_on(...)` calls (resolve_interactive_trust_config, then
    // run_interactive).
    let built_task = built.clone();
    let user_config_dir = user_config.path().to_path_buf();
    let workspace_root = workspace.path().to_path_buf();
    let pre_config = OpiConfig::default();
    let task = tokio::spawn(async move {
        let mut p = prompt;
        let decision =
            resolve_interactive_trust_decision(&plan, &user_config_dir, &workspace_root, &mut p)
                .await
                .unwrap();
        // --- REAL trust-gated build operations (after the oneshot) ---
        let config = if matches!(decision, TrustDecision::Untrusted) {
            pre_config
        } else {
            merge_project_config(pre_config, &workspace_root).expect("merge project config")
        };
        let runtime =
            start_installed_package_runtime_with_trust(&workspace_root, &user_config_dir, decision)
                .await;
        let harness = CodingHarness::builder(
            Box::new(MockProvider::new("mock", Vec::new())),
            "mock:mock-model".into(),
            config,
            workspace_root,
            decision,
        )
        .extension_registry(runtime.extension_registry)
        .installed_packages(runtime.installed_packages)
        .startup_diagnostics(runtime.diagnostics)
        .build();
        let project_skill_loaded = harness.system_prompt().contains("proj-skill");
        *built_task.lock().unwrap() = Some((decision, project_skill_loaded));
    });

    // While the prompt is unresolved, the trust-gated build has not completed
    // (the cell is still empty) — the build cannot pass the pending oneshot await
    // because it consumes the decision the prompt produces.
    tokio::time::sleep(std::time::Duration::from_millis(80)).await;
    assert!(
        built.lock().unwrap().is_none(),
        "trust-gated build (merge + package runtime + harness) must not complete before the prompt resolves"
    );

    // Resolve the prompt to Trust; the real build chain then runs with Trusted
    // and loads the project skill.
    choice_tx.send(TrustChoice::Trust).unwrap();
    task.await.expect("build-chain task");
    let (decision, project_skill_loaded) = built.lock().unwrap().take().unwrap();
    assert_eq!(decision, TrustDecision::Trusted);
    assert!(
        project_skill_loaded,
        "trusted decision must load the project skill through the real harness build"
    );

    // The durable choice was persisted.
    assert_eq!(
        ProjectTrustStore::load(user_config.path())
            .unwrap()
            .decide(workspace.path()),
        TrustDecision::Trusted,
    );

    // Cancel/closed prompt -> safe deny, no resources. A fresh undecided plan;
    // the sender is dropped (channel closed) before the await so ask() yields None.
    let workspace2 = tempfile::tempdir().unwrap();
    init_git(workspace2.path());
    write_project_skill(workspace2.path());
    let plan2 = plan_with_empty_registry(
        ProjectTrustCli::default(),
        user_config.path(),
        workspace2.path(),
        TrustDecision::Undecided,
    )
    .await;
    let (tx2, rx2) = oneshot::channel::<TrustChoice>();
    drop(tx2); // close the channel so the prompt's await resolves to None (cancel)
    let mut prompt2 = FakePrompt { rx: Some(rx2) };
    let decision2 = resolve_interactive_trust_decision(
        &plan2,
        user_config.path(),
        workspace2.path(),
        &mut prompt2,
    )
    .await
    .unwrap();
    assert_eq!(
        decision2,
        TrustDecision::Untrusted,
        "cancelled prompt denies the project"
    );
    // No durable decision recorded for the cancelled project.
    assert_eq!(
        ProjectTrustStore::load(user_config.path())
            .unwrap()
            .decide(workspace2.path()),
        TrustDecision::Undecided,
        "cancel must not persist"
    );

    // An untrusted decision loads no project resources: the harness built with
    // trust_decision=Untrusted omits the project skill.
    let provider = MockProvider::new("mock", Vec::new());
    let harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace2.path().to_path_buf(),
        TrustDecision::Untrusted,
    )
    .build();
    assert!(
        !harness.system_prompt().contains("proj-skill"),
        "untrusted project skill must not load: {}",
        harness.system_prompt()
    );

    // Silence unused-import lint for the PreTrustUiError path used by fakes.
    let _: PreTrustUiError = PreTrustUiError::Unavailable;
}

// ---------------------------------------------------------------------------
// Ask: each TrustChoice maps to the right decision + persistence
// ---------------------------------------------------------------------------

#[tokio::test]
async fn interactive_ask_applies_each_choice() {
    for choice in TrustChoice::all() {
        let user_config = tempfile::tempdir().unwrap();
        let workspace = tempfile::tempdir().unwrap();
        init_git(workspace.path());
        write_project_skill(workspace.path());

        let plan = plan_with_empty_registry(
            ProjectTrustCli::default(),
            user_config.path(),
            workspace.path(),
            TrustDecision::Undecided,
        )
        .await;

        let (tx, rx) = oneshot::channel::<TrustChoice>();
        let prompt = FakePrompt { rx: Some(rx) };
        let user_config_dir = user_config.path().to_path_buf();
        let workspace_root = workspace.path().to_path_buf();
        let resolver = tokio::spawn(async move {
            let mut p = prompt;
            resolve_interactive_trust_decision(&plan, &user_config_dir, &workspace_root, &mut p)
                .await
        });
        // Let the resolver reach the prompt's await before sending.
        tokio::time::sleep(std::time::Duration::from_millis(40)).await;
        tx.send(choice).unwrap();
        let decision = resolver.await.unwrap().unwrap();

        let expected = match choice {
            TrustChoice::Trust | TrustChoice::TrustParent | TrustChoice::TrustSession => {
                TrustDecision::Trusted
            }
            TrustChoice::Deny | TrustChoice::DenySession => TrustDecision::Untrusted,
        };
        assert_eq!(decision, expected, "choice {choice:?} -> decision");

        // Persistence: durable choices record, session-only choices do not.
        let stored = ProjectTrustStore::load(user_config.path())
            .unwrap()
            .decide(workspace.path());
        match choice {
            TrustChoice::Trust | TrustChoice::Deny => {
                assert_eq!(stored, expected, "durable choice {choice:?} persisted");
            }
            TrustChoice::TrustParent => {
                // Recorded on the parent; the project inherits via ancestor walk.
                assert_eq!(stored, TrustDecision::Trusted, "TrustParent covers project");
            }
            TrustChoice::TrustSession | TrustChoice::DenySession => {
                assert_eq!(
                    stored,
                    TrustDecision::Undecided,
                    "session choice not persisted"
                );
            }
        }
    }
}

/// A durable prompt choice must not silently degrade to session-only trust
/// when the trust store cannot be written.
#[tokio::test]
async fn interactive_durable_choice_surfaces_persistence_failure() {
    let user_config = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    init_git(workspace.path());
    write_project_skill(workspace.path());
    let plan = plan_with_empty_registry(
        ProjectTrustCli::default(),
        user_config.path(),
        workspace.path(),
        TrustDecision::Undecided,
    )
    .await;

    // The store can still be loaded, but record cannot create/open its lock
    // sidecar because a directory occupies that exact path.
    std::fs::create_dir(user_config.path().join("trust.json.lock")).unwrap();
    let (tx, rx) = oneshot::channel();
    tx.send(TrustChoice::Trust).unwrap();
    let mut prompt = FakePrompt { rx: Some(rx) };

    let error = resolve_interactive_trust_decision(
        &plan,
        user_config.path(),
        workspace.path(),
        &mut prompt,
    )
    .await
    .unwrap_err();

    assert!(
        matches!(error, TrustError::Io(_)),
        "persistence failure must remain visible, got {error:?}"
    );
    assert!(
        !user_config.path().join("trust.json").exists(),
        "failed durable choice must not create a store entry"
    );
}
