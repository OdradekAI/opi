//! Project trust store, resolver registry, and resolution precedence core.
//!
//! Phase 15 task 15.6 substrate. This module defines the trust contracts that
//! gate loading of project-local resources (`.opi/config.toml`, project
//! skills/fragments/themes/extensions, project packages, and project context
//! files) until a user has consented. It is **substrate only**: it ships the
//! types, the on-disk store, the resolver registry, resource detection, and the
//! shared precedence core. Task 15.7 wires the resource gate into startup,
//! 15.8.1 ships `prepare_project_startup` + headless policy, and 15.8.2 ships
//! the interactive TUI prompt.
//!
//! The store lives in `opi-coding-agent` (not on `opi-agent`) because it is
//! consulted once at session-start, before `discover_resources` consumes any
//! project layer. No trust type enters `opi-agent`.
//!
//! # Store shape
//!
//! `trust.json` is a flat canonical-path-to-bool map with no schema metadata:
//!
//! ```json
//! {
//!   "/home/user/project-a": true,
//!   "/home/user/project-b": false
//! }
//! ```
//!
//! `ProjectTrustStore::load(user_config_dir)` reads exactly
//! `user_config_dir.join("trust.json")` — `%APPDATA%\opi\trust.json` on Windows
//! and `~/.config/opi/trust.json` on Unix defaults, matching
//! [`crate::config::user_config_dir`]. A missing file is an empty store (no
//! error); a malformed file is a [`TrustError::MalformedJson`]. `decide`
//! canonicalizes the project path and returns the nearest ancestor's stored
//! decision; `record` canonicalizes one key, acquires the `trust.json.lock`
//! sidecar, re-reads under the lock, updates one entry, and atomically replaces
//! the file.
//!
//! # Resolution precedence
//!
//! [`resolve_trust`] applies, in order: CLI override, registered resolver votes,
//! the store's nearest-ancestor decision, the global-only default, then ask.
//! The first `Trusted`/`Untrusted` result wins; `Undecided` falls through. The
//! registry accepts deterministic-order registrations through an explicit
//! public embedder/startup API only; a default registry is empty, late
//! registration after [`ProjectTrustResolverRegistry::seal`] is rejected, and
//! the registry exposes no metadata discovery or native-code loading path.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::future::Future;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;

use serde::Deserialize;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Failures raised by the project trust store or resolver registry.
#[derive(Debug, thiserror::Error)]
pub enum TrustError {
    /// An I/O error reading or writing `trust.json` or its lock sidecar.
    #[error("trust store I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// `trust.json` exists but is not a flat canonical-path -> bool JSON map.
    #[error("trust.json is malformed: {0}")]
    MalformedJson(String),
    /// The advisory lock could not be acquired or released.
    #[error("trust store lock error: {0}")]
    Lock(String),
    /// A path passed to `record` could not be canonicalized (it must exist).
    #[error("could not canonicalize path: {0}")]
    Canonicalize(PathBuf),
    /// A resolver was registered after the registry was sealed for resolution.
    #[error("resolver registry is sealed; late registration rejected")]
    RegistrySealed,
    /// `--trust` and `--no-trust` were both set (mutually exclusive).
    #[error("--trust and --no-trust are mutually exclusive")]
    ConflictingCliFlags,
}

/// Errors a [`PreTrustUi`] implementation can surface. The substrate defines
/// exactly one variant: `Unavailable`, returned by the headless non-interactive
/// and RPC UIs, which cannot ask the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PreTrustUiError {
    /// The pre-trust UI cannot prompt in this run mode (non-interactive/RPC).
    #[error("pre-trust UI is unavailable in this run mode")]
    Unavailable,
}

// ---------------------------------------------------------------------------
// Decisions and votes
// ---------------------------------------------------------------------------

/// The resolved trust state for a project. `Undecided` means no layer offered
/// an opinion and (for the substrate's ask step) the UI was unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustDecision {
    Trusted,
    Untrusted,
    Undecided,
}

impl TrustDecision {
    /// `true` for `Trusted`/`Untrusted`; `false` for `Undecided`.
    pub fn is_decided(self) -> bool {
        matches!(self, TrustDecision::Trusted | TrustDecision::Untrusted)
    }
}

/// A single resolver's vote. `Undecided` means "no opinion; keep asking".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustVote {
    Trust,
    Deny,
    Undecided,
}

