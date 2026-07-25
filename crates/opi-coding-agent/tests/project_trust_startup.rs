//! Phase 15 task 15.8.1 integration tests: the public `prepare_project_startup`
//! preflight entry, headless policy, resolver precedence, the standard-CLI empty
//! registry, and proof that an embedder resolver vote reaches the 15.7 resource
//! gate.
//!
//! These drive the real `opi_coding_agent::project_trust` symbols with temp
//! project/user roots, fake resolvers, and the production `HeadlessPreTrustUi`.
//! No host kernel feature is required; no provider is contacted.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use opi_ai::test_support::MockProvider;
use opi_coding_agent::config::OpiConfig;
use opi_coding_agent::harness::CodingHarness;
use opi_coding_agent::project_trust::{
    HeadlessPreTrustUi, PreTrustUi, PreTrustUiError, ProjectTrustCli, ProjectTrustDefault,
    ProjectTrustResolver, ProjectTrustResolverRegistry, ProjectTrustStore, TrustContext,
    TrustDecision, TrustError, TrustVote, cli_trust_override, prepare_project_startup,
};

// ---------------------------------------------------------------------------
// Fakes
// ---------------------------------------------------------------------------

/// Recording fake resolver: returns a configured vote and appends `name` to a
/// shared log so tests can assert whether/when it was invoked.
struct FakeResolver {
    vote: TrustVote,
    name: &'static str,
    log: Arc<Mutex<Vec<&'static str>>>,
}

impl ProjectTrustResolver for FakeResolver {
    fn resolve(
        &self,
        _ctx: &TrustContext,
        _ui: &dyn PreTrustUi,
    ) -> Pin<Box<dyn Future<Output = TrustVote> + Send + '_>> {
        let log = self.log.clone();
        let name = self.name;
        let vote = self.vote;
        Box::pin(async move {
            log.lock().unwrap().push(name);
            vote
        })
    }
}

/// UI that panics if any method is called. Used to prove the no-resource path
/// invokes no UI at all.
struct PanicUi;
impl PreTrustUi for PanicUi {
    fn select(
        &self,
        _: &str,
        _: &[&str],
    ) -> Pin<Box<dyn Future<Output = Result<usize, PreTrustUiError>> + Send + '_>> {
        Box::pin(async { panic!("select must not be called on the no-resource path") })
    }
    fn confirm(
        &self,
        _: &str,
    ) -> Pin<Box<dyn Future<Output = Result<bool, PreTrustUiError>> + Send + '_>> {
        Box::pin(async { panic!("confirm must not be called on the no-resource path") })
    }
    fn input(
        &self,
        _: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, PreTrustUiError>> + Send + '_>> {
        Box::pin(async { panic!("input must not be called on the no-resource path") })
    }
    fn notify(&self, _: &str) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(async { panic!("notify must not be called on the no-resource path") })
    }
}

fn init_git(dir: &std::path::Path) {
    std::fs::create_dir_all(dir.join(".git")).expect("create .git marker");
}

fn write_project_skill(workspace: &std::path::Path) {
    let skill_dir = workspace.join(".opi").join("skills").join("proj-skill");
    std::fs::create_dir_all(&skill_dir).expect("create proj skill dir");
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: proj-skill\ndescription: Project-only skill.\n---\nPROJ SKILL BODY\n",
    )
    .expect("write proj skill");
}

fn write_global_skill(global: &std::path::Path) {
    let skill_dir = global.join("skills").join("global-skill");
    std::fs::create_dir_all(&skill_dir).expect("create global skill dir");
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: global-skill\ndescription: User-global skill.\n---\nGLOBAL SKILL BODY\n",
    )
    .expect("write global skill");
}

/// Build a harness for `workspace` with `global` as the user-config root and a
/// forced trust decision, mirroring the 15.7 resource-gate assertion shape.
fn harness_with_trust(
    workspace: &std::path::Path,
    global: &std::path::Path,
    decision: TrustDecision,
) -> CodingHarness {
    let provider = MockProvider::new("mock", Vec::new());
    CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.to_path_buf(),
    )
    .global_config_dir(global.to_path_buf())
    .trust_decision(decision)
    .build()
}

// ---------------------------------------------------------------------------
// CLI flag validation + global-default mapping
// ---------------------------------------------------------------------------

#[test]
fn cli_trust_override_validates_mutual_exclusion_and_maps_flags() {
    // Neither flag set -> no override (fall through to the rest of the chain).
    assert_eq!(
        cli_trust_override(ProjectTrustCli {
            trust: false,
            no_trust: false
        })
        .unwrap(),
        None
    );
    // --trust -> Trusted.
    assert_eq!(
        cli_trust_override(ProjectTrustCli {
            trust: true,
            no_trust: false
        })
        .unwrap(),
        Some(TrustDecision::Trusted)
    );
    // --no-trust -> Untrusted.
    assert_eq!(
        cli_trust_override(ProjectTrustCli {
            trust: false,
            no_trust: true
        })
        .unwrap(),
        Some(TrustDecision::Untrusted)
    );
    // Both set -> error (mutually exclusive), regardless of resources.
    assert!(matches!(
        cli_trust_override(ProjectTrustCli {
            trust: true,
            no_trust: true
        }),
        Err(TrustError::ConflictingCliFlags)
    ));
}

