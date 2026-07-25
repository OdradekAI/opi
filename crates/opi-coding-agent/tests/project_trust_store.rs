//! Phase 15 task 15.6 integration tests: project trust store, resolver
//! registry, resource detection, and the resolution precedence core.
//!
//! These prove the substrate contracts from the 15.6 DoD against the real
//! `opi_coding_agent::project_trust` symbols with **no host kernel
//! dependency**: temp user-config roots, fake resolvers, and a fake pre-trust
//! UI. The substrate is `substrate_only`: it does not close a product
//! acceptance scenario (15.7 wires the resource gate, 15.8.1
//! `prepare_project_startup`, 15.8.2 the interactive TUI prompt).
//!
//! DoD coverage:
//! - alias canonicalization (realpath key)
//! - nearest-ancestor precedence
//! - concurrent writers (the sidecar lock serializes read-modify-write)
//! - malformed JSON errors
//! - no-decision (absent entry => Undecided)
//! - deterministic explicit registration order
//! - empty default registry
//! - no late registration (seal rejects)
//! - no-resource no-op (bare `.opi` => empty set)
//! - resolution precedence: CLI -> resolvers -> store -> default -> ask

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Barrier, Mutex};

use opi_coding_agent::project_trust::{
    PreTrustUi, PreTrustUiError, ProjectTrustResolver, ProjectTrustResolverRegistry,
    ProjectTrustStore, TrustContext, TrustDecision, TrustError, TrustResource, TrustVote,
    resolve_trust,
};

// ---------------------------------------------------------------------------
// Fakes
// ---------------------------------------------------------------------------

type Log = Arc<Mutex<Vec<usize>>>;

/// Recording fake resolver: returns a configured vote and appends its `name` to
/// a shared log on each invocation, so tests can assert registry iteration
/// order without sleeps.
struct FakeResolver {
    vote: TrustVote,
    name: usize,
    log: Log,
}

impl ProjectTrustResolver for FakeResolver {
    fn resolve(
        &self,
        _ctx: &TrustContext,
        _ui: &dyn PreTrustUi,
    ) -> Pin<Box<dyn Future<Output = TrustVote> + Send + '_>> {
        let log = self.log.clone();
        let name = self.name;
        Box::pin(async move {
            log.lock().unwrap().push(name);
            self.vote
        })
    }
}

fn fake(vote: TrustVote, name: usize, log: &Log) -> FakeResolver {
    FakeResolver {
        vote,
        name,
        log: log.clone(),
    }
}

/// Resolver that panics if invoked, used to prove an earlier precedence layer
/// short-circuits before resolvers run.
struct PanicResolver;
impl ProjectTrustResolver for PanicResolver {
    fn resolve(
        &self,
        _ctx: &TrustContext,
        _ui: &dyn PreTrustUi,
    ) -> Pin<Box<dyn Future<Output = TrustVote> + Send + '_>> {
        Box::pin(async { panic!("resolver must not be invoked when an earlier layer decided") })
    }
}

/// Configurable fake pre-trust UI. By default it is "headless": select/confirm/
/// input return `Unavailable`. Tests override specific method results.
#[derive(Clone)]
struct FakeUi {
    confirm_result: Result<bool, PreTrustUiError>,
    confirm_calls: Arc<Mutex<Vec<String>>>,
}

impl FakeUi {
    fn headless() -> Self {
        Self {
            confirm_result: Err(PreTrustUiError::Unavailable),
            confirm_calls: Arc::new(Mutex::new(vec![])),
        }
    }
    fn confirming(ok: bool) -> Self {
        let mut ui = Self::headless();
        ui.confirm_result = Ok(ok);
        ui
    }
}

impl PreTrustUi for FakeUi {
    fn select(
        &self,
        _prompt: &str,
        _options: &[&str],
    ) -> Pin<Box<dyn Future<Output = Result<usize, PreTrustUiError>> + Send + '_>> {
        Box::pin(async move { Err(PreTrustUiError::Unavailable) })
    }
    fn confirm(
        &self,
        prompt: &str,
    ) -> Pin<Box<dyn Future<Output = Result<bool, PreTrustUiError>> + Send + '_>> {
        let prompt = prompt.to_string();
        let result = self.confirm_result;
        Box::pin(async move {
            self.confirm_calls.lock().unwrap().push(prompt);
            result
        })
    }
    fn input(
        &self,
        _prompt: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, PreTrustUiError>> + Send + '_>> {
        Box::pin(async move { Err(PreTrustUiError::Unavailable) })
    }
    fn notify(&self, _msg: &str) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(async move {})
    }
}

