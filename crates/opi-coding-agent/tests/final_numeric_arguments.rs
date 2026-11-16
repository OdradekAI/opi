//! Real Agent dispatch and disk session recovery of final numeric arguments.
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use opi_agent::authority::{
    AuthorizationDecision, AuthorizationError, CapabilityIdentity, RegisteredTool, RegistrationId,
    ToolAuthorizationRequest, ToolAuthorizer, ToolOrigin, ToolRegistry,
};
use opi_agent::event::AgentEvent;
use opi_agent::evidence::{PermissionReference, PermissionScope, PolicyReference};
use opi_agent::hooks::{AgentHooks, BeforeToolCallContext, BeforeToolCallResult};
use opi_agent::loop_types::{
    AgentError, AgentLoopConfig, AgentLoopContext, InferenceConfig, ModelSelection, NextTurnState,
};
use opi_agent::message::AgentMessage;
use opi_agent::session::SessionReader;
use opi_agent::tool::{Tool, ToolError, ToolResult, UpdateCallback};
use opi_ai::message::{AssistantContent, InputContent, Message, ToolDef, UserMessage};
use opi_ai::test_support::{self, MockProvider};
use opi_coding_agent::session_coordinator::SessionCoordinator;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

const EXPECTED: u64 = 0x4d38_4f03_e93f_f9f5;
type Observations = Arc<Mutex<Vec<(String, Value)>>>;

