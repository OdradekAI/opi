//! Interactive CLI harness (S8.4) and coding-agent product wrapper over the
//! generic opi-agent runtime seams (Phase 10, Workstream 10.2).
//!
//! `CodingHarness` is the coding-agent product wrapper. It composes
//! coding-agent product inputs over the generic opi-agent runtime and owns the
//! product policy the design keeps out of `opi-agent`:
//! - the eight built-in file tools and [`ToolRuntimeConfig`] selection;
//! - CLI/project config ([`OpiConfig`]) and context-file discovery;
//! - package resources/adapters, skills, fragments, and themes;
//! - interactive commands and product defaults;
//! - extension-state restore/persist and session resume/fork/branch.
//!
//! The generic turn lifecycle, phase guards, save points, runtime-config
//! snapshots, and pending-write ordering live in [`opi_agent::harness`] (the
//! `AgentHarness` seam). `CodingHarness` drives turns through the generic
//! [`opi_agent::Agent`] loop and persists through the generic
//! [`opi_agent::session`] storage today; routing the product turn loop through
//! `AgentHarness` itself is a later incremental migration (see the
//! `opi_agent::harness` module docs), intentionally not a thin adapter.
//!
//! Boundary contract: product/CLI/package policy must not move into `opi-agent`.
//! This is pinned by `coding_harness_wrapper_keeps_product_policy_out_of_opi_agent`,
//! and the wrapper composition is exercised by
//! `coding_harness_composes_generic_opi_agent_seams`. Existing CLI/RPC/JSON/
//! interactive behavior continues to run through this wrapper unchanged.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use opi_agent::diagnostic::code::{
    CODE_SESSION_RESUME_MODEL_INCOMPATIBLE, CODE_SESSION_RESUME_THINKING_INCOMPATIBLE,
};
use opi_agent::diagnostic::{
    Diagnostic, DiagnosticPayload, RedactionMode, SOURCE_SESSION, Severity,
};
use opi_agent::event::AgentEvent;
use opi_agent::extension::ExtensionRegistry;
use opi_agent::hooks::AgentHooks;
use opi_agent::loop_types::{AgentError, AgentLoopConfig};
use opi_agent::message::AgentMessage;
use opi_agent::session_context::reconstruct_context;
use opi_agent::session_event::{SessionDiagnosticCounts, ThinkingLevel};
use opi_agent::tool::Tool;
use opi_agent::trace::TraceKind;
use opi_agent::{Agent, DiagnosticSink, RecordingSink, TraceCollector, TraceSink};
use opi_ai::message::Message;
use opi_ai::provider::{ModelInfo, Provider, ThinkingConfig};
use serde::Serialize;

use crate::config::{
    ExecutionConfig, ExecutionRunMode, ExecutionStrategy, OpiConfig, PermissionDecision,
};
use crate::context_files;
use crate::credential_store::KeychainCredentialStore;
use crate::diagnostic_bridge::{
    diagnostic_for_package_discovery_error, diagnostic_for_resource_discovery_error,
    diagnostic_for_resource_layer_message, diagnostic_from_execution_failure,
    diagnostic_from_package, diagnostic_from_package_resolution_error,
};
use crate::execution::LOCAL_ADAPTER_ID;
use crate::execution::permission::{
    InteractivePermissionBroker, PermissionManager, PermissionPolicy,
};
use crate::execution::router::concrete_adapter_id;
use crate::execution::runtime::{ExecutionPlan, execution_plan};
use crate::execution::{
    Eligibility, EnabledIdentity, ExecutionFailure, ExecutionRuntime, IdentitySource,
};
use crate::oauth::{OAuthEndpointConfig, OAuthProviderRegistry};
use crate::package_activation::{
    ActivatedContribution, ActivationError, PackageActivationStore, host_opi_version,
    host_target_triple,
};
use crate::package_discovery::PackageResource;
use crate::policy::{RunMode, ToolRuntimeConfig, ToolSelection};
use crate::project_trust::TrustDecision;
use crate::prompt::SystemPromptBuilder;
use crate::resource::{
    DiscoveryLayerKind, ExplicitResourcePaths, ResourceDiscoveryLayers, standard_discovery_layers,
};
use crate::session_coordinator::{SessionCoordinator, to_wire_result};
use crate::tool::{
    BashOperations, BashTool, EditTool, FileOperations, FindTool, GlobTool, GrepTool,
    LocalBashOperations, LocalFileOperations, LsTool, ReadTool, WriteTool, default_bash_schema,
    with_model_backend_enum,
};
use tokio::sync::mpsc;

/// Phase 16.9: resolved routed-execution inputs threaded into
/// [`ExecutionRuntime::build`]. Production constructs this only after the early
/// Minimal-Runtime/headless-refusal classifier. Tests may inject fixed-local
/// fixtures to drive [`CodingHarness::build_tools`] at the runtime seam.
///
/// `policy` is the resolved [`PermissionPolicy`] derived from
/// `config.execution.permissions` (NOT [`PermissionPolicy::empty`], which would
/// silently drop an explicit user `local = "deny"`/`ask`); in production,
/// `store` is the live package-activation source used to revalidate external
/// adapters per invocation.
#[derive(Clone)]
pub struct ExecutionWiring {
    pub config: ExecutionConfig,
    pub enabled: Vec<EnabledIdentity>,
    pub policy: PermissionPolicy,
    pub store: Arc<dyn IdentitySource>,
    pub mode: ExecutionRunMode,
    pub host_target: String,
    pub host_opi_version: String,
    /// In-memory session-grant store, shared with the routed bash backend and
    /// reset on in-process session switches. One fresh manager per harness.
    pub manager: Arc<PermissionManager>,
    /// Interactive `ask`-prompt broker. `None` is the fail-closed default: an
    /// interactive `ask` surfaces `permission_required` rather than dispatching
    /// or falling back to `local`. Headless modes install no broker; the
    /// interactive startup path installs the TUI-backed broker (Phase 16.10).
    pub broker: Option<Arc<dyn InteractivePermissionBroker>>,
}

/// Resolve the dynamic bash input schema for the current execution config.
///
/// The default (`fixed`/`rules`/default-local) schema is [`default_bash_schema`]
/// byte-for-byte. Under `strategy = "model"` the required bounded `backend` enum
/// is added (listing only model-visible, non-denied adapters) so the model can
/// name a backend the router will accept; `fixed`/`rules` never carry the field.
fn bash_input_schema(
    config: &ExecutionConfig,
    enabled: &[EnabledIdentity],
    policy: &PermissionPolicy,
) -> Option<serde_json::Value> {
    let base = default_bash_schema();
    match config.strategy {
        ExecutionStrategy::Model => {
            // Build the model-visible candidates (available && !deny) annotated
            // with whether each requires interactive approval (ask), so the
            // schema's per-candidate description can flag ask adapters per the
            // design (§Model routing: "An ask candidate is visible with a
            // description that it requires interactive approval. A deny candidate
            // is absent.").
            let eligibility = Eligibility::from_enabled(enabled, policy);
            let candidates: Vec<(&str, bool)> = eligibility
                .0
                .iter()
                .filter(|a| a.available && a.permission != PermissionDecision::Deny)
                .map(|a| (a.id.as_str(), a.permission == PermissionDecision::Ask))
                .collect();
            with_model_backend_enum(base, &candidates)
        }
        _ => Some(base),
    }
}

/// Build routed [`ExecutionWiring`] from the layered config and the global
/// package-activation store. Fixed and rules routing revalidate only the one
/// concrete selected identity; model routing revalidates only non-denied
/// candidates before exposing them. The policy is [`PermissionPolicy::from_map`]
/// over the resolved permissions so explicit user deny/ask/allow for `local`
/// and externals is honored exactly by the routed branch. Fixed-local allow and
/// headless fixed-local ask are classified before this function is called.
fn execution_wiring(
    config: &OpiConfig,
    global_config_dir: &Path,
    mode: ExecutionRunMode,
    policy: PermissionPolicy,
) -> Result<ExecutionWiring, ExecutionFailure> {
    let host_target = host_target_triple().to_string();
    let host_opi_version = host_opi_version().to_string();
    let RoutedStoreState { store, enabled } = routed_store_state(
        global_config_dir,
        &config.execution,
        mode,
        &policy,
        &host_target,
        &host_opi_version,
    )?;
    Ok(ExecutionWiring {
        config: config.execution.clone(),
        enabled,
        policy,
        store,
        mode,
        host_target,
        host_opi_version,
        // Fresh per-harness manager (memory-only grants). The broker defaults to
        // None (fail-closed); the interactive startup path installs the
        // TUI-backed broker (Phase 16.10 interactive wiring).
        manager: new_permission_manager(),
        broker: None,
    })
}

struct RoutedStoreState {
    store: Arc<dyn IdentitySource>,
    enabled: Vec<EnabledIdentity>,
}

fn routed_store_state(
    global_config_dir: &Path,
    config: &ExecutionConfig,
    mode: ExecutionRunMode,
    policy: &PermissionPolicy,
    host_target: &str,
    host_opi_version: &str,
) -> Result<RoutedStoreState, ExecutionFailure> {
    #[cfg(test)]
    if let Some(state) = routed_store_factory_override::invoke() {
        return Ok(state);
    }

    let store = PackageActivationStore::global(global_config_dir.to_path_buf());
    let enabled = match config.strategy {
        ExecutionStrategy::Fixed | ExecutionStrategy::Rules => {
            let selected = concrete_adapter_id(config, mode)
                .filter(|adapter_id| *adapter_id != LOCAL_ADAPTER_ID)
                .map(str::to_owned)
                .into_iter()
                .collect::<Vec<_>>();
            store
                .usable_enabled_identities_for(&selected, host_target, host_opi_version)
                .map_err(ExecutionFailure::from)?
        }
        ExecutionStrategy::Model => {
            let candidates = store
                .enabled_identities()
                .into_iter()
                .filter(|identity| !policy.is_denied(&identity.adapter_id))
                .collect::<Vec<_>>();
            let mut packages = Vec::new();
            let mut usable = Vec::new();
            for identity in &candidates {
                if packages.contains(&identity.package_name) {
                    continue;
                }
                packages.push(identity.package_name.clone());
                let Ok(activated) =
                    store.activate(&identity.package_name, host_target, host_opi_version)
                else {
                    continue;
                };
                usable.extend(
                    candidates
                        .iter()
                        .filter(|candidate| candidate.package_name == identity.package_name)
                        .filter(|candidate| {
                            activated
                                .validated
                                .iter()
                                .any(|contribution| contribution.id == candidate.adapter_id)
                        })
                        .cloned(),
                );
            }
            usable
        }
    };
    Ok(RoutedStoreState {
        store: Arc::new(store),
        enabled,
    })
}

#[cfg(test)]
mod routed_store_factory_override {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::RoutedStoreState;

    type Factory = Rc<dyn Fn() -> RoutedStoreState>;

    thread_local! {
        static FACTORY: RefCell<Option<Factory>> = const { RefCell::new(None) };
    }

    pub(super) struct Guard {
        previous: Option<Factory>,
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            FACTORY.with(|factory| {
                factory.replace(self.previous.take());
            });
        }
    }

    pub(super) fn install(factory: impl Fn() -> RoutedStoreState + 'static) -> Guard {
        let previous = FACTORY.with(|active| active.replace(Some(Rc::new(factory))));
        Guard { previous }
    }

    pub(super) fn invoke() -> Option<RoutedStoreState> {
        let factory = FACTORY.with(|active| active.borrow().clone());
        factory.map(|factory| factory())
    }
}

fn new_permission_manager() -> Arc<PermissionManager> {
    #[cfg(test)]
    crate::execution::runtime::construction_probe::permission_manager_constructed();
    Arc::new(PermissionManager::new())
}

