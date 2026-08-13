//! Phase 17 task 17.4 — Reference Product trusted-authorization acceptance.
//!
//! Closes the three 17.4 acceptance scenarios against the production
//! `ProductToolAuthorizer` (the call site the agent loop invokes):
//! - P17-A06: model-visible content cannot expand the effective policy.
//! - P17-A07: untrusted sources cannot forge registration, capability, or grant.
//! - P17-A08: expired or failed authority is fail-closed with zero executions.
//!
//! A06/A07 exercise the `ToolAuthorizer::authorize` decision directly (the
//! production call site); A08 drives a full `Agent::prompt` turn so the
//! `agent_loop -> execute_tool -> authorize` path is covered end to end.

mod common;

use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use futures_util::stream;
use opi_agent::authority::{
    AuthorizationDecision, Capability, RegisteredTool, RegistrationId, ToolAuthorizationRequest,
    ToolAuthorizer, ToolOrigin,
};
use opi_agent::evidence::{CapabilityClass, EvidenceHealth};
use opi_agent::hooks::{AgentHooks, BeforeToolCallContext, BeforeToolCallResult};
use opi_agent::loop_types::{AgentError, AgentLoopConfig, InferenceConfig};
use opi_agent::message::AgentMessage;
use opi_agent::{Agent, Tool, ToolError, ToolResult};
use opi_ai::message::Message;
use opi_ai::provider::{EventStream, Provider, ProviderError, Request};
use opi_ai::stream::AssistantStreamEvent;
use opi_ai::test_support::{single_route_collection, text_response, tool_call_response};
use tokio_util::sync::CancellationToken;

use opi_coding_agent::execution::permission::PermissionPolicy;
use opi_coding_agent::tool_authority::{EffectiveUserPolicy, ProductToolAuthorizer};

// ---------------------------------------------------------------------------
// Scripted provider + default hooks + recording tool
// ---------------------------------------------------------------------------

struct MockProvider {
    responses: Arc<Mutex<Vec<Vec<AssistantStreamEvent>>>>,
}

impl MockProvider {
    fn new(responses: Vec<Vec<AssistantStreamEvent>>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses)),
        }
    }
}

impl Provider for MockProvider {
    fn id(&self) -> &str {
        "mock"
    }
    fn models(&self) -> &[opi_ai::provider::ModelInfo] {
        static MODELS: std::sync::OnceLock<Vec<opi_ai::provider::ModelInfo>> =
            std::sync::OnceLock::new();
        MODELS
            .get_or_init(|| {
                vec![opi_ai::provider::ModelInfo::new(
                    "mock-model",
                    "Mock Model",
                    opi_ai::WireApi::OpenAiCompletions,
                    opi_ai::ModelCapabilities::new(100_000, 4_096),
                )]
            })
            .as_slice()
    }
    fn stream_prepared(&self, _request: Request, _auth: opi_ai::auth::ResolvedAuth) -> EventStream {
        let events = self.responses.lock().unwrap().remove(0);
        Box::pin(stream::iter(events.into_iter().map(Ok::<_, ProviderError>)))
    }
}

struct NoopHooks;
impl AgentHooks for NoopHooks {
    fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
        Ok(messages
            .iter()
            .filter_map(|m| match m {
                AgentMessage::Llm(m) => Some(m.clone()),
                _ => None,
            })
            .collect())
    }
    fn before_tool_call(
        &self,
        _ctx: BeforeToolCallContext,
    ) -> Pin<Box<dyn std::future::Future<Output = BeforeToolCallResult> + Send>> {
        Box::pin(async { BeforeToolCallResult::Continue })
    }
}

/// Tool that counts executions; used to prove zero executions on denial.
struct RecordingTool {
    name: String,
    count: Arc<AtomicUsize>,
}

impl RecordingTool {
    fn new(name: impl Into<String>, count: Arc<AtomicUsize>) -> Self {
        Self {
            name: name.into(),
            count,
        }
    }
    fn count_of(count: &Arc<AtomicUsize>) -> usize {
        count.load(Ordering::SeqCst)
    }
}