#[test]
fn project_trust_default_maps_ask_always_never_to_decisions() {
    assert_eq!(
        ProjectTrustDefault::Ask.to_decision(),
        TrustDecision::Undecided
    );
    assert_eq!(
        ProjectTrustDefault::Always.to_decision(),
        TrustDecision::Trusted
    );
    assert_eq!(
        ProjectTrustDefault::Never.to_decision(),
        TrustDecision::Untrusted
    );
}

// ---------------------------------------------------------------------------
// prepare_project_startup: no-resource bypass (zero side effects)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn no_resource_bypass_invokes_no_resolver_store_or_ui() {
    let user_config = tempfile::tempdir().unwrap();
    // A MALFORMED trust.json proves the store is never loaded on the no-resource
    // path: if prepare_project_startup loaded it, this would error.
    std::fs::write(user_config.path().join("trust.json"), "{ not valid json").unwrap();
    let workspace = tempfile::tempdir().unwrap();
    init_git(workspace.path());
    // No .opi resources at all (bare workspace).

    let log: Arc<Mutex<Vec<&'static str>>> = Arc::new(Mutex::new(Vec::new()));
    let mut registry = ProjectTrustResolverRegistry::new();
    registry
        .register(Arc::new(FakeResolver {
            vote: TrustVote::Trust,
            name: "should-not-run",
            log: log.clone(),
        }))
        .unwrap();

    let plan = prepare_project_startup(
        ProjectTrustCli {
            trust: false,
            no_trust: false,
        },
        &mut registry,
        user_config.path(),
        workspace.path(),
        TrustDecision::Undecided,
        &PanicUi,
    )
    .await
    .expect("no-resource plan");
    assert_eq!(plan.decision, TrustDecision::Trusted);
    assert!(
        plan.resources.is_empty(),
        "no-resource plan has empty resources"
    );
    assert!(
        log.lock().unwrap().is_empty(),
        "no resolver may run on the no-resource path"
    );
}

// ---------------------------------------------------------------------------
// prepare_project_startup: preflight ordering
// ---------------------------------------------------------------------------

#[tokio::test]
async fn trust_preflight_precedes_project_and_runtime_construction() {
    // (1) No-resource path does not touch the store even when it is malformed
    //     (re-asserts the bypass from a different angle) and does not seal the
    //     registry, so a no-resource project can still register later.
    let user_config_a = tempfile::tempdir().unwrap();
    std::fs::write(user_config_a.path().join("trust.json"), "garbage").unwrap();
    let workspace_a = tempfile::tempdir().unwrap();
    init_git(workspace_a.path());
    let mut registry_a = ProjectTrustResolverRegistry::new();
    let _plan_a = prepare_project_startup(
        ProjectTrustCli::default(),
        &mut registry_a,
        user_config_a.path(),
        workspace_a.path(),
        TrustDecision::Undecided,
        &HeadlessPreTrustUi,
    )
    .await
    .unwrap();
    assert!(
        !registry_a.is_sealed(),
        "no-resource preflight must not seal the registry"
    );

    // (2) Resource-present path seals the registry, so late registration (e.g. a
    //     project extension attempting self-authorization after preflight) is
    //     rejected. The plan is computed before any harness/provider/package
    //     construction (the caller only acts on the returned plan).
    let user_config_b = tempfile::tempdir().unwrap();
    let workspace_b = tempfile::tempdir().unwrap();
    init_git(workspace_b.path());
    write_project_skill(workspace_b.path());
    let mut registry_b = ProjectTrustResolverRegistry::new();
    let plan_b = prepare_project_startup(
        ProjectTrustCli::default(),
        &mut registry_b,
        user_config_b.path(),
        workspace_b.path(),
        TrustDecision::Undecided,
        &HeadlessPreTrustUi,
    )
    .await
    .unwrap();
    assert!(
        registry_b.is_sealed(),
        "resource-present preflight must seal the registry"
    );
    assert!(
        matches!(
            registry_b.register(Arc::new(FakeResolver {
                vote: TrustVote::Trust,
                name: "late",
                log: Arc::new(Mutex::new(Vec::new())),
            })),
            Err(TrustError::RegistrySealed)
        ),
        "late registration after preflight must be rejected"
    );
    // The plan carries the computed decision and triggering resources; the
    // caller consumes it before constructing the harness/provider/packages.
    assert_eq!(plan_b.resources.len(), 1);
    assert_eq!(plan_b.decision, TrustDecision::Undecided); // headless ask unresolved
}

// ---------------------------------------------------------------------------
// prepare_project_startup: resolver precedence + standard-CLI empty registry
// ---------------------------------------------------------------------------