impl TrustVote {
    /// Map a vote into the decision it implies (`Undecided` passes through).
    pub fn to_decision(self) -> TrustDecision {
        match self {
            TrustVote::Trust => TrustDecision::Trusted,
            TrustVote::Deny => TrustDecision::Untrusted,
            TrustVote::Undecided => TrustDecision::Undecided,
        }
    }
}

/// A project resource whose presence in a project requires consent before its
/// project layer is loaded. Each variant's doc names the exact gated file or
/// directory under the project root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustResource {
    /// `.opi/config.toml`
    ProjectConfig,
    /// `.opi/skills`
    Skills,
    /// `.opi/fragments`
    Fragments,
    /// `.opi/themes`
    Themes,
    /// `.opi/extensions`
    Extensions,
    /// `.opi/packages.toml`
    Packages,
    /// `AGENTS.md`
    AgentsMd,
    /// `CLAUDE.md`
    ClaudeMd,
}

impl TrustResource {
    /// The path relative to a project root whose presence triggers this
    /// resource.
    fn rel_path(self) -> &'static Path {
        match self {
            TrustResource::ProjectConfig => Path::new(".opi/config.toml"),
            TrustResource::Skills => Path::new(".opi/skills"),
            TrustResource::Fragments => Path::new(".opi/fragments"),
            TrustResource::Themes => Path::new(".opi/themes"),
            TrustResource::Extensions => Path::new(".opi/extensions"),
            TrustResource::Packages => Path::new(".opi/packages.toml"),
            TrustResource::AgentsMd => Path::new("AGENTS.md"),
            TrustResource::ClaudeMd => Path::new("CLAUDE.md"),
        }
    }

    /// The ordered set of resources whose presence requires consent.
    fn all() -> &'static [TrustResource] {
        &[
            TrustResource::ProjectConfig,
            TrustResource::Skills,
            TrustResource::Fragments,
            TrustResource::Themes,
            TrustResource::Extensions,
            TrustResource::Packages,
            TrustResource::AgentsMd,
            TrustResource::ClaudeMd,
        ]
    }
}

/// Context handed to a resolver: the canonicalizable project path and the
/// triggering resources that caused the gate to fire.
#[derive(Debug, Clone)]
pub struct TrustContext {
    /// The project root being decided on.
    pub project_path: PathBuf,
    /// The trust-requiring resources detected in the project (possibly empty).
    pub triggering_resources: Vec<TrustResource>,
}

// ---------------------------------------------------------------------------
// Resolver + pre-trust UI contracts
// ---------------------------------------------------------------------------

/// A project-trust resolver: an embedder/startup-supplied policy that may
/// authorize or deny a project before the store/default/ask path is reached.
///
/// Only user-global and explicit embedder-registered resolvers may own the
/// decision. Project-local extensions are not loaded until after trust
/// resolves (a 15.7 structural guarantee), so a project extension cannot
/// influence its own decision.
///
/// The trait is dyn-safe via hand-rolled boxed futures (matching the
/// `opi-agent` `Tool`/`AgentHooks` style); no `async-trait` dependency.
pub trait ProjectTrustResolver: Send + Sync {
    /// Vote on the project. `Trust`/`Deny` are authoritative at precedence
    /// position 2; `Undecided` falls through to store/default/ask.
    fn resolve(
        &self,
        ctx: &TrustContext,
        ui: &dyn PreTrustUi,
    ) -> Pin<Box<dyn Future<Output = TrustVote> + Send + '_>>;
}