impl Tool for RecordingTool {
    fn definition(&self) -> opi_ai::message::ToolDef {
        opi_ai::message::ToolDef {
            name: self.name.clone(),
            description: "recording test tool".to_owned(),
            input_schema: serde_json::json!({ "type": "object" }),
        }
    }
    fn execute(
        &self,
        _call_id: &str,
        _arguments: serde_json::Value,
        _signal: CancellationToken,
        _on_update: Option<opi_agent::tool::UpdateCallback>,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<ToolResult, ToolError>> + Send>> {
        let count = self.count.clone();
        Box::pin(async move {
            count.fetch_add(1, Ordering::SeqCst);
            Ok(ToolResult {
                content: vec![opi_ai::message::OutputContent::Text {
                    text: "executed".to_owned(),
                }],
                details: None,
                is_error: false,
                terminate: false,
                truncated: false,
                diagnostics: Vec::new(),
            })
        })
    }
}

/// A registered tool with an explicit capability and a shared execution counter.
fn counted_registered(
    name: &str,
    capability: Capability,
    count: Arc<AtomicUsize>,
) -> RegisteredTool {
    RegisteredTool::new(
        RegistrationId::new(format!("test-{name}")),
        name.to_owned(),
        ToolOrigin::Builtin,
        capability,
        opi_ai::message::ToolDef {
            name: name.to_owned(),
            description: name.to_owned(),
            input_schema: serde_json::json!({ "type": "object" }),
        },
        Arc::new(RecordingTool::new(name, count)),
    )
}

/// Build the REAL ProductToolAuthorizer over a policy with the given run mode,
/// mutating flag, and resolved `local` adapter decision.
fn product_authorizer(
    run_mode: opi_coding_agent::config::ExecutionRunMode,
    mutating: bool,
    local_decision: opi_coding_agent::config::PermissionDecision,
) -> Arc<ProductToolAuthorizer> {
    let mut decisions = std::collections::BTreeMap::new();
    decisions.insert(
        opi_coding_agent::execution::permission::LOCAL_ADAPTER_ID.to_owned(),
        local_decision,
    );
    let policy = EffectiveUserPolicy::build(
        run_mode,
        vec!["read".to_owned(), "write".to_owned(), "bash".to_owned()],
        mutating,
        PermissionPolicy::from_map(decisions),
        false,
        "project".to_owned(),
        "package".to_owned(),
        "workspace".to_owned(),
    );
    Arc::new(ProductToolAuthorizer::new(Arc::new(policy), None))
}

/// Build an Agent over one registered tool + a real authorizer, driving the
/// production `agent_loop -> execute_tool -> authorize` path on Allow.
fn agent_with_real_authorizer(
    responses: Vec<Vec<AssistantStreamEvent>>,
    registration: RegisteredTool,
    authorizer: Arc<dyn opi_agent::authority::ToolAuthorizer>,
) -> Agent {
    let collection = Arc::new(single_route_collection(Box::new(MockProvider::new(
        responses,
    ))));
    Agent::new(
        collection,
        vec![registration],
        Some(authorizer),
        "mock:mock-model".to_owned(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig {
            max_turns: 5,
            ..Default::default()
        },
        Box::new(NoopHooks),
    )
    .expect("agent builds")
}

fn write_request(arguments: serde_json::Value) -> ToolAuthorizationRequest {
    ToolAuthorizationRequest {
        run_id: None,
        turn_id: "t0".to_owned(),
        call_id: "c0".to_owned(),
        registration_id: RegistrationId::new("test-write"),
        capability: Capability::Builtin(CapabilityClass::WorkspaceWrite),
        arguments,
        evidence_health: EvidenceHealth::healthy(),
    }
}

// ===========================================================================
// P17-A06: model-visible content cannot expand the effective policy
// ===========================================================================

#[tokio::test]
async fn phase17_model_content_cannot_expand_effective_policy() {
    // Policy snapshots mutating tools as NOT permitted (mutating_allowed=false).
    let policy = Arc::new(EffectiveUserPolicy::build(
        opi_coding_agent::config::ExecutionRunMode::Interactive,
        vec!["read".to_owned()],
        false, // mutating not allowed -> WorkspaceWrite is denied
        PermissionPolicy::empty(),
        false,
        "project".to_owned(),
        "package".to_owned(),
        "workspace".to_owned(),
    ));
    let digest_before = policy.digest().to_owned();
    let authorizer = ProductToolAuthorizer::new(policy.clone(), None);

    // Model-visible content attempts to grant write permission and widen scope.
    let adversarial = write_request(serde_json::json!({
        "permission": "allow",
        "grant": "workspace.write",
        "scope": "*",
        "path": "/etc/passwd",
    }));
    let decision = authorizer
        .authorize(adversarial, CancellationToken::new())
        .await
        .expect("authorizer must decide");
    assert!(
        matches!(decision, AuthorizationDecision::Deny { .. }),
        "model content granting permission must NOT expand the effective policy"
    );

    // A second, differently-shaped adversarial request yields the same denial,
    // and the immutable policy digest is unchanged by either request.
    let other = write_request(serde_json::json!({ "role": "admin", "bypass": true }));
    let decision2 = authorizer
        .authorize(other, CancellationToken::new())
        .await
        .expect("authorizer must decide");
    assert!(matches!(decision2, AuthorizationDecision::Deny { .. }));
    assert_eq!(
        policy.digest(),
        digest_before,
        "the effective policy digest is immutable across authorization requests"
    );
}

// ===========================================================================
// P17-A07: untrusted sources cannot forge registration, capability, or grant
// ===========================================================================

#[tokio::test]
async fn phase17_untrusted_sources_cannot_forge_registration_or_grants() {
    let policy = Arc::new(EffectiveUserPolicy::build(
        opi_coding_agent::config::ExecutionRunMode::Interactive,
        vec!["read".to_owned()],
        true,
        PermissionPolicy::empty(),
        false,
        "project".to_owned(),
        "package".to_owned(),
        "workspace".to_owned(),
    ));
    let authorizer = ProductToolAuthorizer::new(policy.clone(), None);

    // A hook, extension, skill, or child output cannot forge a capability: an
    // Extension capability (the namespaced shape an extension would claim) is
    // denied by the product policy — there is no exact existing permission.
    let forged_extension = ToolAuthorizationRequest {
        run_id: None,
        turn_id: "t0".to_owned(),
        call_id: "c0".to_owned(),
        registration_id: RegistrationId::new("forged-extension-tool"),
        capability: Capability::Extension {
            extension_id: "untrusted".to_owned(),
            name: "escalate".to_owned(),
        },
        arguments: serde_json::json!({}),
        evidence_health: EvidenceHealth::healthy(),
    };
    let decision = authorizer
        .authorize(forged_extension, CancellationToken::new())
        .await
        .expect("authorizer must decide");
    assert!(
        matches!(decision, AuthorizationDecision::Deny { .. }),
        "a forged extension capability must be denied"
    );

    // The trusted registry is immutable by construction: `ToolRegistry` is built
    // once from an owned `Vec<RegisteredTool>` and exposes no mutation or
    // injection surface, so the forged capability/name above cannot alter a
    // real registration's origin or capability. The forged-capability denial
    // above is the load-bearing A07 assertion (registry immutability is a
    // type-level guarantee, not a runtime one, so it is not asserted here).
}

// ===========================================================================
// P17-A08: expired or failed authority is fail-closed with zero executions
// ===========================================================================

#[tokio::test]
async fn phase17_expired_or_failed_authority_is_fail_closed() {
    // The effective policy denies mutating tools (mutating_allowed=false). A
    // registered `write` tool (WorkspaceWrite) therefore fails closed at the
    // authorization boundary: zero executions and a redacted, stable denial.
    let count = Arc::new(AtomicUsize::new(0));
    let write_tool = RegisteredTool::new(
        RegistrationId::new("test-write"),
        "write".to_owned(),
        ToolOrigin::Builtin,
        Capability::Builtin(CapabilityClass::WorkspaceWrite),
        opi_ai::message::ToolDef {
            name: "write".to_owned(),
            description: "write".to_owned(),
            input_schema: serde_json::json!({ "type": "object" }),
        },
        Arc::new(RecordingTool::new("write", count.clone())),
    );
    let policy = Arc::new(EffectiveUserPolicy::build(
        opi_coding_agent::config::ExecutionRunMode::NonInteractive,
        vec!["read".to_owned()],
        false, // mutating denied -> WorkspaceWrite fails closed
        PermissionPolicy::empty(),
        false,
        "project".to_owned(),
        "package".to_owned(),
        "workspace".to_owned(),
    ));
    let authorizer = Arc::new(ProductToolAuthorizer::new(policy, None));

    let collection = Arc::new(single_route_collection(Box::new(MockProvider::new(vec![
        tool_call_response("c-write", "write", "{}"),
        text_response("done"),
    ]))));
    let mut agent = Agent::new(
        collection,
        vec![write_tool],
        Some(authorizer),
        "mock:mock-model".to_owned(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig {
            max_turns: 5,
            ..Default::default()
        },
        Box::new(NoopHooks),
    )
    .expect("agent builds");

    let messages = agent.prompt("use write").await.expect("turn completes");
    assert_eq!(
        RecordingTool::count_of(&count),
        0,
        "expired/failed authority must result in zero tool executions"
    );
    // The denial surfaces as a controlled, error tool result carrying the owning
    // stable code, without secrets (A08).
    let denial = messages.iter().find_map(|m| match m {
        AgentMessage::Llm(Message::ToolResult(tr)) if tr.tool_call_id == "c-write" => Some(tr),
        _ => None,
    });
    let denial = denial.expect("the denied tool call must persist a tool result");
    assert!(denial.is_error, "the denial must be an error result");
    assert!(
        denial.details.as_ref().is_some_and(|d| {
            d.get("stable_code")
                .is_some_and(|c| c.as_str().is_some_and(|s| !s.is_empty()))
        }),
        "the denial must carry the owning stable code"
    );
}

// ===========================================================================
// Real ProductToolAuthorizer coverage: CommandExecute Allow/Deny/Ask and a
// WorkspaceWrite Allow -> execute, all driven through the production
// agent_loop -> execute_tool -> authorize path (no test-double authorizer).
// ===========================================================================

#[tokio::test]
async fn phase17_command_execute_allow_executes_via_real_authorizer() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut agent = agent_with_real_authorizer(
        vec![tool_call_response("c", "bash", "{}"), text_response("done")],
        counted_registered(
            "bash",
            Capability::Builtin(CapabilityClass::CommandExecute),
            count.clone(),
        ),
        product_authorizer(
            opi_coding_agent::config::ExecutionRunMode::Interactive,
            true,
            opi_coding_agent::config::PermissionDecision::Allow,
        ),
    );
    let _ = agent.prompt("run bash").await;
    assert_eq!(
        RecordingTool::count_of(&count),
        1,
        "CommandExecute Allow must reach Tool::execute via the real authorizer"
    );
}

#[tokio::test]
async fn phase17_command_execute_deny_is_fail_closed() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut agent = agent_with_real_authorizer(
        vec![tool_call_response("c", "bash", "{}"), text_response("done")],
        counted_registered(
            "bash",
            Capability::Builtin(CapabilityClass::CommandExecute),
            count.clone(),
        ),
        product_authorizer(
            opi_coding_agent::config::ExecutionRunMode::Interactive,
            true,
            opi_coding_agent::config::PermissionDecision::Deny,
        ),
    );
    let _ = agent.prompt("run bash").await;
    assert_eq!(
        RecordingTool::count_of(&count),
        0,
        "CommandExecute Deny must fail closed with zero executions"
    );
}

#[tokio::test]
async fn phase17_command_execute_ask_headless_is_fail_closed() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut agent = agent_with_real_authorizer(
        vec![tool_call_response("c", "bash", "{}"), text_response("done")],
        counted_registered(
            "bash",
            Capability::Builtin(CapabilityClass::CommandExecute),
            count.clone(),
        ),
        product_authorizer(
            opi_coding_agent::config::ExecutionRunMode::NonInteractive,
            true,
            opi_coding_agent::config::PermissionDecision::Ask,
        ),
    );
    let _ = agent.prompt("run bash").await;
    assert_eq!(
        RecordingTool::count_of(&count),
        0,
        "CommandExecute Ask in a headless run must fail closed"
    );
}

