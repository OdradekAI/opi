//! Interactive CLI harness and coding-agent product wrapper over the generic
//! opi-agent runtime seams.
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
//! [`opi_agent::Agent`] owns the generic turn loop and complete next-turn state;
//! [`opi_agent::harness::SessionFacade`] owns the product-neutral ordered
//! session seam. `CodingHarness` composes those mechanisms with product policy.
//!
//! Boundary contract: product/CLI/package policy must not move into `opi-agent`.
//! This is pinned by `coding_harness_wrapper_keeps_product_policy_out_of_opi_agent`,
//! and the wrapper composition is exercised by
//! `coding_harness_composes_generic_opi_agent_seams`. Existing CLI/RPC/JSON/
//! interactive behavior continues to run through this wrapper unchanged.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use opi_agent::agent::ArmedAgentRun;
use opi_agent::diagnostic::code::{
    CODE_SESSION_RESUME_MODEL_INCOMPATIBLE, CODE_SESSION_RESUME_ROUTE_AMBIGUOUS,
    CODE_SESSION_RESUME_ROUTE_MISSING, CODE_SESSION_RESUME_THINKING_INCOMPATIBLE,
};
use opi_agent::diagnostic::{
    Diagnostic, DiagnosticPayload, RedactionMode, SOURCE_SESSION, Severity,
};
use opi_agent::event::AgentEvent;
use opi_agent::extension::ExtensionRegistry;
use opi_agent::hooks::AgentHooks;
use opi_agent::loop_types::{AgentError, AgentLoopConfig};
use opi_agent::message::AgentMessage;
use opi_agent::session::ModelInputSource;
use opi_agent::session_context::reconstruct_context;
use opi_agent::session_event::{SessionDiagnosticCounts, ThinkingLevel};
use opi_agent::tool::Tool;
use opi_agent::{Agent, DiagnosticSink, RecordingSink};
use opi_ai::message::Message;

use crate::evidence::{
    EvidenceBuilderConfig, EvidenceCapture, RunDynamicFacts, build_finalized_manifest,
    direct_runtime_input_binding, usage_facts,
};
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

/// Product auth-remediation view over both legacy product errors and the
/// Agent Core's retained typed provider failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProviderAuthFailure<'a> {
    CredentialNeeded(&'a str),
    CredentialRevoked(&'a str),
    AccountIdMissing(&'a str),
}

pub(crate) fn provider_auth_failure(error: &AgentError) -> Option<ProviderAuthFailure<'_>> {
    match error {
        AgentError::CredentialNeeded { provider_id } => {
            Some(ProviderAuthFailure::CredentialNeeded(provider_id))
        }
        AgentError::CredentialRevoked { provider_id } => {
            Some(ProviderAuthFailure::CredentialRevoked(provider_id))
        }
        AgentError::AccountIdMissing { provider_id } => {
            Some(ProviderAuthFailure::AccountIdMissing(provider_id))
        }
        AgentError::Provider(failure) => match failure.provider_error() {
            opi_ai::provider::ProviderError::CredentialNeeded { provider_id } => {
                Some(ProviderAuthFailure::CredentialNeeded(provider_id))
            }
            opi_ai::provider::ProviderError::CredentialRevoked { provider_id } => {
                Some(ProviderAuthFailure::CredentialRevoked(provider_id))
            }
            opi_ai::provider::ProviderError::AccountIdMissing { provider_id } => {
                Some(ProviderAuthFailure::AccountIdMissing(provider_id))
            }
            _ => None,
        },
        _ => None,
    }
}
use crate::session_coordinator::{SessionBatchRollbackError, SessionCoordinator, to_wire_result};
use crate::tool::{
    BashOperations, BashTool, EditTool, FileOperations, FindTool, GlobTool, GrepTool,
    LocalBashOperations, LocalFileOperations, LsTool, ReadTool, WriteTool, default_bash_schema,
    with_model_backend_enum,
};
use tokio::sync::mpsc;

fn compaction_reason_name(reason: opi_agent::session_event::CompactionReason) -> &'static str {
    match reason {
        opi_agent::session_event::CompactionReason::Manual => "manual",
        opi_agent::session_event::CompactionReason::Threshold => "threshold",
        opi_agent::session_event::CompactionReason::Overflow => "overflow",
    }
}

fn compaction_trigger(
    reason: opi_agent::session_event::CompactionReason,
) -> opi_agent::evidence::CompactionTrigger {
    match reason {
        opi_agent::session_event::CompactionReason::Manual => {
            opi_agent::evidence::CompactionTrigger::Manual
        }
        opi_agent::session_event::CompactionReason::Threshold => {
            opi_agent::evidence::CompactionTrigger::Threshold
        }
        opi_agent::session_event::CompactionReason::Overflow => {
            opi_agent::evidence::CompactionTrigger::Overflow
        }
    }
}

fn canonical_json(value: serde_json::Value) -> String {
    fn sorted(value: serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Array(values) => {
                serde_json::Value::Array(values.into_iter().map(sorted).collect())
            }
            serde_json::Value::Object(values) => {
                let mut entries = values.into_iter().collect::<Vec<_>>();
                entries.sort_by(|(left, _), (right, _)| left.cmp(right));
                let mut output = serde_json::Map::new();
                for (key, value) in entries {
                    output.insert(key, sorted(value));
                }
                serde_json::Value::Object(output)
            }
            scalar => scalar,
        }
    }

    serde_json::to_string(&sorted(value)).expect("JSON value serialization cannot fail")
}

fn evidence_digest(value: serde_json::Value) -> opi_agent::evidence::ContentDigest {
    opi_agent::evidence::ContentDigest::from_hex(crate::tool_authority::digest_of(&canonical_json(
        value,
    )))
    .expect("digest_of returns canonical SHA-256 hex")
}

fn permission_decision_name(decision: PermissionDecision) -> &'static str {
    match decision {
        PermissionDecision::Deny => "deny",
        PermissionDecision::Ask => "ask",
        PermissionDecision::Allow => "allow",
    }
}

fn canonical_model_spec(provider_id: &str, input: &str) -> String {
    if input.contains(':') {
        input.to_owned()
    } else {
        format!("{provider_id}:{input}")
    }
}

struct ResolvedRuntimeInput {
    binding: opi_agent::evidence::RuntimeInputBinding,
    config: opi_agent::evidence::ConfigIdentity,
    material_inputs: String,
    system_digest: Option<opi_agent::evidence::ContentDigest>,
    tool_schema_digests: Vec<opi_agent::evidence::ContentDigest>,
    budget: opi_agent::evidence::Measurement,
}

/// Resolve the material facts that both durable session ownership and optional
/// evidence capture must bind before a run begins.
fn resolve_runtime_input(
    source: &opi_agent::evidence::AssemblyIdentity,
    policy_digest: &opi_agent::evidence::ContentDigest,
    agent: &Agent,
    config: &OpiConfig,
    system_prompt: &str,
    model_registry: &opi_ai::ProviderCollection,
) -> ResolvedRuntimeInput {
    let state = agent.state_snapshot();
    let model_spec = state.model_selection.to_spec();
    let model = model_registry
        .resolve(&model_spec)
        .map(|(_, model)| model)
        .expect("agent state is validated against the dispatch collection");
    let tool_definitions = agent.tool_definitions_snapshot();
    let tool_schema_digests = tool_definitions
        .iter()
        .map(|definition| {
            evidence_digest(serde_json::json!({
                "name": definition.name,
                "description": definition.description,
                "input_schema": definition.input_schema,
            }))
        })
        .collect::<Vec<_>>();
    let system_digest = Some(evidence_digest(serde_json::json!({
        "system": system_prompt,
    })));
    let thinking_level = state.inference.thinking.level.wire_name().unwrap_or("none");
    let execution_rules = config
        .execution
        .rules
        .iter()
        .map(|rule| {
            serde_json::json!({
                "modes": rule.modes.as_ref().map(|modes| {
                    modes.iter().map(ToString::to_string).collect::<Vec<_>>()
                }),
                "backend": rule.backend,
            })
        })
        .collect::<Vec<_>>();
    let execution_permissions = config
        .execution
        .permissions
        .iter()
        .map(|(adapter, decision)| {
            serde_json::json!({
                "adapter": adapter,
                "decision": permission_decision_name(*decision),
            })
        })
        .collect::<Vec<_>>();
    let harness_digest = evidence_digest(serde_json::json!({
        "version": 1,
        "model": model_spec,
        "system_digest": system_digest.as_ref().map(|digest| digest.as_hex()),
        "tool_schema_digests": tool_schema_digests
            .iter()
            .map(|digest| digest.as_hex())
            .collect::<Vec<_>>(),
    }));
    let runtime_digest = evidence_digest(serde_json::json!({
        "max_turns": config.defaults.max_iterations,
        "retry": {
            "max_attempts": config.retry.max_attempts,
            "initial_delay_ms": config.retry.initial_delay_ms,
            "max_delay_ms": config.retry.max_delay_ms,
        },
        "inference": {
            "thinking_enabled": state.inference.thinking.enabled,
            "thinking_budget_tokens": state.inference.thinking.budget_tokens,
            "thinking_level": thinking_level,
            "max_tokens": state.inference.max_tokens,
            "temperature": state.inference.temperature,
        },
        "compaction": {
            "enabled": config.compaction.enabled,
            "threshold_tokens": config.compaction.threshold_tokens,
        },
    }));
    let adapter_digest = evidence_digest(serde_json::json!({
        "provider": state.model_selection.provider_id,
        "model": state.model_selection.model_id,
        "wire": model.wire_api,
    }));
    let material_digest = evidence_digest(serde_json::json!({
        "execution": {
            "strategy": config.execution.strategy.to_string(),
            "backend": config.execution.backend,
            "rules": execution_rules,
            "permissions": execution_permissions,
        },
        "tool_timeout_ms": config.defaults.tool_timeout_ms,
        "max_image_bytes": config.defaults.max_image_bytes,
        "allow_mutating_tools": config.defaults.allow_mutating_tools,
    }));
    let material_inputs = canonical_json(serde_json::json!({
        "model": model_spec,
        "system_digest": system_digest.as_ref().map(|digest| digest.as_hex()),
        "tool_schema_digests": tool_schema_digests
            .iter()
            .map(|digest| digest.as_hex())
            .collect::<Vec<_>>(),
        "inference": {
            "thinking_enabled": state.inference.thinking.enabled,
            "thinking_budget_tokens": state.inference.thinking.budget_tokens,
            "thinking_level": thinking_level,
            "max_tokens": state.inference.max_tokens,
            "temperature": state.inference.temperature,
        },
    }));
    let max_iterations = config.defaults.max_iterations;
    let config = opi_agent::evidence::ConfigIdentity {
        harness_digest,
        runtime_digest,
        adapter_digest,
        material_digest,
    };
    let budget = opi_agent::evidence::Measurement::Known {
        value: u64::from(max_iterations),
        origin: opi_agent::evidence::MeasurementOrigin::Quota,
    };
    let binding = direct_runtime_input_binding(source, policy_digest, &config, &material_inputs);
    ResolvedRuntimeInput {
        binding,
        config,
        material_inputs,
        system_digest,
        tool_schema_digests,
        budget,
    }
}

/// Resolved routed-execution inputs threaded into
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
    /// interactive startup path installs the TUI-backed broker.
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
        // TUI-backed broker.
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
#[derive(Clone)]
pub struct ResumeInfo {
    pub path: PathBuf,
    pub session_id: String,
    pub entries: Vec<opi_agent::session::SessionEntry>,
    /// The workspace cwd recorded in the session header. Used to restore the
    /// correct workspace root when resuming from a different directory.
    pub original_cwd: PathBuf,
    /// Structured diagnostics observed while reading the resumed session.
    pub diagnostics: Vec<Diagnostic>,
    /// Latest `model_change` recorded on the active branch, if
    /// any. The harness re-applies it when compatible with the CLI/config
    /// provider, mirroring `CodingHarness::resume_session_id`.
    pub recorded_model: Option<String>,
    /// Latest `thinking_level_change` recorded on the active branch, if any.
    /// Re-applied when compatible with the active model.
    pub recorded_thinking: Option<ThinkingLevel>,
}