/// Minimal pre-trust UI surface exposed to resolvers before the project layer
/// is loaded. It deliberately offers no full TUI access, preventing privilege
/// escalation before consent. The headless non-interactive/RPC implementations
/// return [`PreTrustUiError::Unavailable`] from `select`/`confirm`/`input` and
/// make `notify` a no-op.
pub trait PreTrustUi: Send + Sync {
    /// Present a single-select prompt; returns the chosen index.
    fn select(
        &self,
        prompt: &str,
        options: &[&str],
    ) -> Pin<Box<dyn Future<Output = Result<usize, PreTrustUiError>> + Send + '_>>;
    /// Present a yes/no prompt.
    fn confirm(
        &self,
        prompt: &str,
    ) -> Pin<Box<dyn Future<Output = Result<bool, PreTrustUiError>> + Send + '_>>;
    /// Present a free-text prompt.
    fn input(
        &self,
        prompt: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, PreTrustUiError>> + Send + '_>>;
    /// Display an informational message (no response expected).
    fn notify(&self, msg: &str) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;
}

// ---------------------------------------------------------------------------
// Resolver registry
// ---------------------------------------------------------------------------

/// Deterministic-order registry of explicitly registered project-trust
/// resolvers.
///
/// A newly constructed registry is **empty**. Resolvers are added only through
/// the explicit public embedder/startup API ([`Self::register`]) before
/// resolution; the standard CLI constructs an empty registry and Phase 15 adds
/// no CLI `-e`, metadata discovery, or native-code loading path. Once
/// [`Self::seal`] is called (at resolution time), further registration is
/// rejected with [`TrustError::RegistrySealed`].
#[derive(Default)]
pub struct ProjectTrustResolverRegistry {
    resolvers: Vec<Arc<dyn ProjectTrustResolver>>,
    sealed: bool,
}

impl ProjectTrustResolverRegistry {
    /// Construct a new, empty, unsealed registry (the standard CLI default).
    pub fn new() -> Self {
        Self {
            resolvers: Vec::new(),
            sealed: false,
        }
    }

    /// Register a resolver in deterministic insertion order. Rejected with
    /// [`TrustError::RegistrySealed`] once the registry is sealed.
    pub fn register(&mut self, resolver: Arc<dyn ProjectTrustResolver>) -> Result<(), TrustError> {
        if self.sealed {
            return Err(TrustError::RegistrySealed);
        }
        self.resolvers.push(resolver);
        Ok(())
    }

    /// Seal the registry against further registration. Called when resolution
    /// begins.
    pub fn seal(&mut self) {
        self.sealed = true;
    }

    /// `true` if no resolvers are registered.
    pub fn is_empty(&self) -> bool {
        self.resolvers.is_empty()
    }

    /// `true` if [`Self::seal`] has been called.
    pub fn is_sealed(&self) -> bool {
        self.sealed
    }

    /// Number of registered resolvers.
    pub fn len(&self) -> usize {
        self.resolvers.len()
    }

    /// The registered resolvers in insertion order.
    pub fn resolvers(&self) -> &[Arc<dyn ProjectTrustResolver>] {
        &self.resolvers
    }
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

/// On-disk project trust store backed by a flat `trust.json` map.
///
/// The store is path-only: `decide` re-reads `trust.json` so a `record` on the
/// same instance is immediately observable (the `&self` `record` signature,
/// matched from the design doc, lets concurrent writers share an
/// `Arc<ProjectTrustStore>`). `load` validates the file eagerly so a malformed
/// store surfaces [`TrustError::MalformedJson`] at startup.
#[derive(Debug)]
pub struct ProjectTrustStore {
    path: PathBuf,
    lock_path: PathBuf,
}

impl ProjectTrustStore {
    /// Load the store from `user_config_dir/trust.json`. A missing file is an
    /// empty store; a malformed file is a [`TrustError::MalformedJson`].
    pub fn load(user_config_dir: &Path) -> Result<Self, TrustError> {
        let path = user_config_dir.join("trust.json");
        let lock_path = user_config_dir.join("trust.json.lock");
        // Validate parse eagerly so a malformed store fails at startup rather
        // than silently degrading per-decide.
        let _ = read_map(&path)?;
        Ok(Self { path, lock_path })
    }

    /// Canonicalize `project_path` and return the nearest ancestor's stored
    /// decision. A path that cannot be canonicalized, or one with no stored
    /// ancestor, yields [`TrustDecision::Undecided`].
    pub fn decide(&self, project_path: &Path) -> TrustDecision {
        let canon = match std::fs::canonicalize(project_path) {
            Ok(p) => p,
            Err(_) => return TrustDecision::Undecided,
        };
        let map = match read_map(&self.path) {
            Ok(Some(m)) => m,
            Ok(None) => BTreeMap::new(),
            // A store that was valid at load but is unreadable now degrades to
            // no-decision rather than panicking.
            Err(_) => return TrustDecision::Undecided,
        };
        let mut current: Option<&Path> = Some(&canon);
        while let Some(candidate) = current {
            if let Some(trusted) = map.iter().find_map(|(k, &v)| {
                if Path::new(k) == candidate {
                    Some(v)
                } else {
                    None
                }
            }) {
                return if trusted {
                    TrustDecision::Trusted
                } else {
                    TrustDecision::Untrusted
                };
            }
            current = candidate.parent();
        }
        TrustDecision::Undecided
    }