fn ctx(path: impl AsRef<std::path::Path>) -> TrustContext {
    TrustContext {
        project_path: path.as_ref().to_path_buf(),
        triggering_resources: vec![],
    }
}

fn ctx_with(path: impl AsRef<std::path::Path>, resources: Vec<TrustResource>) -> TrustContext {
    TrustContext {
        project_path: path.as_ref().to_path_buf(),
        triggering_resources: resources,
    }
}

// ---------------------------------------------------------------------------
// Store: load / decide / record
// ---------------------------------------------------------------------------

/// `load` on a missing trust.json yields an empty store (no error); `decide`
/// on any path is `Undecided`.
#[tokio::test]
async fn load_missing_store_is_empty_and_undecided() {
    let user_dir = tempfile::tempdir().unwrap();
    let store = ProjectTrustStore::load(user_dir.path()).unwrap();
    let project = tempfile::tempdir().unwrap();
    assert_eq!(store.decide(project.path()), TrustDecision::Undecided);
}

/// `record` canonicalizes the project path and persists a flat bool under the
/// realpath key; a fresh `load` observes the decision.
#[tokio::test]
async fn record_persists_canonical_decision_and_reload_observes_it() {
    let user_dir = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let store = ProjectTrustStore::load(user_dir.path()).unwrap();
    store.record(project.path(), true).unwrap();

    let reloaded = ProjectTrustStore::load(user_dir.path()).unwrap();
    assert_eq!(reloaded.decide(project.path()), TrustDecision::Trusted);

    // Record false for a different project and reload again.
    let project_b = tempfile::tempdir().unwrap();
    store.record(project_b.path(), false).unwrap();
    let reloaded2 = ProjectTrustStore::load(user_dir.path()).unwrap();
    assert_eq!(reloaded2.decide(project.path()), TrustDecision::Trusted);
    assert_eq!(reloaded2.decide(project_b.path()), TrustDecision::Untrusted);
}

/// Alias canonicalization: a non-canonical path (a real child dir joined with
/// `..`) resolves to the same realpath key as the stored project.
#[tokio::test]
async fn alias_canonicalization_matches_stored_realpath() {
    let user_dir = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(project.path().join("child")).unwrap();

    let store = ProjectTrustStore::load(user_dir.path()).unwrap();
    store.record(project.path(), true).unwrap();

    // Non-canonical alias: `<project>/child/..` canonicalizes to `<project>`.
    let alias = project.path().join("child").join("..");
    assert_eq!(
        store.decide(&alias),
        TrustDecision::Trusted,
        "alias must canonicalize to the stored realpath key"
    );
}

/// Nearest-ancestor precedence: a stored parent decision governs a child path,
/// and a nearer ancestor overrides a farther one.
#[tokio::test]
async fn nearest_ancestor_wins_over_farther_ancestor() {
    let user_dir = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let mid = root.path().join("mid");
    let leaf = mid.join("leaf");
    std::fs::create_dir_all(&leaf).unwrap();

    let store = ProjectTrustStore::load(user_dir.path()).unwrap();
    // Store a far-ancestor Trusted decision.
    store.record(root.path(), true).unwrap();
    assert_eq!(store.decide(&leaf), TrustDecision::Trusted);

    // A nearer ancestor Untrusted decision overrides the farther Trusted one.
    store.record(&mid, false).unwrap();
    assert_eq!(store.decide(&leaf), TrustDecision::Untrusted);
    // The far ancestor still governs its own subtree where the nearer one does
    // not apply.
    let sibling = root.path().join("other");
    std::fs::create_dir_all(&sibling).unwrap();
    assert_eq!(store.decide(&sibling), TrustDecision::Trusted);
}