/// Coding-agent product wrapper over the generic opi-agent runtime seams.
///
/// Owns coding-agent product policy (built-in file tools, CLI/project config,
/// context files, package resources/adapters, interactive commands, product
/// defaults, extension-state restore/persist) and composes it over the generic
/// [`Agent`] loop, [`AgentHooks`], [`ExtensionRegistry`], generic session
/// storage, [`opi_ai::ProviderCollection`], and compaction.
pub struct CodingHarness {
    agent: Agent,
    /// Cancellation generation armed for the next public run. Product modes
    /// clone its control surface before moving the harness into an async task;
    /// the run consumes this exact generation before awaited preflight.
    armed_run: Option<ArmedAgentRun>,
    config: OpiConfig,
    system_prompt: String,
    resources: HarnessResources,
    /// The single dispatch and model-lookup collection. Serves
    /// `model_info`, `model_picker_items`, and thinking-validation in addition
    /// to the Agent's dispatch path; the active provider is a real dispatchable
    /// route in it (no metadata proxy).
    model_registry: Arc<opi_ai::ProviderCollection>,
    /// The dispatchable provider ids in [`Self::model_registry`]:
    /// the active route plus every extra route registered with an auth resolver.
    /// Lookup-only extension providers are excluded. Used by read-side legacy
    /// route normalization to prove exactly one dispatchable route for a bare
    /// model without guessing the active provider.
    dispatchable_provider_ids: Vec<String>,
    extension_registry: Option<ExtensionRegistry>,
    session: Option<SessionCoordinator>,
    /// Deferred typed failure from a requested builder-driven resume. Public
    /// operations surface it before dispatch instead of silently running
    /// without the requested session.
    session_resume_error: Option<String>,
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
    /// diagnostic sink unset.
    diagnostics: Option<Arc<RecordingSink>>,
    /// Evidence capture: the recorder plus per-run binding facts used
    /// to assemble the finalized manifest. `None` is the capture-
    /// disabled no-op (Minimal Runtime); the Agent's evidence sink stays unset.
    evidence: Option<EvidenceCapture>,
    /// The trusted product assembly that owns direct runtime-input bindings
    /// even when evidence capture is disabled.
    runtime_input_source: opi_agent::evidence::AssemblyIdentity,
    /// Effective policy identity shared by session headers and manifests.
    runtime_input_policy_digest: opi_agent::evidence::ContentDigest,
    /// The OS-keychain-backed credential store, set by production startup.
    /// Used by the interactive loop for `/login` and `/logout`.
    pub credential_store: Option<Arc<KeychainCredentialStore>>,
    /// The built-in OAuth provider registry, set by production startup.
    pub oauth_registry: Option<OAuthProviderRegistry>,
    pub(crate) oauth_endpoints: OAuthEndpointConfig,
    pub(crate) oauth_http_client: reqwest::Client,
    /// In-memory capability-permission grants. Present only for
    /// routed execution, shared with the routed bash backend, and reset on
    /// in-process session switches so an `allow-for-session` choice does not
    /// survive resume/fork/branch. Minimal and startup-refused execution use
    /// `None` and construct no permission state.
    pub(crate) permission_manager: Option<Arc<PermissionManager>>,
    /// The interactive permission-prompt channel receiver.
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

/// Typed read-side failure when a recorded route cannot be normalized against
/// the dispatchable collection. Surfaced as
/// distinct diagnostic codes so callers can distinguish ambiguity from absence
/// without parsing strings; resolution never dispatches a provider.
enum RouteRemediation {
    /// The bare model matches more than one dispatchable route.
    Ambiguous { candidates: Vec<String> },
    /// The model matches no dispatchable route.
    Missing,
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

/// Aggregated live session metadata surfaced by `/session info`
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
    /// Opt-in evidence capture (recorder plus direct-assembly source).
    /// `None` is the capture-disabled no-op Minimal Runtime.
    evidence: Option<EvidenceBuilderConfig>,
    runtime_input_source: opi_agent::evidence::AssemblyIdentity,
    fork_on_start: bool,
    trust_decision: TrustDecision,
    execution_mode: ExecutionRunMode,
    /// Collection-owned auth resolver for the active dispatch route
    /// (production passes the `ProviderBundle` resolver; `None` defaults to a
    /// dummy static resolver so mock-provider tests dispatch via `prepare_call`).
    auth_resolver: Option<Arc<dyn opi_ai::auth::AuthResolver>>,
    /// Additional dispatchable routes (adapter plus lazy auth resolver)
    /// constructed eagerly at startup. Registered alongside the active route so
    /// a cross-provider model switch resolves through the same collection.
    extra_routes: Vec<crate::provider_factory::ProviderAuthPair>,
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
            evidence: None,
            runtime_input_source: crate::evidence::SDK_ASSEMBLY.clone(),
            fork_on_start: false,
            trust_decision,
            execution_mode: ExecutionRunMode::Interactive,
            auth_resolver: None,
            extra_routes: Vec::new(),
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