/// A no-state [`IdentitySource`] that panics if activated. Fixed-local routing
/// never selects an external package, so this sentinel is used by explicit
/// interactive local-ask wiring and Minimal-Runtime test fixtures.
struct PanicIdentitySource;
impl IdentitySource for PanicIdentitySource {
    fn activate(
        &self,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<ActivatedContribution, ActivationError> {
        panic!("Minimal Runtime must not activate any package");
    }
}

/// A default-local / no-enabled-extensions execution fixture. Tests use this to
/// drive [`CodingHarness::build_tools`] at the runtime seam; the real harness
/// classifies Minimal Runtime before constructing [`ExecutionWiring`].
pub fn minimal_runtime_wiring(mode: ExecutionRunMode) -> ExecutionWiring {
    ExecutionWiring {
        config: ExecutionConfig::default(),
        enabled: Vec::new(),
        policy: PermissionPolicy::empty(),
        store: Arc::new(PanicIdentitySource),
        mode,
        host_target: host_target_triple().to_string(),
        host_opi_version: host_opi_version().to_string(),
        manager: Arc::new(PermissionManager::new()),
        broker: None,
    }
}

enum HarnessExecution {
    DirectLocal,
    Refused(ExecutionFailure),
    Routed(ExecutionWiring),
}

/// Resolve the execution shape before opening activation state or constructing
/// permission/routing state. This decision uses the fully resolved config and
/// effective local decision, so only fixed-local allow enters the Minimal
/// Runtime. Explicit interactive local ask remains routed through the broker;
/// headless ask is refused at build time without any prompt channel.
fn harness_execution(
    config: &OpiConfig,
    global_config_dir: &Path,
    mode: ExecutionRunMode,
) -> HarnessExecution {
    let policy = PermissionPolicy::from_map(config.execution.permissions.clone());
    let plan = execution_plan(&config.execution, mode, &policy);
    if let Some(failure) = plan.refusal(mode) {
        return HarnessExecution::Refused(failure);
    }

    match plan {
        ExecutionPlan::DirectLocal => HarnessExecution::DirectLocal,
        ExecutionPlan::InteractiveAskRouted => HarnessExecution::Routed(ExecutionWiring {
            config: config.execution.clone(),
            enabled: Vec::new(),
            policy,
            store: Arc::new(PanicIdentitySource),
            mode,
            host_target: host_target_triple().to_string(),
            host_opi_version: host_opi_version().to_string(),
            manager: new_permission_manager(),
            broker: None,
        }),
        ExecutionPlan::GeneralRouted => {
            match execution_wiring(config, global_config_dir, mode, policy) {
                Ok(wiring) => HarnessExecution::Routed(wiring),
                Err(failure) => HarnessExecution::Refused(failure),
            }
        }
        ExecutionPlan::PolicyDenied | ExecutionPlan::HeadlessAskRefused => {
            unreachable!("refused execution plans returned above")
        }
    }
}

/// Optional pre-existing session the harness can adopt instead of creating
/// a new JSONL file. Produced by `--resume` flows.
pub struct ResumeInfo {
    pub path: PathBuf,
    pub session_id: String,
    pub entries: Vec<opi_agent::session::SessionEntry>,
    /// The workspace cwd recorded in the session header. Used to restore the
    /// correct workspace root when resuming from a different directory.
    pub original_cwd: PathBuf,
    /// Structured diagnostics observed while reading the resumed session.
    pub diagnostics: Vec<Diagnostic>,
    /// Latest `model_change` recorded on the active branch (Phase 13.3), if
    /// any. The harness re-applies it when compatible with the CLI/config
    /// provider, mirroring `CodingHarness::resume_session_id`.
    pub recorded_model: Option<String>,
    /// Latest `thinking_level_change` recorded on the active branch (Phase
    /// 13.3), if any. Re-applied when compatible with the active model.
    pub recorded_thinking: Option<ThinkingLevel>,
}

/// Opt-in trace configuration handed to the harness (Phase 7 task 7.5).
///
/// When set, the harness builds a fresh [`TraceCollector`] per run over the
/// shared `sink`, prepares it before the run (fail-closed), and finishes it
/// after. `TraceSink::prepare` defines per-run reset semantics: file sinks
/// truncate on each run, while recording sinks keep only the latest run.
/// `mode` controls redaction of record details (Summary by default).
#[derive(Clone)]
pub struct TraceConfig {
    pub sink: Arc<dyn TraceSink>,
    pub mode: RedactionMode,
}

/// Coding-agent product wrapper over the generic opi-agent runtime seams.
///
/// Owns coding-agent product policy (built-in file tools, CLI/project config,
/// context files, package resources/adapters, interactive commands, product
/// defaults, extension-state restore/persist) and composes it over the generic
/// [`Agent`] loop, [`AgentHooks`], [`ExtensionRegistry`], generic session
/// storage, [`opi_ai::ProviderCollection`], and compaction. Generic turn
/// lifecycle / phase / save-point / pending-write semantics are owned by
/// [`opi_agent::harness`]. See the module docs for the product-vs-generic
/// boundary and the incremental `AgentHarness`-adoption note.
pub struct CodingHarness {
    agent: Agent,
    config: OpiConfig,
    system_prompt: String,
    resources: HarnessResources,
    model_registry: opi_ai::ProviderCollection,
    extension_registry: Option<ExtensionRegistry>,
    session: Option<SessionCoordinator>,
    /// Message count before the current turn - used to slice only new messages for persistence.
    turn_offset: usize,
    /// Images queued from --image CLI flag, injected into the first prompt.
    pending_images: Vec<opi_ai::message::InputContent>,
    /// Extension state loaded from a resumed session and restored before the
    /// next async agent operation.
    pending_extension_state: Option<serde_json::Value>,
    /// Optional recording sink that captures runtime diagnostics (retry,
    /// cancellation, provider/tool failures) emitted during a run, so a run
    /// summary can report severity counts. `None` (the default) leaves the
    /// diagnostic sink unset, preserving pre-7.5 behavior.
    diagnostics: Option<Arc<RecordingSink>>,
    /// Opt-in trace configuration. When set, each prompt run is traced.
    trace: Option<TraceConfig>,
    /// The collector prepared for the run in progress, shared with the agent
    /// (loop records) and the harness (compaction record). `None` between runs.
    active_trace: Option<Arc<TraceCollector>>,
    /// Monotonic counter minting per-run trace run ids.
    run_seq: u64,
    /// The OS-keychain-backed credential store, set by production startup.
    /// Used by the interactive loop for `/login` and `/logout`.
    pub credential_store: Option<Arc<KeychainCredentialStore>>,
    /// The built-in OAuth provider registry, set by production startup.
    pub oauth_registry: Option<OAuthProviderRegistry>,
    pub(crate) oauth_endpoints: OAuthEndpointConfig,
    pub(crate) oauth_http_client: reqwest::Client,
    /// In-memory capability-permission grants (Phase 16.10). Present only for
    /// routed execution, shared with the routed bash backend, and reset on
    /// in-process session switches so an `allow-for-session` choice does not
    /// survive resume/fork/branch. Minimal and startup-refused execution use
    /// `None` and construct no permission state.
    pub(crate) permission_manager: Option<Arc<PermissionManager>>,
    /// The interactive permission-prompt channel receiver (Phase 16.10).
    /// `Some` only for interactive routed execution (the TUI broker is
    /// installed); taken by `run_interactive_tui` to drain prompt requests.
    /// Minimal Runtime and headless modes use `None`.
    pub(crate) permission_prompt_rx:
        Option<mpsc::Receiver<crate::interactive::PermissionPromptRequest>>,
    /// Unit-test-only per-instance session lookup root. Production session
    /// creation and resume always use [`crate::session_cli::session_dir`].
    #[cfg(test)]
    session_dir_override: Option<PathBuf>,
}

pub struct RuntimeThinkingState {
    pub level: String,
    pub enabled: bool,
    pub budget_tokens: Option<u64>,
}

struct PendingThinkingChange {
    persisted_level: ThinkingLevel,
    thinking: Option<ThinkingConfig>,
    max_tokens: Option<u64>,
    state: RuntimeThinkingState,
}

/// Aggregated live session metadata (Phase 13.4) surfaced by `/session info`
/// and RPC `session_info`. Name and labels are UI-visible metadata that never
/// enter provider context; `active_branch`, `model`, and `thinking` mirror the
/// current runtime state.
#[derive(Debug, Clone)]
pub struct SessionMetadata {
    /// Latest `session_info` name on the active branch, if any.
    pub name: Option<String>,
    /// Active label set on the active branch (Add/Remove applied in append
    /// order, deduplicated, first-Add order).
    pub labels: Vec<String>,
    /// Entry id at the tip of the active branch.
    pub active_branch: Option<String>,
    /// Current provider:model spec.
    pub model: String,
    /// Current thinking/reasoning configuration.
    pub thinking: ThinkingConfig,
}

/// Public metadata for resources discovered by the coding harness.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct DiscoveredResourceMetadata {
    pub extensions: Vec<ResourceMetadataEntry>,
    pub packages: Vec<ResourceMetadataEntry>,
    pub skills: Vec<ResourceMetadataEntry>,
    pub fragments: Vec<ResourceMetadataEntry>,
    pub themes: Vec<ResourceMetadataEntry>,
    #[serde(serialize_with = "serialize_redacted_diagnostics")]
    pub diagnostics: Vec<Diagnostic>,
}

/// One metadata entry exposed to prompts, RPC clients, and embedders.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ResourceMetadataEntry {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct HarnessResources {
    metadata: DiscoveredResourceMetadata,
    theme_resources: Vec<crate::theme_discovery::ThemeResource>,
}