/// A path with no stored decision (and no ancestor decision) is `Undecided`.
#[tokio::test]
async fn no_decision_is_undecided() {
    let user_dir = tempfile::tempdir().unwrap();
    let unrelated = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let store = ProjectTrustStore::load(user_dir.path()).unwrap();
    store.record(unrelated.path(), true).unwrap();
    assert_eq!(store.decide(project.path()), TrustDecision::Undecided);
}

/// Malformed trust.json produces a `MalformedJson` error on load.
#[tokio::test]
async fn malformed_json_is_a_named_error() {
    let user_dir = tempfile::tempdir().unwrap();
    std::fs::write(user_dir.path().join("trust.json"), b"{ not valid json").unwrap();
    let err = ProjectTrustStore::load(user_dir.path()).unwrap_err();
    assert!(
        matches!(err, TrustError::MalformedJson(_)),
        "expected MalformedJson, got {err:?}"
    );
}

/// The on-disk shape is exactly a flat canonical-path -> bool map with no
/// schema metadata, version, or wrapper object.
#[tokio::test]
async fn trust_json_is_flat_map_with_no_schema_metadata() {
    let user_dir = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let store = ProjectTrustStore::load(user_dir.path()).unwrap();
    store.record(project.path(), true).unwrap();

    let raw = std::fs::read_to_string(user_dir.path().join("trust.json")).unwrap();
    let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let map = value
        .as_object()
        .expect("trust.json must be a flat JSON object");
    assert!(
        !map.contains_key("version") && !map.contains_key("schema") && !map.contains_key("entries"),
        "trust.json must carry no schema metadata, got keys: {:?}",
        map.keys().collect::<Vec<_>>()
    );
    assert_eq!(map.len(), 1, "exactly one entry after one record");
    let val = map.values().next().unwrap();
    assert_eq!(val.as_bool(), Some(true), "value must be a bare bool");
}