    /// Fork the supplied resume source only after trusted product assembly has
    /// resolved the current runtime-input binding.
    pub fn fork_on_start(mut self) -> Self {
        self.fork_on_start = true;
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
    /// severity counts. Off by default; enabling installs a
    /// [`RecordingSink`] on the agent with no other behavior change.
    pub fn record_diagnostics(mut self, enabled: bool) -> Self {
        self.record_diagnostics = enabled;
        self
    }

    /// Enable evidence capture. When set, each prompt run binds the
    /// recorder as the Agent's [`opi_agent::evidence::EvidenceSink`], calls
    /// `setup` before the run (fail-closed), and finalizes one strict
    /// `DirectRuntimeInput`-bound manifest after the run. `source` labels the
    /// direct-assembly origin (CLI / SDK / RPC). Absent capture is the no-op
    /// Minimal Runtime.
    pub fn evidence(mut self, config: EvidenceBuilderConfig) -> Self {
        self.runtime_input_source = config.source.clone();
        self.evidence = Some(config);
        self
    }

    /// Set the trusted product assembly used for direct bindings when capture
    /// is disabled. SDK construction keeps the SDK default; CLI and RPC
    /// startup select their own existing assembly identities.
    pub fn runtime_input_source(mut self, source: opi_agent::evidence::AssemblyIdentity) -> Self {
        self.runtime_input_source = source;
        self
    }

    /// Set the resolved project-trust decision. When
    /// [`TrustDecision::Untrusted`], `discover_resources` skips the project
    /// resource layer and context-file discovery skips project `AGENTS.md`/
    /// `CLAUDE.md`. Only [`TrustDecision::Trusted`] loads project resources;
    /// `Untrusted` and `Undecided` both fail closed.
    pub fn trust_decision(mut self, decision: TrustDecision) -> Self {
        self.trust_decision = decision;
        self
    }

    /// Set the execution run mode threaded into
    /// `ExecutionRuntime::build`. Defaults to [`ExecutionRunMode::Interactive`];
    /// headless startup paths set this to `NonInteractive` (runner/text/NDJSON)
    /// or `Rpc` (RPC). It cannot be derived from `tool_config.run_mode`, which
    /// collapses RPC into `NonInteractive`.
    pub fn execution_mode(mut self, mode: ExecutionRunMode) -> Self {
        self.execution_mode = mode;
        self
    }

    /// Set the collection-owned auth resolver for the active
    /// dispatch route. Production startup passes the `ProviderBundle` resolver
    /// (a `CredentialResolver`). When unset, the harness installs a dummy static
    /// resolver so mock-provider tests dispatch through `prepare_call` without
    /// credentials.
    pub fn auth_resolver(mut self, resolver: Arc<dyn opi_ai::auth::AuthResolver>) -> Self {
        self.auth_resolver = Some(resolver);
        self
    }

    /// Attach additional dispatchable routes (adapter plus lazy auth
    /// resolver) constructed eagerly at startup. Registered alongside the active
    /// route so a cross-provider model switch resolves without reconstructing
    /// the Agent.
    pub fn extra_routes(mut self, routes: Vec<crate::provider_factory::ProviderAuthPair>) -> Self {
        self.extra_routes = routes;
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
        let tool_config = self.tool_config.unwrap_or_else(|| {
            ToolRuntimeConfig::resolve(RunMode::Interactive, true, self.tool_selection.clone())
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
                record_diagnostics: self.record_diagnostics,
                evidence: self.evidence,
                runtime_input_source: self.runtime_input_source,
                fork_on_start: self.fork_on_start,
                trust_decision: self.trust_decision,
                execution_mode: self.execution_mode,
                auth_resolver: self.auth_resolver,
                extra_routes: self.extra_routes,
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
    record_diagnostics: bool,
    /// Opt-in evidence capture (recorder plus direct-assembly source).
    evidence: Option<EvidenceBuilderConfig>,
    runtime_input_source: opi_agent::evidence::AssemblyIdentity,
    fork_on_start: bool,
    trust_decision: TrustDecision,
    /// The run mode threaded into `ExecutionRuntime::build`.
    /// Legacy constructors derive interactive/non-interactive from tool config;
    /// RPC remains available only through startup paths that set it explicitly.
    execution_mode: ExecutionRunMode,
    /// The collection-owned auth resolver for the active dispatch
    /// route. Production startup passes the `ProviderBundle` resolver (a
    /// `CredentialResolver`); when `None`, the harness installs a dummy static
    /// resolver so mock-provider tests dispatch through `prepare_call` without
    /// supplying credentials (the mock ignores the resolved auth).
    auth_resolver: Option<Arc<dyn opi_ai::auth::AuthResolver>>,
    /// Additional dispatchable routes (adapter plus lazy auth resolver).
    extra_routes: Vec<crate::provider_factory::ProviderAuthPair>,
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
            record_diagnostics: false,
            evidence: None,
            runtime_input_source: crate::evidence::SDK_ASSEMBLY.clone(),
            fork_on_start: false,
            trust_decision: TrustDecision::Undecided,
            execution_mode: ExecutionRunMode::Interactive,
            auth_resolver: None,
            extra_routes: Vec::new(),
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
        let is_legacy_resume = resume.as_ref().is_some_and(|info| {
            opi_agent::session::SessionReader::read_with_recovery(&info.path)
                .map(|(header, _, _)| header.version == opi_agent::session::LEGACY_FORMAT_VERSION)
                .unwrap_or(false)
        });
        let deferred_resume = (is_legacy_resume || build_options.fork_on_start)
            .then(|| resume.clone())
            .flatten();
        // Gather extension providers plus model overrides up front, and
        // materialize the active provider's overrides onto the provider itself
        // while it is still mutable (before it becomes a dispatch route). The
        // registry override layer (built later by `build_harness_collection`)
        // only holds non-active providers.
        let overrides = extension_registry
            .as_ref()
            .map(ExtensionRegistry::collect_model_overrides)
            .unwrap_or_default();
        let extension_providers = extension_registry
            .as_ref()
            .map(ExtensionRegistry::collect_providers)
            .unwrap_or_default();
        let active_provider_id = provider.id().to_owned();
        let model_registry_diagnostics =
            crate::provider_factory::materialize_active_overrides(provider.as_mut(), &overrides);
        if let Some(registry) = extension_registry.as_ref() {
            extension_event_registry = Some(registry.clone());
            injected_extension_names = registry
                .names()
                .into_iter()
                .map(str::to_owned)
                .collect::<Vec<_>>();
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
        let workspace_scope_digest =
            crate::tool_authority::digest_of(&workspace_root.to_string_lossy());
        let (
            tools,
            tool_diagnostics,
            permission_manager,
            permission_prompt_rx,
            command_authorization,
        ) = match execution {
            HarnessExecution::DirectLocal => {
                let (tools, diagnostics) =
                    Self::build_minimal_runtime_tools(&workspace_root, &tool_config);
                let policy = PermissionPolicy::from_map(config.execution.permissions.clone());
                let command = crate::tool_authority::CommandAuthorizationContext::new(
                    config.execution.clone(),
                    build_options.execution_mode,
                    Eligibility::from_enabled(&[], &policy),
                    None,
                    None,
                    workspace_scope_digest.clone(),
                    std::collections::BTreeMap::new(),
                );
                (tools, diagnostics, None, None, command)
            }
            HarnessExecution::Refused(failure) => {
                let (tools, diagnostics) =
                    Self::build_refused_execution_tools(&workspace_root, &tool_config, failure);
                let policy = PermissionPolicy::from_map(config.execution.permissions.clone());
                let command = crate::tool_authority::CommandAuthorizationContext::new(
                    config.execution.clone(),
                    build_options.execution_mode,
                    Eligibility::from_enabled(&[], &policy),
                    None,
                    None,
                    workspace_scope_digest.clone(),
                    std::collections::BTreeMap::new(),
                );
                (tools, diagnostics, None, None, command)
            }
            HarnessExecution::Routed(mut execution) => {
                let permission_manager = Some(Arc::clone(&execution.manager));
                let permission_prompt_rx = if build_options.execution_mode
                    == ExecutionRunMode::Interactive
                {
                    #[cfg(test)]
                    crate::execution::runtime::construction_probe::broker_constructed();
                    let (tx, rx) = mpsc::channel::<crate::interactive::PermissionPromptRequest>(8);
                    execution.broker =
                        Some(Arc::new(crate::interactive::TuiPermissionBroker::new(tx)));
                    Some(rx)
                } else {
                    None
                };
                let (tools, diagnostics) =
                    Self::build_tools(&workspace_root, &tool_config, &execution);
                let package_names = execution
                    .enabled
                    .iter()
                    .map(|identity| (identity.adapter_id.clone(), identity.package_name.clone()))
                    .collect();
                let command = crate::tool_authority::CommandAuthorizationContext::new(
                    execution.config.clone(),
                    execution.mode,
                    Eligibility::from_enabled(&execution.enabled, &execution.policy),
                    Some(Arc::clone(&execution.manager)),
                    execution.broker.clone(),
                    workspace_scope_digest.clone(),
                    package_names,
                );
                (
                    tools,
                    diagnostics,
                    permission_manager,
                    permission_prompt_rx,
                    command,
                )
            }
        };
        // Register the built-in Reference Product tools as trusted
        // registrations with their fixed capabilities. Extension tool
        // contributions are excluded before registration because the product
        // defines no implicit extension permission, so an extension named
        // read/write/bash cannot acquire Builtin origin. The system-prompt
        // projection uses only the permitted trusted registrations (AUT-008).
        let registrations = crate::tool_authority::register_product_tools(tools);
        let tool_defs: Vec<_> = registrations.iter().map(|r| r.definition.clone()).collect();

        // Build the immutable digest-addressed effective user policy and the
        // trusted authorizer bound to it. The authorization decision derives only
        // from these facts + the capability + the current evidence health
        // snapshot (AUT-003/004); model content is never consulted for permission.
        let active_tool_names = tool_config.active_tool_names.clone();
        let mutating_allowed = active_tool_names
            .iter()
            .any(|n| matches!(n.as_str(), "write" | "edit" | "bash"));
        let command_execute_permission = crate::execution::permission::PermissionPolicy::from_map(
            config.execution.permissions.clone(),
        );
        let effective_policy = Arc::new(crate::tool_authority::EffectiveUserPolicy::build(
            build_options.execution_mode,
            active_tool_names,
            mutating_allowed,
            command_execute_permission,
            // Complete evidence is required iff capture is
            // configured (CLI --trace / SDK embedder / RPC recording). The
            // closed mapping adds no config key; absent capture is the no-op
            // Minimal Runtime (complete_evidence_required = false).
            build_options.evidence.is_some(),
            crate::tool_authority::digest_of(&format!("{:?}", build_options.trust_decision)),
            crate::tool_authority::digest_of(&format!("{:?}", build_options.installed_packages)),
            // Path/operation-scope fact: the workspace boundary anchor. This
            // policy does not encode a finer protected-path scope.
            workspace_scope_digest,
        ));
        // Capture the policy digest before it moves into the authorizer; the
        // evidence manifest addresses the effective policy by this digest.
        let evidence_policy_digest_hex = effective_policy.digest().to_owned();
        let runtime_input_policy_digest =
            opi_agent::evidence::ContentDigest::from_hex(evidence_policy_digest_hex.clone())
                .expect("effective policy digest is canonical SHA-256 hex");
        let runtime_input_source = build_options.runtime_input_source.clone();
        let authorizer = Arc::new(crate::tool_authority::ProductToolAuthorizer::new(
            effective_policy,
            Some(command_authorization),
        ));

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

        let model_for_capability_lookup = canonical_model_spec(provider.id(), &model);

        // Assemble the single dispatch plus model-lookup collection.
        // The Agent routes every model call through this one collection via
        // `prepare_call` (route + auth resolved once per turn); the same
        // collection also serves model listing/picker/resolution, so a
        // cross-provider model switch resolves through one collection without
        // reconstructing the Agent. Production supplies the `ProviderBundle`
        // resolver; when absent (mock-provider tests) a dummy static resolver is
        // used since the mock ignores auth.
        let auth_resolver: Arc<dyn opi_ai::auth::AuthResolver> =
            build_options.auth_resolver.unwrap_or_else(|| {
                Arc::new(opi_ai::auth::StaticAuthResolver::new(
                    opi_ai::auth::AuthScheme::ApiKey,
                    secrecy::SecretString::from("opi-mock-auth"),
                ))
            });
        let mut routes = Vec::with_capacity(build_options.extra_routes.len() + 1);
        routes.push((provider, auth_resolver));
        routes.extend(build_options.extra_routes);
        // The dispatchable provider ids are exactly the routes that
        // carry an auth resolver (active + extra); lookup-only extension
        // providers are not dispatchable. Captured before the collection build
        // consumes `routes`, so read-side legacy normalization can prove exactly
        // one dispatchable route for a bare model.
        let dispatchable_provider_ids: Vec<String> =
            routes.iter().map(|(p, _)| p.id().to_owned()).collect();
        let dispatch_collection = crate::provider_factory::build_harness_collection(
            routes,
            extension_providers,
            overrides,
            &active_provider_id,
        );

        let (thinking, max_tokens) = initial_thinking_request_config(
            &dispatch_collection,
            &model_for_capability_lookup,
            &config,
        );
        let inference = opi_agent::loop_types::InferenceConfig {
            thinking: thinking.unwrap_or_default(),
            max_tokens,
            temperature: None,
        };
        let agent_config = AgentLoopConfig {
            max_turns: config.defaults.max_iterations,
            retry: Some(config.retry.clone()),
        };

        let mut agent = Agent::new(
            dispatch_collection.clone(),
            registrations,
            Some(authorizer),
            model_for_capability_lookup.clone(),
            Some(system_prompt.clone()),
            inference,
            agent_config,
            hooks,
        )
        .expect("startup model selection must resolve to a dispatchable route");
        if let Some(registry) = extension_event_registry {
            agent.subscribe(Box::new(move |event| registry.dispatch_event(event)));
        }

        let initial_len = initial_messages.len();
        if !initial_messages.is_empty() {
            let mut initial_state = agent.state_snapshot();
            initial_state.context = initial_messages;
            agent
                .replace_state(initial_state)
                .expect("initial context replace must keep the resolved route");
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
        let initial_runtime_input = resolve_runtime_input(
            &runtime_input_source,
            &runtime_input_policy_digest,
            &agent,
            &config,
            &system_prompt,
            &dispatch_collection,
        );

        // Capture recorded model/thinking up front so `resume` can still move
        // into SessionCoordinator::open_existing below. Applied after the
        // harness is assembled.
        let recorded_model = resume.as_ref().and_then(|info| info.recorded_model.clone());
        let recorded_thinking = resume.as_ref().and_then(|info| info.recorded_thinking);

        let (session, session_resume_error) = if deferred_resume.is_some() {
            (None, None)
        } else if let Some(info) = resume {
            let path_display = info.path.display().to_string();
            match SessionCoordinator::open_existing(
                info.path,
                info.session_id.clone(),
                &info.entries,
                initial_len,
                compaction_config,
                model.clone(),
            ) {
                Ok(session) => (Some(session), None),
                Err(error) => (
                    None,
                    Some(format!(
                        "could not open requested session '{}' at {path_display}: {error}",
                        info.session_id
                    )),
                ),
            }
        } else {
            #[cfg(test)]
            let session_dir = session_dir_override
                .clone()
                .unwrap_or_else(crate::session_cli::session_dir);
            #[cfg(not(test))]
            let session_dir = crate::session_cli::session_dir();
            (
                SessionCoordinator::new(
                    &session_dir,
                    &cwd,
                    compaction_config,
                    model.clone(),
                    initial_runtime_input.binding.clone(),
                )
                .ok(),
                None,
            )
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

        // Assemble the evidence capture (recorder plus per-run binding
        // facts) and bind the recorder as the Agent's
        // EvidenceSink so the loop emits through it. Absent capture leaves the
        // sink unset (Minimal Runtime no-op).
        let evidence = build_options.evidence.map(|cfg| {
            let placeholder_digest = opi_agent::evidence::ContentDigest::from_hex(
                crate::tool_authority::digest_of("pending per-run evidence binding"),
            )
            .expect("digest_of returns canonical SHA-256 hex");
            let material_inputs = format!("{system_prompt}\n{model}\n{evidence_policy_digest_hex}");
            let mut capture = EvidenceCapture::new(
                cfg.recorder,
                cfg.source,
                opi_agent::evidence::ContentDigest::from_hex(evidence_policy_digest_hex)
                    .expect("effective policy digest is canonical SHA-256 hex"),
                opi_agent::evidence::ConfigIdentity {
                    harness_digest: placeholder_digest.clone(),
                    runtime_digest: placeholder_digest.clone(),
                    adapter_digest: placeholder_digest.clone(),
                    material_digest: placeholder_digest,
                },
                &material_inputs,
            );
            capture.rebind(
                initial_runtime_input.config.clone(),
                &initial_runtime_input.material_inputs,
                initial_runtime_input.system_digest.clone(),
                initial_runtime_input.tool_schema_digests.clone(),
                initial_runtime_input.budget,
            );
            // The recorder is also the Agent's evidence sink (EvidenceRecorder
            // is a sub-trait of EvidenceSink), so the loop emits through it.
            agent.set_evidence_sink(Some(capture.recorder.clone()));
            capture
        });

        let mut harness = Self {
            agent,
            armed_run: None,
            config,
            system_prompt,
            resources,
            model_registry: dispatch_collection,
            dispatchable_provider_ids,
            extension_registry: active_extension_registry,
            session,
            session_resume_error,
            turn_offset: initial_len,
            pending_images: Vec::new(),
            pending_extension_state: resume_extension_state,
            diagnostics,
            evidence,
            runtime_input_source,
            runtime_input_policy_digest,
            credential_store: None,
            oauth_registry: None,
            oauth_endpoints: OAuthEndpointConfig::production(),
            oauth_http_client: crate::oauth::production_oauth_client(),
            permission_manager,
            permission_prompt_rx,
            #[cfg(test)]
            session_dir_override,
        };

        if let Some(info) = deferred_resume {
            let adoption = if is_legacy_resume {
                harness.adopt_legacy_resume(info)
            } else {
                harness.fork_initial_resume(info)
            };
            if let Err(error) = adoption {
                harness.session_resume_error = Some(error);
            }
        } else {
            // Re-apply recorded model/thinking on the CLI --resume path
            // (and any other builder-driven resume), mirroring resume_session_id.
            // The diagnostic sink is already wired above so incompat warnings flow
            // through the same channel as the interactive path.
            harness.apply_recorded_model(recorded_model.as_deref());
            harness.apply_recorded_thinking(recorded_thinking);
            harness.sync_session_cost_model();
        }
        harness.sync_session_id();

        harness
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
        let current_provider = self.agent.provider_id();
        crate::picker::model_picker_items(self.model_registry.registry())
            .into_iter()
            .filter(|item| item.metadata == current_provider)
            .collect()
    }

    /// Change the model used by subsequent prompts.
    pub fn set_model(&mut self, model: String) {
        self.apply_agent_model(&model)
            .expect("model change must keep a dispatchable route");
        self.sync_session_cost_model();
    }

    fn apply_agent_model(&mut self, model: &str) -> Result<(), String> {
        let spec = canonical_model_spec(self.agent.provider_id(), model);
        let selection = opi_agent::loop_types::ModelSelection::parse_spec(&spec)
            .ok_or_else(|| format!("invalid provider:model selection '{spec}'"))?;
        let mut candidate = self.agent.state_snapshot();
        candidate.model_selection = selection;
        self.agent
            .replace_state(candidate)
            .map_err(|error| error.to_string())
    }

    fn replace_agent_context(&mut self, context: Vec<AgentMessage>) -> Result<(), String> {
        let mut candidate = self.agent.state_snapshot();
        candidate.context = context;
        self.agent
            .replace_state(candidate)
            .map_err(|error| error.to_string())
    }

    fn rewind_agent_context(&mut self, len: usize) -> Result<(), AgentError> {
        let mut candidate = self.agent.state_snapshot();
        candidate.context.truncate(len);
        self.agent.replace_state(candidate)
    }

    fn committed_prefix_is_preserved(
        run_messages: &[AgentMessage],
        pre_run_state: &opi_agent::loop_types::NextTurnState,
        offset: usize,
    ) -> bool {
        let Some(run_prefix) = run_messages.get(..offset) else {
            return false;
        };
        let Some(committed_prefix) = pre_run_state.context.get(..offset) else {
            return false;
        };
        match (
            serde_json::to_value(run_prefix),
            serde_json::to_value(committed_prefix),
        ) {
            (Ok(run_prefix), Ok(committed_prefix)) => run_prefix == committed_prefix,
            _ => false,
        }
    }

    fn reject_uncommitted_run(
        &mut self,
        result: &mut opi_agent::AgentRunResult,
        pre_run_state: opi_agent::loop_types::NextTurnState,
        offset: usize,
        failure: AgentError,
        evidence_detail: &str,
    ) -> AgentError {
        self.record_harness_diagnostic(Diagnostic::from(&failure));
        if let Err(reconciliation_error) = self.agent.replace_state(pre_run_state) {
            self.record_harness_diagnostic(Diagnostic::from(&reconciliation_error));
        }
        self.turn_offset = offset;
        if self.evidence.is_some() {
            let evidence_error = opi_agent::evidence::EvidenceError::Finalization {
                detail: evidence_detail.to_owned(),
            };
            let cleanup = result.abandon_evidence(&evidence_error);
            self.record_harness_diagnostic(Self::evidence_error_diagnostic(&evidence_error));
            if let Err(cleanup) = cleanup {
                self.record_harness_diagnostic(Diagnostic::from(
                    &AgentError::EvidenceFinalization(format!(
                        "evidence cleanup failed after session failure: {cleanup}"
                    )),
                ));
            }
        }
        failure
    }

    /// Validate and change the model used by subsequent prompts.
    ///
    /// On success the change is also persisted as a `model_change` entry on the
    /// active session branch, parented to the current content tip
    /// without advancing it. A later resume observes the recorded model and
    /// re-applies it when compatible with the CLI/config provider.
    pub fn set_model_validated(&mut self, model: String) -> Result<String, String> {
        // Validate and normalize in one step: a bare model id canonicalizes
        // only when exactly one dispatchable route serves it, so the durable
        // entry carries BOTH the canonical selection and the distinct
        // bare-source fact, and an unknown, ambiguous, or lookup-only
        // selection fails BEFORE anything is persisted.
        let canonical = self.try_configure_model(&model)?;
        let input_source = if model.contains(':') {
            ModelInputSource::Canonical
        } else {
            ModelInputSource::BareNormalized
        };
        if let Some(session) = self.session.as_mut() {
            session
                .append_model_change(canonical.clone(), input_source)
                .map_err(|e| format!("model change write failed: {e}"))?;
        }
        self.apply_agent_model(&canonical)?;
        self.sync_session_cost_model();
        Ok(self.agent.model_spec())
    }

    /// Validate that `model` is a known, dispatchable spec and compatible with
    /// the current thinking configuration, without persisting or mutating
    /// session state. Returns the canonical `provider:model` spec. Used by
    /// [`Self::set_model_validated`] (persists) and by resume (applies a
    /// recorded model without re-persisting the entry).
    ///
    /// A `provider:model` spec is accepted when it resolves to a registered
    /// route AND the provider holds a dispatchable route (a live auth
    /// resolver); dispatch still resolves the route at the next
    /// `prepare_call`. A bare model id normalizes only when exactly one
    /// dispatchable provider serves it — ambiguity or absence is a typed
    /// error, never a guess from the active provider.
    fn try_configure_model(&mut self, model: &str) -> Result<String, String> {
        let model_spec = if model.contains(':') {
            model.to_owned()
        } else {
            self.normalize_recorded_route(model)
                .map_err(|remediation| match remediation {
                    RouteRemediation::Ambiguous { candidates } => format!(
                        "bare model '{model}' matches more than one dispatchable route: {}",
                        candidates.join(", ")
                    ),
                    RouteRemediation::Missing => {
                        format!("bare model '{model}' matches no dispatchable route")
                    }
                })?
        };
        let (requested_provider, requested_model) =
            crate::provider_factory::parse_model_spec(&model_spec)?;

        let requested_model_info = self.model_info(&model_spec);
        let Some(requested_model_info) = requested_model_info else {
            return Err(format!(
                "unknown model '{requested_model}' for provider '{requested_provider}'"
            ));
        };
        self.model_registry
            .validate_dispatchable_route(&model_spec)
            .map_err(|e| e.to_string())?;

        self.validate_current_thinking_for_model(&requested_model_info)?;
        Ok(model_spec)
    }

    /// Change the thinking level used by subsequent provider requests.
    ///
    /// On success the change is also persisted as a `thinking_level_change`
    /// entry on the active session branch, parented to the current
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
        let mut candidate = self.agent.state_snapshot();
        candidate.inference.max_tokens = change.max_tokens;
        candidate.inference.thinking = change.thinking.unwrap_or_default();
        self.agent
            .replace_state(candidate)
            .expect("thinking change must preserve the dispatchable route");
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

    /// Set the session name (`/name <name>`). Persists a
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

    /// Add a label to the active branch (`/label <label>`). Persists
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

    /// Remove a label from the active branch (`/unlabel <label>`).
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

    /// Aggregate the live session metadata surfaced by
    /// `/session info` and RPC `session_info`: name, labels, active branch,
    /// model, and thinking config. Returns `None` when no session is active.
    pub fn session_metadata(&self) -> Option<SessionMetadata> {
        let session = self.session.as_ref()?;
        Some(SessionMetadata {
            name: session.name().map(str::to_owned),
            labels: session.labels().to_vec(),
            active_branch: session.active_branch_id().map(str::to_owned),
            model: self.agent.model_spec(),
            thinking: self.agent.thinking_config(),
        })
    }

    fn active_model_info(&self) -> Option<ModelInfo> {
        // The active selection is always a canonical `provider:model` spec; the
        // dispatch collection resolves it regardless of which provider is active.
        self.model_info(&self.agent.model_spec())
    }

    fn sync_session_cost_model(&mut self) {
        let model_spec = self.agent.model_spec();
        let pricing = self.active_model_info().and_then(|model| model.pricing);
        if let Some(session) = self.session.as_mut() {
            session.set_cost_model(model_spec, pricing);
        }
    }

    /// Resolve a canonical `provider:model` spec against the dispatch collection.
    fn model_info(&self, spec: &str) -> Option<ModelInfo> {
        self.model_registry
            .resolve(spec)
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

    /// Adopt a genuine v1 source through one immutable, parented v2 child.
    /// The source is read-only: route normalization and exact binding assembly
    /// happen before the child is allocated, so ambiguity, missing routes, and
    /// recovery damage fail before provider or tool dispatch.
    fn adopt_legacy_resume(&mut self, info: ResumeInfo) -> Result<(), String> {
        let (header, _, recovery) =
            opi_agent::session::SessionReader::read_with_recovery(&info.path).map_err(|error| {
                format!(
                    "could not read legacy session '{}': {error}",
                    info.session_id
                )
            })?;
        if header.version != opi_agent::session::LEGACY_FORMAT_VERSION {
            return Err(format!(
                "session '{}' is not a legacy v1 source",
                info.session_id
            ));
        }
        if !recovery.is_clean() {
            return Err(format!(
                "legacy session '{}' requires clean recovery before migration",
                info.session_id
            ));
        }
        let recorded_model = info.recorded_model.as_deref().ok_or_else(|| {
            format!(
                "legacy session '{}' has no recorded route to normalize",
                info.session_id
            )
        })?;
        let canonical = match self.normalize_recorded_route(recorded_model) {
            Ok(canonical) => canonical,
            Err(RouteRemediation::Ambiguous { candidates }) => {
                self.record_harness_diagnostic(
                    Diagnostic::new(
                        Severity::Warning,
                        CODE_SESSION_RESUME_ROUTE_AMBIGUOUS,
                        SOURCE_SESSION,
                        "legacy bare model matches more than one dispatchable route",
                    )
                    .details(serde_json::json!({
                        "recorded_model": recorded_model,
                        "candidates": candidates,
                    })),
                );
                return Err(format!(
                    "legacy session route is ambiguous for model '{recorded_model}'"
                ));
            }
            Err(RouteRemediation::Missing) => {
                self.record_harness_diagnostic(
                    Diagnostic::new(
                        Severity::Warning,
                        CODE_SESSION_RESUME_ROUTE_MISSING,
                        SOURCE_SESSION,
                        "legacy bare model matches no dispatchable route",
                    )
                    .details(serde_json::json!({ "recorded_model": recorded_model })),
                );
                return Err(format!(
                    "legacy session route is missing for model '{recorded_model}'"
                ));
            }
        };
        self.try_configure_model(&canonical)?;
        self.apply_agent_model(&canonical)?;
        self.apply_recorded_thinking(info.recorded_thinking);

        let runtime_input = resolve_runtime_input(
            &self.runtime_input_source,
            &self.runtime_input_policy_digest,
            &self.agent,
            &self.config,
            &self.system_prompt,
            &self.model_registry,
        );
        let dir = info.path.parent().ok_or_else(|| {
            format!(
                "legacy session '{}' has no parent directory",
                info.session_id
            )
        })?;
        let forked = crate::session_cli::fork_session_with_runtime_input_binding(
            dir,
            &info.session_id,
            runtime_input.binding,
        )
        .map_err(|error| {
            format!(
                "failed to migrate legacy session '{}': {error}",
                info.session_id
            )
        })?;
        self.adopt_session_entries(
            &forked.entries,
            &forked.recovery,
            forked.path,
            forked.header.id,
            "failed to adopt migrated legacy session",
            true,
        )?;
        self.sync_session_id();
        Ok(())
    }

    /// Complete a requested v2 startup fork only after product assembly has
    /// resolved the binding for the current startup inputs.
    fn fork_initial_resume(&mut self, info: ResumeInfo) -> Result<(), String> {
        let runtime_input = resolve_runtime_input(
            &self.runtime_input_source,
            &self.runtime_input_policy_digest,
            &self.agent,
            &self.config,
            &self.system_prompt,
            &self.model_registry,
        );
        let dir = info
            .path
            .parent()
            .ok_or_else(|| format!("session '{}' has no parent directory", info.session_id))?;
        let forked = crate::session_cli::fork_session_with_runtime_input_binding(
            dir,
            &info.session_id,
            runtime_input.binding,
        )
        .map_err(|error| {
            format!(
                "failed to fork requested session '{}': {error}",
                info.session_id
            )
        })?;
        self.adopt_session_entries(
            &forked.entries,
            &forked.recovery,
            forked.path,
            forked.header.id,
            "failed to open startup fork",
            true,
        )?;
        self.sync_session_id();
        Ok(())
    }

    /// Resume an existing session by ID into this harness.
    ///
    /// Reconstructs the active-branch context through the opi-agent context API:
    /// messages drive the agent buffer; the latest
    /// recorded `model_change` and `thinking_level_change` on the active chain
    /// are re-applied when compatible with the CLI/config provider selection,
    /// and a diagnostic is emitted (without aborting the resume) when
    /// they are not. Missing-parent warnings from the context builder are
    /// surfaced alongside the load-time recovery diagnostics.
    pub fn resume_session_id(&mut self, session_id: &str) -> Result<usize, String> {
        // An allow-for-session grant must not survive a session
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

        if session.header.version == opi_agent::session::LEGACY_FORMAT_VERSION {
            let ctx = reconstruct_context(&session.entries, &session.recovery);
            self.adopt_legacy_resume(ResumeInfo {
                path: session.path,
                session_id: session.header.id,
                entries: session.entries,
                original_cwd: PathBuf::from(session.header.cwd),
                diagnostics: ctx.diagnostics,
                recorded_model: ctx.model,
                recorded_thinking: ctx.thinking_level,
            })?;
            return Ok(self.turn_offset);
        }

        let (path, loaded_session_id) = (session.path, session.header.id);
        let message_count = self.adopt_session_entries(
            &session.entries,
            &session.recovery,
            path,
            loaded_session_id,
            "failed to reopen resumed session",
            true,
        )?;
        self.sync_session_id();
        Ok(message_count)
    }

    /// Adopt a loaded session's entries into this harness: reconstruct the
    /// agent context through the opi-agent context API, re-apply the recorded
    /// model/thinking metadata, surface reconstruction diagnostics, and reopen
    /// the session coordinator for the adopted entries. Shared by session
    /// resume, fork, and branch adoption. `error_context` names the caller in
    /// the reopen failure; `clear_resume_error` resets a stale startup resume
    /// error only where adoption replaces the whole session identity.
    fn adopt_session_entries(
        &mut self,
        entries: &[opi_agent::session::SessionEntry],
        recovery: &opi_agent::session::CrashRecovery,
        path: PathBuf,
        session_id: String,
        error_context: &str,
        clear_resume_error: bool,
    ) -> Result<usize, String> {
        // Build the agent buffer and metadata view through the
        // reusable opi-agent context API instead of the product-only walker.
        let ctx = reconstruct_context(entries, recovery);
        let message_count = ctx.messages.len();
        self.replace_agent_context(ctx.messages)?;
        self.defer_extension_state_from_entries(entries);

        // Apply recorded model/thinking metadata (latest-wins on the active
        // chain). Each branch keeps the CLI/config selection when the recorded
        // value is incompatible and emits a diagnostic instead.
        self.apply_recorded_model(ctx.model.as_deref());
        self.apply_recorded_thinking(ctx.thinking_level);

        // Surface recovery + missing-parent diagnostics. `reconstruct_context`
        // already forwards load-time recovery diagnostics, so do not append
        // loaded-session diagnostics separately.
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
                entries,
                message_count,
                compaction_config,
                self.agent.model().to_string(),
            )
            .map_err(|error| format!("{error_context}: {error}"))?,
        );
        if clear_resume_error {
            self.session_resume_error = None;
        }
        self.sync_session_cost_model();
        self.turn_offset = message_count;
        Ok(message_count)
    }

    /// Re-apply a recorded `model_change` model spec on resume. Configures the
    /// agent in place without persisting a new entry (the entry is already in
    /// the source session). Emits a diagnostic and keeps the CLI/config
    /// model when the recorded spec is incompatible.
    ///
    /// A recorded spec is first normalized against the dispatchable collection.
    /// An exact `provider:model` route is accepted; a
    /// legacy bare model normalizes to a canonical route only when exactly one
    /// dispatchable provider serves it. Ambiguity or absence returns typed
    /// remediation (distinct codes, no provider dispatch) and keeps the
    /// CLI/config model — the session loads but the wrong route is never
    /// guessed from the active provider.
    fn apply_recorded_model(&mut self, recorded: Option<&str>) {
        let Some(spec) = recorded else {
            return;
        };
        let normalized = match self.normalize_recorded_route(spec) {
            Ok(canonical) => canonical,
            Err(RouteRemediation::Ambiguous { candidates }) => {
                self.record_harness_diagnostic(
                    Diagnostic::new(
                        Severity::Warning,
                        CODE_SESSION_RESUME_ROUTE_AMBIGUOUS,
                        SOURCE_SESSION,
                        "recorded bare model matches more than one dispatchable route; keeping CLI/config model",
                    )
                    .details(serde_json::json!({
                        "recorded_model": spec,
                        "active_model": self.agent.model(),
                        "candidates": candidates,
                    })),
                );
                return;
            }
            Err(RouteRemediation::Missing) => {
                self.record_harness_diagnostic(
                    Diagnostic::new(
                        Severity::Warning,
                        CODE_SESSION_RESUME_ROUTE_MISSING,
                        SOURCE_SESSION,
                        "recorded bare model matches no dispatchable route; keeping CLI/config model",
                    )
                    .details(serde_json::json!({
                        "recorded_model": spec,
                        "active_model": self.agent.model(),
                    })),
                );
                return;
            }
        };
        if let Err(reason) = self.try_configure_model(&normalized) {
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
        if let Err(reason) = self.apply_agent_model(&normalized) {
            // Unreachable for every validated input today (validation covers
            // registry resolution, dispatchability, and thinking
            // compatibility), but a resume must fail visibly with the same
            // typed diagnostic rather than panic if application and
            // validation ever diverge.
            self.record_harness_diagnostic(
                Diagnostic::new(
                    Severity::Warning,
                    CODE_SESSION_RESUME_MODEL_INCOMPATIBLE,
                    SOURCE_SESSION,
                    "recorded model_change could not be applied to the active runtime; keeping CLI/config model",
                )
                .details(serde_json::json!({
                    "recorded_model": spec,
                    "active_model": self.agent.model(),
                    "reason": reason,
                })),
            );
        }
    }

    /// Normalize a recorded model spec against the dispatchable collection. An
    /// exact `provider:model`
    /// spec is accepted unchanged; a legacy bare model normalizes to a canonical
    /// route ONLY when exactly one dispatchable provider serves it. Ambiguity or
    /// absence returns typed remediation rather than guessing the active
    /// provider. Resolution is lookup-only — no provider is dispatched.
    fn normalize_recorded_route(&self, recorded: &str) -> Result<String, RouteRemediation> {
        if recorded.contains(':') {
            return Ok(recorded.to_owned());
        }
        let matches: Vec<String> = self
            .dispatchable_provider_ids
            .iter()
            .filter(|pid| self.model_info(&format!("{pid}:{recorded}")).is_some())
            .map(|pid| format!("{pid}:{recorded}"))
            .collect();
        match matches.len() {
            1 => Ok(matches.into_iter().next().expect("exactly one match")),
            0 => Err(RouteRemediation::Missing),
            _ => Err(RouteRemediation::Ambiguous {
                candidates: matches,
            }),
        }
    }

    /// Re-apply a recorded `thinking_level_change` on resume. Configures the
    /// agent in place without persisting a new entry. Emits a
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
        // Grants do not survive a fork boundary.
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
        // Adopt the forked chain's entries (recorded route re-applied:
        // canonical accepted; legacy bare normalized against the dispatchable
        // collection or kept fail-closed with typed remediation).
        let session_id = forked.header.id;
        let message_count = self.adopt_session_entries(
            &forked.entries,
            &forked.recovery,
            forked.path,
            session_id.clone(),
            "failed to open forked session",
            false,
        )?;
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
        // Grants do not survive a branch switch.
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
        // Adopt the selected branch's entries (recorded route re-applied:
        // canonical accepted; legacy bare normalized against the dispatchable
        // collection or kept fail-closed with typed remediation). The session
        // identity is unchanged, so no session-id resync is needed.
        let message_count = self.adopt_session_entries(
            &entries,
            &recovery,
            path,
            session_id,
            "failed to reopen selected branch",
            false,
        )?;
        Ok(message_count)
    }

    /// Send a user prompt and run the agent loop.
    pub async fn prompt(&mut self, text: &str) -> Result<Vec<AgentMessage>, AgentError> {
        let run = self.take_or_arm_run();
        self.ensure_session_resume_ready()?;
        self.restore_pending_extension_state().await;
        // C5: discard any unpersisted failed-turn user message before starting
        // a fresh turn so it is not absorbed into this turn's persistence slice.
        // (retry_last_prompt intentionally does NOT rewind — it reuses the
        // failed-turn user message after an interactive login.)
        self.rewind_agent_context(self.turn_offset)?;
        self.setup_evidence_run()?;
        let offset = self.turn_offset;
        let pre_run_state = self.agent.state_snapshot();
        let result = self.agent.prompt_armed(text, run).await;
        self.complete_agent_run(
            result,
            opi_agent::evidence::ExecutionTrigger::Invocation,
            text,
            offset,
            pre_run_state,
        )
        .await
    }

    /// Send a user message with arbitrary content (text + images) and run the
    /// agent loop.
    pub async fn prompt_with_content(
        &mut self,
        content: Vec<opi_ai::message::InputContent>,
    ) -> Result<Vec<AgentMessage>, AgentError> {
        let run = self.take_or_arm_run();
        self.ensure_session_resume_ready()?;
        self.restore_pending_extension_state().await;
        // C5: discard any unpersisted failed-turn user message before starting a
        // fresh turn (see `prompt`).
        self.rewind_agent_context(self.turn_offset)?;
        self.setup_evidence_run()?;
        let offset = self.turn_offset;
        let pre_run_state = self.agent.state_snapshot();
        let prompt_text = Self::render_input_content(&content);
        let result = self.agent.prompt_with_content_armed(content, run).await;
        self.complete_agent_run(
            result,
            opi_agent::evidence::ExecutionTrigger::Invocation,
            &prompt_text,
            offset,
            pre_run_state,
        )
        .await
    }

    /// Retry the agent loop with the current messages (no new user message),
    /// used after a `CredentialNeeded` error is resolved via interactive login.
    /// The user message from the original `prompt`/`continue_` call is already
    /// in the agent's message list, so re-prompting would duplicate it.
    pub async fn retry_last_prompt(&mut self) -> Result<Vec<AgentMessage>, AgentError> {
        let run = self.take_or_arm_run();
        self.ensure_session_resume_ready()?;
        self.restore_pending_extension_state().await;
        self.setup_evidence_run()?;
        let offset = self.turn_offset;
        let pre_run_state = self.agent.state_snapshot();
        let prompt_text = Self::last_user_prompt_text(&self.agent.messages_snapshot());
        let result = self.agent.retry_last_turn_armed(run).await;
        self.complete_agent_run(
            result,
            opi_agent::evidence::ExecutionTrigger::Retry,
            &prompt_text,
            offset,
            pre_run_state,
        )
        .await
    }

    /// Continue the conversation with an additional message.
    pub async fn continue_(&mut self, text: &str) -> Result<Vec<AgentMessage>, AgentError> {
        let run = self.take_or_arm_run();
        self.ensure_session_resume_ready()?;
        self.restore_pending_extension_state().await;
        // A continuation is also a fresh entry. Discard any unpersisted failed
        // prompt/retry before appending its new user message.
        self.rewind_agent_context(self.turn_offset)?;
        self.setup_evidence_run()?;
        let offset = self.turn_offset;
        let pre_run_state = self.agent.state_snapshot();
        let result = self.agent.continue_armed(text, run).await;
        self.complete_agent_run(
            result,
            opi_agent::evidence::ExecutionTrigger::Continuation,
            text,
            offset,
            pre_run_state,
        )
        .await
    }

    /// Complete one prompt/retry/continuation as an ordered run transaction.
    /// A successful execution persists and commits the live boundary before
    /// evidence publication. Lifecycle failures remain secondary to an owning
    /// execution error and are recorded separately by `finalize_evidence_run`.
    async fn complete_agent_run(
        &mut self,
        mut result: opi_agent::AgentRunResult,
        trigger: opi_agent::evidence::ExecutionTrigger,
        prompt_text: &str,
        offset: usize,
        pre_run_state: opi_agent::loop_types::NextTurnState,
    ) -> Result<Vec<AgentMessage>, AgentError> {
        let run_messages = result.messages().to_vec();
        let committed_prefix_preserved =
            Self::committed_prefix_is_preserved(&run_messages, &pre_run_state, offset);
        if self.session.is_some() && !committed_prefix_preserved {
            let failure = AgentError::SessionPersist(
                "prepared next-turn context did not preserve the committed session prefix"
                    .to_owned(),
            );
            let execution_failed = result.error().is_some();
            let failure = self.reject_uncommitted_run(
                &mut result,
                pre_run_state,
                offset,
                failure,
                "session prefix validation failed before the run boundary committed",
            );
            if execution_failed {
                return match result.into_execution_result() {
                    Err(error) => Err(error),
                    Ok(_) => Err(failure),
                };
            }
            return Err(failure);
        }
        let new_messages = if committed_prefix_preserved {
            &run_messages[offset..]
        } else {
            &run_messages
        };
        let execution_succeeded = result.error().is_none();
        let committed_messages = if execution_succeeded {
            if let Err(persistence_error) =
                self.persist_turn(&mut result, new_messages, offset).await
            {
                return Err(self.reject_uncommitted_run(
                    &mut result,
                    pre_run_state,
                    offset,
                    persistence_error,
                    "session persistence failed before the run boundary committed",
                ));
            }
            let committed = self.current_messages();
            // Commit the live/persisted projection before evidence finalization
            // so a publication failure cannot rewind durable turn bytes later.
            self.turn_offset = committed.len();
            Some(committed)
        } else {
            None
        };

        let evidence_result =
            self.finalize_evidence_run(&mut result, trigger, new_messages, prompt_text);
        let execution_result = result.into_execution_result();
        match execution_result {
            Err(error) => Err(error),
            Ok(_) => {
                evidence_result?;
                Ok(committed_messages.expect("successful execution committed its live boundary"))
            }
        }
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
    /// provider calls no longer carry the compacted history. The run's typed
    /// compaction start must complete before that mutation; its matching
    /// terminal records the actual result. Public `CompactionStart`/
    /// `CompactionEnd` events mirror, but do not replace, that lifecycle.
    async fn persist_turn(
        &mut self,
        run: &mut opi_agent::AgentRunResult,
        messages: &[AgentMessage],
        turn_start_agent_index: usize,
    ) -> Result<(), AgentError> {
        if let Some(session) = &mut self.session {
            let usage = Self::aggregate_turn_usage(messages);
            let compaction_reason =
                match session.on_turn_end(messages, &usage, turn_start_agent_index) {
                    Ok(reason) => reason,
                    Err(e) => {
                        self.agent.emit_event(AgentEvent::SessionPersistError {
                            message: format!("session write failed: {e}"),
                        });
                        return Err(AgentError::SessionPersist(e.to_string()));
                    }
                };

            if let Some(reason) = compaction_reason {
                match run.begin_compaction(compaction_trigger(reason)) {
                    Ok(pending) => {
                        self.agent
                            .emit_event(AgentEvent::CompactionStart { reason });
                        let (outcome, event, session_error) = match session
                            .execute_compaction(reason)
                        {
                            Ok(Some(out)) => {
                                let wire = to_wire_result(&out);
                                self.record_harness_diagnostic(out.diagnostic.clone());
                                let mut candidate = self.agent.state_snapshot();
                                candidate.context = out.new_agent_messages;
                                self.agent
                                    .replace_state(candidate)
                                    .expect("compaction must preserve the dispatchable route");
                                (
                                    opi_agent::evidence::CompactionOutcome::Succeeded,
                                    AgentEvent::CompactionEnd {
                                        reason,
                                        result: Some(wire),
                                        aborted: false,
                                        error_message: None,
                                    },
                                    None,
                                )
                            }
                            Ok(None) => (
                                opi_agent::evidence::CompactionOutcome::Aborted,
                                AgentEvent::CompactionEnd {
                                    reason,
                                    result: None,
                                    aborted: true,
                                    error_message: Some("compaction produced no output".to_owned()),
                                },
                                None,
                            ),
                            Err(error) => {
                                let outcome = if error
                                    .get_ref()
                                    .and_then(|source| {
                                        source.downcast_ref::<SessionBatchRollbackError>()
                                    })
                                    .is_some()
                                {
                                    opi_agent::evidence::CompactionOutcome::CleanupUnknown
                                } else {
                                    opi_agent::evidence::CompactionOutcome::Failed
                                };
                                let message = format!("compaction write failed: {error}");
                                (
                                    outcome,
                                    AgentEvent::CompactionEnd {
                                        reason,
                                        result: None,
                                        aborted: true,
                                        error_message: Some(format!(
                                            "compaction persist failed: {error}"
                                        )),
                                    },
                                    Some(AgentEvent::SessionPersistError { message }),
                                )
                            }
                        };
                        let terminal_emission_failed = if let Err(error) =
                            run.finish_compaction(&pending, outcome)
                        {
                            self.record_harness_diagnostic(Self::evidence_error_diagnostic(&error));
                            true
                        } else {
                            false
                        };
                        self.agent.emit_event(event);
                        if let Some(session_error) = session_error {
                            self.agent.emit_event(session_error);
                        }
                        if terminal_emission_failed {
                            // The actual compaction result is already retained.
                            // Stop before any unrelated post-compaction session
                            // mutation; finalization owns the one abandonment.
                            return Ok(());
                        }
                    }
                    Err(error) => {
                        // No token means the typed start did not complete, so
                        // no later session/context mutation or public lifecycle
                        // is emitted. Finalization performs the single explicit
                        // abandonment before the caller observes the failure.
                        self.record_harness_diagnostic(Self::evidence_error_diagnostic(&error));
                        return Ok(());
                    }
                }
            }
        }
        self.persist_extension_state().await;
        Ok(())
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
        self.agent.messages_snapshot()
    }

    // -- Per-run setup: diagnostic reset + evidence capture -----------------

    /// Reset the per-run diagnostic buffer so run-summary counts reflect only
    /// the current run (RPC shares one harness and one recording sink across
    /// prompt/continue runs in a session). No-op when no recording sink is
    /// attached.
    fn clear_run_diagnostics(&self) {
        if let Some(sink) = &self.diagnostics {
            sink.clear();
        }
    }

    fn ensure_session_resume_ready(&self) -> Result<(), AgentError> {
        match &self.session_resume_error {
            Some(error) => Err(AgentError::SessionResume(error.clone())),
            None => Ok(()),
        }
    }

    /// Ensure the active mutable branch carries the exact binding resolved for
    /// this external run. A mismatch creates a parented v2 child before any
    /// evidence setup, provider dispatch, or tool side effect.
    fn ensure_session_runtime_input_binding(
        &mut self,
        binding: &opi_agent::evidence::RuntimeInputBinding,
    ) -> Result<(), AgentError> {
        let needs_child = self
            .session
            .as_ref()
            .is_some_and(|session| session.runtime_input_binding() != binding);
        if !needs_child {
            return Ok(());
        }

        let (source_path, source_session_id) = {
            let session = self
                .session
                .as_ref()
                .expect("session was present when binding mismatch was observed");
            (
                session.session_path().to_path_buf(),
                session.session_id().to_owned(),
            )
        };
        let forked = crate::session_cli::fork_session_path_with_runtime_input_binding(
            &source_path,
            binding.clone(),
        )
        .map_err(|error| {
            AgentError::SessionResume(format!(
                "failed to create a bound child for session '{source_session_id}': {error}"
            ))
        })?;
        let model = self.agent.model_spec();
        let message_count = self.agent.messages_snapshot().len();
        let coordinator = SessionCoordinator::open_existing(
            forked.path,
            forked.header.id,
            &forked.entries,
            message_count,
            opi_agent::compaction::CompactionConfig {
                enabled: self.config.compaction.enabled,
                threshold_tokens: self.config.compaction.threshold_tokens,
            },
            model,
        )
        .map_err(|error| {
            AgentError::SessionResume(format!("failed to adopt bound child session: {error}"))
        })?;
        self.session = Some(coordinator);
        self.sync_session_cost_model();
        self.sync_session_id();
        Ok(())
    }

    /// Prepare the evidence sink before the run (fail-closed). A setup failure
    /// aborts the run as `AgentError::EvidenceSetup` before its first
    /// provider/tool call so the run never starts with unprepared capture.
    /// No-op when capture is not configured.
    fn setup_evidence_run(&mut self) -> Result<(), AgentError> {
        self.clear_run_diagnostics();
        let runtime_input = resolve_runtime_input(
            &self.runtime_input_source,
            &self.runtime_input_policy_digest,
            &self.agent,
            &self.config,
            &self.system_prompt,
            &self.model_registry,
        );
        self.ensure_session_runtime_input_binding(&runtime_input.binding)?;
        if let Some(capture) = &mut self.evidence {
            capture.rebind(
                runtime_input.config,
                &runtime_input.material_inputs,
                runtime_input.system_digest,
                runtime_input.tool_schema_digests,
                runtime_input.budget,
            );
            debug_assert_eq!(capture.binding, runtime_input.binding);
            capture
                .recorder
                .setup(&capture.binding)
                .map_err(|e| AgentError::EvidenceSetup(e.to_string()))?;
        }
        Ok(())
    }

    /// Finalize the evidence run after the loop returns. Assembles the strict
    /// manifest from the recorder's ordered records (call-graph correlation +
    /// route) plus the run's terminal `outcome` and finalizes it through the
    /// sink. If any lifecycle phase failed, or no provider call emitted
    /// evidence, the manifest is withheld and provisional sink state is
    /// explicitly abandoned, except that a typed cancellation record can
    /// truthfully finalize a run cancelled before any provider call. The actual
    /// execution outcome is retained in the failure detail. `prompt_text`
    /// addresses the prompt digest.
    fn finalize_evidence_run(
        &mut self,
        run: &mut opi_agent::AgentRunResult,
        trigger: opi_agent::evidence::ExecutionTrigger,
        messages: &[AgentMessage],
        prompt_text: &str,
    ) -> Result<(), AgentError> {
        let Some(capture) = self.evidence.as_ref() else {
            return Ok(());
        };
        let recorder = capture.recorder.clone();
        let manifest = (|| -> Result<_, opi_agent::evidence::EvidenceError> {
            // A failed setup/emission phase withholds the manifest and requires
            // explicit sink abandonment.
            if recorder.has_failure() {
                return Err(opi_agent::evidence::EvidenceError::Finalization {
                    detail: "evidence recorder became incomplete before finalization".to_owned(),
                });
            }
            let records = recorder.records();
            if records.is_empty() {
                // No provider call emitted evidence: there is no graph to
                // finalize, but setup still created provisional sink state.
                return Err(opi_agent::evidence::EvidenceError::Finalization {
                    detail: "evidence recorder produced no records".to_owned(),
                });
            }
            let (input_tokens, output_tokens) = Self::reported_token_usage(messages);
            let usage = usage_facts(input_tokens, output_tokens);
            let session = match self
                .session
                .as_ref()
                .and_then(SessionCoordinator::active_branch_id)
            {
                Some(tip) => opi_agent::evidence::SessionBinding::branch(tip.to_owned()).map_err(
                    |error| opi_agent::evidence::EvidenceError::Finalization {
                        detail: error.to_string(),
                    },
                )?,
                None => opi_agent::evidence::SessionBinding::NoSession,
            };
            let prompt_digest = opi_agent::evidence::ContentDigest::from_hex(
                crate::tool_authority::digest_of(prompt_text),
            )
            .expect("digest_of returns canonical SHA-256 hex");
            let dynamic = RunDynamicFacts {
                outcome: run.terminal_outcome().clone(),
                usage,
                session,
                prompt_digest,
                trigger,
            };
            build_finalized_manifest(capture, &records, dynamic)
        })();

        let manifest = match manifest {
            Ok(manifest) => manifest,
            Err(error) => {
                let cleanup = run.abandon_evidence(&error);
                let owning = AgentError::EvidenceFinalization(error.to_string());
                self.record_harness_diagnostic(Diagnostic::from(&owning));
                if let Err(cleanup) = cleanup {
                    self.record_harness_diagnostic(Diagnostic::from(
                        &AgentError::EvidenceFinalization(format!(
                            "evidence cleanup failed after finalization failure: {cleanup}"
                        )),
                    ));
                }
                return Err(owning);
            }
        };
        // A finalization failure marks the run incomplete (manifest withheld);
        // the run's actual outcome is already preserved.
        run.finalize_evidence(&manifest).map_err(|error| {
            let owning = AgentError::EvidenceFinalization(error.to_string());
            self.record_harness_diagnostic(Diagnostic::from(&owning));
            if let Some(cleanup) = run.evidence_cleanup_error() {
                self.record_harness_diagnostic(Diagnostic::from(
                    &AgentError::EvidenceFinalization(format!(
                        "evidence cleanup failed after finalization failure: {cleanup}"
                    )),
                ));
            }
            owning
        })
    }

    fn finalize_standalone_compaction_evidence(
        &self,
        outcome: opi_agent::evidence::TerminalOutcome,
        reason: opi_agent::session_event::CompactionReason,
        prompt_text: &str,
    ) -> Result<(), opi_agent::evidence::EvidenceError> {
        let Some(capture) = self.evidence.as_ref() else {
            return Ok(());
        };
        let records = capture.recorder.records();
        let session =
            match self
                .session
                .as_ref()
                .and_then(SessionCoordinator::active_branch_id)
            {
                Some(tip) => opi_agent::evidence::SessionBinding::branch(tip.to_owned()).map_err(
                    |error| opi_agent::evidence::EvidenceError::Finalization {
                        detail: error.to_string(),
                    },
                )?,
                None => opi_agent::evidence::SessionBinding::NoSession,
            };
        let dynamic = RunDynamicFacts {
            outcome,
            usage: usage_facts(None, None),
            session,
            prompt_digest: opi_agent::evidence::ContentDigest::from_hex(
                crate::tool_authority::digest_of(prompt_text),
            )
            .expect("digest_of returns canonical SHA-256 hex"),
            trigger: opi_agent::evidence::ExecutionTrigger::Compaction {
                reason: compaction_trigger(reason),
            },
        };
        let manifest = build_finalized_manifest(capture, &records, dynamic)?;
        capture.recorder.finalize_run(&manifest)
    }

    fn complete_standalone_compaction_evidence(
        &mut self,
        reason: opi_agent::session_event::CompactionReason,
        compaction_outcome: opi_agent::evidence::CompactionOutcome,
        terminal_outcome: opi_agent::evidence::TerminalOutcome,
        prompt_text: &str,
    ) -> Result<(), opi_agent::evidence::EvidenceError> {
        let result = self
            .emit_manual_compaction_evidence(reason, compaction_outcome)
            .and_then(|()| {
                self.finalize_standalone_compaction_evidence(
                    terminal_outcome.clone(),
                    reason,
                    prompt_text,
                )
            });
        let Err(owning) = result else {
            return Ok(());
        };

        let recorder = self
            .evidence
            .as_ref()
            .map(|capture| capture.recorder.clone());
        if let Some(recorder) = recorder
            && let Err(cleanup) = recorder.abandon_run(&terminal_outcome)
        {
            self.record_harness_diagnostic(Diagnostic::from(&AgentError::EvidenceFinalization(
                format!("evidence cleanup failed after standalone compaction failure: {cleanup}"),
            )));
        }
        Err(owning)
    }

    /// Render user input content to a stable prompt-identity string (text parts
    /// joined; images addressed by a placeholder). This addresses the
    /// runtime-input binding digest for content-based prompts without embedding
    /// raw image data.
    fn render_input_content(content: &[opi_ai::message::InputContent]) -> String {
        content
            .iter()
            .map(|c| match c {
                opi_ai::message::InputContent::Text { text } => text.clone(),
                opi_ai::message::InputContent::Image { .. } => "[image]".to_owned(),
                _ => "[unknown]".to_owned(),
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The last user message's rendered content, used to address the
    /// runtime-input binding digest when retrying a prior prompt (no new user
    /// text is supplied).
    fn last_user_prompt_text(messages: &[AgentMessage]) -> String {
        messages
            .iter()
            .rev()
            .find_map(|m| match m {
                AgentMessage::Llm(opi_ai::message::Message::User(u)) => {
                    Some(Self::render_input_content(&u.content))
                }
                _ => None,
            })
            .unwrap_or_default()
    }

    /// Aggregate provider-reported token usage across a turn's assistant
    /// messages. Returns `None` for input/output when no message reported usage,
    /// so an unknown measurement stays distinct from a measured zero.
    fn reported_token_usage(messages: &[AgentMessage]) -> (Option<u64>, Option<u64>) {
        let mut input: u64 = 0;
        let mut output: u64 = 0;
        let mut any_reported = false;
        for m in messages {
            if let AgentMessage::Llm(Message::Assistant(a)) = m
                && a.usage.is_reported()
            {
                any_reported = true;
                input = input.saturating_add(a.usage.input_tokens as u64);
                output = output.saturating_add(a.usage.output_tokens as u64);
            }
        }
        if any_reported {
            (Some(input), Some(output))
        } else {
            (None, None)
        }
    }

    fn record_harness_diagnostic(&self, diagnostic: Diagnostic) {
        if let Some(sink) = &self.diagnostics {
            sink.record(diagnostic.clone());
        }
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

    /// Return the canonical `provider:model` spec for the active selection. Use
    /// this for any persisted or reported surface (session
    /// summary, RPC responses, session metadata); [`CodingHarness::model`]
    /// returns only the bare model-id half and mirrors [`Agent::model`].
    pub fn model_spec(&self) -> String {
        self.agent.model_spec()
    }

    /// Return the current thinking configuration, including a resumed
    /// `thinking_level_change` when one was applied.
    pub fn thinking_config(&self) -> ThinkingConfig {
        self.agent.thinking_config()
    }

    /// Return the diagnostics recorded during the run, when diagnostic
    /// recording is enabled. Includes resume-emitted warnings such as
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

    /// Arm the next run and return its cancellation token.
    pub fn cancel_token(&mut self) -> tokio_util::sync::CancellationToken {
        self.ensure_run_armed();
        self.armed_run
            .as_ref()
            .expect("run was armed")
            .cancel_token()
    }

    /// Arm the next run and return a control handle targeting that generation.
    pub fn control_handle(&mut self) -> opi_agent::agent::AgentControl {
        self.ensure_run_armed();
        self.agent
            .control_handle_for_run(self.armed_run.as_ref().expect("run was armed"))
            .expect("harness run was armed by this Agent and remains latest")
    }

    /// Reset cancellation state before cloning a control handle for a new turn.
    pub fn reset_cancel_if_cancelled(&mut self) {
        self.armed_run = None;
        self.agent.reset_cancel_if_cancelled();
    }

    fn ensure_run_armed(&mut self) {
        if self.armed_run.is_none() {
            self.armed_run = Some(self.agent.arm_run());
        }
    }

    fn take_or_arm_run(&mut self) -> ArmedAgentRun {
        self.armed_run
            .take()
            .unwrap_or_else(|| self.agent.arm_run())
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
        self.ensure_session_resume_ready()
            .map_err(|error| error.to_string())?;
        if self.session.is_none() {
            return Err("no active session".into());
        }
        self.setup_evidence_run()
            .map_err(|error| error.to_string())?;
        let result = self
            .session
            .as_mut()
            .expect("active session checked above")
            .execute_compaction(reason);
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                let original = format!("compaction failed: {error}");
                if let Err(evidence) = self.complete_standalone_compaction_evidence(
                    reason,
                    opi_agent::evidence::CompactionOutcome::Failed,
                    opi_agent::evidence::TerminalOutcome::Failed,
                    &format!("manual-compaction:{}", compaction_reason_name(reason)),
                ) {
                    self.record_harness_diagnostic(Self::evidence_error_diagnostic(&evidence));
                }
                return Err(original);
            }
        };
        match result {
            Some(out) => {
                let wire = crate::session_coordinator::to_wire_result(&out);
                let diagnostic = out.diagnostic.clone();
                self.record_harness_diagnostic(diagnostic.clone());
                self.replace_agent_context(out.new_agent_messages)?;
                self.complete_standalone_compaction_evidence(
                    reason,
                    opi_agent::evidence::CompactionOutcome::Succeeded,
                    opi_agent::evidence::TerminalOutcome::Success,
                    &format!("manual-compaction:{}", compaction_reason_name(reason)),
                )
                .map_err(|error| error.to_string())?;
                Ok((Some(wire), diagnostic))
            }
            None => {
                let error = opi_agent::compaction::CompactionError::NothingToCompact;
                let diagnostic = Diagnostic::from(&error);
                self.record_harness_diagnostic(diagnostic.clone());
                self.complete_standalone_compaction_evidence(
                    reason,
                    opi_agent::evidence::CompactionOutcome::Aborted,
                    opi_agent::evidence::TerminalOutcome::Success,
                    &format!("manual-compaction:{}", compaction_reason_name(reason)),
                )
                .map_err(|error| error.to_string())?;
                Ok((None, diagnostic))
            }
        }
    }

    fn emit_manual_compaction_evidence(
        &self,
        reason: opi_agent::session_event::CompactionReason,
        outcome: opi_agent::evidence::CompactionOutcome,
    ) -> Result<(), opi_agent::evidence::EvidenceError> {
        let Some(capture) = self.evidence.as_ref() else {
            return Ok(());
        };
        let mut identities = opi_agent::evidence::IdentityAllocator::new();
        let call = identities.next_call();
        let trigger = compaction_trigger(reason);
        let started = opi_agent::evidence::EvidenceRecord {
            run: identities.run_id(),
            turn: None,
            call,
            parent: None,
            sequence: identities.next_sequence(),
            kind: opi_agent::evidence::CallKind::Compaction,
            payload: opi_agent::evidence::EvidencePayload::Compaction(
                opi_agent::evidence::CompactionEvidenceFacts::started(trigger),
            ),
        };
        capture.recorder.emit(&started)?;
        let terminal = opi_agent::evidence::EvidenceRecord {
            run: identities.run_id(),
            turn: None,
            call,
            parent: None,
            sequence: identities.next_sequence(),
            kind: opi_agent::evidence::CallKind::Compaction,
            payload: opi_agent::evidence::EvidencePayload::Compaction(
                opi_agent::evidence::CompactionEvidenceFacts::terminal(trigger, outcome),
            ),
        };
        capture.recorder.emit(&terminal)
    }

    fn evidence_error_diagnostic(error: &opi_agent::evidence::EvidenceError) -> Diagnostic {
        match error {
            opi_agent::evidence::EvidenceError::Setup { .. } => {
                Diagnostic::from(&AgentError::EvidenceSetup(error.to_string()))
            }
            opi_agent::evidence::EvidenceError::Emission { .. } => Diagnostic::new(
                Severity::Error,
                opi_agent::diagnostic::code::CODE_EVIDENCE_EMISSION_FAILED,
                opi_agent::diagnostic::SOURCE_AGENT,
                "evidence emission failed",
            )
            .details(serde_json::json!({ "evidence_error": error.to_string() }))
            .action("check evidence capture completeness and destination durability"),
            opi_agent::evidence::EvidenceError::Finalization { .. } => {
                Diagnostic::from(&AgentError::EvidenceFinalization(error.to_string()))
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
    /// `build_tools` constructs the local Operations defaults
    /// (`LocalFileOperations` / `LocalBashOperations`), threads the resolved
    /// execution context through [`ExecutionRuntime::build`], and injects the
    /// selected [`BashOperations`] plus the dynamic bash schema into the
    /// production `BashTool`. The four navigation tools (`grep`/`find`/`ls`/
    /// `glob`) keep their local-walk constructors unchanged — their `ignore`-
    /// crate walker cannot be cleanly redirected to a backend. Returns any
    /// execution-startup diagnostics so they surface in
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
        // An untrusted project skips its project layer
        // (skills/fragments/themes/extensions/packages) so project-local
        // resources cannot resolve. User-global and explicit layers remain; this
        // is also the `/skill:`/`/fragment:` filter point.
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
mod route_normalization_tests {
    use super::canonical_model_spec;

    #[test]
    fn canonical_model_spec_preserves_caller_owned_identity_semantics() {
        assert_eq!(canonical_model_spec("alpha", "model"), "alpha:model");
        assert_eq!(canonical_model_spec("alpha", "beta:model"), "beta:model");
        assert_eq!(canonical_model_spec("alpha", "beta:"), "beta:");
    }
}

#[cfg(test)]
mod permission_boundary_tests {
    use super::*;

    const PRESERVE_PREFIX: usize = 0;
    const REPLACE_WITH_SHORTER_CONTEXT: usize = 1;
    const REWRITE_COMMITTED_PREFIX: usize = 2;
    const REWRITE_COMMITTED_PREFIX_AND_CONTINUE: usize = 3;

    struct ReplaceCommittedPrefixHooks {
        replacement: Arc<std::sync::atomic::AtomicUsize>,
        stop_after_replacement: std::sync::atomic::AtomicBool,
    }

    impl AgentHooks for ReplaceCommittedPrefixHooks {
        fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
            Ok(agent_messages_to_llm(messages))
        }

        fn should_stop_after_turn(
            &self,
            _ctx: opi_agent::hooks::ShouldStopAfterTurnContext,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send>> {
            let stop = self
                .stop_after_replacement
                .swap(false, std::sync::atomic::Ordering::SeqCst);
            Box::pin(async move { stop })
        }

        fn prepare_next_turn(
            &self,
            ctx: opi_agent::hooks::PrepareNextTurnContext,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<Option<opi_agent::loop_types::NextTurnState>, AgentError>,
                    > + Send,
            >,
        > {
            let replacement = self
                .replacement
                .swap(PRESERVE_PREFIX, std::sync::atomic::Ordering::SeqCst);
            if matches!(
                replacement,
                REPLACE_WITH_SHORTER_CONTEXT | REWRITE_COMMITTED_PREFIX
            ) {
                self.stop_after_replacement
                    .store(true, std::sync::atomic::Ordering::SeqCst);
            }
            Box::pin(async move {
                let mut candidate = ctx.state;
                match replacement {
                    PRESERVE_PREFIX => return Ok(None),
                    REPLACE_WITH_SHORTER_CONTEXT => candidate.context.truncate(1),
                    REWRITE_COMMITTED_PREFIX | REWRITE_COMMITTED_PREFIX_AND_CONTINUE => {
                        candidate.context[0] =
                            AgentMessage::Llm(Message::User(opi_ai::message::UserMessage {
                                content: vec![opi_ai::message::InputContent::Text {
                                    text: "rewritten committed prefix".to_owned(),
                                }],
                                timestamp_ms: 0,
                            }));
                    }
                    other => panic!("unknown test replacement mode {other}"),
                }
                candidate.inference.max_tokens = Some(17);
                candidate.inference.temperature = Some(0.25);
                Ok(Some(candidate))
            })
        }
    }

    async fn assert_committed_prefix_replacement_is_rejected(replacement: usize) {
        let workspace = tempfile::tempdir().unwrap();
        let global = tempfile::tempdir().unwrap();
        let sessions = tempfile::tempdir().unwrap();
        let provider = opi_ai::test_support::MockProvider::new(
            "mock",
            vec![
                opi_ai::test_support::text_response("baseline response"),
                opi_ai::test_support::text_response("rejected response"),
                opi_ai::test_support::text_response("recovered response"),
            ],
        );
        let replacement_mode = Arc::new(std::sync::atomic::AtomicUsize::new(PRESERVE_PREFIX));
        let mut harness = CodingHarness::builder(
            Box::new(provider),
            "mock:mock-model".to_owned(),
            OpiConfig::default(),
            workspace.path().to_path_buf(),
            crate::project_trust::TrustDecision::Trusted,
        )
        .global_config_dir(global.path().to_path_buf())
        .session_dir_for_test(sessions.path().to_path_buf())
        .hooks(Box::new(ReplaceCommittedPrefixHooks {
            replacement: replacement_mode.clone(),
            stop_after_replacement: std::sync::atomic::AtomicBool::new(false),
        }))
        .build();
        harness.prompt("baseline prompt").await.unwrap();

        let session_path = harness.session().unwrap().session_path().to_path_buf();
        let session_before = std::fs::read(&session_path).unwrap();
        let state_before = harness.agent.state_snapshot();
        let offset_before = harness.turn_offset;
        replacement_mode.store(replacement, std::sync::atomic::Ordering::SeqCst);

        let error = harness
            .prompt("rejected prompt")
            .await
            .expect_err("a session-backed run cannot replace its committed prefix");
        assert!(matches!(error, AgentError::SessionPersist(_)));
        assert_eq!(std::fs::read(&session_path).unwrap(), session_before);
        let state_after = harness.agent.state_snapshot();
        assert_eq!(
            serde_json::to_value(&state_after.context).unwrap(),
            serde_json::to_value(&state_before.context).unwrap()
        );
        assert_eq!(state_after.model_selection, state_before.model_selection);
        assert_eq!(
            state_after.inference.max_tokens,
            state_before.inference.max_tokens
        );
        assert_eq!(
            state_after.inference.temperature,
            state_before.inference.temperature
        );
        assert_eq!(
            state_after.inference.thinking.enabled,
            state_before.inference.thinking.enabled
        );
        assert_eq!(
            state_after.inference.thinking.budget_tokens,
            state_before.inference.thinking.budget_tokens
        );
        assert_eq!(
            state_after.inference.thinking.level,
            state_before.inference.thinking.level
        );
        assert_eq!(harness.turn_offset, offset_before);

        harness.prompt("recovered prompt").await.unwrap();
        let recovered = String::from_utf8(std::fs::read(&session_path).unwrap()).unwrap();
        assert!(!recovered.contains("rejected prompt"));
        assert!(recovered.contains("recovered prompt"));
    }

    #[tokio::test]
    async fn shorter_prepared_context_cannot_replace_committed_session_prefix() {
        assert_committed_prefix_replacement_is_rejected(REPLACE_WITH_SHORTER_CONTEXT).await;
    }

    #[tokio::test]
    async fn rewritten_prepared_context_cannot_replace_committed_session_prefix() {
        assert_committed_prefix_replacement_is_rejected(REWRITE_COMMITTED_PREFIX).await;
    }

    #[tokio::test]
    async fn credential_failure_owns_prefix_violation_and_restores_committed_state() {
        let workspace = tempfile::tempdir().unwrap();
        let global = tempfile::tempdir().unwrap();
        let sessions = tempfile::tempdir().unwrap();
        let evidence = tempfile::tempdir().unwrap();
        let provider = opi_ai::test_support::MockProvider::new_with_errors(
            "mock",
            vec![
                opi_ai::test_support::MockResponse::Events(opi_ai::test_support::text_response(
                    "baseline response",
                )),
                opi_ai::test_support::MockResponse::Events(opi_ai::test_support::text_response(
                    "rewrite response",
                )),
                opi_ai::test_support::MockResponse::Error(
                    opi_ai::provider::ProviderError::CredentialNeeded {
                        provider_id: "mock".to_owned(),
                    },
                ),
                opi_ai::test_support::MockResponse::Events(opi_ai::test_support::text_response(
                    "recovered response",
                )),
            ],
        );
        let sink = Arc::new(crate::evidence::FileEvidenceSink::new(evidence.path()));
        let recorder: Arc<dyn opi_agent::evidence::EvidenceRecorder> = sink.clone();
        let replacement_mode = Arc::new(std::sync::atomic::AtomicUsize::new(PRESERVE_PREFIX));
        let mut harness = CodingHarness::builder(
            Box::new(provider),
            "mock:mock-model".to_owned(),
            OpiConfig::default(),
            workspace.path().to_path_buf(),
            crate::project_trust::TrustDecision::Trusted,
        )
        .global_config_dir(global.path().to_path_buf())
        .session_dir_for_test(sessions.path().to_path_buf())
        .record_diagnostics(true)
        .evidence(EvidenceBuilderConfig {
            recorder,
            source: crate::evidence::CLI_ASSEMBLY.clone(),
        })
        .hooks(Box::new(ReplaceCommittedPrefixHooks {
            replacement: replacement_mode.clone(),
            stop_after_replacement: std::sync::atomic::AtomicBool::new(false),
        }))
        .build();
        harness.prompt("baseline prompt").await.unwrap();

        let session_path = harness.session().unwrap().session_path().to_path_buf();
        let session_before = std::fs::read(&session_path).unwrap();
        let state_before = harness.agent.state_snapshot();
        let offset_before = harness.turn_offset;
        let completed_before = sink.completed_run_dirs().len();
        replacement_mode.store(
            REWRITE_COMMITTED_PREFIX_AND_CONTINUE,
            std::sync::atomic::Ordering::SeqCst,
        );

        let error = harness.prompt("rejected prompt").await.unwrap_err();

        assert!(
            matches!(
                &error,
                AgentError::Provider(failure)
                    if matches!(
                        failure.provider_error(),
                        opi_ai::provider::ProviderError::CredentialNeeded { provider_id }
                            if provider_id == "mock"
                    )
            ),
            "expected CredentialNeeded to remain owning, got {error:?}"
        );
        assert_eq!(std::fs::read(&session_path).unwrap(), session_before);
        let state_after = harness.agent.state_snapshot();
        assert_eq!(
            serde_json::to_value(&state_after.context).unwrap(),
            serde_json::to_value(&state_before.context).unwrap()
        );
        assert_eq!(state_after.model_selection, state_before.model_selection);
        assert_eq!(
            state_after.inference.max_tokens,
            state_before.inference.max_tokens
        );
        assert_eq!(
            state_after.inference.temperature,
            state_before.inference.temperature
        );
        assert_eq!(harness.turn_offset, offset_before);
        assert_eq!(sink.completed_run_dirs().len(), completed_before);
        assert_eq!(
            std::fs::read_dir(evidence.path())
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry.path().join("manifest.json").is_file())
                .count(),
            completed_before,
            "the rejected run is abandoned without a manifest"
        );
        let diagnostics = harness.recorded_diagnostics();
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| {
                    diagnostic.code == opi_agent::diagnostic::code::CODE_SESSION_PERSIST_FAILED
                })
                .count(),
            1,
            "the prefix violation remains a secondary session diagnostic"
        );
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| {
                    diagnostic.code
                        == opi_agent::diagnostic::code::CODE_EVIDENCE_FINALIZATION_FAILED
                })
                .count(),
            1,
            "the evidence abandonment remains separately observable"
        );

        harness.prompt("recovered prompt").await.unwrap();
        let recovered = String::from_utf8(std::fs::read(&session_path).unwrap()).unwrap();
        assert!(!recovered.contains("rejected prompt"));
        assert!(recovered.contains("recovered prompt"));
        assert_eq!(sink.completed_run_dirs().len(), completed_before + 1);
    }

    #[tokio::test]
    async fn no_session_allows_complete_prepared_context_replacement() {
        let workspace = tempfile::tempdir().unwrap();
        let global = tempfile::tempdir().unwrap();
        let session_parent = tempfile::tempdir().unwrap();
        let unavailable_session_dir = session_parent.path().join("not-a-directory");
        std::fs::write(&unavailable_session_dir, b"block directory creation").unwrap();
        let provider = opi_ai::test_support::MockProvider::new(
            "mock",
            vec![
                opi_ai::test_support::text_response("baseline response"),
                opi_ai::test_support::text_response("replacement response"),
                opi_ai::test_support::text_response("continued response"),
            ],
        );
        let replacement_mode = Arc::new(std::sync::atomic::AtomicUsize::new(PRESERVE_PREFIX));
        let mut harness = CodingHarness::builder(
            Box::new(provider),
            "mock:mock-model".to_owned(),
            OpiConfig::default(),
            workspace.path().to_path_buf(),
            crate::project_trust::TrustDecision::Trusted,
        )
        .global_config_dir(global.path().to_path_buf())
        .session_dir_for_test(unavailable_session_dir)
        .hooks(Box::new(ReplaceCommittedPrefixHooks {
            replacement: replacement_mode.clone(),
            stop_after_replacement: std::sync::atomic::AtomicBool::new(false),
        }))
        .build();
        assert!(harness.session().is_none());
        harness.prompt("baseline prompt").await.unwrap();
        replacement_mode.store(
            REPLACE_WITH_SHORTER_CONTEXT,
            std::sync::atomic::Ordering::SeqCst,
        );

        harness.prompt("replacement prompt").await.unwrap();

        let state = harness.agent.state_snapshot();
        assert_eq!(state.context.len(), 1);
        assert_eq!(state.inference.max_tokens, Some(17));
        assert_eq!(state.inference.temperature, Some(0.25));

        harness.prompt("continued prompt").await.unwrap();
        assert_eq!(harness.agent.messages_snapshot().len(), 3);
    }

    fn build_persistence_failure_harness(
        workspace: &Path,
        global: &Path,
        session_dir: &Path,
        evidence_dir: &Path,
    ) -> (CodingHarness, Arc<crate::evidence::FileEvidenceSink>) {
        let provider = opi_ai::test_support::MockProvider::new(
            "mock",
            vec![
                opi_ai::test_support::text_response("uncommitted response"),
                opi_ai::test_support::text_response("committed response"),
            ],
        );
        let sink = Arc::new(crate::evidence::FileEvidenceSink::new(evidence_dir));
        let recorder: Arc<dyn opi_agent::evidence::EvidenceRecorder> = sink.clone();
        let harness = CodingHarness::builder(
            Box::new(provider),
            "mock:mock-model".to_owned(),
            OpiConfig::default(),
            workspace.to_path_buf(),
            crate::project_trust::TrustDecision::Trusted,
        )
        .global_config_dir(global.to_path_buf())
        .session_dir_for_test(session_dir.to_path_buf())
        .evidence(EvidenceBuilderConfig {
            recorder,
            source: crate::evidence::CLI_ASSEMBLY.clone(),
        })
        .build();
        (harness, sink)
    }

    #[tokio::test]
    async fn failed_turn_append_keeps_boundary_and_recovers_next_prompt() {
        let workspace = tempfile::tempdir().unwrap();
        let global = tempfile::tempdir().unwrap();
        let sessions = tempfile::tempdir().unwrap();
        let evidence = tempfile::tempdir().unwrap();
        let (mut harness, sink) = build_persistence_failure_harness(
            workspace.path(),
            global.path(),
            sessions.path(),
            evidence.path(),
        );
        let session_path = harness.session().unwrap().session_path().to_path_buf();
        let bytes_before = std::fs::read(&session_path).unwrap();
        let offset_before = harness.turn_offset;
        harness
            .session
            .as_mut()
            .unwrap()
            .inject_append_failure_after_for_test(0);

        assert!(matches!(
            harness.prompt("failed append prompt").await,
            Err(AgentError::SessionPersist(_))
        ));
        assert_eq!(std::fs::read(&session_path).unwrap(), bytes_before);
        assert_eq!(harness.turn_offset, offset_before);
        assert_eq!(harness.agent.messages_snapshot().len(), offset_before);
        assert!(sink.completed_run_dirs().is_empty());
        assert!(
            std::fs::read_dir(evidence.path())
                .unwrap()
                .all(|entry| !entry.unwrap().path().join("manifest.json").exists())
        );

        harness.prompt("recovered prompt").await.unwrap();
        let recovered = std::fs::read(&session_path).unwrap();
        assert!(recovered.starts_with(&bytes_before));
        let recovered = String::from_utf8(recovered).unwrap();
        assert!(!recovered.contains("failed append prompt"));
        assert!(recovered.contains("recovered prompt"));
        assert_eq!(sink.completed_run_dirs().len(), 1);
    }

    #[tokio::test]
    async fn partial_turn_append_rolls_back_bytes_and_recovers_next_prompt() {
        let workspace = tempfile::tempdir().unwrap();
        let global = tempfile::tempdir().unwrap();
        let sessions = tempfile::tempdir().unwrap();
        let evidence = tempfile::tempdir().unwrap();
        let (mut harness, sink) = build_persistence_failure_harness(
            workspace.path(),
            global.path(),
            sessions.path(),
            evidence.path(),
        );
        let session_path = harness.session().unwrap().session_path().to_path_buf();
        let bytes_before = std::fs::read(&session_path).unwrap();
        let offset_before = harness.turn_offset;
        harness
            .session
            .as_mut()
            .unwrap()
            .inject_append_failure_after_for_test(1);

        assert!(matches!(
            harness.prompt("partial append prompt").await,
            Err(AgentError::SessionPersist(_))
        ));
        assert_eq!(
            std::fs::read(&session_path).unwrap(),
            bytes_before,
            "the durable prefix is restored after a mid-turn append failure"
        );
        assert_eq!(harness.turn_offset, offset_before);
        assert_eq!(harness.agent.messages_snapshot().len(), offset_before);
        assert!(sink.completed_run_dirs().is_empty());
        assert!(
            std::fs::read_dir(evidence.path())
                .unwrap()
                .all(|entry| !entry.unwrap().path().join("manifest.json").exists())
        );

        harness.prompt("recovered prompt").await.unwrap();
        let recovered = std::fs::read(&session_path).unwrap();
        assert!(recovered.starts_with(&bytes_before));
        let recovered = String::from_utf8(recovered).unwrap();
        assert!(!recovered.contains("partial append prompt"));
        assert!(recovered.contains("recovered prompt"));
        assert_eq!(sink.completed_run_dirs().len(), 1);
    }

    #[tokio::test]
    async fn capture_disabled_persistence_failure_has_no_evidence_lifecycle_diagnostic() {
        let workspace = tempfile::tempdir().unwrap();
        let global = tempfile::tempdir().unwrap();
        let sessions = tempfile::tempdir().unwrap();
        let provider = opi_ai::test_support::MockProvider::new(
            "mock",
            vec![opi_ai::test_support::text_response("uncommitted response")],
        );
        let mut harness = CodingHarness::builder(
            Box::new(provider),
            "mock:mock-model".to_owned(),
            OpiConfig::default(),
            workspace.path().to_path_buf(),
            crate::project_trust::TrustDecision::Trusted,
        )
        .global_config_dir(global.path().to_path_buf())
        .session_dir_for_test(sessions.path().to_path_buf())
        .record_diagnostics(true)
        .build();
        harness
            .session
            .as_mut()
            .unwrap()
            .inject_append_failure_after_for_test(0);

        assert!(matches!(
            harness.prompt("failed without capture").await,
            Err(AgentError::SessionPersist(_))
        ));
        let diagnostics = harness.recorded_diagnostics();
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == opi_agent::diagnostic::code::CODE_SESSION_PERSIST_FAILED
        }));
        assert!(
            diagnostics.iter().all(|diagnostic| !matches!(
                diagnostic.code,
                opi_agent::diagnostic::code::CODE_EVIDENCE_SETUP_FAILED
                    | opi_agent::diagnostic::code::CODE_EVIDENCE_EMISSION_FAILED
                    | opi_agent::diagnostic::code::CODE_EVIDENCE_FINALIZATION_FAILED
            )),
            "capture-disabled persistence cannot fabricate an evidence lifecycle: {diagnostics:?}"
        );
    }

    #[tokio::test]
    async fn automatic_compaction_marker_failure_emits_failed_terminal_on_the_run_identity() {
        let workspace = tempfile::tempdir().unwrap();
        let global = tempfile::tempdir().unwrap();
        let sessions = tempfile::tempdir().unwrap();
        let sink = Arc::new(opi_agent::evidence::InMemoryEvidenceSink::new());
        let recorder: Arc<dyn opi_agent::evidence::EvidenceRecorder> = sink.clone();
        let mut config = OpiConfig::default();
        config.compaction.threshold_tokens = 0;
        let mut harness = CodingHarness::builder(
            Box::new(opi_ai::test_support::MockProvider::new(
                "mock",
                vec![opi_ai::test_support::text_response("response")],
            )),
            "mock:mock-model".to_owned(),
            config,
            workspace.path().to_path_buf(),
            crate::project_trust::TrustDecision::Trusted,
        )
        .global_config_dir(global.path().to_path_buf())
        .session_dir_for_test(sessions.path().to_path_buf())
        .evidence(EvidenceBuilderConfig {
            recorder,
            source: crate::evidence::CLI_ASSEMBLY.clone(),
        })
        .build();
        // The turn writes user + assistant + Leaf. Fail the next append, which
        // is the compaction marker, and let the checkpoint rollback succeed.
        harness
            .session
            .as_mut()
            .unwrap()
            .inject_append_failure_after_for_test(3);

        harness
            .prompt("prompt")
            .await
            .expect("turn remains committed");

        let compaction = sink
            .records()
            .into_iter()
            .filter(|record| record.kind == opi_agent::evidence::CallKind::Compaction)
            .collect::<Vec<_>>();
        assert_eq!(compaction.len(), 2);
        assert_eq!(compaction[0].run, compaction[1].run);
        assert_eq!(compaction[0].call, compaction[1].call);
        assert!(matches!(
            &compaction[0].payload,
            opi_agent::evidence::EvidencePayload::Compaction(facts)
                if facts.outcome().is_none()
                    && facts.trigger() == opi_agent::evidence::CompactionTrigger::Threshold
        ));
        assert!(matches!(
            &compaction[1].payload,
            opi_agent::evidence::EvidencePayload::Compaction(facts)
                if facts.outcome() == Some(opi_agent::evidence::CompactionOutcome::Failed)
        ));
        assert!(matches!(
            sink.completed_manifest().unwrap().outcome,
            opi_agent::evidence::TerminalOutcome::Failed
        ));
        let (_, entries) =
            opi_agent::session::SessionReader::read_all(harness.session().unwrap().session_path())
                .unwrap();
        assert!(
            entries
                .iter()
                .all(|entry| !matches!(entry, opi_agent::session::SessionEntry::Compaction(_)))
        );
    }

    #[tokio::test]
    async fn automatic_compaction_rollback_failure_emits_cleanup_unknown_terminal() {
        let workspace = tempfile::tempdir().unwrap();
        let global = tempfile::tempdir().unwrap();
        let sessions = tempfile::tempdir().unwrap();
        let sink = Arc::new(opi_agent::evidence::InMemoryEvidenceSink::new());
        let recorder: Arc<dyn opi_agent::evidence::EvidenceRecorder> = sink.clone();
        let mut config = OpiConfig::default();
        config.compaction.threshold_tokens = 0;
        let mut harness = CodingHarness::builder(
            Box::new(opi_ai::test_support::MockProvider::new(
                "mock",
                vec![opi_ai::test_support::text_response("response")],
            )),
            "mock:mock-model".to_owned(),
            config,
            workspace.path().to_path_buf(),
            crate::project_trust::TrustDecision::Trusted,
        )
        .global_config_dir(global.path().to_path_buf())
        .session_dir_for_test(sessions.path().to_path_buf())
        .evidence(EvidenceBuilderConfig {
            recorder,
            source: crate::evidence::CLI_ASSEMBLY.clone(),
        })
        .build();
        // The turn writes user + assistant + Leaf, then the compaction marker.
        // Fail its Leaf and the real checkpoint rollback, leaving cleanup truth
        // unknown and the writer poisoned.
        harness
            .session
            .as_mut()
            .unwrap()
            .inject_append_failure_after_for_test(4);
        harness
            .session
            .as_mut()
            .unwrap()
            .inject_rollback_failure_for_test();

        harness
            .prompt("prompt")
            .await
            .expect("turn remains committed");

        let compaction = sink
            .records()
            .into_iter()
            .filter(|record| record.kind == opi_agent::evidence::CallKind::Compaction)
            .collect::<Vec<_>>();
        assert_eq!(compaction.len(), 2);
        assert_eq!(compaction[0].run, compaction[1].run);
        assert_eq!(compaction[0].call, compaction[1].call);
        assert!(matches!(
            &compaction[1].payload,
            opi_agent::evidence::EvidencePayload::Compaction(facts)
                if facts.outcome() == Some(opi_agent::evidence::CompactionOutcome::CleanupUnknown)
        ));
        assert!(matches!(
            sink.completed_manifest().unwrap().outcome,
            opi_agent::evidence::TerminalOutcome::CleanupUnknown
        ));
        assert!(
            harness
                .session()
                .unwrap()
                .compaction_entries()
                .iter()
                .all(|entry| !matches!(&entry.message, AgentMessage::CompactionSummary(_)))
        );
        let raw = std::fs::read_to_string(harness.session().unwrap().session_path()).unwrap();
        assert!(
            raw.contains("\"type\":\"compaction\""),
            "the failed rollback leaves a real durable partial marker"
        );
    }

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