    /// Persist one canonical-key decision. Canonicalizes `path`, acquires the
    /// `trust.json.lock` sidecar, re-reads after locking, updates one key, and
    /// atomically replaces the file. No schema metadata is written.
    pub fn record(&self, path: &Path, trusted: bool) -> Result<(), TrustError> {
        let key = std::fs::canonicalize(path)
            .map_err(|_| TrustError::Canonicalize(path.to_path_buf()))?;

        // Create (if absent) and exclusively lock the sidecar lock file. The
        // lock file holds no data; it is pure cross-process coordination.
        let lock_file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&self.lock_path)?;
        fs4::FileExt::lock(&lock_file).map_err(|e| TrustError::Lock(e.to_string()))?;

        let outcome = self.record_locked(&key, trusted);
        // Always release the lock, even on failure.
        let _ = fs4::FileExt::unlock(&lock_file);
        outcome
    }

    fn record_locked(&self, key: &Path, trusted: bool) -> Result<(), TrustError> {
        let mut map = read_map(&self.path)?.unwrap_or_default();
        map.insert(key.to_string_lossy().into_owned(), trusted);
        write_json_atomic(&self.path, &map)
    }

    /// Detect the trust-requiring resources present under `project_root`. A
    /// bare `.opi` directory (present but containing none of the gated
    /// files/directories) yields an empty set.
    pub fn detect_resources(project_root: &Path) -> Vec<TrustResource> {
        TrustResource::all()
            .iter()
            .copied()
            .filter(|r| project_root.join(r.rel_path()).exists())
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Resolution precedence
// ---------------------------------------------------------------------------

/// Resolve the trust decision for `ctx` using the pi-style precedence chain:
/// CLI override, registered resolver votes (deterministic order), the store's
/// nearest-ancestor decision, the global-only default, then ask via
/// [`PreTrustUi::confirm`]. The first `Trusted`/`Untrusted` wins; `Undecided`
/// falls through. `global_default == Undecided` means "ask". An unavailable ask
/// yields a terminal [`TrustDecision::Undecided`] that headless callers
/// (15.8.1) map to `Untrusted`.
pub async fn resolve_trust(
    cli_override: Option<TrustDecision>,
    registry: &ProjectTrustResolverRegistry,
    store: &ProjectTrustStore,
    ctx: &TrustContext,
    global_default: TrustDecision,
    ui: &dyn PreTrustUi,
) -> TrustDecision {
    // 1. CLI override (`--trust`/`--no-trust`): authoritative when present.
    if let Some(decision) = cli_override {
        return decision;
    }

    // 2. Registered resolver votes, in deterministic registration order.
    for resolver in registry.resolvers() {
        let decision = resolver.resolve(ctx, ui).await.to_decision();
        if decision.is_decided() {
            return decision;
        }
    }

    // 3. Store: nearest ancestor decision.
    let stored = store.decide(&ctx.project_path);
    if stored.is_decided() {
        return stored;
    }

    // 4. Global-only default ([defaults] default_project_trust). A decided
    //    always/never wins; `Undecided` (the `ask` default) falls through.
    if global_default.is_decided() {
        return global_default;
    }

    // 5. Ask via the minimal pre-trust UI.
    match ui
        .confirm("Trust this project and load its resources?")
        .await
    {
        Ok(true) => TrustDecision::Trusted,
        Ok(false) => TrustDecision::Untrusted,
        Err(_) => TrustDecision::Undecided,
    }
}

/// Resolve the project trust decision for production startup (task 15.7).
///
/// Detects trust-requiring resources under `project_root`; an empty set yields
/// [`TrustDecision::Trusted`] (no gate fires in any mode). Otherwise the store's
/// nearest-ancestor decision is authoritative. In this phase an
/// [`TrustDecision::Undecided`] result (no prior stored decision) is mapped to
/// [`TrustDecision::Trusted`] so existing single-layer projects keep loading
/// unchanged; task 15.8.1 replaces this with `prepare_project_startup`, which
/// adds `--trust`/`--no-trust`, `[defaults] default_project_trust`, and the
/// headless ask policy and tightens the undecided case.
pub fn resolve_project_trust_decision(
    store: &ProjectTrustStore,
    project_root: &Path,
) -> TrustDecision {
    if ProjectTrustStore::detect_resources(project_root).is_empty() {
        return TrustDecision::Trusted;
    }
    match store.decide(project_root) {
        TrustDecision::Untrusted => TrustDecision::Untrusted,
        TrustDecision::Trusted => TrustDecision::Trusted,
        TrustDecision::Undecided => TrustDecision::Trusted,
    }
}

// ---------------------------------------------------------------------------
// Task 15.8.1: startup policy, preflight entry, headless UI
// ---------------------------------------------------------------------------

/// `[defaults] default_project_trust` policy (task 15.8.1).
///
/// `Ask` (default) falls through to the prompt/headless resolution;
/// `Always` authorizes every project; `Never` denies every project. The value
/// is read from the **pre-trust** (global) config so a project cannot
/// self-authorize by setting its own `[defaults] default_project_trust`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectTrustDefault {
    #[default]
    Ask,
    Always,
    Never,
}

impl ProjectTrustDefault {
    /// Map the policy to the decision it implies at precedence position 4.
    /// `Ask` yields [`TrustDecision::Undecided`] ("keep asking"); `Always`/`Never`
    /// are decisive.
    pub fn to_decision(self) -> TrustDecision {
        match self {
            ProjectTrustDefault::Ask => TrustDecision::Undecided,
            ProjectTrustDefault::Always => TrustDecision::Trusted,
            ProjectTrustDefault::Never => TrustDecision::Untrusted,
        }
    }
}

/// Parsed `--trust` / `--no-trust` CLI input (task 15.8.1). The two flags are
/// mutually exclusive; [`cli_trust_override`] validates that and maps the pair
/// to an override decision.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProjectTrustCli {
    /// `--trust` was set on the command line.
    pub trust: bool,
    /// `--no-trust` was set on the command line.
    pub no_trust: bool,
}

/// Validate `--trust`/`--no-trust` and map them to a CLI override decision.
/// `Err(TrustError::ConflictingCliFlags)` if both are set; `Ok(Some(..))` when
/// exactly one is set; `Ok(None)` when neither is set (fall through).
pub fn cli_trust_override(cli: ProjectTrustCli) -> Result<Option<TrustDecision>, TrustError> {
    match (cli.trust, cli.no_trust) {
        (true, true) => Err(TrustError::ConflictingCliFlags),
        (true, false) => Ok(Some(TrustDecision::Trusted)),
        (false, true) => Ok(Some(TrustDecision::Untrusted)),
        (false, false) => Ok(None),
    }
}

/// The resolved startup plan returned by [`prepare_project_startup`].
///
/// `decision` may be [`TrustDecision::Undecided`] when every layer abstained
/// and the ask could not resolve (headless UI). Headless callers apply
/// [`Self::headless_decision`] to map that to [`TrustDecision::Untrusted`];
/// interactive callers (15.8.2) render the prompt instead.
#[derive(Debug, Clone)]
pub struct ProjectStartupPlan {
    /// The raw resolved decision (may be `Undecided`).
    pub decision: TrustDecision,
    /// The trust-requiring resources detected in the project (empty => no gate).
    pub resources: Vec<TrustResource>,
    /// The validated CLI override (`--trust`/`--no-trust`), if any.
    pub cli_override: Option<TrustDecision>,
}

impl ProjectStartupPlan {
    /// The resolved decision with the headless ask policy applied: an
    /// unresolved ask denies project resources (non-interactive/RPC never
    /// prompt).
    pub fn headless_decision(&self) -> TrustDecision {
        match self.decision {
            TrustDecision::Undecided => TrustDecision::Untrusted,
            decided => decided,
        }
    }
}

/// Headless [`PreTrustUi`] for non-interactive and RPC startup (task 15.8.1).
///
/// `select`/`confirm`/`input` immediately return [`PreTrustUiError::Unavailable`]
/// and `notify` is a no-op. Neither headless mode can prompt, so an unresolved
/// ask yields a terminal [`TrustDecision::Undecided`] that the caller maps to
/// `Untrusted`. RPC emits no UI request because trust resolves in `main` before
/// the RPC runner (and its wire) exists; this type has no request surface.
#[derive(Debug, Clone, Copy, Default)]
pub struct HeadlessPreTrustUi;

impl PreTrustUi for HeadlessPreTrustUi {
    fn select(
        &self,
        _prompt: &str,
        _options: &[&str],
    ) -> Pin<Box<dyn Future<Output = Result<usize, PreTrustUiError>> + Send + '_>> {
        Box::pin(async { Err(PreTrustUiError::Unavailable) })
    }
    fn confirm(
        &self,
        _prompt: &str,
    ) -> Pin<Box<dyn Future<Output = Result<bool, PreTrustUiError>> + Send + '_>> {
        Box::pin(async { Err(PreTrustUiError::Unavailable) })
    }
    fn input(
        &self,
        _prompt: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, PreTrustUiError>> + Send + '_>> {
        Box::pin(async { Err(PreTrustUiError::Unavailable) })
    }
    fn notify(&self, _msg: &str) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(async {})
    }
}