fn raw_arguments() -> String {
    format!(r#"{{"n":1{},"password":"numeric-canary"}}"#, "0".repeat(64))
}

fn definition(bounded: bool) -> ToolDef {
    let number_schema = if bounded {
        json!({"type":"number","minimum":1e64,"maximum":1e64})
    } else {
        json!({"type":"number"})
    };
    ToolDef {
        name: "record_number".into(),
        description: "Record numeric arguments without external effects".into(),
        input_schema: json!({"type":"object","properties":{
            "n":number_schema,
            "password":{"type":"string"}},"required":["n"],"additionalProperties":false}),
        constrained_sampling: None,
    }
}

struct Recorder(Observations, bool);

impl Tool for Recorder {
    fn definition(&self) -> ToolDef {
        definition(self.1)
    }

    fn execute(
        &self,
        _: &str,
        arguments: Value,
        _: CancellationToken,
        _: Option<UpdateCallback>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send>> {
        self.0.lock().unwrap().push(("execute".into(), arguments));
        Box::pin(async { Ok(opi_agent::tool::result::ok(vec![], json!({}))) })
    }
}

impl AgentHooks for Recorder {
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
        context: BeforeToolCallContext,
    ) -> Pin<Box<dyn Future<Output = BeforeToolCallResult> + Send>> {
        self.0.lock().unwrap().push(("hook".into(), context.args));
        Box::pin(async { BeforeToolCallResult::Continue })
    }
}

impl ToolAuthorizer for Recorder {
    fn authorize(
        &self,
        request: ToolAuthorizationRequest,
        _: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<AuthorizationDecision, AuthorizationError>> + Send>>
    {
        self.0
            .lock()
            .unwrap()
            .push(("authorize".into(), request.arguments.clone()));
        Box::pin(async move {
            Ok(AuthorizationDecision::Allow {
                policy_ref: PolicyReference::new("numeric-policy").unwrap(),
                permission_ref: PermissionReference::new("numeric-permission").unwrap(),
                permission_scope: PermissionScope::new("numeric-scope").unwrap(),
                scoped_grant_ref: None,
                registration_id: request.registration_id,
                capability: request.capability,
                evidence_health_generation: request.evidence_health.generation(),
            })
        })
    }
}

fn user() -> AgentMessage {
    AgentMessage::Llm(Message::User(UserMessage {
        content: vec![InputContent::Text {
            text: "record the supplied number".into(),
        }],
        timestamp_ms: 0,
    }))
}

fn calls(messages: &[AgentMessage]) -> Vec<(String, String)> {
    messages
        .iter()
        .filter_map(|m| match m {
            AgentMessage::Llm(Message::Assistant(a)) => Some(a),
            _ => None,
        })
        .flat_map(|a| a.content.iter())
        .filter_map(|c| match c {
            AssistantContent::ToolCall { tool_call } => {
                Some((tool_call.id.clone(), tool_call.arguments.clone()))
            }
            _ => None,
        })
        .collect()
}

async fn run(
    messages: Vec<AgentMessage>,
    raw: &str,
    id: &str,
    bounded: bool,
) -> (Vec<AgentMessage>, Vec<Value>) {
    let observations = Observations::default();
    let registry = ToolRegistry::from_tools(vec![RegisteredTool::new(
        RegistrationId::new("numeric-registration"),
        "record_number".into(),
        ToolOrigin::Builtin,
        CapabilityIdentity::new("opi.workspace.read").unwrap(),
        definition(bounded),
        Arc::new(Recorder(observations.clone(), bounded)),
    )])
    .unwrap();
    let provider = MockProvider::new(
        "mock",
        vec![
            test_support::tool_call_response(id, "record_number", raw),
            test_support::text_response("done"),
        ],
    );
    let provider_calls = provider.call_log_handle();
    let context = AgentLoopContext {
        collection: Arc::new(test_support::single_route_collection(Box::new(provider))),
        registry: Arc::new(registry),
        authorizer: Some(Arc::new(Recorder(observations.clone(), bounded))),
        evidence_health: opi_agent::evidence::EvidenceHealth::healthy(),
        evidence_sink: None,
        state: NextTurnState::new(
            messages.clone(),
            ModelSelection::new("mock", "mock-model"),
            InferenceConfig::default(),
        ),
        system: None,
        steering_queue: None,
        follow_up_queue: None,
        diagnostic_sink: None,
        session_id: None,
    };
    let event_observations = observations.clone();
    let state = opi_agent::agent_loop(
        context,
        AgentLoopConfig::default(),
        &Recorder(observations.clone(), bounded),
        Box::new(move |event| {
            if let AgentEvent::ToolExecutionStart { args, .. } = event {
                event_observations
                    .lock()
                    .unwrap()
                    .push(("event".into(), args));
            }
        }),
        CancellationToken::new(),
    )
    .await
    .into_execution_result()
    .unwrap();
    // Restored historical calls reach the next model request, but never execute again.
    let seen = provider_calls.lock().unwrap();
    assert_eq!(seen.len(), 2);
    let request_messages = seen[0]
        .context
        .messages()
        .iter()
        .cloned()
        .map(AgentMessage::Llm)
        .collect::<Vec<_>>();
    assert_eq!(calls(&request_messages), calls(&messages));
    drop(seen);
    let observed = observations.lock().unwrap();
    let mut rows = Vec::new();
    for stage in ["hook", "authorize", "execute", "event"] {
        let values = observed
            .iter()
            .filter(|(name, _)| name == stage)
            .collect::<Vec<_>>();
        assert_eq!(values.len(), 1, "{stage}: {observed:?}");
        let bits = values[0].1["n"].as_f64().unwrap().to_bits();
        assert_eq!(bits, EXPECTED, "{stage}: {observed:?}");
        rows.push(json!({"stage":stage,"bits":format!("{bits:016x}")}));
    }
    assert_eq!(
        calls(&state.context).last().unwrap(),
        &(id.into(), raw.into())
    );
    (state.context, rows)
}

#[tokio::test]
async fn final_number_reaches_hook_schema_authorization_and_tool() {
    let (_, rows) = run(vec![user()], &raw_arguments(), "initial", false).await;
    println!("\n{}", json!({"agent_final_number":rows}));
}

#[tokio::test]
async fn persisted_number_recovers_and_executes_without_another_rounding() {
    let dir = tempfile::tempdir().unwrap();
    let config = opi_agent::compaction::CompactionConfig {
        enabled: false,
        ..Default::default()
    };
    // Persist the raw model message independently so this regression detects
    // session decoding even if initial Agent dispatch also fails its test.
    let raw = raw_arguments();
    let response = test_support::tool_call_response("historical", "record_number", &raw);
    let assistant = response
        .iter()
        .find_map(|event| match event {
            opi_ai::stream::AssistantStreamEvent::Done { message, .. } => Some(message.clone()),
            _ => None,
        })
        .unwrap();
    let messages = vec![user(), AgentMessage::Llm(Message::Assistant(assistant))];
    let mut coordinator = SessionCoordinator::new_for_test(
        dir.path(),
        dir.path().to_str().unwrap(),
        config.clone(),
        "mock:mock-model",
    )
    .unwrap();
    coordinator
        .on_turn_end(&messages, &Default::default(), 0)
        .unwrap();
    let path = coordinator.session_path().to_path_buf();
    let session_id = coordinator.session_id().to_owned();
    drop(coordinator);
    let before = std::fs::read(&path).unwrap();
    assert!(!String::from_utf8_lossy(&before).contains("numeric-canary"));
    let (_, entries, recovery) = SessionReader::read_with_recovery(&path).unwrap();
    assert!(recovery.is_clean());
    let restored = opi_agent::session_context::reconstruct_context(&entries, &recovery);
    let restored_calls = calls(&restored.messages);
    assert_eq!(restored_calls.len(), 1);
    let persisted_raw = &restored_calls[0].1;
    assert_eq!(
        persisted_raw,
        &json!({"n":1e64,"password":"[REDACTED]"}).to_string()
    );
    let value: Value = serde_json::from_str(persisted_raw).unwrap();
    assert_eq!(
        value["n"].as_f64().unwrap().to_bits(),
        EXPECTED,
        "session decoding"
    );
    assert_eq!(value["password"], "[REDACTED]");
    let prior = restored.messages.len();
    let mut coordinator = SessionCoordinator::open_existing(
        path.clone(),
        session_id,
        &entries,
        prior,
        config,
        "mock:mock-model",
    )
    .unwrap();
    assert_eq!(
        std::fs::read(&path).unwrap(),
        before,
        "opening must not rewrite history"
    );
    let mut next_messages = restored.messages;
    next_messages.push(user());
    let (next, rows) = run(next_messages, persisted_raw, "after-resume", true).await;
    coordinator
        .on_turn_end(&next[prior..], &Default::default(), prior)
        .unwrap();
    drop(coordinator);
    assert!(std::fs::read(&path).unwrap().starts_with(&before));
    let (_, entries, recovery) = SessionReader::read_with_recovery(&path).unwrap();
    let again = opi_agent::session_context::reconstruct_context(&entries, &recovery);
    let final_calls = calls(&again.messages);
    assert_eq!(final_calls.len(), 2);
    assert!(final_calls.iter().all(|(_, text)| text == persisted_raw));
    println!(
        "\n{}",
        json!({"session_final_number":{"persisted":persisted_raw,"restored_calls":final_calls.len(),"next_execution":rows}})
    );
}

#[tokio::test]
async fn exact_numeric_schema_admits_the_correctly_rounded_final_number() {
    let (_, rows) = run(vec![user()], &raw_arguments(), "bounded", true).await;
    println!("\n{}", json!({"bounded_agent_final_number":rows}));
}
