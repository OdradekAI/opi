//! Phase 17 task 17.4 — trusted tool authorization behavioral tests.
//!
//! Drives the mandatory fail-closed authorization boundary through the public
//! Agent / `agent_loop` seam: a missing, denying, or stale authorizer yields
//! zero `Tool::execute` calls, and only a current `Allow` reaches execution
//! (AUT-001/003/005, OUT-003). The stale-generation case is synthetic — it
//! exercises the freshness gate directly; evidence-failure-driven health
//! advancement and product reauthorization belong to 17.7.

mod common;

use std::pin::Pin;
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex};

use futures_util::stream;
use opi_agent::agent::Agent;
use opi_agent::authority::{Capability, RegisteredTool, RegistrationId, ToolOrigin};
use opi_agent::evidence::{CapabilityClass, EvidenceGeneration, EvidenceHealth};
use opi_agent::hooks::{
    AgentHooks, BeforeToolCallContext, BeforeToolCallResult, ShouldStopAfterTurnContext,
};
use opi_agent::loop_types::{
    AgentError, AgentLoopConfig, AgentLoopContext, InferenceConfig, ModelSelection, NextTurnState,
};
use opi_agent::message::AgentMessage;
use opi_agent::{Tool, agent_loop};
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{EventStream, Provider, ProviderError, Request};
use opi_ai::stream::AssistantStreamEvent;
use opi_ai::test_support::{single_route_collection, text_response, tool_call_response};

use common::{DenyingAuthorizer, RecordingTool, StaleGenerationAuthorizer};

// ---------------------------------------------------------------------------
// Scripted mock provider + default hooks
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
    fn stream(&self, _request: Request) -> EventStream {
        let events = self.responses.lock().unwrap().remove(0);
        Box::pin(stream::iter(events.into_iter().map(Ok::<_, ProviderError>)))
    }
}

struct TestHooks;
impl AgentHooks for TestHooks {
    fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
        Ok(messages
            .iter()
            .filter_map(|m| match m {
                AgentMessage::Llm(m) => Some(m.clone()),
                _ => None,
            })
            .collect())
    }
    fn should_stop_after_turn(
        &self,
        _ctx: ShouldStopAfterTurnContext,
    ) -> Pin<Box<dyn std::future::Future<Output = bool> + Send>> {
        Box::pin(async { false })
    }
    fn before_tool_call(
        &self,
        _ctx: BeforeToolCallContext,
    ) -> Pin<Box<dyn std::future::Future<Output = BeforeToolCallResult> + Send>> {
        Box::pin(async { BeforeToolCallResult::Continue })
    }
}

fn recording_registry(count: Arc<AtomicUsize>) -> Vec<RegisteredTool> {
    common::registrations_from(vec![Box::new(RecordingTool::new("mytool", count))])
}

fn make_agent(
    responses: Vec<Vec<AssistantStreamEvent>>,
    registrations: Vec<RegisteredTool>,
    authorizer: Option<Arc<dyn opi_agent::authority::ToolAuthorizer>>,
) -> Agent {
    let collection = Arc::new(single_route_collection(Box::new(MockProvider::new(
        responses,
    ))));
    Agent::new(
        collection,
        registrations,
        authorizer,
        "mock:mock-model".to_owned(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig {
            max_turns: 5,
            ..Default::default()
        },
        Box::new(TestHooks),
    )
    .expect("agent builds")
}

// ===========================================================================
// AUT-005 / OUT-003: missing, denying, or stale authority -> zero executions
// ===========================================================================

#[tokio::test]
async fn missing_authorizer_yields_zero_executions() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut agent = make_agent(
        vec![
            tool_call_response("c1", "mytool", "{}"),
            text_response("done"),
        ],
        recording_registry(count.clone()),
        None, // fail-closed: no authorizer bound
    );
    let _ = agent.prompt("go").await;
    assert_eq!(
        RecordingTool::count_of(&count),
        0,
        "missing authorizer must result in zero executions"
    );
}

#[tokio::test]
async fn permissive_authorizer_executes_once() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut agent = make_agent(
        vec![
            tool_call_response("c1", "mytool", "{}"),
            text_response("done"),
        ],
        recording_registry(count.clone()),
        Some(common::permissive_authorizer()),
    );
    let _ = agent.prompt("go").await;
    assert_eq!(
        RecordingTool::count_of(&count),
        1,
        "a current Allow reaches Tool::execute exactly once"
    );
}