impl DiscoveredResourceMetadata {
    fn format_for_system_prompt(&self) -> String {
        let mut sections = Vec::new();
        push_metadata_section(&mut sections, "Discovered packages", &self.packages);
        push_metadata_section(&mut sections, "Discovered extensions", &self.extensions);
        push_metadata_section(&mut sections, "Discovered skills", &self.skills);
        push_metadata_section(
            &mut sections,
            "Discovered prompt fragments",
            &self.fragments,
        );
        push_metadata_section(&mut sections, "Discovered themes", &self.themes);
        if !self.diagnostics.is_empty() {
            sections.push(format!(
                "Resource discovery diagnostics:\n{}",
                self.diagnostics
                    .iter()
                    .map(|diagnostic| {
                        format!("- {}", diagnostic.redacted_payload(RedactionMode::Summary))
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
        sections.join("\n\n")
    }

    pub fn to_rpc_json(&self) -> serde_json::Value {
        serde_json::json!({
            "extensions": metadata_names(&self.extensions),
            "packages": metadata_names(&self.packages),
            "skills": metadata_names(&self.skills),
            "fragments": metadata_names(&self.fragments),
            "themes": metadata_names(&self.themes),
            "diagnostics": self.diagnostic_payloads(RedactionMode::Summary),
        })
    }

    pub fn diagnostic_payloads(&self, mode: RedactionMode) -> Vec<DiagnosticPayload> {
        self.diagnostics
            .iter()
            .map(|diagnostic| diagnostic.redacted_payload(mode))
            .collect()
    }

    fn add_extension_name(&mut self, name: String) {
        if self.extensions.iter().any(|entry| entry.name == name) {
            return;
        }
        self.extensions.push(ResourceMetadataEntry {
            name,
            description: None,
            version: None,
        });
        self.extensions.sort_by(|a, b| a.name.cmp(&b.name));
    }
}

fn serialize_redacted_diagnostics<S>(
    diagnostics: &[Diagnostic],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let payloads = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.redacted_payload(RedactionMode::Summary))
        .collect::<Vec<_>>();
    payloads.serialize(serializer)
}

fn metadata_names(entries: &[ResourceMetadataEntry]) -> Vec<&str> {
    entries.iter().map(|entry| entry.name.as_str()).collect()
}

fn push_metadata_section(
    sections: &mut Vec<String>,
    title: &str,
    entries: &[ResourceMetadataEntry],
) {
    if entries.is_empty() {
        return;
    }
    let lines = entries
        .iter()
        .map(|entry| {
            let mut line = format!("- {}", entry.name);
            if let Some(description) = &entry.description {
                line.push_str(": ");
                line.push_str(description);
            }
            if let Some(version) = &entry.version {
                line.push_str(" v");
                line.push_str(version);
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n");
    sections.push(format!("{title}:\n{lines}"));
}

fn filter_extension_tools(
    tools: Vec<Box<dyn Tool>>,
    selection: &ToolSelection,
) -> Vec<Box<dyn Tool>> {
    match selection {
        ToolSelection::Default | ToolSelection::NoBuiltin => tools,
        ToolSelection::Disabled => Vec::new(),
        ToolSelection::Allowlist(names) => tools
            .into_iter()
            .filter(|tool| {
                let name = tool.definition().name;
                names.iter().any(|allowed| allowed == &name)
            })
            .collect(),
    }
}

/// Builder for SDK embedders that need to inject extension registries or
/// precomputed discovery metadata without dynamic loading.
pub struct CodingHarnessBuilder {
    provider: Box<dyn Provider>,
    model: String,
    config: OpiConfig,
    workspace_root: PathBuf,
    hooks: Option<Box<dyn AgentHooks>>,
    user_system_prompt: Option<String>,
    initial_messages: Vec<AgentMessage>,
    resume: Option<ResumeInfo>,
    tool_config: Option<ToolRuntimeConfig>,
    tool_selection: ToolSelection,
    global_config_dir: Option<PathBuf>,
    extension_registry: Option<ExtensionRegistry>,
    resource_layers: Option<ResourceDiscoveryLayers>,
    resource_metadata: Option<DiscoveredResourceMetadata>,
    installed_packages: Option<Vec<PackageResource>>,
    startup_diagnostics: Vec<Diagnostic>,
    record_diagnostics: bool,
    trace: Option<TraceConfig>,
    trust_decision: TrustDecision,
    execution_mode: ExecutionRunMode,
    #[cfg(test)]
    session_dir_override: Option<PathBuf>,
}

impl CodingHarnessBuilder {
    fn new(
        provider: Box<dyn Provider>,
        model: String,
        config: OpiConfig,
        workspace_root: PathBuf,
        trust_decision: TrustDecision,
    ) -> Self {
        Self {
            provider,
            model,
            config,
            workspace_root,
            hooks: None,
            user_system_prompt: None,
            initial_messages: Vec::new(),
            resume: None,
            tool_config: None,
            tool_selection: ToolSelection::Default,
            global_config_dir: None,
            extension_registry: None,
            resource_layers: None,
            resource_metadata: None,
            installed_packages: None,
            startup_diagnostics: Vec::new(),
            record_diagnostics: false,
            trace: None,
            trust_decision,
            execution_mode: ExecutionRunMode::Interactive,
            #[cfg(test)]
            session_dir_override: None,
        }
    }

    pub fn hooks(mut self, hooks: Box<dyn AgentHooks>) -> Self {
        self.hooks = Some(hooks);
        self
    }

    pub fn user_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.user_system_prompt = Some(prompt.into());
        self
    }

    pub fn initial_messages(mut self, messages: Vec<AgentMessage>) -> Self {
        self.initial_messages = messages;
        self
    }

    pub fn resume(mut self, resume: ResumeInfo) -> Self {
        self.resume = Some(resume);
        self
    }

    pub fn tool_selection(mut self, selection: ToolSelection) -> Self {
        self.tool_selection = selection;
        self
    }

    pub fn tool_config(mut self, config: ToolRuntimeConfig) -> Self {
        self.tool_config = Some(config);
        self
    }

    pub fn global_config_dir(mut self, dir: PathBuf) -> Self {
        self.global_config_dir = Some(dir);
        self
    }

    pub fn extension_registry(mut self, registry: ExtensionRegistry) -> Self {
        self.extension_registry = Some(registry);
        self
    }

    pub fn resource_layers(mut self, layers: ResourceDiscoveryLayers) -> Self {
        self.resource_layers = Some(layers);
        self
    }

    pub fn resource_metadata(mut self, metadata: DiscoveredResourceMetadata) -> Self {
        self.resource_metadata = Some(metadata);
        self
    }

    pub fn installed_packages(mut self, packages: Vec<PackageResource>) -> Self {
        self.installed_packages = Some(packages);
        self
    }

    pub fn startup_diagnostics(mut self, diagnostics: Vec<Diagnostic>) -> Self {
        self.startup_diagnostics = diagnostics;
        self
    }

    /// Record runtime diagnostics during runs so a run summary can report
    /// severity counts (Phase 7 task 7.5). Off by default; enabling installs a
    /// [`RecordingSink`] on the agent with no other behavior change.
    pub fn record_diagnostics(mut self, enabled: bool) -> Self {
        self.record_diagnostics = enabled;
        self
    }

    /// Enable per-run tracing (Phase 7 task 7.5). When set, each prompt run
    /// emits a versioned redacted trace envelope to `config.sink`. Tracing is
    /// opt-in and fail-open; a sink prepare failure is fail-closed (the run
    /// aborts rather than running untraced when tracing was requested).
    pub fn trace(mut self, config: Option<TraceConfig>) -> Self {
        self.trace = config;
        self
    }

    /// Set the resolved project-trust decision (task 15.7). When
    /// [`TrustDecision::Untrusted`], `discover_resources` skips the project
    /// resource layer and context-file discovery skips project `AGENTS.md`/
    /// `CLAUDE.md`. Only [`TrustDecision::Trusted`] loads project resources;
    /// `Untrusted` and `Undecided` both fail closed.
    pub fn trust_decision(mut self, decision: TrustDecision) -> Self {
        self.trust_decision = decision;
        self
    }

    /// Phase 16.9: set the execution run mode threaded into
    /// `ExecutionRuntime::build`. Defaults to [`ExecutionRunMode::Interactive`];
    /// headless startup paths set this to `NonInteractive` (runner/text/NDJSON)
    /// or `Rpc` (RPC). It cannot be derived from `tool_config.run_mode`, which
    /// collapses RPC into `NonInteractive`.
    pub fn execution_mode(mut self, mode: ExecutionRunMode) -> Self {
        self.execution_mode = mode;
        self
    }

    /// Unit-test-only instance seam for isolating session persistence without
    /// mutating the process-global `OPI_SESSIONS_DIR` environment variable.
    #[cfg(test)]
    fn session_dir_for_test(mut self, dir: PathBuf) -> Self {
        self.session_dir_override = Some(dir);
        self
    }

    pub fn build(self) -> CodingHarness {
        let tool_selection = self.tool_selection;
        let tool_config = self.tool_config.unwrap_or_else(|| {
            ToolRuntimeConfig::resolve(RunMode::Interactive, true, tool_selection.clone())
                .expect("interactive tool config should be valid")
        });
        CodingHarness::new_with_build_options(
            self.provider,
            self.model,
            self.config,
            self.workspace_root,
            self.hooks.unwrap_or_else(|| Box::new(CodingAgentHooks)),
            self.user_system_prompt,
            self.initial_messages,
            self.resume,
            tool_config,
            self.global_config_dir,
            HarnessBuildOptions {
                extension_registry: self.extension_registry,
                resource_layers: self.resource_layers,
                resource_metadata: self.resource_metadata,
                installed_packages: self.installed_packages,
                startup_diagnostics: self.startup_diagnostics,
                tool_selection,
                record_diagnostics: self.record_diagnostics,
                trace: self.trace,
                trust_decision: self.trust_decision,
                execution_mode: self.execution_mode,
                #[cfg(test)]
                session_dir_override: self.session_dir_override,
            },
        )
    }
}

struct HarnessBuildOptions {
    extension_registry: Option<ExtensionRegistry>,
    resource_layers: Option<ResourceDiscoveryLayers>,
    resource_metadata: Option<DiscoveredResourceMetadata>,
    installed_packages: Option<Vec<PackageResource>>,
    startup_diagnostics: Vec<Diagnostic>,
    tool_selection: ToolSelection,
    record_diagnostics: bool,
    trace: Option<TraceConfig>,
    trust_decision: TrustDecision,
    /// Phase 16.9: the run mode threaded into `ExecutionRuntime::build`.
    /// Legacy constructors derive interactive/non-interactive from tool config;
    /// RPC remains available only through startup paths that set it explicitly.
    execution_mode: ExecutionRunMode,
    #[cfg(test)]
    session_dir_override: Option<PathBuf>,
}

impl Default for HarnessBuildOptions {
    fn default() -> Self {
        Self {
            extension_registry: None,
            resource_layers: None,
            resource_metadata: None,
            installed_packages: None,
            startup_diagnostics: Vec::new(),
            tool_selection: ToolSelection::Default,
            record_diagnostics: false,
            trace: None,
            trust_decision: TrustDecision::Undecided,
            execution_mode: ExecutionRunMode::Interactive,
            #[cfg(test)]
            session_dir_override: None,
        }
    }
}

impl CodingHarness {
    /// Start building a harness for SDK/embedder use.
    pub fn builder(
        provider: Box<dyn Provider>,
        model: String,
        config: OpiConfig,
        workspace_root: PathBuf,
        trust_decision: TrustDecision,
    ) -> CodingHarnessBuilder {
        CodingHarnessBuilder::new(provider, model, config, workspace_root, trust_decision)
    }

    /// Create a new harness with the given provider, model, config, and workspace root.
    pub fn new(
        provider: Box<dyn Provider>,
        model: String,
        config: OpiConfig,
        workspace_root: PathBuf,
        trust_decision: TrustDecision,
    ) -> Self {
        Self::new_with_hooks(
            provider,
            model,
            config,
            workspace_root,
            Box::new(CodingAgentHooks),
            None,
            Vec::new(),
            ToolSelection::Default,
            trust_decision,
        )
    }

    /// Create a new harness with an explicit tool selection.
    pub fn new_with_selection(
        provider: Box<dyn Provider>,
        model: String,
        config: OpiConfig,
        workspace_root: PathBuf,
        tool_selection: ToolSelection,
        trust_decision: TrustDecision,
    ) -> Self {
        Self::new_with_hooks(
            provider,
            model,
            config,
            workspace_root,
            Box::new(CodingAgentHooks),
            None,
            Vec::new(),
            tool_selection,
            trust_decision,
        )
    }

    /// Create a new harness with already resolved tool runtime config.
    pub fn new_with_tool_config(
        provider: Box<dyn Provider>,
        model: String,
        config: OpiConfig,
        workspace_root: PathBuf,
        tool_config: ToolRuntimeConfig,
        trust_decision: TrustDecision,
    ) -> Self {
        Self::new_with_hooks_and_resume_tool_config(
            provider,
            model,
            config,
            workspace_root,
            Box::new(CodingAgentHooks),
            None,
            Vec::new(),
            None,
            tool_config,
            trust_decision,
        )
    }

    /// Create a new harness with custom hooks.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_hooks(
        provider: Box<dyn Provider>,
        model: String,
        config: OpiConfig,
        workspace_root: PathBuf,
        hooks: Box<dyn AgentHooks>,
        user_system_prompt: Option<String>,
        initial_messages: Vec<AgentMessage>,
        tool_selection: ToolSelection,
        trust_decision: TrustDecision,
    ) -> Self {
        Self::new_with_hooks_and_resume(
            provider,
            model,
            config,
            workspace_root,
            hooks,
            user_system_prompt,
            initial_messages,
            None,
            tool_selection,
            trust_decision,
        )
    }

    /// Create a new harness, optionally adopting an existing session (resume).
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_hooks_and_resume(
        provider: Box<dyn Provider>,
        model: String,
        config: OpiConfig,
        workspace_root: PathBuf,
        hooks: Box<dyn AgentHooks>,
        user_system_prompt: Option<String>,
        initial_messages: Vec<AgentMessage>,
        resume: Option<ResumeInfo>,
        tool_selection: ToolSelection,
        trust_decision: TrustDecision,
    ) -> Self {
        let tool_config = ToolRuntimeConfig::resolve(RunMode::Interactive, true, tool_selection)
            .expect("interactive tool config should be valid");
        Self::new_with_hooks_and_resume_tool_config(
            provider,
            model,
            config,
            workspace_root,
            hooks,
            user_system_prompt,
            initial_messages,
            resume,
            tool_config,
            trust_decision,
        )
    }

    /// Create a new harness, optionally adopting an existing session (resume),
    /// with already resolved tool runtime config.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_hooks_and_resume_tool_config(
        provider: Box<dyn Provider>,
        model: String,
        config: OpiConfig,
        workspace_root: PathBuf,
        hooks: Box<dyn AgentHooks>,
        user_system_prompt: Option<String>,
        initial_messages: Vec<AgentMessage>,
        resume: Option<ResumeInfo>,
        tool_config: ToolRuntimeConfig,
        trust_decision: TrustDecision,
    ) -> Self {
        Self::new_with_global_config_dir_tool_config(
            provider,
            model,
            config,
            workspace_root,
            hooks,
            user_system_prompt,
            initial_messages,
            resume,
            tool_config,
            None,
            trust_decision,
        )
    }

    /// Create a new harness with an explicit global config directory override.
    ///
    /// When `global_config_dir` is `None`, uses the platform default from
    /// [`crate::config::user_config_dir`]. Pass `Some(path)` in tests to
    /// isolate global context file discovery from the real user config dir.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_global_config_dir(
        provider: Box<dyn Provider>,
        model: String,
        config: OpiConfig,
        workspace_root: PathBuf,
        hooks: Box<dyn AgentHooks>,
        user_system_prompt: Option<String>,
        initial_messages: Vec<AgentMessage>,
        resume: Option<ResumeInfo>,
        tool_selection: ToolSelection,
        global_config_dir: Option<PathBuf>,
        trust_decision: TrustDecision,
    ) -> Self {
        let tool_config = ToolRuntimeConfig::resolve(RunMode::Interactive, true, tool_selection)
            .expect("interactive tool config should be valid");
        Self::new_with_global_config_dir_tool_config(
            provider,
            model,
            config,
            workspace_root,
            hooks,
            user_system_prompt,
            initial_messages,
            resume,
            tool_config,
            global_config_dir,
            trust_decision,
        )
    }

    /// Create a new harness with an explicit global config directory override
    /// and already resolved tool runtime config.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_global_config_dir_tool_config(
        provider: Box<dyn Provider>,
        model: String,
        config: OpiConfig,
        workspace_root: PathBuf,
        hooks: Box<dyn AgentHooks>,
        user_system_prompt: Option<String>,
        initial_messages: Vec<AgentMessage>,
        resume: Option<ResumeInfo>,
        tool_config: ToolRuntimeConfig,
        global_config_dir: Option<PathBuf>,
        trust_decision: TrustDecision,
    ) -> Self {
        let execution_mode = match tool_config.run_mode {
            RunMode::Interactive => ExecutionRunMode::Interactive,
            RunMode::NonInteractive => ExecutionRunMode::NonInteractive,
        };
        Self::new_with_build_options(
            provider,
            model,
            config,
            workspace_root,
            hooks,
            user_system_prompt,
            initial_messages,
            resume,
            tool_config,
            global_config_dir,
            HarnessBuildOptions {
                trust_decision,
                execution_mode,
                ..HarnessBuildOptions::default()
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new_with_build_options(
        mut provider: Box<dyn Provider>,
        model: String,
        config: OpiConfig,
        workspace_root: PathBuf,
        hooks: Box<dyn AgentHooks>,
        user_system_prompt: Option<String>,
        initial_messages: Vec<AgentMessage>,
        resume: Option<ResumeInfo>,
        tool_config: ToolRuntimeConfig,
        global_config_dir: Option<PathBuf>,
        build_options: HarnessBuildOptions,
    ) -> Self {
        #[cfg(test)]
        let session_dir_override = build_options.session_dir_override.clone();
        let mut hooks = hooks;
        let mut extension_tools = Vec::new();
        let mut injected_extension_names = Vec::new();
        let mut extension_event_registry = None;
        let extension_registry = build_options.extension_registry;
        let active_extension_registry = extension_registry.clone();
        let resume_extension_state = resume
            .as_ref()
            .and_then(|info| crate::session_coordinator::latest_extension_state(&info.entries));
        let resume_diagnostics = resume
            .as_ref()
            .map(|info| info.diagnostics.clone())
            .unwrap_or_default();
        let (model_registry, model_registry_diagnostics) =
            crate::provider_factory::assemble_harness_collection(
                provider.as_mut(),
                extension_registry.as_ref(),
            );
        if let Some(registry) = extension_registry.as_ref() {
            extension_event_registry = Some(registry.clone());
            injected_extension_names = registry
                .names()
                .into_iter()
                .map(str::to_owned)
                .collect::<Vec<_>>();
            extension_tools =
                filter_extension_tools(registry.collect_tools(), &build_options.tool_selection);
            hooks = registry.wrap_hooks(hooks);
        }

        // Resolve the global config dir once. Routed execution may use it for
        // package activation, and resource discovery reuses it below.
        let resolved_global_dir = global_config_dir.unwrap_or_else(crate::config::user_config_dir);

        // Resolve fixed-local allow and headless fixed-local ask before opening
        // package activation state or constructing permission/router/protocol
        // state. Other execution configurations retain routed assembly.
        let execution =
            harness_execution(&config, &resolved_global_dir, build_options.execution_mode);
        let (mut tools, tool_diagnostics, permission_manager, permission_prompt_rx) =
            match execution {
                HarnessExecution::DirectLocal => {
                    let (tools, diagnostics) =
                        Self::build_minimal_runtime_tools(&workspace_root, &tool_config);
                    (tools, diagnostics, None, None)
                }
                HarnessExecution::Refused(failure) => {
                    let (tools, diagnostics) =
                        Self::build_refused_execution_tools(&workspace_root, &tool_config, failure);
                    (tools, diagnostics, None, None)
                }
                HarnessExecution::Routed(mut execution) => {
                    let permission_manager = Some(Arc::clone(&execution.manager));
                    let permission_prompt_rx =
                        if build_options.execution_mode == ExecutionRunMode::Interactive {
                            #[cfg(test)]
                            crate::execution::runtime::construction_probe::broker_constructed();
                            let (tx, rx) =
                                mpsc::channel::<crate::interactive::PermissionPromptRequest>(8);
                            execution.broker =
                                Some(Arc::new(crate::interactive::TuiPermissionBroker::new(tx)));
                            Some(rx)
                        } else {
                            None
                        };
                    let (tools, diagnostics) =
                        Self::build_tools(&workspace_root, &tool_config, &execution);
                    (tools, diagnostics, permission_manager, permission_prompt_rx)
                }
            };
        tools.extend(extension_tools);
        let tool_defs: Vec<_> = tools.iter().map(|t| t.definition()).collect();
        let mut builder = SystemPromptBuilder::new().tools(tool_defs);
        if let Some(content) = user_system_prompt {
            builder = builder.user_system(content);
        }
        let mut resources = match build_options.resource_metadata {
            Some(metadata) => HarnessResources {
                metadata,
                theme_resources: Vec::new(),
            },
            None => Self::discover_resources(
                &workspace_root,
                &config,
                Some(resolved_global_dir.as_path()),
                build_options.resource_layers,
                build_options.installed_packages,
                build_options.trust_decision,
            ),
        };
        resources
            .metadata
            .diagnostics
            .extend(model_registry_diagnostics);
        resources
            .metadata
            .diagnostics
            .extend(build_options.startup_diagnostics);
        resources.metadata.diagnostics.extend(tool_diagnostics);
        resources.metadata.diagnostics.extend(resume_diagnostics);
        for name in injected_extension_names {
            resources.metadata.add_extension_name(name);
        }

        let project_trusted = matches!(build_options.trust_decision, TrustDecision::Trusted);
        let context = context_files::discover_context_files_with_trust(
            &workspace_root,
            Some(resolved_global_dir.as_path()),
            project_trusted,
        );
        let resource_prompt = resources.metadata.format_for_system_prompt();
        let mut context_content = context.content;
        if !resource_prompt.is_empty() {
            if !context_content.is_empty() {
                context_content.push_str("\n\n");
            }
            context_content.push_str(&resource_prompt);
        }
        if !context_content.is_empty() {
            builder = builder.context_files(context_content);
        }
        let system_prompt = builder.build();

        let model_for_capability_lookup = if model.contains(':') {
            model.clone()
        } else {
            format!("{}:{model}", provider.id())
        };
        let (thinking, max_tokens) =
            initial_thinking_request_config(&model_registry, &model_for_capability_lookup, &config);
        let agent_config = AgentLoopConfig {
            max_turns: config.defaults.max_iterations,
            max_tokens,
            retry: Some(config.retry.clone()),
            thinking,
            ..Default::default()
        };

        let mut agent = Agent::new(
            provider,
            tools,
            model.clone(),
            Some(system_prompt.clone()),
            agent_config,
            hooks,
        );
        if let Some(registry) = extension_event_registry {
            agent.subscribe(Box::new(move |event| registry.dispatch_event(event)));
        }

        let initial_len = initial_messages.len();
        if !initial_messages.is_empty() {
            agent.set_initial_messages(initial_messages);
        }

        let cwd = if let Some(ref info) = resume {
            // When resuming, use the workspace cwd from the session header so
            // tools operate in the correct workspace even if the process was
            // launched from a different directory.
            info.original_cwd.to_string_lossy().into_owned()
        } else {
            std::env::current_dir()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        };
        let compaction_config = opi_agent::compaction::CompactionConfig {
            enabled: config.compaction.enabled,
            threshold_tokens: config.compaction.threshold_tokens,
        };

        // Capture recorded model/thinking up front so `resume` can still move
        // into SessionCoordinator::open_existing below. Applied after the
        // harness is assembled (Phase 13.3).
        let recorded_model = resume.as_ref().and_then(|info| info.recorded_model.clone());
        let recorded_thinking = resume.as_ref().and_then(|info| info.recorded_thinking);

        let session = if let Some(info) = resume {
            SessionCoordinator::open_existing(
                info.path,
                info.session_id,
                &info.entries,
                initial_len,
                compaction_config,
                model.clone(),
            )
            .ok()
        } else {
            #[cfg(test)]
            let session_dir = session_dir_override
                .clone()
                .unwrap_or_else(crate::session_cli::session_dir);
            #[cfg(not(test))]
            let session_dir = crate::session_cli::session_dir();
            SessionCoordinator::new(&session_dir, &cwd, compaction_config, model.clone()).ok()
        };

        // Opt-in diagnostic recording: install a RecordingSink on the agent so
        // runtime diagnostics (retry/cancel/provider/tool failures) are captured
        // for run-summary severity counts. Off by default -> no behavior change.
        let diagnostics = if build_options.record_diagnostics {
            let sink = Arc::new(RecordingSink::new());
            agent.set_diagnostic_sink(Some(sink.clone() as Arc<dyn DiagnosticSink>));
            Some(sink)
        } else {
            None
        };
        let trace = build_options.trace;

        let mut harness = Self {
            agent,
            config,
            system_prompt,
            resources,
            model_registry,
            extension_registry: active_extension_registry,
            session,
            turn_offset: initial_len,
            pending_images: Vec::new(),
            pending_extension_state: resume_extension_state,
            diagnostics,
            trace,
            active_trace: None,
            run_seq: 0,
            credential_store: None,
            oauth_registry: None,
            oauth_endpoints: OAuthEndpointConfig::production(),
            oauth_http_client: crate::oauth::production_oauth_client(),
            permission_manager,
            permission_prompt_rx,
            #[cfg(test)]
            session_dir_override,
        };

        // Phase 13.3: re-apply recorded model/thinking on the CLI --resume path
        // (and any other builder-driven resume), mirroring resume_session_id.
        // The diagnostic sink is already wired above so incompat warnings flow
        // through the same channel as the interactive path.
        harness.apply_recorded_model(recorded_model.as_deref());
        harness.apply_recorded_thinking(recorded_thinking);
        harness.sync_session_cost_model();
        harness.sync_session_id();

        harness
    }

    /// Add an extra tool to the harness (for testing with mock tools).
    pub fn add_tool(&mut self, tool: Box<dyn Tool>) {
        self.agent.add_tool(tool);
    }

    /// Queue images to be injected into the next prompt.
    pub fn queue_images(&mut self, images: Vec<opi_ai::message::InputContent>) {
        self.pending_images.extend(images);
    }

    /// Take and clear queued images.
    pub fn take_pending_images(&mut self) -> Vec<opi_ai::message::InputContent> {
        std::mem::take(&mut self.pending_images)
    }

    /// Return model picker items from the active provider.
    pub fn model_picker_items(&self) -> Vec<opi_tui::SelectItem> {
        let current_provider = self.agent.provider().id();
        crate::picker::model_picker_items(self.model_registry.registry())
            .into_iter()
            .filter(|item| item.metadata == current_provider)
            .collect()
    }

    /// Change the model used by subsequent prompts.
    pub fn set_model(&mut self, model: String) {
        self.agent.set_model(model);
        self.sync_session_cost_model();
    }

    /// Validate and change the model used by subsequent prompts.
    ///
    /// On success the change is also persisted as a `model_change` entry on the
    /// active session branch (Phase 13.3), parented to the current content tip
    /// without advancing it. A later resume observes the recorded model and
    /// re-applies it when compatible with the CLI/config provider.
    pub fn set_model_validated(&mut self, model: String) -> Result<&str, String> {
        self.try_configure_model(&model)?;
        if let Some(session) = self.session.as_mut() {
            session
                .append_model_change(model.clone())
                .map_err(|e| format!("model change write failed: {e}"))?;
        }
        self.agent.set_model(model);
        self.sync_session_cost_model();
        Ok(self.agent.model())
    }

    /// Validate that `model` is a known same-provider spec and compatible with
    /// the current thinking configuration, without persisting or mutating
    /// session state. Used by [`Self::set_model_validated`] (persists) and by
    /// resume (applies a recorded model without re-persisting the entry).
    fn try_configure_model(&mut self, model: &str) -> Result<(), String> {
        let current_provider = self.agent.provider().id();
        let normalized;
        let model_spec = if model.contains(':') || self.model_info(model).is_none() {
            model
        } else {
            normalized = format!("{current_provider}:{model}");
            &normalized
        };
        let (requested_provider, requested_model) =
            crate::provider_factory::parse_model_spec(model_spec)?;
        if requested_provider != current_provider {
            return Err(format!(
                "cannot switch provider from {current_provider} to {requested_provider} at runtime"
            ));
        }

        let requested_model_info = self.model_info(requested_model);
        let Some(requested_model_info) = requested_model_info else {
            return Err(format!(
                "unknown model '{requested_model}' for provider '{requested_provider}'"
            ));
        };

        self.validate_current_thinking_for_model(&requested_model_info)?;
        Ok(())
    }

    /// Change the thinking level used by subsequent provider requests.
    ///
    /// On success the change is also persisted as a `thinking_level_change`
    /// entry on the active session branch (Phase 13.3), parented to the current
    /// content tip without advancing it. A later resume observes the recorded
    /// level and re-applies it when compatible with the active model.
    pub fn set_thinking_level(&mut self, level: &str) -> Result<RuntimeThinkingState, String> {
        let change = self.prepare_thinking_change(level)?;
        self.persist_thinking_level_change(change.persisted_level)?;
        Ok(self.apply_thinking_change(change))
    }

    /// Validate `level`, apply it to the agent, and return the resulting
    /// runtime state, without persisting a session entry. Used by
    /// [`Self::set_thinking_level`] (persists) and by resume (applies a
    /// recorded level without re-persisting the entry).
    fn try_configure_thinking(&mut self, level: &str) -> Result<RuntimeThinkingState, String> {
        let change = self.prepare_thinking_change(level)?;
        Ok(self.apply_thinking_change(change))
    }

    fn prepare_thinking_change(&self, level: &str) -> Result<PendingThinkingChange, String> {
        let default_budget = self.config.thinking.budget_tokens as u64;
        let (persisted_level, budget_tokens) = match level {
            "off" => (ThinkingLevel::None, None),
            "minimal" => (ThinkingLevel::Minimal, Some(1_024)),
            "low" => (ThinkingLevel::Low, Some(2_048)),
            "medium" => (ThinkingLevel::Medium, Some(default_budget)),
            "high" => (ThinkingLevel::High, Some(default_budget.max(20_000))),
            "xhigh" => (ThinkingLevel::XHigh, Some(default_budget.max(20_000))),
            "max" => (ThinkingLevel::Max, Some(default_budget.max(20_000))),
            _ => {
                return Err(format!(
                    "invalid thinking level '{level}': expected off, minimal, low, medium, high, xhigh, or max"
                ));
            }
        };

        let (thinking, max_tokens) = match budget_tokens {
            Some(budget_tokens) => {
                let (mut thinking, max_tokens) = request_config_for_thinking_budget(budget_tokens)?;
                thinking.level = persisted_level;
                if let Some(model) = self.active_model_info() {
                    validate_thinking_budget_for_model(&model, budget_tokens, max_tokens)?;
                    // After the broad capability + budget check passes, also
                    // reject levels the model's thinking_level_map cannot
                    // resolve, so a persisted setting does not deterministically
                    // fail the next prompt's preflight (C-3.1).
                    if model.thinking_level_map.resolve(persisted_level).is_err() {
                        return Err(format!(
                            "thinking level '{level}' is not supported by model '{}'",
                            model.id
                        ));
                    }
                }
                (Some(thinking), Some(max_tokens))
            }
            None => (None, None),
        };

        let state = RuntimeThinkingState {
            level: level.to_owned(),
            enabled: thinking.as_ref().is_some_and(|thinking| thinking.enabled),
            budget_tokens: thinking
                .as_ref()
                .and_then(|thinking| thinking.budget_tokens),
        };
        Ok(PendingThinkingChange {
            persisted_level,
            thinking,
            max_tokens,
            state,
        })
    }

    fn apply_thinking_change(&mut self, change: PendingThinkingChange) -> RuntimeThinkingState {
        self.agent.set_max_tokens(change.max_tokens);
        self.agent.set_thinking_config(change.thinking);
        change.state
    }

    /// Persist a `thinking_level_change` entry before applying the runtime
    /// change, if a session is active.
    fn persist_thinking_level_change(&mut self, level: ThinkingLevel) -> Result<(), String> {
        let Some(session) = self.session.as_mut() else {
            return Ok(());
        };
        session
            .append_thinking_level_change(level)
            .map_err(|e| format!("thinking level write failed: {e}"))
    }

    /// Set the session name (Phase 13.4 `/name <name>`). Persists a
    /// `session_info` entry parented to the current content tip without
    /// advancing it. Best-effort: a failed metadata write does not roll back
    /// the in-memory name. Returns `Err` if no session is active.
    pub fn set_session_name(&mut self, name: String) -> Result<(), String> {
        let session = self
            .session
            .as_mut()
            .ok_or_else(|| "no active session".to_string())?;
        session
            .append_session_info(name)
            .map_err(|e| format!("session name write failed: {e}"))
    }

    /// Add a label to the active branch (Phase 13.4 `/label <label>`). Persists
    /// a `label` entry with `Add` action parented to the current content tip
    /// without advancing it. Best-effort: a failed metadata write does not roll
    /// back the in-memory label set. Returns `Err` if no session is active.
    pub fn add_label(&mut self, label: String) -> Result<(), String> {
        let session = self
            .session
            .as_mut()
            .ok_or_else(|| "no active session".to_string())?;
        session
            .append_label(label, opi_agent::session::LabelAction::Add)
            .map_err(|e| format!("label write failed: {e}"))
    }

    /// Remove a label from the active branch (Phase 13.4 `/unlabel <label>`).
    /// Persists a `label` entry with `Remove` action parented to the current
    /// content tip without advancing it. Returns `Err` if no session is active.
    pub fn remove_label(&mut self, label: String) -> Result<(), String> {
        let session = self
            .session
            .as_mut()
            .ok_or_else(|| "no active session".to_string())?;
        session
            .append_label(label, opi_agent::session::LabelAction::Remove)
            .map_err(|e| format!("label write failed: {e}"))
    }

    /// Aggregate the live session metadata (Phase 13.4 read path) surfaced by
    /// `/session info` and RPC `session_info`: name, labels, active branch,
    /// model, and thinking config. Returns `None` when no session is active.
    pub fn session_metadata(&self) -> Option<SessionMetadata> {
        let session = self.session.as_ref()?;
        Some(SessionMetadata {
            name: session.name().map(str::to_owned),
            labels: session.labels().to_vec(),
            active_branch: session.active_branch_id().map(str::to_owned),
            model: self.agent.model().to_owned(),
            thinking: self.agent.thinking_config(),
        })
    }

    fn active_model_info(&self) -> Option<ModelInfo> {
        let current_provider = self.agent.provider().id();
        let active_model = self.agent.model();
        let normalized;
        let model_spec = if active_model.contains(':') {
            active_model
        } else {
            normalized = format!("{current_provider}:{active_model}");
            &normalized
        };
        let Ok((provider_id, model_id)) = crate::provider_factory::parse_model_spec(model_spec)
        else {
            return None;
        };
        if provider_id != current_provider {
            return None;
        }
        self.model_info(model_id)
    }

    fn sync_session_cost_model(&mut self) {
        let model_spec = self.agent.model().to_owned();
        let pricing = self.active_model_info().and_then(|model| model.pricing);
        if let Some(session) = self.session.as_mut() {
            session.set_cost_model(model_spec, pricing);
        }
    }

    fn model_info(&self, model_id: &str) -> Option<ModelInfo> {
        let spec = format!("{}:{model_id}", self.agent.provider().id());
        self.model_registry
            .resolve(&spec)
            .ok()
            .map(|(_, model)| model.clone())
    }

    fn validate_current_thinking_for_model(&self, model: &ModelInfo) -> Result<(), String> {
        let thinking = self.agent.thinking_config();
        if !thinking.enabled {
            return Ok(());
        }
        let Some(budget_tokens) = thinking.budget_tokens else {
            return Ok(());
        };
        let max_tokens = max_tokens_for_thinking_budget(budget_tokens)?;
        validate_thinking_budget_for_model(model, budget_tokens, max_tokens)
    }

    /// Resume an existing session by ID into this harness.
    ///
    /// Reconstructs the active-branch context through the opi-agent context
    /// API (Phase 13.2/13.3): messages drive the agent buffer; the latest
    /// recorded `model_change` and `thinking_level_change` on the active chain
    /// are re-applied when compatible with the CLI/config provider selection,
    /// and a Phase 7 diagnostic is emitted (without aborting the resume) when
    /// they are not. Missing-parent warnings from the context builder are
    /// surfaced alongside the load-time recovery diagnostics.
    pub fn resume_session_id(&mut self, session_id: &str) -> Result<usize, String> {
        // Phase 16.10: an allow-for-session grant must not survive a session
        // switch. Reset on the boundary (re-prompt is the safe failure mode).
        if let Some(manager) = &self.permission_manager {
            manager.reset_grants();
        }
        #[cfg(test)]
        let dir = self
            .session_dir_override
            .clone()
            .unwrap_or_else(crate::session_cli::session_dir);
        #[cfg(not(test))]
        let dir = crate::session_cli::session_dir();
        let session =
            crate::session_cli::resume_session(&dir, session_id).map_err(|e| e.to_string())?;

        // Phase 13.3: build the agent buffer and metadata view through the
        // reusable opi-agent context API instead of the product-only walker.
        let recovery = session.recovery.clone();
        let ctx = reconstruct_context(&session.entries, &recovery);
        let message_count = ctx.messages.len();
        self.agent.replace_messages(ctx.messages);
        self.defer_extension_state_from_entries(&session.entries);

        // Apply recorded model/thinking metadata (latest-wins on the active
        // chain). Each branch keeps the CLI/config selection when the recorded
        // value is incompatible and emits a Phase 7 diagnostic instead.
        self.apply_recorded_model(ctx.model.as_deref());
        self.apply_recorded_thinking(ctx.thinking_level);

        // Surface recovery + missing-parent diagnostics. `reconstruct_context`
        // already forwards load-time recovery diagnostics, so do not append
        // `session.diagnostics` separately.
        for diagnostic in ctx.diagnostics {
            self.resources.metadata.diagnostics.push(diagnostic.clone());
            self.record_harness_diagnostic(diagnostic);
        }

        let compaction_config = opi_agent::compaction::CompactionConfig {
            enabled: self.config.compaction.enabled,
            threshold_tokens: self.config.compaction.threshold_tokens,
        };
        self.session = SessionCoordinator::open_existing(
            session.path,
            session.header.id,
            &session.entries,
            message_count,
            compaction_config,
            self.agent.model().to_string(),
        )
        .ok();
        self.sync_session_cost_model();
        self.sync_session_id();
        self.turn_offset = message_count;
        Ok(message_count)
    }

    /// Re-apply a recorded `model_change` model spec on resume. Configures the
    /// agent in place without persisting a new entry (the entry is already in
    /// the source session). Emits a Phase 7 diagnostic and keeps the CLI/config
    /// model when the recorded spec is incompatible.
    fn apply_recorded_model(&mut self, recorded: Option<&str>) {
        let Some(spec) = recorded else {
            return;
        };
        if let Err(reason) = self.try_configure_model(spec) {
            self.record_harness_diagnostic(
                Diagnostic::new(
                    Severity::Warning,
                    CODE_SESSION_RESUME_MODEL_INCOMPATIBLE,
                    SOURCE_SESSION,
                    "recorded model_change is incompatible with the active provider; keeping CLI/config model",
                )
                .details(serde_json::json!({
                    "recorded_model": spec,
                    "active_model": self.agent.model(),
                    "reason": reason,
                })),
            );
            return;
        }
        self.agent.set_model(spec.to_owned());
    }

    /// Re-apply a recorded `thinking_level_change` on resume. Configures the
    /// agent in place without persisting a new entry. Emits a Phase 7
    /// diagnostic and keeps the CLI/config thinking level when the recorded
    /// level is incompatible with the active model.
    fn apply_recorded_thinking(&mut self, recorded: Option<ThinkingLevel>) {
        let Some(level) = recorded else {
            return;
        };
        let level_str = match level {
            ThinkingLevel::None => "off",
            ThinkingLevel::Minimal => "minimal",
            ThinkingLevel::Low => "low",
            ThinkingLevel::Medium => "medium",
            ThinkingLevel::High => "high",
            ThinkingLevel::XHigh => "xhigh",
            ThinkingLevel::Max => "max",
        };
        if let Err(reason) = self.try_configure_thinking(level_str) {
            self.record_harness_diagnostic(
                Diagnostic::new(
                    Severity::Warning,
                    CODE_SESSION_RESUME_THINKING_INCOMPATIBLE,
                    SOURCE_SESSION,
                    "recorded thinking_level_change is incompatible with the active model; keeping CLI/config thinking level",
                )
                .details(serde_json::json!({
                    "recorded_level": level_str,
                    "reason": reason,
                })),
            );
        }
    }

    /// Fork the active session into a new parented session and switch to it.
    pub fn fork_current_session(&mut self) -> Result<(String, usize), String> {
        // Phase 16.10: grants do not survive a fork boundary.
        if let Some(manager) = &self.permission_manager {
            manager.reset_grants();
        }
        let (dir, source_session_id) = {
            let session = self
                .session
                .as_ref()
                .ok_or_else(|| "no active session".to_owned())?;
            let dir = session
                .session_path()
                .parent()
                .ok_or_else(|| "active session has no parent directory".to_owned())?
                .to_path_buf();
            (dir, session.session_id().to_owned())
        };

        let forked = crate::session_cli::fork_session(&dir, &source_session_id)
            .map_err(|e| e.to_string())?;
        let ctx = reconstruct_context(&forked.entries, &forked.recovery);
        let message_count = ctx.messages.len();
        self.agent.replace_messages(ctx.messages);
        self.defer_extension_state_from_entries(&forked.entries);
        for diagnostic in ctx.diagnostics {
            self.resources.metadata.diagnostics.push(diagnostic.clone());
            self.record_harness_diagnostic(diagnostic);
        }

        let compaction_config = opi_agent::compaction::CompactionConfig {
            enabled: self.config.compaction.enabled,
            threshold_tokens: self.config.compaction.threshold_tokens,
        };
        let path = forked.path;
        let session_id = forked.header.id;
        let entries = forked.entries;
        self.session = Some(
            SessionCoordinator::open_existing(
                path,
                session_id.clone(),
                &entries,
                message_count,
                compaction_config,
                self.agent.model().to_string(),
            )
            .map_err(|e| format!("failed to open forked session: {e}"))?,
        );
        self.sync_session_cost_model();
        self.turn_offset = message_count;
        self.sync_session_id();
        Ok((session_id, message_count))
    }

    /// Return branch picker items for the currently active session.
    pub fn branch_picker_items(&self) -> Result<Vec<opi_tui::SelectItem>, String> {
        let session = self
            .session
            .as_ref()
            .ok_or_else(|| "no active session".to_owned())?;
        let (tree, _recovery) = Self::read_session_tree(session.session_path())?;
        Ok(crate::picker::branch_picker_items(&tree))
    }

    /// Return the reconstructed session tree for the active session, plus the
    /// read recovery metadata from the JSONL load.
    pub fn session_tree(
        &self,
    ) -> Result<
        (
            opi_agent::session_branch::SessionTree,
            opi_agent::session::CrashRecovery,
        ),
        String,
    > {
        let session = self
            .session
            .as_ref()
            .ok_or_else(|| "no active session".to_owned())?;
        Self::read_session_tree(session.session_path())
    }

    fn read_session_tree(
        path: &Path,
    ) -> Result<
        (
            opi_agent::session_branch::SessionTree,
            opi_agent::session::CrashRecovery,
        ),
        String,
    > {
        let (_, entries, recovery) = opi_agent::session::SessionReader::read_with_recovery(path)
            .map_err(|e| format!("failed to read session: {e}"))?;
        Ok((
            opi_agent::session_branch::SessionTree::from_entries(&entries),
            recovery,
        ))
    }

    /// Switch the current session to the branch ending at `tip_id`.
    pub fn resume_session_branch_tip(&mut self, tip_id: &str) -> Result<usize, String> {
        // Phase 16.10: grants do not survive a branch switch.
        if let Some(manager) = &self.permission_manager {
            manager.reset_grants();
        }
        let (path, session_id) = {
            let session = self
                .session
                .as_ref()
                .ok_or_else(|| "no active session".to_owned())?;
            (
                session.session_path().to_path_buf(),
                session.session_id().to_owned(),
            )
        };
        let (tree, _recovery) = Self::read_session_tree(&path)?;
        if !tree.branches().iter().any(|branch| branch.tip_id == tip_id) {
            return Err(format!("unknown branch tip: {tip_id}"));
        }

        self.session
            .as_mut()
            .ok_or_else(|| "no active session".to_owned())?
            .append_leaf(tip_id)
            .map_err(|e| format!("failed to select branch: {e}"))?;
        let (_, entries, recovery) = opi_agent::session::SessionReader::read_with_recovery(&path)
            .map_err(|e| format!("failed to read selected branch: {e}"))?;
        let ctx = reconstruct_context(&entries, &recovery);
        let message_count = ctx.messages.len();
        self.agent.replace_messages(ctx.messages);
        self.defer_extension_state_from_entries(&entries);
        for diagnostic in ctx.diagnostics {
            self.resources.metadata.diagnostics.push(diagnostic.clone());
            self.record_harness_diagnostic(diagnostic);
        }

        let compaction_config = opi_agent::compaction::CompactionConfig {
            enabled: self.config.compaction.enabled,
            threshold_tokens: self.config.compaction.threshold_tokens,
        };
        self.session = Some(
            SessionCoordinator::open_existing(
                path,
                session_id,
                &entries,
                message_count,
                compaction_config,
                self.agent.model().to_string(),
            )
            .map_err(|e| format!("failed to reopen selected branch: {e}"))?,
        );
        self.sync_session_cost_model();
        self.turn_offset = message_count;
        Ok(message_count)
    }

    /// Send a user prompt and run the agent loop.
    pub async fn prompt(&mut self, text: &str) -> Result<Vec<AgentMessage>, AgentError> {
        self.restore_pending_extension_state().await;
        self.prepare_trace_run()?;
        // C5: discard any unpersisted failed-turn user message before starting
        // a fresh turn so it is not absorbed into this turn's persistence slice.
        // (retry_last_prompt intentionally does NOT rewind — it reuses the
        // failed-turn user message after an interactive login.)
        self.agent.rewind_to(self.turn_offset);
        let offset = self.turn_offset;
        let messages = match self.agent.prompt(text).await {
            Ok(m) => m,
            Err(e) => {
                self.finish_trace_run();
                return Err(e);
            }
        };
        let new = &messages[offset..];
        self.persist_turn(new, offset).await;
        self.finish_trace_run();
        let final_messages = self.current_messages();
        self.turn_offset = final_messages.len();
        Ok(final_messages)
    }

    /// Send a user message with arbitrary content (text + images) and run the
    /// agent loop.
    pub async fn prompt_with_content(
        &mut self,
        content: Vec<opi_ai::message::InputContent>,
    ) -> Result<Vec<AgentMessage>, AgentError> {
        self.restore_pending_extension_state().await;
        self.prepare_trace_run()?;
        // C5: discard any unpersisted failed-turn user message before starting a
        // fresh turn (see `prompt`).
        self.agent.rewind_to(self.turn_offset);
        let offset = self.turn_offset;
        let messages = match self.agent.prompt_with_content(content).await {
            Ok(m) => m,
            Err(e) => {
                self.finish_trace_run();
                return Err(e);
            }
        };
        let new = &messages[offset..];
        self.persist_turn(new, offset).await;
        self.finish_trace_run();
        let final_messages = self.current_messages();
        self.turn_offset = final_messages.len();
        Ok(final_messages)
    }

    /// Retry the agent loop with the current messages (no new user message),
    /// used after a `CredentialNeeded` error is resolved via interactive login.
    /// The user message from the original `prompt`/`continue_` call is already
    /// in the agent's message list, so re-prompting would duplicate it.
    pub async fn retry_last_prompt(&mut self) -> Result<Vec<AgentMessage>, AgentError> {
        self.restore_pending_extension_state().await;
        self.prepare_trace_run()?;
        let offset = self.turn_offset;
        let messages = match self.agent.retry_last_turn().await {
            Ok(m) => m,
            Err(e) => {
                self.finish_trace_run();
                return Err(e);
            }
        };
        let new = &messages[offset..];
        self.persist_turn(new, offset).await;
        self.finish_trace_run();
        let final_messages = self.current_messages();
        self.turn_offset = final_messages.len();
        Ok(final_messages)
    }

    /// Continue the conversation with an additional message.
    pub async fn continue_(&mut self, text: &str) -> Result<Vec<AgentMessage>, AgentError> {
        self.restore_pending_extension_state().await;
        self.prepare_trace_run()?;
        let offset = self.turn_offset;
        let messages = match self.agent.continue_(text).await {
            Ok(m) => m,
            Err(e) => {
                self.finish_trace_run();
                return Err(e);
            }
        };
        let new = &messages[offset..];
        self.persist_turn(new, offset).await;
        self.finish_trace_run();
        let final_messages = self.current_messages();
        self.turn_offset = final_messages.len();
        Ok(final_messages)
    }

    /// Sum usage across every assistant message produced during a turn.
    ///
    /// A single user prompt can drive multiple provider calls (e.g.
    /// tool-call response followed by a final response). Each emitted
    /// assistant message carries its own `usage`; the cumulative session
    /// total must include all of them, not just the last one.
    fn aggregate_turn_usage(messages: &[AgentMessage]) -> opi_ai::stream::Usage {
        let mut saw_assistant = false;
        let mut all_reported = true;
        let mut input_tokens = 0u32;
        let mut output_tokens = 0u32;
        let mut cache_read_tokens = 0u32;
        let mut cache_write_tokens = 0u32;
        let mut cache_write_1h_tokens = None;
        let mut reasoning_tokens = None;
        for m in messages {
            if let AgentMessage::Llm(Message::Assistant(a)) = m {
                saw_assistant = true;
                all_reported &= a.usage.is_reported();
                input_tokens = input_tokens.saturating_add(a.usage.input_tokens);
                output_tokens = output_tokens.saturating_add(a.usage.output_tokens);
                cache_read_tokens = cache_read_tokens.saturating_add(a.usage.cache_read_tokens);
                cache_write_tokens = cache_write_tokens.saturating_add(a.usage.cache_write_tokens);
                if let Some(tokens) = a.usage.cache_write_1h_tokens {
                    cache_write_1h_tokens =
                        Some(cache_write_1h_tokens.unwrap_or(0u64).saturating_add(tokens));
                }
                if let Some(tokens) = a.usage.reasoning_tokens {
                    reasoning_tokens =
                        Some(reasoning_tokens.unwrap_or(0u64).saturating_add(tokens));
                }
            }
        }
        if saw_assistant && all_reported {
            opi_ai::stream::Usage::reported(
                input_tokens,
                output_tokens,
                cache_read_tokens,
                cache_write_tokens,
                cache_write_1h_tokens,
                reasoning_tokens,
            )
        } else {
            opi_ai::stream::Usage {
                input_tokens,
                output_tokens,
                cache_read_tokens,
                cache_write_tokens,
                cache_write_1h_tokens,
                reasoning_tokens,
                reported: false,
            }
        }
    }

    /// Aggregate usage across all assistant messages in a turn and persist.
    ///
    /// If compaction was triggered during persistence, this also rewrites
    /// the Agent's message buffer to `[summary, ...kept]` so subsequent
    /// provider calls no longer carry the compacted history. Emits
    /// `CompactionStart`/`CompactionEnd` events for subscribers.
    async fn persist_turn(&mut self, messages: &[AgentMessage], turn_start_agent_index: usize) {
        if let Some(session) = &mut self.session {
            let usage = Self::aggregate_turn_usage(messages);
            let compaction_reason =
                match session.on_turn_end(messages, &usage, turn_start_agent_index) {
                    Ok(reason) => reason,
                    Err(e) => {
                        self.agent.emit_event(AgentEvent::SessionPersistError {
                            message: format!("session write failed: {e}"),
                        });
                        return;
                    }
                };

            if let Some(reason) = compaction_reason {
                self.agent
                    .emit_event(AgentEvent::CompactionStart { reason });
                match session.execute_compaction(reason) {
                    Ok(Some(out)) => {
                        let wire = to_wire_result(&out);
                        self.record_harness_diagnostic(out.diagnostic.clone());
                        self.agent.replace_messages(out.new_agent_messages);
                        self.agent.emit_event(AgentEvent::CompactionEnd {
                            reason,
                            result: Some(wire),
                            aborted: false,
                            error_message: None,
                        });
                    }
                    Ok(None) => {
                        self.agent.emit_event(AgentEvent::CompactionEnd {
                            reason,
                            result: None,
                            aborted: true,
                            error_message: Some("compaction produced no output".into()),
                        });
                    }
                    Err(e) => {
                        // Compaction marker failed to persist - leave in-memory
                        // state un-compacted (SessionCoordinator already skipped
                        // the mutation) and surface the error to subscribers.
                        self.agent.emit_event(AgentEvent::CompactionEnd {
                            reason,
                            result: None,
                            aborted: true,
                            error_message: Some(format!("compaction persist failed: {e}")),
                        });
                        self.agent.emit_event(AgentEvent::SessionPersistError {
                            message: format!("compaction write failed: {e}"),
                        });
                    }
                }
            }
        }
        self.persist_extension_state().await;
    }

    async fn persist_extension_state(&mut self) {
        if self.session.is_none() {
            return;
        }
        let Some(registry) = self.extension_registry.clone() else {
            return;
        };

        let state = match registry.serialize_states_async().await {
            Ok(state) if state.as_object().is_some_and(|map| !map.is_empty()) => state,
            Ok(_) => return,
            Err(e) => {
                self.agent.emit_event(AgentEvent::SessionPersistError {
                    message: format!("extension state serialize failed: {e}"),
                });
                return;
            }
        };

        let result = self
            .session
            .as_mut()
            .expect("checked session is present")
            .append_extension_state(state);
        if let Err(e) = result {
            self.agent.emit_event(AgentEvent::SessionPersistError {
                message: format!("extension state write failed: {e}"),
            });
        }
    }

    /// Return the current message buffer (after any compaction).
    fn current_messages(&self) -> Vec<AgentMessage> {
        // The Agent's `set_initial_messages` / `replace_messages` API doesn't
        // expose a getter, so we re-derive the buffer from what was returned
        // by the loop plus any post-loop mutation. Simplest correct option:
        // ask the Agent via a new getter.
        self.agent.messages_snapshot()
    }

    // -- Phase 7 task 7.5: per-run trace bracketing + diagnostic counts -----

    fn next_run_id(&mut self) -> String {
        let id = format!("run-{}", self.run_seq);
        self.run_seq += 1;
        id
    }

    /// Build and prepare (fail-closed) a per-run trace collector when tracing
    /// is configured, and hand it to the agent so the loop emits records.
    /// Fail-closed: a prepare error aborts the run as `AgentError::TraceSetup`
    /// rather than running untraced. No-op when tracing is not configured.
    fn prepare_trace_run(&mut self) -> Result<(), AgentError> {
        // Reset the per-run diagnostic buffer so run-summary counts reflect
        // only the current run. RPC shares one harness (and one recording
        // sink) across multiple prompt/continue runs in a session.
        if let Some(sink) = &self.diagnostics {
            sink.clear();
        }
        let Some(config) = self.trace.clone() else {
            // Clear any stale collector left on the agent from a prior run.
            self.agent.set_trace_collector(None);
            if let Some(registry) = &self.extension_registry {
                registry.set_trace_collector(None);
            }
            return Ok(());
        };
        let run_id = self.next_run_id();
        let diagnostics = self
            .diagnostics
            .clone()
            .map(|sink| sink as Arc<dyn DiagnosticSink>);
        let collector = TraceCollector::new(run_id, config.mode, config.sink, diagnostics);
        collector
            .prepare()
            .map_err(|e| AgentError::TraceSetup(e.to_string()))?;
        let collector = Arc::new(collector);
        self.agent.set_trace_collector(Some(collector.clone()));
        if let Some(registry) = &self.extension_registry {
            registry.set_trace_collector(Some(collector.clone()));
        }
        self.active_trace = Some(collector);
        Ok(())
    }

    /// Finish the active run's collector (best-effort) and detach it from the
    /// agent. Safe to call when no collector is active.
    fn finish_trace_run(&mut self) {
        if let Some(collector) = self.active_trace.take() {
            collector.finish();
        }
        self.agent.set_trace_collector(None);
        if let Some(registry) = &self.extension_registry {
            registry.set_trace_collector(None);
        }
    }

    /// Mirror a diagnostic into the active run's trace as a diagnostic-linked
    /// record (fail-open). Used by harness-owned events (e.g. compaction) that
    /// do not flow through the agent loop's own observe() path.
    fn trace_diagnostic(&self, diagnostic: &Diagnostic) {
        if let Some(collector) = &self.active_trace {
            collector
                .record(diagnostic.source, TraceKind::DiagnosticLinked)
                .severity(diagnostic.severity)
                .diagnostic_code(diagnostic.code)
                .emit();
        }
    }

    fn record_harness_diagnostic(&self, diagnostic: Diagnostic) {
        if let Some(sink) = &self.diagnostics {
            sink.record(diagnostic.clone());
        }
        self.trace_diagnostic(&diagnostic);
    }

    /// Severity counts captured during the most recent run, when diagnostic
    /// recording is enabled. `None` when no recording sink is attached
    /// (preserving pre-7.5 behavior for callers that do not opt in).
    pub fn diagnostic_counts(&self) -> Option<SessionDiagnosticCounts> {
        let sink = self.diagnostics.as_ref()?;
        let mut counts = SessionDiagnosticCounts::default();
        for d in sink.snapshot() {
            match d.severity {
                Severity::Info => counts.info += 1,
                Severity::Warning => counts.warning += 1,
                Severity::Error => counts.error += 1,
            }
        }
        Some(counts)
    }

    /// Whether diagnostic recording is attached (for runner/rpc summary
    /// decisions). True when a recording sink was installed at build time.
    pub fn records_diagnostics(&self) -> bool {
        self.diagnostics.is_some()
    }

    /// Return the current model name.
    pub fn model(&self) -> &str {
        self.agent.model()
    }

    /// Return the current thinking configuration (Phase 13.3 read-side accessor
    /// used to verify that a resumed `thinking_level_change` was applied).
    pub fn thinking_config(&self) -> ThinkingConfig {
        self.agent.thinking_config()
    }

    /// Return the diagnostics recorded during the run, when diagnostic
    /// recording is enabled. Includes resume-emitted Phase 7 warnings such as
    /// incompatible recorded `model_change`/`thinking_level_change`.
    pub fn recorded_diagnostics(&self) -> Vec<Diagnostic> {
        self.diagnostics
            .as_ref()
            .map(|sink| sink.snapshot())
            .unwrap_or_default()
    }

    /// Queue a steering message for the next provider call.
    pub fn steer(&self, message: String) {
        self.agent.steer(message);
    }

    /// Queue a follow-up message for when the agent would otherwise stop.
    pub fn follow_up(&self, message: String) {
        self.agent.follow_up(message);
    }

    /// Register an event subscriber.
    pub fn subscribe(&mut self, callback: Box<dyn Fn(&AgentEvent) + Send + Sync>) {
        self.agent.subscribe(callback);
    }

    /// Return the assembled system prompt (for testing).
    pub fn system_prompt(&self) -> &str {
        &self.system_prompt
    }

    /// Return read-only discovered resource metadata.
    pub fn resource_metadata(&self) -> &DiscoveredResourceMetadata {
        &self.resources.metadata
    }

    /// Return resource metadata in the compact RPC/session-info shape.
    pub fn resource_metadata_json(&self) -> serde_json::Value {
        self.resources.metadata.to_rpc_json()
    }

    /// Dispatch a custom command to registered extensions.
    pub async fn dispatch_extension_command(
        &mut self,
        name: &str,
        id: Option<&str>,
        args: serde_json::Value,
    ) -> Result<Option<serde_json::Value>, String> {
        let Some(registry) = self.extension_registry.clone() else {
            return Ok(None);
        };
        self.restore_pending_extension_state().await;
        let mut command = opi_agent::extension::ExtensionCommand::new(name, args);
        if let Some(id) = id {
            command = command.with_id(id);
        }
        let result = registry
            .dispatch_command(&command)
            .await
            .map_err(|e| e.to_string())?;
        if result.is_some() {
            self.persist_extension_state().await;
        }
        Ok(result)
    }

    fn defer_extension_state_from_entries(&mut self, entries: &[opi_agent::session::SessionEntry]) {
        self.pending_extension_state = crate::session_coordinator::latest_extension_state(entries);
    }

    async fn restore_pending_extension_state(&mut self) {
        let Some(state) = self.pending_extension_state.take() else {
            return;
        };
        let Some(registry) = self.extension_registry.clone() else {
            return;
        };
        if let Err(e) = registry.restore_states_async(state).await {
            self.agent.emit_event(AgentEvent::SessionPersistError {
                message: format!("extension state restore failed: {e}"),
            });
        }
    }

    /// Resolve a theme using discovered themes first, then built-ins.
    pub fn resolve_theme(
        &self,
        name: &str,
    ) -> Result<opi_tui::Theme, crate::theme_discovery::ThemeDiscoveryError> {
        crate::theme_discovery::ThemeRegistry::from_resources(
            self.resources.theme_resources.clone(),
        )
        .resolve_theme(name)
    }

    /// Return a reference to the config.
    pub fn config(&self) -> &OpiConfig {
        &self.config
    }

    /// Cancel the running operation.
    pub fn cancel(&self) {
        self.agent.abort();
    }

    /// Return a clonable cancellation token for external cancellation.
    pub fn cancel_token(&self) -> tokio_util::sync::CancellationToken {
        self.agent.cancel_token()
    }

    /// Return a clonable control handle for an active agent turn.
    pub fn control_handle(&self) -> opi_agent::agent::AgentControl {
        self.agent.control_handle()
    }

    /// Reset cancellation state before cloning a control handle for a new turn.
    pub fn reset_cancel_if_cancelled(&mut self) {
        self.agent.reset_cancel_if_cancelled();
    }

    /// Return the session coordinator, if active.
    pub fn session(&self) -> Option<&SessionCoordinator> {
        self.session.as_ref()
    }

    /// Propagate the active session id from [`SessionCoordinator`] into the
    /// agent loop so providers receive it on every Request. Called after
    /// new-session creation, resume, and fork.
    fn sync_session_id(&mut self) {
        let id = self.session.as_ref().map(|s| s.session_id().to_owned());
        self.agent.set_session_id(id);
    }

    /// Execute compaction on the session and return the public result plus the
    /// diagnostic that describes the compaction outcome.
    pub fn compact_with_diagnostic(
        &mut self,
        reason: opi_agent::session_event::CompactionReason,
    ) -> Result<
        (
            Option<opi_agent::session_event::CompactionResult>,
            Diagnostic,
        ),
        String,
    > {
        let session = match &mut self.session {
            Some(s) => s,
            None => return Err("no active session".into()),
        };
        let result = session
            .execute_compaction(reason)
            .map_err(|e| format!("compaction failed: {e}"))?;
        match result {
            Some(out) => {
                let wire = crate::session_coordinator::to_wire_result(&out);
                let diagnostic = out.diagnostic.clone();
                self.record_harness_diagnostic(diagnostic.clone());
                self.agent.replace_messages(out.new_agent_messages);
                Ok((Some(wire), diagnostic))
            }
            None => {
                let error = opi_agent::compaction::CompactionError::NothingToCompact;
                let diagnostic = Diagnostic::from(&error);
                self.record_harness_diagnostic(diagnostic.clone());
                Ok((None, diagnostic))
            }
        }
    }

    /// Execute manual compaction on the session, if one is active.
    /// Returns the compaction result, or None if compaction produced no output
    /// or no session exists.
    pub fn compact(
        &mut self,
        reason: opi_agent::session_event::CompactionReason,
    ) -> Result<Option<opi_agent::session_event::CompactionResult>, String> {
        self.compact_with_diagnostic(reason)
            .map(|(result, _)| result)
    }

    /// Construct the eight built-in tools, filtered to the active selection.
    ///
    /// Phase 15 T5 + 16.9: `build_tools` constructs the local Operations defaults
    /// (`LocalFileOperations` / `LocalBashOperations`), threads the resolved
    /// execution context through [`ExecutionRuntime::build`], and injects the
    /// selected [`BashOperations`] plus the dynamic bash schema into the
    /// production `BashTool`. The four navigation tools (`grep`/`find`/`ls`/
    /// `glob`) keep their local-walk constructors unchanged — their `ignore`-
    /// crate walker cannot be cleanly redirected to a backend. Returns any
    /// execution-startup diagnostics (Phase 16.9) so they surface in
    /// interactive, non-interactive, and RPC modes.
    pub fn build_tools(
        workspace_root: &Path,
        tool_config: &ToolRuntimeConfig,
        execution: &ExecutionWiring,
    ) -> (Vec<Box<dyn Tool>>, Vec<Diagnostic>) {
        let local_ops: Arc<dyn BashOperations> = Arc::new(LocalBashOperations::new());
        let (bash_tool, exec_diagnostics) =
            Self::build_bash_tool(workspace_root, local_ops, execution);
        Self::build_tools_from_resolved_bash(
            workspace_root,
            tool_config,
            bash_tool,
            exec_diagnostics,
        )
    }

    /// Construct the direct-local tool set without activation, permission,
    /// router, adapter, or protocol state.
    fn build_minimal_runtime_tools(
        workspace_root: &Path,
        tool_config: &ToolRuntimeConfig,
    ) -> (Vec<Box<dyn Tool>>, Vec<Diagnostic>) {
        let local_ops: Arc<dyn BashOperations> = Arc::new(LocalBashOperations::new());
        let bash: Box<dyn Tool> = Box::new(BashTool::new_with_ops_and_schema(
            workspace_root.to_path_buf(),
            local_ops,
            default_bash_schema(),
        ));
        Self::build_tools_from_resolved_bash(workspace_root, tool_config, Some(bash), Vec::new())
    }

    /// Build the non-bash tool set for an execution configuration refused at
    /// startup. Headless fixed-local ask reaches this path with
    /// `permission_required` and no prompt channel.
    fn build_refused_execution_tools(
        workspace_root: &Path,
        tool_config: &ToolRuntimeConfig,
        failure: ExecutionFailure,
    ) -> (Vec<Box<dyn Tool>>, Vec<Diagnostic>) {
        Self::build_tools_from_resolved_bash(
            workspace_root,
            tool_config,
            None,
            vec![diagnostic_from_execution_failure(&failure)],
        )
    }

    fn build_tools_from_resolved_bash(
        workspace_root: &Path,
        tool_config: &ToolRuntimeConfig,
        bash_tool: Option<Box<dyn Tool>>,
        startup_diagnostics: Vec<Diagnostic>,
    ) -> (Vec<Box<dyn Tool>>, Vec<Diagnostic>) {
        let read_policy = match tool_config.run_mode {
            RunMode::Interactive => crate::tool::PathPolicy::AllowOutsideWorkspace,
            RunMode::NonInteractive => crate::tool::PathPolicy::WorkspaceOnly,
        };
        let file_ops: Arc<dyn FileOperations> =
            Arc::new(LocalFileOperations::new(workspace_root.to_path_buf()));
        let mut tools: Vec<(&str, Box<dyn Tool>)> = Vec::with_capacity(8);
        tools.push((
            "read",
            Box::new(ReadTool::new_with_ops(
                workspace_root.to_path_buf(),
                read_policy,
                file_ops.clone(),
            )),
        ));
        tools.push((
            "write",
            Box::new(WriteTool::new_with_ops(
                workspace_root.to_path_buf(),
                file_ops.clone(),
            )),
        ));
        tools.push((
            "edit",
            Box::new(EditTool::new_with_ops(
                workspace_root.to_path_buf(),
                file_ops.clone(),
            )),
        ));
        if let Some(bash) = bash_tool {
            tools.push(("bash", bash));
        }
        tools.push((
            "grep",
            Box::new(GrepTool::new(workspace_root.to_path_buf())),
        ));
        tools.push((
            "find",
            Box::new(FindTool::new(workspace_root.to_path_buf())),
        ));
        tools.push(("ls", Box::new(LsTool::new(workspace_root.to_path_buf()))));
        tools.push((
            "glob",
            Box::new(GlobTool::new(workspace_root.to_path_buf())),
        ));
        let tools = tools
            .drain(..)
            .filter(|(name, _)| {
                tool_config
                    .active_tool_names
                    .iter()
                    .any(|active| active == name)
            })
            .map(|(_, tool)| tool)
            .collect();
        (tools, startup_diagnostics)
    }

    /// Assemble a routed `BashTool` via [`ExecutionRuntime::build`]. A startup
    /// failure omits `bash` and returns its stable diagnostic; no fallback
    /// backend is substituted.
    fn build_bash_tool(
        workspace_root: &Path,
        local_ops: Arc<dyn BashOperations>,
        execution: &ExecutionWiring,
    ) -> (Option<Box<dyn Tool>>, Vec<Diagnostic>) {
        let Some(schema) =
            bash_input_schema(&execution.config, &execution.enabled, &execution.policy)
        else {
            let failure = crate::execution::ExecutionFailure::NoEligibleAdapter {
                strategy: execution.config.strategy,
                mode: execution.mode,
            };
            return (None, vec![diagnostic_from_execution_failure(&failure)]);
        };
        match ExecutionRuntime::build(
            &execution.config,
            execution.mode,
            &execution.enabled,
            &execution.policy,
            Arc::clone(&execution.store),
            local_ops,
            workspace_root,
            &execution.host_target,
            &execution.host_opi_version,
            Arc::clone(&execution.manager),
            execution.broker.clone(),
        ) {
            Ok(ops) => (
                Some(Box::new(BashTool::new_with_ops_and_schema(
                    workspace_root.to_path_buf(),
                    ops,
                    schema,
                ))),
                Vec::new(),
            ),
            Err(failure) => (None, vec![diagnostic_from_execution_failure(&failure)]),
        }
    }

    fn discover_resources(
        workspace_root: &Path,
        config: &OpiConfig,
        user_config_dir: Option<&Path>,
        resource_layers: Option<ResourceDiscoveryLayers>,
        installed_packages: Option<Vec<PackageResource>>,
        trust_decision: TrustDecision,
    ) -> HarnessResources {
        let explicit = ExplicitResourcePaths {
            extensions: config.extensions.paths.clone(),
            packages: config.packages.paths.clone(),
            ..Default::default()
        };
        let mut layers = resource_layers.unwrap_or_else(|| {
            standard_discovery_layers(workspace_root, user_config_dir, explicit)
        });
        // T6 gate (task 15.7): an untrusted project skips its project layer
        // (skills/fragments/themes/extensions/packages) so project-local
        // resources cannot resolve. User-global and explicit layers remain; this
        // same seam is the Phase 16 `/skill:`/`/fragment:` filter point.
        if !matches!(trust_decision, TrustDecision::Trusted) {
            for kind_layers in [
                &mut layers.extensions,
                &mut layers.packages,
                &mut layers.skills,
                &mut layers.fragments,
                &mut layers.themes,
            ] {
                kind_layers.retain(|layer| layer.kind != DiscoveryLayerKind::Project);
            }
        }
        let mut metadata = DiscoveredResourceMetadata::default();

        let packages = match crate::package_discovery::discover_packages(&layers.packages) {
            Ok(packages) => packages,
            Err(e) => {
                metadata
                    .diagnostics
                    .push(diagnostic_for_package_discovery_error(e));
                Vec::new()
            }
        };
        let mut packages = packages;
        match installed_packages {
            Some(installed_packages) => merge_package_resources(&mut packages, installed_packages),
            None if user_config_dir.is_some() => {
                let user_config_dir = user_config_dir.expect("checked Some");
                let package_scopes = if matches!(trust_decision, TrustDecision::Trusted) {
                    &[
                        crate::package_resolver::InstalledPackageScope::Global,
                        crate::package_resolver::InstalledPackageScope::Project,
                    ][..]
                } else {
                    &[crate::package_resolver::InstalledPackageScope::Global][..]
                };
                match crate::package_resolver::resolve_installed_packages_for_scopes(
                    workspace_root,
                    user_config_dir,
                    package_scopes,
                ) {
                    Ok(resolution) => {
                        metadata
                            .diagnostics
                            .extend(resolution.diagnostics.iter().map(diagnostic_from_package));
                        merge_package_resources(
                            &mut packages,
                            resolution
                                .packages
                                .into_iter()
                                .map(|package| package.package)
                                .collect(),
                        );
                    }
                    Err(e) => metadata
                        .diagnostics
                        .push(diagnostic_from_package_resolution_error(e)),
                }
            }
            None => {}
        }
        metadata.packages = packages
            .iter()
            .map(|package| ResourceMetadataEntry {
                name: package.manifest.name.clone(),
                description: Some(package.manifest.description.clone()),
                version: package.manifest.version.clone(),
            })
            .collect();

        let package_layers = crate::package_discovery::package_composed_resource_layers(&packages);
        metadata.diagnostics.extend(
            package_layers
                .diagnostics
                .into_iter()
                .map(diagnostic_for_resource_layer_message),
        );
        layers.extensions.extend(package_layers.extensions);
        layers.skills.extend(package_layers.skills);
        layers.fragments.extend(package_layers.fragments);
        layers.themes.extend(package_layers.themes);

        match crate::resource::discover_extension_resources(&layers.extensions) {
            Ok(extensions) => {
                metadata.extensions = extensions
                    .iter()
                    .map(|extension| ResourceMetadataEntry {
                        name: extension.manifest.name.clone(),
                        description: extension.manifest.description.clone(),
                        version: extension.manifest.version.clone(),
                    })
                    .collect();
            }
            Err(e) => metadata
                .diagnostics
                .push(diagnostic_for_resource_discovery_error("extension", e)),
        }

        match crate::skill::discover_skills(&layers.skills) {
            Ok(skills) => {
                metadata.skills = skills
                    .iter()
                    .map(|skill| ResourceMetadataEntry {
                        name: skill.manifest.name.clone(),
                        description: Some(skill.manifest.description.clone()),
                        version: None,
                    })
                    .collect();
            }
            Err(e) => metadata
                .diagnostics
                .push(diagnostic_for_resource_discovery_error("skill", e)),
        }

        match crate::prompt_fragment::discover_fragments(&layers.fragments) {
            Ok(fragments) => {
                metadata.fragments = fragments
                    .iter()
                    .map(|fragment| ResourceMetadataEntry {
                        name: fragment.manifest.name.clone(),
                        description: Some(fragment.manifest.description.clone()),
                        version: None,
                    })
                    .collect();
            }
            Err(e) => metadata
                .diagnostics
                .push(diagnostic_for_resource_discovery_error("fragment", e)),
        }

        let theme_resources = match crate::theme_discovery::discover_themes(&layers.themes) {
            Ok(themes) => {
                metadata.themes = themes
                    .iter()
                    .map(|theme| ResourceMetadataEntry {
                        name: theme.manifest.name.clone(),
                        description: Some(theme.manifest.description.clone()),
                        version: None,
                    })
                    .collect();
                themes
            }
            Err(e) => {
                metadata
                    .diagnostics
                    .push(diagnostic_for_resource_discovery_error("theme", e));
                Vec::new()
            }
        };

        HarnessResources {
            metadata,
            theme_resources,
        }
    }
}

fn merge_package_resources(
    packages: &mut Vec<crate::package_discovery::PackageResource>,
    installed: Vec<crate::package_discovery::PackageResource>,
) {
    for package in installed {
        if let Some(existing) = packages
            .iter_mut()
            .find(|existing| existing.manifest.name == package.manifest.name)
        {
            if package.layer_precedence >= existing.layer_precedence {
                *existing = package;
            }
        } else {
            packages.push(package);
        }
    }
    packages.sort_by(|a, b| {
        a.layer_precedence
            .cmp(&b.layer_precedence)
            .then_with(|| a.manifest.name.cmp(&b.manifest.name))
    });
}

fn initial_thinking_request_config(
    collection: &opi_ai::ProviderCollection,
    model: &str,
    config: &OpiConfig,
) -> (Option<ThinkingConfig>, Option<u64>) {
    if !config.thinking.enabled {
        return (None, None);
    }

    let budget_tokens = config.thinking.budget_tokens as u64;
    let Ok((mut thinking, mut max_tokens)) = request_config_for_thinking_budget(budget_tokens)
    else {
        return (None, None);
    };

    if let Ok((_, model)) = collection.resolve(model) {
        if !model.capabilities.supports_thinking {
            return (None, None);
        }
        if max_tokens > model.capabilities.max_output_tokens {
            if model.capabilities.max_output_tokens <= 1 {
                return (None, None);
            }
            let adjusted_budget = model.capabilities.max_output_tokens - 1;
            let Ok((adjusted_thinking, adjusted_max_tokens)) =
                request_config_for_thinking_budget(adjusted_budget)
            else {
                return (None, None);
            };
            thinking = adjusted_thinking;
            max_tokens = adjusted_max_tokens;
        }
    }

    (Some(thinking), Some(max_tokens))
}

fn request_config_for_thinking_budget(budget_tokens: u64) -> Result<(ThinkingConfig, u64), String> {
    let max_tokens = max_tokens_for_thinking_budget(budget_tokens)?;
    Ok((
        ThinkingConfig {
            enabled: true,
            budget_tokens: Some(budget_tokens),
            level: ThinkingLevel::Medium,
        },
        max_tokens,
    ))
}

fn max_tokens_for_thinking_budget(budget_tokens: u64) -> Result<u64, String> {
    budget_tokens.checked_add(1).ok_or_else(|| {
        format!("thinking budget {budget_tokens} cannot fit a valid max_tokens value")
    })
}

fn validate_thinking_budget_for_model(
    model: &ModelInfo,
    budget_tokens: u64,
    max_tokens: u64,
) -> Result<(), String> {
    if !model.capabilities.supports_thinking {
        return Err(model_does_not_support_thinking(&model.id));
    }
    if max_tokens > model.capabilities.max_output_tokens {
        return Err(thinking_budget_exceeds_model_limit(
            budget_tokens,
            max_tokens,
            model.capabilities.max_output_tokens,
            &model.id,
        ));
    }
    Ok(())
}

fn model_does_not_support_thinking(model_id: &str) -> String {
    format!("model '{model_id}' does not support thinking")
}

fn thinking_budget_exceeds_model_limit(
    budget_tokens: u64,
    max_tokens: u64,
    max_output_tokens: u64,
    model_id: &str,
) -> String {
    format!(
        "thinking budget {budget_tokens} requires max_tokens {max_tokens}, exceeding max output tokens {max_output_tokens} for model '{model_id}'"
    )
}

// ---------------------------------------------------------------------------
// Hooks
// ---------------------------------------------------------------------------

/// Shared conversion of agent-level messages to the provider-facing Message
/// stream. Used by every hook in this crate so resume/compaction semantics
/// stay consistent between interactive and non-interactive paths.
///
/// - `AgentMessage::Llm` is forwarded directly.
/// - `AgentMessage::CompactionSummary` is rendered as a synthetic user
///   message so the provider sees a textual marker for context that was
///   compacted away.
/// - `AgentMessage::BranchSummary` is rendered as a synthetic user message so
///   reconstructed parent-branch context reaches the provider when present.
/// - `AgentMessage::Custom` is dropped; extension provider-context semantics
///   remain deferred.
pub(crate) fn agent_messages_to_llm(messages: &[AgentMessage]) -> Vec<Message> {
    let mut result = Vec::with_capacity(messages.len());
    for msg in messages {
        match msg {
            AgentMessage::Llm(m) => result.push(m.clone()),
            AgentMessage::CompactionSummary(summary) => {
                result.push(Message::User(opi_ai::message::UserMessage {
                    content: vec![opi_ai::message::InputContent::Text {
                        text: format!(
                            "[Context was compacted. Summary of earlier conversation: {}]",
                            summary.summary
                        ),
                    }],
                    timestamp_ms: opi_ai::time::now_ms(),
                }));
            }
            AgentMessage::BranchSummary(summary) => {
                result.push(Message::User(opi_ai::message::UserMessage {
                    content: vec![opi_ai::message::InputContent::Text {
                        text: format!("[Context from parent branch: {}]", summary.summary),
                    }],
                    timestamp_ms: opi_ai::time::now_ms(),
                }));
            }
            _ => {}
        }
    }
    result
}

/// Default hooks for the coding agent -- pass-through message conversion.
pub struct CodingAgentHooks;

impl AgentHooks for CodingAgentHooks {
    fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
        Ok(agent_messages_to_llm(messages))
    }
}

/// Interactive hooks for the coding agent.
///
/// Tool safety is controlled by active tool selection and extension hooks, not
/// by a core interactive permission popup.
pub struct InteractiveCodingHooks;

impl InteractiveCodingHooks {
    pub fn new(_allow_mutating: bool) -> Self {
        Self
    }
}

impl AgentHooks for InteractiveCodingHooks {
    fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
        Ok(agent_messages_to_llm(messages))
    }
}

#[cfg(test)]
mod permission_boundary_tests {
    use super::*;

    fn build_interactive_harness(
        workspace: &Path,
        global: &Path,
        session_dir: &Path,
    ) -> CodingHarness {
        let provider = opi_ai::test_support::MockProvider::new(
            "mock",
            vec![opi_ai::test_support::text_response("ok")],
        );
        let mut config = OpiConfig::default();
        config
            .execution
            .permissions
            .insert(LOCAL_ADAPTER_ID.to_string(), PermissionDecision::Ask);
        CodingHarness::builder(
            Box::new(provider),
            "mock:mock-model".to_string(),
            config,
            workspace.to_path_buf(),
            crate::project_trust::TrustDecision::Trusted,
        )
        .global_config_dir(global.to_path_buf())
        .execution_mode(ExecutionRunMode::Interactive)
        .session_dir_for_test(session_dir.to_path_buf())
        .build()
    }

    /// Phase 16.10 D.2 must-fix: the production harness session-switch methods
    /// (resume_session_id / fork_current_session / resume_session_branch_tip)
    /// reset permission grants at the boundary. A regression deleting any of the
    /// three `reset_grants()` calls would leak an allow-for-session grant across
    /// a session switch; this test drives the production call sites (not the
    /// manager helper) and fails then. It also proves the production constructor
    /// installs the TUI broker for Interactive mode (`permission_prompt_rx` Some).
    #[test]
    fn session_switches_reset_permission_grants_at_production_call_sites() {
        let sessions_env_before = std::env::var_os("OPI_SESSIONS_DIR");
        let ws = tempfile::tempdir().unwrap();
        let global = tempfile::tempdir().unwrap();
        let sessions = tempfile::tempdir().unwrap();
        let mut harness = build_interactive_harness(ws.path(), global.path(), sessions.path());
        let source_session_path = harness
            .session()
            .expect("builder creates an isolated session")
            .session_path()
            .to_path_buf();
        let source_session_id = harness
            .session()
            .expect("builder creates an isolated session")
            .session_id()
            .to_string();
        assert!(source_session_path.starts_with(sessions.path()));
        assert_eq!(std::env::var_os("OPI_SESSIONS_DIR"), sessions_env_before);

        // The production constructor installs the TUI broker for Interactive mode
        // (permission_prompt_rx Some); headless modes leave it None (fail-closed).
        assert!(
            harness.permission_prompt_rx.is_some(),
            "interactive harness installs the permission broker"
        );

        // Resume the real source fixture by id. Before the per-instance lookup
        // fix this searched the process-global session directory and failed (or
        // could have opened an unrelated user session with the same id).
        let manager = Arc::clone(
            harness
                .permission_manager
                .as_ref()
                .expect("interactive ask constructs a permission manager"),
        );
        manager.grant_session("opi-sandbox");
        assert!(manager.has_session_grant("opi-sandbox"));
        assert_eq!(
            harness
                .resume_session_id(&source_session_id)
                .expect("resume resolves inside the instance session root"),
            0
        );
        assert!(
            !manager.has_session_grant("opi-sandbox"),
            "resume_session_id must reset permission grants at the boundary"
        );
        assert_eq!(
            harness
                .session()
                .expect("resumed session remains active")
                .session_path(),
            source_session_path
        );

        manager.grant_session("opi-sandbox");
        let (fork_session_id, _) = harness
            .fork_current_session()
            .expect("fork derives from the instance-scoped resumed path");
        assert!(
            !manager.has_session_grant("opi-sandbox"),
            "fork_current_session must reset permission grants at the boundary"
        );
        let fork_session_path = harness
            .session()
            .expect("forked session remains active")
            .session_path()
            .to_path_buf();
        assert!(fork_session_path.starts_with(sessions.path()));
        assert_ne!(fork_session_path, source_session_path);
        assert!(fork_session_path.ends_with(format!("{fork_session_id}.jsonl")));

        manager.grant_session("opi-sandbox");
        let _ = harness.resume_session_branch_tip("ghost");
        assert!(
            !manager.has_session_grant("opi-sandbox"),
            "resume_session_branch_tip must reset permission grants at the boundary"
        );
        assert_eq!(
            harness
                .session()
                .expect("failed branch selection keeps the fork active")
                .session_path(),
            fork_session_path
        );
        drop(harness);
        assert!(source_session_path.exists());
        assert!(fork_session_path.exists());
        assert_eq!(std::env::var_os("OPI_SESSIONS_DIR"), sessions_env_before);
    }

    #[test]
    fn default_allow_real_constructor_opens_no_extended_execution_state() {
        let ws = tempfile::tempdir().unwrap();
        let global = tempfile::tempdir().unwrap();
        let _store_factory = routed_store_factory_override::install(|| {
            panic!("Minimal Runtime must not invoke the routed store factory")
        });
        let (counts, _probe) = crate::execution::runtime::construction_probe::install();

        let provider = opi_ai::test_support::MockProvider::new(
            "mock",
            vec![opi_ai::test_support::text_response("ok")],
        );
        let harness = CodingHarness::builder(
            Box::new(provider),
            "mock:mock-model".to_string(),
            OpiConfig::default(),
            ws.path().to_path_buf(),
            crate::project_trust::TrustDecision::Trusted,
        )
        .global_config_dir(global.path().to_path_buf())
        .execution_mode(ExecutionRunMode::Interactive)
        .build();

        assert_eq!(counts.permission_managers(), 0);
        assert_eq!(counts.brokers(), 0);
        assert_eq!(counts.routers(), 0);
        assert_eq!(counts.protocol_states(), 0);
        assert!(harness.permission_manager.is_none());
        assert!(harness.permission_prompt_rx.is_none());
    }

    #[test]
    fn headless_ask_real_constructor_refuses_without_extended_execution_state() {
        for mode in [ExecutionRunMode::NonInteractive, ExecutionRunMode::Rpc] {
            let ws = tempfile::tempdir().unwrap();
            let global = tempfile::tempdir().unwrap();
            let _store_factory = routed_store_factory_override::install(|| {
                panic!("headless refusal must not invoke the routed store factory")
            });
            let (counts, _probe) = crate::execution::runtime::construction_probe::install();
            let mut config = OpiConfig::default();
            config
                .execution
                .permissions
                .insert(LOCAL_ADAPTER_ID.to_string(), PermissionDecision::Ask);
            let provider = opi_ai::test_support::MockProvider::new(
                "mock",
                vec![opi_ai::test_support::text_response("ok")],
            );

            let harness = CodingHarness::builder(
                Box::new(provider),
                "mock:mock-model".to_string(),
                config,
                ws.path().to_path_buf(),
                crate::project_trust::TrustDecision::Trusted,
            )
            .global_config_dir(global.path().to_path_buf())
            .execution_mode(mode)
            .build();

            assert_eq!(counts.permission_managers(), 0);
            assert_eq!(counts.brokers(), 0);
            assert_eq!(counts.routers(), 0);
            assert_eq!(counts.protocol_states(), 0);
            assert!(harness.permission_manager.is_none());
            assert!(harness.permission_prompt_rx.is_none());
            assert!(harness.resource_metadata().diagnostics.iter().any(|d| {
                d.details
                    .as_ref()
                    .and_then(|details| details.get("code"))
                    .and_then(serde_json::Value::as_str)
                    == Some("permission_required")
            }));
        }
    }

    #[test]
    fn interactive_ask_real_constructor_installs_permission_broker() {
        let ws = tempfile::tempdir().unwrap();
        let global = tempfile::tempdir().unwrap();
        let (counts, _probe) = crate::execution::runtime::construction_probe::install();
        let mut config = OpiConfig::default();
        config
            .execution
            .permissions
            .insert(LOCAL_ADAPTER_ID.to_string(), PermissionDecision::Ask);
        let provider = opi_ai::test_support::MockProvider::new(
            "mock",
            vec![opi_ai::test_support::text_response("ok")],
        );

        let harness = CodingHarness::builder(
            Box::new(provider),
            "mock:mock-model".to_string(),
            config,
            ws.path().to_path_buf(),
            crate::project_trust::TrustDecision::Trusted,
        )
        .global_config_dir(global.path().to_path_buf())
        .execution_mode(ExecutionRunMode::Interactive)
        .build();

        assert_eq!(counts.permission_managers(), 1);
        assert_eq!(counts.brokers(), 1);
        assert_eq!(counts.routers(), 1);
        assert_eq!(counts.protocol_states(), 0);
        assert!(harness.permission_manager.is_some());
        assert!(harness.permission_prompt_rx.is_some());
    }

    #[tokio::test]
    async fn general_routed_external_ask_builder_uses_installed_manager_and_channel() {
        struct MissingPackageSource;
        impl IdentitySource for MissingPackageSource {
            fn activate(
                &self,
                name: &str,
                _: &str,
                _: &str,
            ) -> Result<ActivatedContribution, ActivationError> {
                Err(ActivationError::NotInstalled(name.to_string()))
            }
        }

        let sessions_env_before = std::env::var_os("OPI_SESSIONS_DIR");
        let sessions = tempfile::tempdir().unwrap();
        let ws = tempfile::tempdir().unwrap();
        let global = tempfile::tempdir().unwrap();
        let _store_factory = routed_store_factory_override::install(|| RoutedStoreState {
            store: Arc::new(MissingPackageSource),
            enabled: vec![EnabledIdentity {
                adapter_id: "external-ask".to_string(),
                package_name: "external-package".to_string(),
            }],
        });
        let (counts, _probe) = crate::execution::runtime::construction_probe::install();
        let mut config = OpiConfig::default();
        config.execution.backend = "external-ask".to_string();
        config
            .execution
            .permissions
            .insert("external-ask".to_string(), PermissionDecision::Ask);
        let provider = opi_ai::test_support::MockProvider::new(
            "mock",
            vec![
                opi_ai::test_support::tool_call_response(
                    "external-ask-call",
                    "bash",
                    r#"{"command":"echo hi"}"#,
                ),
                opi_ai::test_support::text_response("done"),
            ],
        );

        let mut harness = CodingHarness::builder(
            Box::new(provider),
            "mock:mock-model".to_string(),
            config,
            ws.path().to_path_buf(),
            crate::project_trust::TrustDecision::Trusted,
        )
        .global_config_dir(global.path().to_path_buf())
        .execution_mode(ExecutionRunMode::Interactive)
        .session_dir_for_test(sessions.path().to_path_buf())
        .build();
        let session_path = harness
            .session()
            .expect("builder creates an isolated session")
            .session_path()
            .to_path_buf();
        assert!(
            session_path.starts_with(sessions.path()),
            "session artifact escaped the isolated root: {}",
            session_path.display()
        );
        assert_eq!(std::env::var_os("OPI_SESSIONS_DIR"), sessions_env_before);

        assert_eq!(counts.permission_managers(), 1);
        assert_eq!(counts.brokers(), 1);
        assert_eq!(counts.routers(), 1);
        assert_eq!(counts.protocol_states(), 1);
        let manager = Arc::clone(
            harness
                .permission_manager
                .as_ref()
                .expect("GeneralRouted installs its permission manager on the harness"),
        );
        let mut permission_rx = harness
            .permission_prompt_rx
            .take()
            .expect("interactive GeneralRouted installs its prompt channel");

        let (result, ()) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let prompt = harness.prompt("run the external adapter");
            let respond = async {
                let request =
                    tokio::time::timeout(std::time::Duration::from_secs(2), permission_rx.recv())
                        .await
                        .expect("production broker must send on the harness channel")
                        .expect("permission channel remains open");
                assert_eq!(request.summary.adapter_id, "external-ask");
                assert_eq!(request.summary.package_name, "external-package");
                assert_eq!(request.summary.run_mode_label, "interactive");
                request
                    .responder
                    .send(opi_tui::PermissionChoice::AllowSession)
                    .expect("production broker awaits the TUI response");
            };
            tokio::join!(prompt, respond)
        })
        .await
        .expect("production harness prompt and permission exchange timed out");

        assert!(
            result.is_ok(),
            "tool error returns to the agent loop: {result:?}"
        );
        assert!(
            manager.has_session_grant("external-ask"),
            "the runtime and harness must retain the same permission manager"
        );
        drop(harness);
        assert!(
            session_path.exists(),
            "prompt must persist only inside the isolated session root"
        );
        assert_eq!(std::env::var_os("OPI_SESSIONS_DIR"), sessions_env_before);
    }

    #[test]
    fn legacy_tool_config_constructor_derives_noninteractive_execution_mode() {
        let ws = tempfile::tempdir().unwrap();
        let (counts, _probe) = crate::execution::runtime::construction_probe::install();
        let mut config = OpiConfig::default();
        config
            .execution
            .permissions
            .insert(LOCAL_ADAPTER_ID.to_string(), PermissionDecision::Ask);
        let tool_config =
            ToolRuntimeConfig::resolve(RunMode::NonInteractive, true, ToolSelection::Default)
                .expect("non-interactive mutating tool config");
        let provider = opi_ai::test_support::MockProvider::new(
            "mock",
            vec![opi_ai::test_support::text_response("ok")],
        );

        let harness = CodingHarness::new_with_tool_config(
            Box::new(provider),
            "mock:mock-model".to_string(),
            config,
            ws.path().to_path_buf(),
            tool_config,
            crate::project_trust::TrustDecision::Trusted,
        );

        assert_eq!(counts.brokers(), 0, "legacy headless mode must not prompt");
        assert_eq!(counts.routers(), 0, "headless ask is refused at startup");
        assert!(harness.permission_manager.is_none());
        assert!(harness.permission_prompt_rx.is_none());
        assert!(harness.resource_metadata().diagnostics.iter().any(|d| {
            d.details
                .as_ref()
                .and_then(|details| details.get("code"))
                .and_then(serde_json::Value::as_str)
                == Some("permission_required")
        }));
    }
}