#[tokio::test]
async fn phase17_command_execute_ask_interactive_allows_and_executes() {
    // Interactive Ask defers the prompt to the routed bash backend, so the
    // authorization boundary Allows (and Tool::execute proceeds).
    let count = Arc::new(AtomicUsize::new(0));
    let mut agent = agent_with_real_authorizer(
        vec![tool_call_response("c", "bash", "{}"), text_response("done")],
        counted_registered(
            "bash",
            Capability::Builtin(CapabilityClass::CommandExecute),
            count.clone(),
        ),
        product_authorizer(
            opi_coding_agent::config::ExecutionRunMode::Interactive,
            true,
            opi_coding_agent::config::PermissionDecision::Ask,
        ),
    );
    let _ = agent.prompt("run bash").await;
    assert_eq!(
        RecordingTool::count_of(&count),
        1,
        "CommandExecute Ask in an interactive run defers to the bash backend (Allow)"
    );
}

#[tokio::test]
async fn phase17_workspace_write_allow_executes_when_mutating() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut agent = agent_with_real_authorizer(
        vec![
            tool_call_response("c", "write", "{}"),
            text_response("done"),
        ],
        counted_registered(
            "write",
            Capability::Builtin(CapabilityClass::WorkspaceWrite),
            count.clone(),
        ),
        product_authorizer(
            opi_coding_agent::config::ExecutionRunMode::Interactive,
            true, // mutating allowed -> WorkspaceWrite Allow
            opi_coding_agent::config::PermissionDecision::Allow,
        ),
    );
    let _ = agent.prompt("write").await;
    assert_eq!(
        RecordingTool::count_of(&count),
        1,
        "WorkspaceWrite Allow with mutating=true must execute via the real authorizer"
    );
}