#[tokio::test]
async fn denying_authorizer_yields_zero_executions() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut agent = make_agent(
        vec![
            tool_call_response("c1", "mytool", "{}"),
            text_response("done"),
        ],
        recording_registry(count.clone()),
        Some(Arc::new(DenyingAuthorizer)),
    );
    let _ = agent.prompt("go").await;
    assert_eq!(
        RecordingTool::count_of(&count),
        0,
        "an authorizer Deny must result in zero executions"
    );
}

// ===========================================================================
// AUT-001: every model-proposed call must resolve to one trusted registration
// ===========================================================================

#[tokio::test]
async fn unknown_tool_yields_zero_executions() {
    // The model proposes a name that is NOT in the trusted registry.
    let count = Arc::new(AtomicUsize::new(0));
    let mut agent = make_agent(
        vec![
            tool_call_response("c1", "nope", "{}"),
            text_response("done"),
        ],
        recording_registry(count.clone()),
        Some(common::permissive_authorizer()),
    );
    let _ = agent.prompt("go").await;
    assert_eq!(
        RecordingTool::count_of(&count),
        0,
        "an unregistered tool name must not execute"
    );
}

#[test]
fn duplicate_provider_visible_name_is_rejected_at_construction() {
    let count = Arc::new(AtomicUsize::new(0));
    let tool_impl: Arc<dyn Tool> = Arc::new(RecordingTool::new("dup", count));
    let def = tool_impl.definition();
    let make = || {
        RegisteredTool::new(
            RegistrationId::new("reg-dup"),
            "dup".to_owned(),
            ToolOrigin::Builtin,
            Capability::Builtin(CapabilityClass::WorkspaceRead),
            def.clone(),
            tool_impl.clone(),
        )
    };
    let collection = Arc::new(single_route_collection(Box::new(MockProvider::new(vec![]))));
    let result = Agent::new(
        collection,
        vec![make(), make()],
        Some(common::permissive_authorizer()),
        "mock:mock-model".to_owned(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig::default(),
        Box::new(TestHooks),
    );
    assert!(
        matches!(result, Err(AgentError::InvalidToolRegistration(_))),
        "duplicate provider-visible name must be rejected"
    );
}

// ===========================================================================
// AUT-003 (synthetic): a stale evidence-health generation yields zero exec
// ===========================================================================

#[tokio::test]
async fn stale_evidence_health_generation_yields_zero_executions() {
    // The run's current health is G1, but the authorizer stamps a FIXED stale
    // generation (INITIAL/G0) onto every Allow. The freshness gate detects the
    // mismatch, reauthorizes once (still stale), and denies with zero execution.
    // This is the synthetic stand-in for the 17.7 scenario where evidence
    // emission advances health after an Allow was computed.
    let count = Arc::new(AtomicUsize::new(0));
    let registry = Arc::new(
        opi_agent::authority::ToolRegistry::from_tools(recording_registry(count.clone()))
            .expect("distinct names"),
    );
    let collection = Arc::new(single_route_collection(Box::new(MockProvider::new(vec![
        tool_call_response("c1", "mytool", "{}"),
        text_response("done"),
    ]))));
    let context = AgentLoopContext {
        collection,
        registry,
        authorizer: Some(Arc::new(StaleGenerationAuthorizer::default())),
        evidence_health: EvidenceHealth::Healthy {
            generation: EvidenceGeneration::INITIAL.next(),
        },
        state: NextTurnState::new(
            vec![AgentMessage::Llm(Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "go".to_owned(),
                }],
                timestamp_ms: opi_ai::time::now_ms(),
            }))],
            ModelSelection::new("mock", "mock-model"),
            InferenceConfig::default(),
        ),
        system: None,
        steering_queue: None,
        follow_up_queue: None,
        diagnostic_sink: None,
        trace: None,
        session_id: None,
    };
    let cancel = tokio_util::sync::CancellationToken::new();
    let _ = agent_loop(
        context,
        AgentLoopConfig::default(),
        &TestHooks,
        Box::new(|_| {}),
        cancel,
    )
    .await;
    assert_eq!(
        RecordingTool::count_of(&count),
        0,
        "a stale evidence-health generation must deny execution"
    );
}