/// The sole preflight entry used by the standard CLI and embedders (task
/// 15.8.1).
///
/// Validates the CLI trust flags, detects trust-requiring resources, and returns
/// a no-resource [`ProjectStartupPlan`] with decision [`TrustDecision::Trusted`]
/// **before** resolver invocation, trust-store loading, global-default
/// evaluation, or [`PreTrustUi`] calls. When resources exist, it seals
/// `registry` (rejecting late registration), loads `trust.json` from
/// `user_config_dir`, then runs [`resolve_trust`]: CLI override, registered
/// resolver votes, nearest stored decision, `global_default`, then ask. The
/// first `Trust`/`Deny` wins; `Undecided` falls through.
///
/// `global_default` must be the **global** (pre-trust) `[defaults]
/// default_project_trust` value so a project cannot self-authorize via its own
/// config. The returned plan's [`ProjectStartupPlan::decision`] is raw (possibly
/// `Undecided`); headless callers apply [`ProjectStartupPlan::headless_decision`].
pub async fn prepare_project_startup(
    cli: ProjectTrustCli,
    registry: &mut ProjectTrustResolverRegistry,
    user_config_dir: &Path,
    project_root: &Path,
    global_default: TrustDecision,
    ui: &dyn PreTrustUi,
) -> Result<ProjectStartupPlan, TrustError> {
    // 1. Validate CLI flags before any resource/store work.
    let cli_override = cli_trust_override(cli)?;

    // 2. Detect trust-requiring resources. An empty set short-circuits to a
    //    trusted plan with NO resolver, store, default, or UI side effects.
    let resources = ProjectTrustStore::detect_resources(project_root);
    if resources.is_empty() {
        return Ok(ProjectStartupPlan {
            decision: TrustDecision::Trusted,
            resources,
            cli_override,
        });
    }

    // 3. Seal the registry so late registration (project-extension
    //    self-authorization) is impossible once resolution has begun.
    registry.seal();

    // 4. Load the store only now (the no-resource path never touches it).
    let store = ProjectTrustStore::load(user_config_dir)?;
    let ctx = TrustContext {
        project_path: project_root.to_path_buf(),
        triggering_resources: resources.clone(),
    };

    // 5. Full precedence: CLI -> resolvers -> store -> global default -> ask.
    let decision = resolve_trust(cli_override, registry, &store, &ctx, global_default, ui).await;

    Ok(ProjectStartupPlan {
        decision,
        resources,
        cli_override,
    })
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Read and parse `trust.json` as a flat canonical-path -> bool map. `Ok(None)`
/// means the file is absent (empty store); `Err(MalformedJson)` means it exists
/// but is not a flat bool map.
fn read_map(path: &Path) -> Result<Option<BTreeMap<String, bool>>, TrustError> {
    match std::fs::read_to_string(path) {
        Ok(text) => {
            let map: BTreeMap<String, bool> = serde_json::from_str(&text)
                .map_err(|e| TrustError::MalformedJson(e.to_string()))?;
            Ok(Some(map))
        }
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(TrustError::Io(e)),
    }
}

/// Atomically replace `path` with the compact flat map, via a same-directory
/// `trust.json.tmp` + rename. Must be called while holding the advisory lock.
fn write_json_atomic(path: &Path, map: &BTreeMap<String, bool>) -> Result<(), TrustError> {
    let bytes = serde_json::to_vec(map).map_err(|e| TrustError::Lock(format!("serialize: {e}")))?;
    let directory = path.parent().unwrap_or_else(|| Path::new("."));
    let stage = directory.join("trust.json.tmp");
    std::fs::write(&stage, &bytes)?;
    std::fs::rename(&stage, path)?;
    Ok(())
}