/// Concurrent writers do not lose updates: the `trust.json.lock` sidecar
/// serializes the read-modify-write so two threads recording distinct keys
/// both persist.
#[tokio::test]
async fn concurrent_writers_do_not_lose_updates() {
    let user_dir = Arc::new(tempfile::tempdir().unwrap());
    let store = Arc::new(ProjectTrustStore::load(user_dir.path()).unwrap());

    let keys: Vec<std::path::PathBuf> = (0..8)
        .map(|i| {
            let d = user_dir.path().join(format!("proj-{i}"));
            std::fs::create_dir_all(&d).unwrap();
            d
        })
        .collect();

    // Two threads, each recording a disjoint half of the keys, released at a
    // shared barrier so they contend on the lock simultaneously.
    let barrier = Arc::new(Barrier::new(2));
    let mut handles = vec![];
    for half in [keys[..4].to_vec(), keys[4..].to_vec()] {
        let store = store.clone();
        let barrier = barrier.clone();
        handles.push(std::thread::spawn(move || {
            barrier.wait();
            for (i, k) in half.into_iter().enumerate() {
                store.record(&k, i % 2 == 0).unwrap();
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }

    let reloaded = ProjectTrustStore::load(user_dir.path()).unwrap();
    for k in &keys {
        assert!(
            reloaded.decide(k).is_decided(),
            "concurrent record lost update for {:?}",
            k
        );
    }
}

// ---------------------------------------------------------------------------
// Resource detection
// ---------------------------------------------------------------------------

/// A bare `.opi` directory (present but empty) triggers no resources.
#[tokio::test]
async fn bare_opi_directory_has_no_resources() {
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(project.path().join(".opi")).unwrap();
    let resources = ProjectTrustStore::detect_resources(project.path());
    assert!(
        resources.is_empty(),
        "bare .opi must produce an empty resource set, got {resources:?}"
    );
}

/// `detect_resources` names exactly the gated project files/directories.
#[tokio::test]
async fn detect_resources_names_exact_gated_set() {
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(project.path().join(".opi")).unwrap();
    std::fs::write(project.path().join(".opi").join("config.toml"), b"").unwrap();
    std::fs::create_dir_all(project.path().join(".opi").join("skills")).unwrap();
    std::fs::create_dir_all(project.path().join(".opi").join("fragments")).unwrap();
    std::fs::create_dir_all(project.path().join(".opi").join("themes")).unwrap();
    std::fs::create_dir_all(project.path().join(".opi").join("extensions")).unwrap();
    std::fs::write(project.path().join(".opi").join("packages.toml"), b"").unwrap();
    std::fs::write(project.path().join("AGENTS.md"), b"").unwrap();
    std::fs::write(project.path().join("CLAUDE.md"), b"").unwrap();

    let mut resources = ProjectTrustStore::detect_resources(project.path());
    resources.sort_by_key(|r| format!("{r:?}"));
    let mut expected = vec![
        TrustResource::ProjectConfig,
        TrustResource::Skills,
        TrustResource::Fragments,
        TrustResource::Themes,
        TrustResource::Extensions,
        TrustResource::Packages,
        TrustResource::AgentsMd,
        TrustResource::ClaudeMd,
    ];
    expected.sort_by_key(|r| format!("{r:?}"));
    assert_eq!(resources, expected);
}

/// Only present resources are reported; absent ones are omitted.
#[tokio::test]
async fn detect_resources_reports_only_present_set() {
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(project.path().join(".opi")).unwrap();
    std::fs::write(project.path().join(".opi").join("config.toml"), b"").unwrap();
    std::fs::write(project.path().join("CLAUDE.md"), b"").unwrap();
    let resources = ProjectTrustStore::detect_resources(project.path());
    assert_eq!(resources.len(), 2);
    assert!(resources.contains(&TrustResource::ProjectConfig));
    assert!(resources.contains(&TrustResource::ClaudeMd));
}

// ---------------------------------------------------------------------------
// Resolver registry
// ---------------------------------------------------------------------------

/// A newly constructed registry is empty and not sealed.
#[test]
fn default_registry_is_empty() {
    let reg = ProjectTrustResolverRegistry::new();
    assert!(reg.is_empty());
    assert_eq!(reg.len(), 0);
    assert!(!reg.is_sealed());
    assert_eq!(reg.resolvers().len(), 0);
}

/// The 15.6 library gate requires this exact test name. A default registry is
/// empty; explicit registrations are iterated in registration order.
#[tokio::test]
async fn explicit_registry_is_ordered_and_default_is_empty() {
    // Default is empty.
    let reg0 = ProjectTrustResolverRegistry::new();
    assert!(reg0.is_empty());

    let log: Log = Arc::new(Mutex::new(vec![]));
    let mut reg = ProjectTrustResolverRegistry::new();
    reg.register(Arc::new(fake(TrustVote::Undecided, 1, &log)))
        .unwrap();
    reg.register(Arc::new(fake(TrustVote::Undecided, 2, &log)))
        .unwrap();
    assert!(!reg.is_empty());
    assert_eq!(reg.len(), 2);

    // Drive resolution on an empty store + ask-default so every registered
    // resolver is consulted in registration order.
    let user_dir = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let store = ProjectTrustStore::load(user_dir.path()).unwrap();
    let ui = FakeUi::headless();
    let decision = resolve_trust(
        None,
        &reg,
        &store,
        &ctx(project.path()),
        TrustDecision::Undecided,
        &ui,
    )
    .await;
    // Both abstained and ask is unavailable => Undecided.
    assert_eq!(decision, TrustDecision::Undecided);
    // Resolvers were invoked in registration order: 1 then 2.
    assert_eq!(*log.lock().unwrap(), vec![1, 2]);
}

/// First Trust/Deny vote wins; later resolvers are not consulted once a
/// resolver decides.
#[tokio::test]
async fn first_resolver_decision_wins_and_short_circuits() {
    let log: Log = Arc::new(Mutex::new(vec![]));
    let mut reg = ProjectTrustResolverRegistry::new();
    reg.register(Arc::new(fake(TrustVote::Trust, 1, &log)))
        .unwrap();
    // A later resolver that panics if reached proves the first short-circuits.
    reg.register(Arc::new(PanicResolver)).unwrap();

    let user_dir = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let store = ProjectTrustStore::load(user_dir.path()).unwrap();
    let ui = FakeUi::headless();
    let decision = resolve_trust(
        None,
        &reg,
        &store,
        &ctx(project.path()),
        TrustDecision::Undecided,
        &ui,
    )
    .await;
    assert_eq!(decision, TrustDecision::Trusted);
    assert_eq!(*log.lock().unwrap(), vec![1]);
}

/// An Undecided resolver vote falls through to the next layer.
#[tokio::test]
async fn undecided_resolver_vote_falls_through() {
    let log: Log = Arc::new(Mutex::new(vec![]));
    let mut reg = ProjectTrustResolverRegistry::new();
    reg.register(Arc::new(fake(TrustVote::Undecided, 1, &log)))
        .unwrap();

    let user_dir = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let store = ProjectTrustStore::load(user_dir.path()).unwrap();
    store.record(project.path(), false).unwrap();
    let ui = FakeUi::headless();
    let decision = resolve_trust(
        None,
        &reg,
        &store,
        &ctx(project.path()),
        TrustDecision::Undecided,
        &ui,
    )
    .await;
    assert_eq!(decision, TrustDecision::Untrusted);
}

/// Once sealed, the registry rejects further registrations.
#[test]
fn sealed_registry_rejects_late_registration() {
    let log: Log = Arc::new(Mutex::new(vec![]));
    let mut reg = ProjectTrustResolverRegistry::new();
    reg.register(Arc::new(fake(TrustVote::Undecided, 1, &log)))
        .unwrap();
    reg.seal();
    assert!(reg.is_sealed());
    let err = reg
        .register(Arc::new(fake(TrustVote::Undecided, 2, &log)))
        .unwrap_err();
    assert!(
        matches!(err, TrustError::RegistrySealed),
        "expected RegistrySealed, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// Resolution precedence: CLI -> resolvers -> store -> default -> ask
// ---------------------------------------------------------------------------

/// CLI override is authoritative and skips resolvers/store/default/ask.
#[tokio::test]
async fn cli_override_short_circuits_before_everything() {
    let mut reg = ProjectTrustResolverRegistry::new();
    reg.register(Arc::new(PanicResolver)).unwrap(); // must not run
    let user_dir = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let store = ProjectTrustStore::load(user_dir.path()).unwrap();
    store.record(project.path(), false).unwrap(); // contradicts CLI; CLI wins
    let ui = FakeUi::headless();
    let decision = resolve_trust(
        Some(TrustDecision::Trusted),
        &reg,
        &store,
        &ctx(project.path()),
        TrustDecision::Untrusted,
        &ui,
    )
    .await;
    assert_eq!(decision, TrustDecision::Trusted);
}

/// Store wins over default when resolvers abstain.
#[tokio::test]
async fn store_wins_over_global_default() {
    let reg = ProjectTrustResolverRegistry::new();
    let user_dir = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let store = ProjectTrustStore::load(user_dir.path()).unwrap();
    store.record(project.path(), true).unwrap();
    let ui = FakeUi::headless();
    let decision = resolve_trust(
        None,
        &reg,
        &store,
        &ctx(project.path()),
        TrustDecision::Untrusted,
        &ui,
    )
    .await;
    assert_eq!(decision, TrustDecision::Trusted);
}

/// Global default wins when CLI/resolvers/store all abstain and default is
/// decided (no ask).
#[tokio::test]
async fn global_default_wins_when_layers_abstain() {
    let reg = ProjectTrustResolverRegistry::new();
    let user_dir = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let store = ProjectTrustStore::load(user_dir.path()).unwrap();
    let ui = FakeUi::headless();
    let decision = resolve_trust(
        None,
        &reg,
        &store,
        &ctx(project.path()),
        TrustDecision::Untrusted,
        &ui,
    )
    .await;
    assert_eq!(decision, TrustDecision::Untrusted);
}

/// When every prior layer abstains and the default is `ask`, the UI confirm is
/// consulted last; Ok(true) => Trusted, and confirm is invoked exactly once.
#[tokio::test]
async fn ask_is_consulted_last_and_maps_confirm() {
    let reg = ProjectTrustResolverRegistry::new();
    let user_dir = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let store = ProjectTrustStore::load(user_dir.path()).unwrap();
    let ui = FakeUi::confirming(true);
    let decision = resolve_trust(
        None,
        &reg,
        &store,
        &ctx(project.path()),
        TrustDecision::Undecided,
        &ui,
    )
    .await;
    assert_eq!(decision, TrustDecision::Trusted);
    assert_eq!(ui.confirm_calls.lock().unwrap().len(), 1);

    // Ok(false) => Untrusted.
    let ui2 = FakeUi::confirming(false);
    let decision2 = resolve_trust(
        None,
        &reg,
        &store,
        &ctx(project.path()),
        TrustDecision::Undecided,
        &ui2,
    )
    .await;
    assert_eq!(decision2, TrustDecision::Untrusted);
}

/// Ask is NOT consulted when an earlier layer (store) already decided.
#[tokio::test]
async fn ask_skipped_when_store_decided() {
    let reg = ProjectTrustResolverRegistry::new();
    let user_dir = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let store = ProjectTrustStore::load(user_dir.path()).unwrap();
    store.record(project.path(), true).unwrap();
    let ui = FakeUi::confirming(false); // would deny, but must not be asked
    let decision = resolve_trust(
        None,
        &reg,
        &store,
        &ctx(project.path()),
        TrustDecision::Undecided,
        &ui,
    )
    .await;
    assert_eq!(decision, TrustDecision::Trusted);
    assert!(
        ui.confirm_calls.lock().unwrap().is_empty(),
        "ask must be skipped when the store already decided"
    );
}

/// An unavailable ask (headless) with all-prior-abstain and ask-default yields
/// `Undecided`; the 15.8.1 headless policy maps that terminal to Untrusted.
#[tokio::test]
async fn unavailable_ask_yields_undecided_terminal() {
    let reg = ProjectTrustResolverRegistry::new();
    let user_dir = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let store = ProjectTrustStore::load(user_dir.path()).unwrap();
    let ui = FakeUi::headless();
    let decision = resolve_trust(
        None,
        &reg,
        &store,
        &ctx(project.path()),
        TrustDecision::Undecided,
        &ui,
    )
    .await;
    assert_eq!(decision, TrustDecision::Undecided);
}

/// The triggering resources discovered for a project are carried into the
/// resolver context (proves TrustContext is wired, not just a placeholder).
#[tokio::test]
async fn resolver_context_carries_triggering_resources() {
    let observed: Arc<Mutex<Vec<Vec<TrustResource>>>> = Arc::new(Mutex::new(vec![]));
    struct Capturing {
        observed: Arc<Mutex<Vec<Vec<TrustResource>>>>,
    }
    impl ProjectTrustResolver for Capturing {
        fn resolve(
            &self,
            ctx: &TrustContext,
            _ui: &dyn PreTrustUi,
        ) -> Pin<Box<dyn Future<Output = TrustVote> + Send + '_>> {
            let observed = self.observed.clone();
            let resources = ctx.triggering_resources.clone();
            Box::pin(async move {
                observed.lock().unwrap().push(resources);
                TrustVote::Trust
            })
        }
    }

    let mut reg = ProjectTrustResolverRegistry::new();
    reg.register(Arc::new(Capturing {
        observed: observed.clone(),
    }))
    .unwrap();

    let user_dir = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(project.path().join(".opi")).unwrap();
    std::fs::write(project.path().join(".opi").join("config.toml"), b"").unwrap();
    let resources = ProjectTrustStore::detect_resources(project.path());
    let store = ProjectTrustStore::load(user_dir.path()).unwrap();
    let ui = FakeUi::headless();
    let _ = resolve_trust(
        None,
        &reg,
        &store,
        &ctx_with(project.path(), resources),
        TrustDecision::Undecided,
        &ui,
    )
    .await;
    let captured = observed.lock().unwrap();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0], vec![TrustResource::ProjectConfig]);
}

/// A resolver's vote maps through `TrustVote::to_decision` for all three
/// variants.
#[test]
fn trust_vote_maps_to_decision() {
    assert_eq!(TrustVote::Trust.to_decision(), TrustDecision::Trusted);
    assert_eq!(TrustVote::Deny.to_decision(), TrustDecision::Untrusted);
    assert_eq!(TrustVote::Undecided.to_decision(), TrustDecision::Undecided);
}

/// `is_decided` distinguishes a decision from `Undecided`.
#[test]
fn trust_decision_is_decided_predicate() {
    assert!(TrustDecision::Trusted.is_decided());
    assert!(TrustDecision::Untrusted.is_decided());
    assert!(!TrustDecision::Undecided.is_decided());
}