#[tokio::test]
async fn standard_cli_passes_empty_resolver_registry() {
    // The standard opi CLI constructs an empty registry (Phase 15 adds no CLI -e
    // or native loader). With resources present, no store entry, and no CLI
    // override, the chain falls through to the headless ask, which is
    // Unavailable -> Undecided (headless callers map that to Untrusted).
    let user_config = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    init_git(workspace.path());
    write_project_skill(workspace.path());
    let mut registry = ProjectTrustResolverRegistry::new();
    assert!(registry.is_empty(), "standard CLI registry starts empty");

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
    assert!(registry.is_sealed(), "preflight sealed the empty registry");
    assert_eq!(plan.decision, TrustDecision::Undecided);
    assert_eq!(plan.headless_decision(), TrustDecision::Untrusted);
}

#[tokio::test]
async fn explicit_embedder_resolver_reaches_resource_path() {
    // An explicitly registered embedder resolver's vote flows through
    // prepare_project_startup into BOTH the trusted and untrusted 15.7 resource
    // paths (proven via the production CodingHarness resource gate).
    let cases = [
        (TrustVote::Trust, TrustDecision::Trusted, true),
        (TrustVote::Deny, TrustDecision::Untrusted, false),
    ];
    for (vote, expected_decision, expect_proj_skill) in cases {
        let workspace = tempfile::tempdir().unwrap();
        init_git(workspace.path());
        write_project_skill(workspace.path());
        let global = tempfile::tempdir().unwrap();
        write_global_skill(global.path());

        let mut registry = ProjectTrustResolverRegistry::new();
        registry
            .register(Arc::new(FakeResolver {
                vote,
                name: "embedder",
                log: Arc::new(Mutex::new(Vec::new())),
            }))
            .unwrap();
        let plan = prepare_project_startup(
            ProjectTrustCli::default(),
            &mut registry,
            global.path(),
            workspace.path(),
            TrustDecision::Undecided,
            &HeadlessPreTrustUi,
        )
        .await
        .unwrap();
        assert_eq!(
            plan.decision, expected_decision,
            "embedder resolver vote must decide the plan"
        );

        // The decision reaches the 15.7 resource gate: a trusted project loads
        // its skill, an untrusted project does not. The global skill always loads.
        let harness = harness_with_trust(workspace.path(), global.path(), plan.decision);
        let prompt = harness.system_prompt();
        assert!(
            prompt.contains("global-skill"),
            "global skill should load: {prompt}"
        );
        if expect_proj_skill {
            assert!(
                prompt.contains("proj-skill"),
                "trusted project skill should load: {prompt}"
            );
        } else {
            assert!(
                !prompt.contains("proj-skill"),
                "untrusted project skill must not load: {prompt}"
            );
        }
    }
}

#[tokio::test]
async fn prepare_project_startup_precedence_cli_store_default_ask() {
    let user_config = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    init_git(workspace.path());
    write_project_skill(workspace.path());

    // Helper running the preflight with a fresh empty registry.
    async fn run(
        cli: ProjectTrustCli,
        user_config: &std::path::Path,
        workspace: &std::path::Path,
        global_default: TrustDecision,
    ) -> TrustDecision {
        let mut registry = ProjectTrustResolverRegistry::new();
        let plan = prepare_project_startup(
            cli,
            &mut registry,
            user_config,
            workspace,
            global_default,
            &HeadlessPreTrustUi,
        )
        .await
        .unwrap();
        plan.decision
    }

    // No override, empty store, ask default -> Undecided (headless -> Untrusted).
    assert_eq!(
        run(
            ProjectTrustCli::default(),
            user_config.path(),
            workspace.path(),
            TrustDecision::Undecided
        )
        .await,
        TrustDecision::Undecided
    );
    // CLI --trust wins over everything.
    assert_eq!(
        run(
            ProjectTrustCli {
                trust: true,
                no_trust: false
            },
            user_config.path(),
            workspace.path(),
            TrustDecision::Undecided
        )
        .await,
        TrustDecision::Trusted
    );
    // CLI --no-trust wins over everything.
    assert_eq!(
        run(
            ProjectTrustCli {
                trust: false,
                no_trust: true
            },
            user_config.path(),
            workspace.path(),
            TrustDecision::Undecided
        )
        .await,
        TrustDecision::Untrusted
    );
    // global default `always` decides when CLI/store/resolvers are silent.
    assert_eq!(
        run(
            ProjectTrustCli::default(),
            user_config.path(),
            workspace.path(),
            TrustDecision::Trusted
        )
        .await,
        TrustDecision::Trusted
    );
    // global default `never` decides.
    assert_eq!(
        run(
            ProjectTrustCli::default(),
            user_config.path(),
            workspace.path(),
            TrustDecision::Untrusted
        )
        .await,
        TrustDecision::Untrusted
    );
    // Store entry wins over the ask default.
    let store = ProjectTrustStore::load(user_config.path()).unwrap();
    store.record(workspace.path(), true).unwrap();
    assert_eq!(
        run(
            ProjectTrustCli::default(),
            user_config.path(),
            workspace.path(),
            TrustDecision::Undecided
        )
        .await,
        TrustDecision::Trusted
    );
}
