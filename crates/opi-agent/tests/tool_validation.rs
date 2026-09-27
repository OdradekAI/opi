//! Behavioral tests for tool trait and schema validation (task 1.5).
//!
//! DoD: "invalid args become error tool result"

mod common;

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use futures_util::stream;
use opi_agent::ToolDef;
use opi_agent::authority::{
    AuthorizationDecision, AuthorizationError, ToolAuthorizationRequest, ToolAuthorizer,
};
use opi_agent::evidence::{PermissionReference, PermissionScope, PolicyReference};
use opi_agent::hooks::{
    AfterToolCallContext, AfterToolCallResult, AgentHooks, BeforeToolCallContext,
    BeforeToolCallResult, ShouldStopAfterTurnContext,
};
use opi_agent::loop_types::{
    AgentError, AgentLoopConfig, AgentLoopContext, InferenceConfig, ModelSelection, NextTurnState,
};
use opi_agent::message::AgentMessage;
use opi_agent::tool::{ExecutionMode, Tool, ToolError, ToolResult};
use opi_agent::validation::{self, ValidationError};
use opi_ai::message::{
    AssistantContent, AssistantMessage, InputContent, Message, OutputContent, ToolCall,
};
use opi_ai::provider::{EventStream, Provider, ProviderError, Request};
use opi_ai::stream::{AssistantStreamEvent, StopReason, Usage};
use opi_ai::test_support::single_route_collection;
use serde_json::json;
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Test tool implementations
// ---------------------------------------------------------------------------

/// A tool with a schema requiring a `name` string property.
struct GreetTool;

impl Tool for GreetTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "greet".into(),
            description: "Greet someone by name.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" }
                },
                "required": ["name"]
            }),
        }
    }

    fn execute(
        &self,
        _call_id: &str,
        arguments: serde_json::Value,
        _signal: CancellationToken,
        _on_update: Option<opi_agent::tool::UpdateCallback>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send>> {
        let name = arguments
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("world")
            .to_owned();
        Box::pin(async move {
            Ok(ToolResult {
                content: vec![opi_ai::message::OutputContent::Text {
                    text: format!("Hello, {name}!"),
                }],
                details: None,
                is_error: false,
                terminate: false,
                truncated: false,
                diagnostics: vec![],
            })
        })
    }
}

/// A tool with an empty object schema (accepts anything).
struct EchoTool;

impl Tool for EchoTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "echo".into(),
            description: "Echoes input.".into(),
            input_schema: json!({ "type": "object" }),
        }
    }

    fn execute(
        &self,
        _call_id: &str,
        _arguments: serde_json::Value,
        _signal: CancellationToken,
        _on_update: Option<opi_agent::tool::UpdateCallback>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send>> {
        Box::pin(async move {
            Ok(ToolResult {
                content: vec![opi_ai::message::OutputContent::Text {
                    text: "echo".into(),
                }],
                details: None,
                is_error: false,
                terminate: false,
                truncated: false,
                diagnostics: vec![],
            })
        })
    }

    fn execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Sequential
    }
}

// ---------------------------------------------------------------------------
// Schema validation
// ---------------------------------------------------------------------------

#[test]
fn valid_args_pass_validation() {
    let schema = json!({
        "type": "object",
        "properties": { "name": { "type": "string" } },
        "required": ["name"]
    });
    let args = json!({ "name": "Alice" });
    assert!(validation::validate(&schema, &args).is_ok());
}

#[test]
fn missing_required_field_fails_validation() {
    let schema = json!({
        "type": "object",
        "properties": { "name": { "type": "string" } },
        "required": ["name"]
    });
    let args = json!({});
    let result = validation::validate(&schema, &args);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(!err.errors.is_empty());
}

#[test]
fn wrong_type_fails_validation() {
    let schema = json!({
        "type": "object",
        "properties": { "name": { "type": "string" } },
        "required": ["name"]
    });
    let args = json!({ "name": 123 });
    let result = validation::validate(&schema, &args);
    assert!(result.is_err());
}

#[test]
fn validation_error_text_never_contains_the_offending_instance() {
    let canary = "HOSTILE_BACKEND_VALIDATION_CANARY";
    let schema = json!({
        "type": "object",
        "properties": {
            "backend": {
                "oneOf": [
                    { "const": "local" },
                    { "const": "opi-sandbox" }
                ]
            }
        },
        "required": ["backend"],
        "additionalProperties": false
    });
    let args = json!({ "backend": canary });

    let public = validation::validate(&schema, &args)
        .unwrap_err()
        .to_string();

    assert!(
        !public.contains(canary),
        "offending instance leaked: {public}"
    );
    assert!(public.contains("schema validation failed"));
}

#[test]
fn extra_properties_allowed_by_default() {
    let schema = json!({
        "type": "object",
        "properties": { "name": { "type": "string" } },
        "required": ["name"]
    });
    let args = json!({ "name": "Alice", "extra": true });
    assert!(validation::validate(&schema, &args).is_ok());
}

#[test]
fn empty_schema_accepts_any_object() {
    let schema = json!({ "type": "object" });
    let args = json!({ "anything": "goes" });
    assert!(validation::validate(&schema, &args).is_ok());
}

#[test]
fn empty_object_passes_empty_schema() {
    let schema = json!({ "type": "object" });
    let args = json!({});
    assert!(validation::validate(&schema, &args).is_ok());
}

// ---------------------------------------------------------------------------
// Validation → error ToolResult
// ---------------------------------------------------------------------------

#[test]
fn validation_error_produces_error_tool_result() {
    let err = ValidationError {
        errors: vec!["'name' is required".into()],
    };
    let result = ToolResult::from_validation_error(err);
    assert!(result.is_error);
    assert!(!result.terminate);
    let text = result.content.iter().find_map(|c| match c {
        opi_ai::message::OutputContent::Text { text } => Some(text.as_str()),
        _ => None,
    });
    assert!(text.is_some());
    assert!(text.unwrap().contains("'name' is required"));
}

// ---------------------------------------------------------------------------
// Tool definition
// ---------------------------------------------------------------------------

#[test]
fn tool_definition_returns_correct_schema() {
    let tool = GreetTool;
    let def = tool.definition();
    assert_eq!(def.name, "greet");
    assert_eq!(def.description, "Greet someone by name.");
    assert_eq!(def.input_schema["type"], "object");
    assert!(def.input_schema["required"].is_array());
}

// ---------------------------------------------------------------------------
// ExecutionMode
// ---------------------------------------------------------------------------

#[test]
fn default_execution_mode_is_parallel() {
    let tool = GreetTool;
    assert_eq!(tool.execution_mode(), ExecutionMode::Parallel);
}

#[test]
fn tool_can_override_execution_mode() {
    let tool = EchoTool;
    assert_eq!(tool.execution_mode(), ExecutionMode::Sequential);
}

// ---------------------------------------------------------------------------
// Full flow: validate then execute
// ---------------------------------------------------------------------------

#[tokio::test]
async fn valid_args_execute_successfully() {
    let tool = GreetTool;
    let args = json!({ "name": "World" });
    let schema = &tool.definition().input_schema;

    validation::validate(schema, &args).unwrap();

    let result = tool
        .execute("call-1", args, CancellationToken::new(), None)
        .await
        .unwrap();
    assert!(!result.is_error);
    let text = result.content.iter().find_map(|c| match c {
        opi_ai::message::OutputContent::Text { text } => Some(text.clone()),
        _ => None,
    });
    assert_eq!(text.unwrap(), "Hello, World!");
}

#[tokio::test]
async fn invalid_args_become_error_tool_result() {
    let tool = GreetTool;
    let args = json!({});
    let schema = &tool.definition().input_schema;

    let validation_result = validation::validate(schema, &args);
    assert!(validation_result.is_err());

    // Agent loop would convert validation error to error ToolResult
    let result = ToolResult::from_validation_error(validation_result.unwrap_err());
    assert!(result.is_error);
    assert!(!result.terminate);
}

// ---------------------------------------------------------------------------
// Phase 8: tool validation failure contract (task 8.3)
//
// Drives opi_agent::agent_loop with a tool call whose arguments fail schema
// validation and pins the contract: the failure is a normal runtime outcome
// (Ok return, an error ToolResult persisted, run continues), and Tool::execute
// never runs on invalid arguments. Under the Phase 17.4 invocation order the
// before_tool_call hook runs BEFORE schema validation, so the hook may observe
// schema-unvalidated args; the tool still never executes on invalid arguments.
// ---------------------------------------------------------------------------

struct ScriptedProvider {
    responses: Arc<Mutex<Vec<Vec<AssistantStreamEvent>>>>,
    call_count: Arc<Mutex<usize>>,
}

impl ScriptedProvider {
    fn new(responses: Vec<Vec<AssistantStreamEvent>>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses)),
            call_count: Arc::new(Mutex::new(0)),
        }
    }
}

impl Provider for ScriptedProvider {
    fn id(&self) -> &str {
        "scripted"
    }
    fn models(&self) -> &[opi_ai::provider::ModelInfo] {
        static MODELS: std::sync::OnceLock<Vec<opi_ai::provider::ModelInfo>> =
            std::sync::OnceLock::new();
        MODELS
            .get_or_init(|| {
                vec![opi_ai::provider::ModelInfo::new(
                    "mock",
                    "Mock",
                    opi_ai::WireApi::OpenAiCompletions,
                    opi_ai::ModelCapabilities::new(100_000, 4_096),
                )]
            })
            .as_slice()
    }
    fn stream_prepared(&self, _request: Request, _auth: opi_ai::auth::ResolvedAuth) -> EventStream {
        *self.call_count.lock().unwrap() += 1;
        let events = self.responses.lock().unwrap().remove(0);
        Box::pin(stream::iter(events.into_iter().map(Ok::<_, ProviderError>)))
    }
}

/// A tool that records whether `execute` was reached. Its schema requires a
/// `name` string so an empty-args call fails validation before execute.
struct ProbeTool {
    executed: Arc<Mutex<bool>>,
}

struct CountingAuthorizer {
    calls: Arc<AtomicUsize>,
}

impl ToolAuthorizer for CountingAuthorizer {
    fn authorize(
        &self,
        request: ToolAuthorizationRequest,
        _cancel: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<AuthorizationDecision, AuthorizationError>> + Send>>
    {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            Ok(AuthorizationDecision::Allow {
                policy_ref: PolicyReference::new("test-policy").unwrap(),
                permission_ref: PermissionReference::new("test-permission").unwrap(),
                permission_scope: PermissionScope::new("test-scope").unwrap(),
                scoped_grant_ref: None,
                registration_id: request.registration_id,
                capability: request.capability,
                evidence_health_generation: request.evidence_health.generation(),
            })
        })
    }
}

impl Tool for ProbeTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "greet".into(),
            description: "probe tool".into(),
            input_schema: json!({
                "type": "object",
                "properties": { "name": { "type": "string" } },
                "required": ["name"]
            }),
        }
    }

    fn execute(
        &self,
        _call_id: &str,
        _arguments: serde_json::Value,
        _signal: CancellationToken,
        _on_update: Option<opi_agent::tool::UpdateCallback>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send>> {
        let executed = self.executed.clone();
        Box::pin(async move {
            *executed.lock().unwrap() = true;
            Ok(ToolResult {
                content: vec![OutputContent::Text {
                    text: "execute must not run on validation failure".into(),
                }],
                details: None,
                is_error: false,
                terminate: false,
                truncated: false,
                diagnostics: vec![],
            })
        })
    }
}

/// A permissive tool that records whether malformed JSON reached execute.
struct PermissiveProbeTool {
    executed: Arc<Mutex<bool>>,
}

impl Tool for PermissiveProbeTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "echo".into(),
            description: "permissive probe tool".into(),
            input_schema: json!({ "type": "object" }),
        }
    }

    fn execute(
        &self,
        _call_id: &str,
        _arguments: serde_json::Value,
        _signal: CancellationToken,
        _on_update: Option<opi_agent::tool::UpdateCallback>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send>> {
        let executed = self.executed.clone();
        Box::pin(async move {
            *executed.lock().unwrap() = true;
            Ok(ToolResult {
                content: vec![OutputContent::Text {
                    text: "execute must not run on malformed arguments".into(),
                }],
                details: None,
                is_error: false,
                terminate: false,
                truncated: false,
                diagnostics: vec![],
            })
        })
    }

    fn execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Sequential
    }
}

/// A permissive tool that keeps the default parallel execution mode.
struct ParallelPermissiveProbeTool {
    executed: Arc<Mutex<bool>>,
}

impl Tool for ParallelPermissiveProbeTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "echo_parallel".into(),
            description: "parallel permissive probe tool".into(),
            input_schema: json!({ "type": "object" }),
        }
    }

    fn execute(
        &self,
        _call_id: &str,
        _arguments: serde_json::Value,
        _signal: CancellationToken,
        _on_update: Option<opi_agent::tool::UpdateCallback>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send>> {
        let executed = self.executed.clone();
        Box::pin(async move {
            *executed.lock().unwrap() = true;
            Ok(ToolResult {
                content: vec![OutputContent::Text {
                    text: "execute must not run on malformed arguments".into(),
                }],
                details: None,
                is_error: false,
                terminate: false,
                truncated: false,
                diagnostics: vec![],
            })
        })
    }
}

/// Hooks that record whether `before_tool_call` was reached.
struct ProbeHooks {
    before_called: Arc<Mutex<bool>>,
}

impl AgentHooks for ProbeHooks {
    fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
        Ok(messages
            .iter()
            .filter_map(|m| match m {
                AgentMessage::Llm(msg) => Some(msg.clone()),
                _ => None,
            })
            .collect())
    }

    fn should_stop_after_turn(
        &self,
        _: ShouldStopAfterTurnContext,
    ) -> Pin<Box<dyn Future<Output = bool> + Send>> {
        Box::pin(async { false })
    }

    fn before_tool_call(
        &self,
        _: BeforeToolCallContext,
    ) -> Pin<Box<dyn Future<Output = BeforeToolCallResult> + Send>> {
        let before_called = self.before_called.clone();
        Box::pin(async move {
            *before_called.lock().unwrap() = true;
            BeforeToolCallResult::Continue
        })
    }

    fn after_tool_call(
        &self,
        _: AfterToolCallContext,
    ) -> Pin<Box<dyn Future<Output = AfterToolCallResult> + Send>> {
        Box::pin(async { AfterToolCallResult::Keep })
    }
}

fn base_msg() -> AssistantMessage {
    AssistantMessage {
        content: vec![],
        api: opi_ai::ApiKind::Anthropic,
        provider: "scripted".into(),
        model: "mock".into(),
        response_model: None,
        response_id: None,
        usage: Usage::default(),
        stop_reason: StopReason::Stop,
        error_message: None,
        timestamp_ms: 0,
    }
}

fn text_response(text: &str) -> Vec<AssistantStreamEvent> {
    let mut partial = base_msg();
    partial
        .content
        .push(AssistantContent::Text { text: text.into() });
    vec![
        AssistantStreamEvent::Start {
            partial: base_msg(),
        },
        AssistantStreamEvent::TextDelta {
            content_index: 0,
            delta: text.into(),
            partial: partial.clone(),
        },
        AssistantStreamEvent::Done {
            reason: StopReason::Stop,
            message: partial,
        },
    ]
}

fn tool_call_response(call_id: &str, name: &str, args: &str) -> Vec<AssistantStreamEvent> {
    let tool_call = ToolCall {
        id: call_id.to_string(),
        name: name.to_string(),
        arguments: args.to_string(),
    };
    let mut partial = base_msg();
    partial.content.push(AssistantContent::ToolCall {
        tool_call: tool_call.clone(),
    });
    partial.stop_reason = StopReason::ToolUse;
    vec![
        AssistantStreamEvent::Start {
            partial: base_msg(),
        },
        AssistantStreamEvent::ToolCallEnd {
            content_index: 0,
            tool_call,
            partial: partial.clone(),
        },
        AssistantStreamEvent::Done {
            reason: StopReason::ToolUse,
            message: partial,
        },
    ]
}

// DoD: invalid arguments are surfaced through the production tool scheduler as
// a normal runtime outcome: the run returns Ok, an error ToolResult is
// persisted, the run continues to the next turn, and Tool::execute never runs.
// Under the Phase 17.4 invocation order the before_tool_call hook runs BEFORE
// schema validation, so the hook may observe schema-unvalidated args; the tool
// still never executes on invalid arguments.
#[tokio::test]
async fn phase8_tool_validation_failure_contract() {
    let executed = Arc::new(Mutex::new(false));
    let before_called = Arc::new(Mutex::new(false));
    let authorizer_calls = Arc::new(AtomicUsize::new(0));

    // greet requires `name`; this call omits it, so validation fails.
    let provider = ScriptedProvider::new(vec![
        tool_call_response("call-1", "greet", r#"{}"#),
        text_response("done"),
    ]);
    let call_count = provider.call_count.clone();

    let tools: Vec<Box<dyn Tool>> = vec![Box::new(ProbeTool {
        executed: executed.clone(),
    })];
    let hooks = ProbeHooks {
        before_called: before_called.clone(),
    };

    let context = AgentLoopContext {
        collection: Arc::new(single_route_collection(Box::new(provider))),
        registry: common::test_registry(tools),
        authorizer: Some(Arc::new(CountingAuthorizer {
            calls: authorizer_calls.clone(),
        })),
        evidence_health: opi_agent::evidence::EvidenceHealth::healthy(),
        state: NextTurnState::new(
            vec![AgentMessage::Llm(Message::User(
                opi_ai::message::UserMessage {
                    content: vec![InputContent::Text { text: "hi".into() }],
                    timestamp_ms: 0,
                },
            ))],
            ModelSelection::parse_spec("scripted:mock").unwrap(),
            InferenceConfig::default(),
        ),
        system: None,
        steering_queue: None,
        follow_up_queue: None,
        diagnostic_sink: None,
        session_id: None,
        evidence_sink: None,
    };

    let messages = opi_agent::agent_loop(
        context,
        AgentLoopConfig::default(),
        &hooks,
        Box::new(|_| {}),
        CancellationToken::new(),
    )
    .await
    .into_execution_result()
    .expect("validation failure is a normal runtime outcome, not a loop error");

    assert_eq!(
        *call_count.lock().unwrap(),
        2,
        "run continues past the validation failure to a second provider call"
    );
    assert!(
        !*executed.lock().unwrap(),
        "Tool::execute must not run when arguments fail validation"
    );
    assert!(
        *before_called.lock().unwrap(),
        "hook may observe schema-unvalidated args; the tool never executes on invalid arguments"
    );
    assert_eq!(
        authorizer_calls.load(Ordering::SeqCst),
        0,
        "invalid schema arguments must be rejected before trusted authorization"
    );

    let error_result = messages
        .context
        .iter()
        .find_map(|m| match m {
            AgentMessage::Llm(Message::ToolResult(trm)) if trm.tool_call_id == "call-1" => {
                Some(trm.clone())
            }
            _ => None,
        })
        .expect("validation failure produces a persisted tool result");

    assert!(
        error_result.is_error,
        "validation failure result is an error result"
    );
    let text = error_result.content.iter().find_map(|c| match c {
        OutputContent::Text { text } => Some(text.clone()),
        _ => None,
    });
    assert!(
        text.as_ref().is_some_and(|t| !t.is_empty()),
        "error result carries a non-empty message: {text:?}"
    );
}

#[tokio::test]
async fn phase8_malformed_tool_arguments_do_not_execute_permissive_tool() {
    use opi_agent::diagnostic::code::CODE_TOOL_VALIDATION_FAILED;
    use opi_agent::diagnostic_sink::RecordingSink;
    use opi_agent::event::AgentEvent;

    let executed = Arc::new(Mutex::new(false));
    let before_called = Arc::new(Mutex::new(false));
    let diagnostic_sink = Arc::new(RecordingSink::new());
    let start_args = Arc::new(Mutex::new(Vec::new()));
    let end_errors = Arc::new(Mutex::new(Vec::new()));

    let provider = ScriptedProvider::new(vec![
        tool_call_response("call-1", "echo", "{not-json"),
        text_response("done"),
    ]);
    let call_count = provider.call_count.clone();

    let tools: Vec<Box<dyn Tool>> = vec![Box::new(PermissiveProbeTool {
        executed: executed.clone(),
    })];
    let hooks = ProbeHooks {
        before_called: before_called.clone(),
    };

    let context = AgentLoopContext {
        collection: Arc::new(single_route_collection(Box::new(provider))),
        registry: common::test_registry(tools),
        authorizer: Some(common::permissive_authorizer()),
        evidence_health: opi_agent::evidence::EvidenceHealth::healthy(),
        state: NextTurnState::new(
            vec![AgentMessage::Llm(Message::User(
                opi_ai::message::UserMessage {
                    content: vec![InputContent::Text { text: "hi".into() }],
                    timestamp_ms: 0,
                },
            ))],
            ModelSelection::parse_spec("scripted:mock").unwrap(),
            InferenceConfig::default(),
        ),
        system: None,
        steering_queue: None,
        follow_up_queue: None,
        diagnostic_sink: Some(diagnostic_sink.clone()),
        session_id: None,
        evidence_sink: None,
    };

    let messages = opi_agent::agent_loop(
        context,
        AgentLoopConfig::default(),
        &hooks,
        Box::new({
            let start_args = start_args.clone();
            let end_errors = end_errors.clone();
            move |event| match event {
                AgentEvent::ToolExecutionStart { args, .. } => {
                    start_args.lock().unwrap().push(args);
                }
                AgentEvent::ToolExecutionEnd { is_error, .. } => {
                    end_errors.lock().unwrap().push(is_error);
                }
                _ => {}
            }
        }),
        CancellationToken::new(),
    )
    .await
    .into_execution_result()
    .expect("malformed tool arguments are a normal runtime outcome");

    assert_eq!(*call_count.lock().unwrap(), 2);
    assert!(!*executed.lock().unwrap());
    assert!(!*before_called.lock().unwrap());
    assert_eq!(
        start_args.lock().unwrap().as_slice(),
        &[serde_json::Value::Null]
    );
    assert_eq!(end_errors.lock().unwrap().as_slice(), &[true]);
    assert!(
        diagnostic_sink
            .snapshot()
            .iter()
            .any(|d| d.code == CODE_TOOL_VALIDATION_FAILED)
    );

    let error_result = messages
        .context
        .iter()
        .find_map(|m| match m {
            AgentMessage::Llm(Message::ToolResult(trm)) if trm.tool_call_id == "call-1" => {
                Some(trm.clone())
            }
            _ => None,
        })
        .expect("malformed arguments produce a persisted tool result");
    assert!(error_result.is_error);
    assert!(error_result.content.iter().any(
        |c| matches!(c, OutputContent::Text { text } if text.contains("tool arguments were not valid JSON"))
    ));
}

// ---------------------------------------------------------------------------
// Phase 12 task 12.4 — malformed tool arguments become a structured result.
//
// DoD: "Malformed tool arguments reach agent/runtime validation through the
// opi-agent tool validation path as structured errors or diagnostics, not
// panics or provider-level crashes." Drives the production agent_loop with a
// tool call whose arguments are invalid JSON and pins the structured outcome:
// the run returns Ok (no panic), Tool::execute never runs, a tool-validation
// diagnostic is emitted, and a persisted error ToolResult carries the
// validation message. Named to satisfy the acceptance scenario's verification
// filter `malformed_tool_arguments_result`.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn malformed_tool_arguments_result_is_structured_error_not_panic() {
    use opi_agent::diagnostic::code::CODE_TOOL_VALIDATION_FAILED;
    use opi_agent::diagnostic_sink::RecordingSink;

    let executed = Arc::new(Mutex::new(false));
    let diagnostic_sink = Arc::new(RecordingSink::new());

    let provider = ScriptedProvider::new(vec![
        tool_call_response("call-1", "echo", "{not-json"),
        text_response("done"),
    ]);
    let tools: Vec<Box<dyn Tool>> = vec![Box::new(PermissiveProbeTool {
        executed: executed.clone(),
    })];
    let hooks = ProbeHooks {
        before_called: Arc::new(Mutex::new(false)),
    };

    let context = AgentLoopContext {
        collection: Arc::new(single_route_collection(Box::new(provider))),
        registry: common::test_registry(tools),
        authorizer: Some(common::permissive_authorizer()),
        evidence_health: opi_agent::evidence::EvidenceHealth::healthy(),
        state: NextTurnState::new(
            vec![AgentMessage::Llm(Message::User(
                opi_ai::message::UserMessage {
                    content: vec![InputContent::Text { text: "hi".into() }],
                    timestamp_ms: 0,
                },
            ))],
            ModelSelection::parse_spec("scripted:mock").unwrap(),
            InferenceConfig::default(),
        ),
        system: None,
        steering_queue: None,
        follow_up_queue: None,
        diagnostic_sink: Some(diagnostic_sink.clone()),
        session_id: None,
        evidence_sink: None,
    };

    let messages = opi_agent::agent_loop(
        context,
        AgentLoopConfig::default(),
        &hooks,
        Box::new(|_| {}),
        CancellationToken::new(),
    )
    .await
    .into_execution_result()
    .expect("malformed arguments are a normal runtime outcome, not a panic");

    assert!(!*executed.lock().unwrap(), "Tool::execute must not run");
    assert!(
        diagnostic_sink
            .snapshot()
            .iter()
            .any(|d| d.code == CODE_TOOL_VALIDATION_FAILED),
        "malformed arguments must emit a tool-validation diagnostic"
    );

    let error_result = messages
        .context
        .iter()
        .find_map(|m| match m {
            AgentMessage::Llm(Message::ToolResult(trm)) if trm.tool_call_id == "call-1" => {
                Some(trm.clone())
            }
            _ => None,
        })
        .expect("malformed arguments produce a persisted tool result");
    assert!(error_result.is_error, "result is a structured error");
    assert!(
        error_result.content.iter().any(|c| matches!(
            c,
            OutputContent::Text { text } if text.contains("tool arguments were not valid JSON")
        )),
        "error result carries the structured validation message"
    );
}

#[tokio::test]
async fn phase8_malformed_tool_arguments_do_not_execute_parallel_permissive_tool() {
    use opi_agent::diagnostic::code::CODE_TOOL_VALIDATION_FAILED;
    use opi_agent::diagnostic_sink::RecordingSink;
    use opi_agent::event::AgentEvent;

    let executed = Arc::new(Mutex::new(false));
    let before_called = Arc::new(Mutex::new(false));
    let diagnostic_sink = Arc::new(RecordingSink::new());
    let start_args = Arc::new(Mutex::new(Vec::new()));
    let end_errors = Arc::new(Mutex::new(Vec::new()));

    let provider = ScriptedProvider::new(vec![
        tool_call_response("call-1", "echo_parallel", "{not-json"),
        text_response("done"),
    ]);
    let call_count = provider.call_count.clone();

    let tools: Vec<Box<dyn Tool>> = vec![Box::new(ParallelPermissiveProbeTool {
        executed: executed.clone(),
    })];
    let hooks = ProbeHooks {
        before_called: before_called.clone(),
    };

    let context = AgentLoopContext {
        collection: Arc::new(single_route_collection(Box::new(provider))),
        registry: common::test_registry(tools),
        authorizer: Some(common::permissive_authorizer()),
        evidence_health: opi_agent::evidence::EvidenceHealth::healthy(),
        state: NextTurnState::new(
            vec![AgentMessage::Llm(Message::User(
                opi_ai::message::UserMessage {
                    content: vec![InputContent::Text { text: "hi".into() }],
                    timestamp_ms: 0,
                },
            ))],
            ModelSelection::parse_spec("scripted:mock").unwrap(),
            InferenceConfig::default(),
        ),
        system: None,
        steering_queue: None,
        follow_up_queue: None,
        diagnostic_sink: Some(diagnostic_sink.clone()),
        session_id: None,
        evidence_sink: None,
    };

    let messages = opi_agent::agent_loop(
        context,
        AgentLoopConfig::default(),
        &hooks,
        Box::new({
            let start_args = start_args.clone();
            let end_errors = end_errors.clone();
            move |event| match event {
                AgentEvent::ToolExecutionStart { args, .. } => {
                    start_args.lock().unwrap().push(args);
                }
                AgentEvent::ToolExecutionEnd { is_error, .. } => {
                    end_errors.lock().unwrap().push(is_error);
                }
                _ => {}
            }
        }),
        CancellationToken::new(),
    )
    .await
    .into_execution_result()
    .expect("malformed tool arguments are a normal runtime outcome");

    assert_eq!(*call_count.lock().unwrap(), 2);
    assert!(!*executed.lock().unwrap());
    assert!(!*before_called.lock().unwrap());
    assert_eq!(
        start_args.lock().unwrap().as_slice(),
        &[serde_json::Value::Null]
    );
    assert_eq!(end_errors.lock().unwrap().as_slice(), &[true]);
    assert!(
        diagnostic_sink
            .snapshot()
            .iter()
            .any(|d| d.code == CODE_TOOL_VALIDATION_FAILED)
    );

    let error_result = messages
        .context
        .iter()
        .find_map(|m| match m {
            AgentMessage::Llm(Message::ToolResult(trm)) if trm.tool_call_id == "call-1" => {
                Some(trm.clone())
            }
            _ => None,
        })
        .expect("malformed arguments produce a persisted tool result");
    assert!(error_result.is_error);
    assert!(error_result.content.iter().any(
        |c| matches!(c, OutputContent::Text { text } if text.contains("tool arguments were not valid JSON"))
    ));
}

struct LengthProbeTool {
    mode: ExecutionMode,
    executions: Arc<AtomicUsize>,
}

impl Tool for LengthProbeTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "length_probe".into(),
            description: "Count actual tool executions.".into(),
            input_schema: json!({
                "type": "object",
                "properties": { "value": { "type": "string" } },
                "required": ["value"]
            }),
        }
    }

    fn execution_mode(&self) -> ExecutionMode {
        self.mode
    }

    fn execute(
        &self,
        _call_id: &str,
        _arguments: serde_json::Value,
        _signal: CancellationToken,
        _on_update: Option<opi_agent::tool::UpdateCallback>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send>> {
        self.executions.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {
            Ok(ToolResult {
                content: vec![OutputContent::Text {
                    text: "executed".into(),
                }],
                details: None,
                is_error: false,
                terminate: false,
                truncated: false,
                diagnostics: vec![],
            })
        })
    }
}

#[derive(Default)]
struct LengthProbeHooks {
    before: AtomicUsize,
    after: AtomicUsize,
    prepare: AtomicUsize,
    completed: Mutex<Vec<ShouldStopAfterTurnContext>>,
    stop: bool,
}

impl AgentHooks for LengthProbeHooks {
    fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
        Ok(messages
            .iter()
            .filter_map(|message| match message {
                AgentMessage::Llm(message) => Some(message.clone()),
                _ => None,
            })
            .collect())
    }

    fn before_tool_call(
        &self,
        _: BeforeToolCallContext,
    ) -> Pin<Box<dyn Future<Output = BeforeToolCallResult> + Send>> {
        self.before.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { BeforeToolCallResult::Continue })
    }

    fn after_tool_call(
        &self,
        _: AfterToolCallContext,
    ) -> Pin<Box<dyn Future<Output = AfterToolCallResult> + Send>> {
        self.after.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { AfterToolCallResult::Keep })
    }

    fn should_stop_after_turn(
        &self,
        context: ShouldStopAfterTurnContext,
    ) -> Pin<Box<dyn Future<Output = bool> + Send>> {
        self.completed.lock().unwrap().push(context);
        let stop = self.stop;
        Box::pin(async move { stop })
    }

    fn prepare_next_turn(
        &self,
        _: opi_agent::hooks::PrepareNextTurnContext,
    ) -> Pin<Box<dyn Future<Output = Result<Option<NextTurnState>, AgentError>> + Send>> {
        self.prepare.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Ok(None) })
    }
}

fn length_response(arguments: &[&str]) -> Vec<AssistantStreamEvent> {
    let mut message = base_msg();
    message.stop_reason = StopReason::Length;
    message.content = arguments
        .iter()
        .enumerate()
        .map(|(index, arguments)| AssistantContent::ToolCall {
            tool_call: ToolCall {
                id: format!("partial-{index}"),
                name: "length_probe".into(),
                arguments: (*arguments).into(),
            },
        })
        .collect();
    vec![
        AssistantStreamEvent::Start {
            partial: base_msg(),
        },
        AssistantStreamEvent::Done {
            reason: StopReason::Length,
            message,
        },
    ]
}

fn length_context(
    provider: ScriptedProvider,
    mode: ExecutionMode,
    executions: Arc<AtomicUsize>,
    authorizations: Arc<AtomicUsize>,
) -> AgentLoopContext {
    AgentLoopContext {
        collection: Arc::new(single_route_collection(Box::new(provider))),
        registry: common::test_registry(vec![Box::new(LengthProbeTool { mode, executions })]),
        authorizer: Some(Arc::new(CountingAuthorizer {
            calls: authorizations,
        })),
        evidence_health: opi_agent::evidence::EvidenceHealth::healthy(),
        state: NextTurnState::new(
            Vec::new(),
            ModelSelection::new("scripted", "mock"),
            InferenceConfig::default(),
        ),
        system: None,
        steering_queue: None,
        follow_up_queue: None,
        diagnostic_sink: None,
        session_id: None,
        evidence_sink: None,
    }
}

async fn check_length_tool_batch(mode: ExecutionMode, final_arguments: &str) {
    use opi_agent::event::AgentEvent;

    let provider = ScriptedProvider::new(vec![
        length_response(&[r#"{"value":"complete-looking"}"#, final_arguments]),
        tool_call_response("fresh", "length_probe", r#"{"value":"complete"}"#),
        text_response("done"),
    ]);
    let provider_calls = provider.call_count.clone();
    let executions = Arc::new(AtomicUsize::new(0));
    let authorizations = Arc::new(AtomicUsize::new(0));
    let hooks = LengthProbeHooks::default();
    let events = Arc::new(Mutex::new(Vec::new()));
    let result = opi_agent::agent_loop(
        length_context(provider, mode, executions.clone(), authorizations.clone()),
        AgentLoopConfig::default(),
        &hooks,
        Box::new({
            let events = events.clone();
            move |event| events.lock().unwrap().push(event)
        }),
        CancellationToken::new(),
    )
    .await;

    assert!(result.error().is_none(), "{result:?}");
    assert_eq!(
        executions.load(Ordering::SeqCst),
        1,
        "only the fresh call executes"
    );
    assert_eq!(authorizations.load(Ordering::SeqCst), 1);
    assert_eq!(hooks.before.load(Ordering::SeqCst), 1);
    assert_eq!(hooks.after.load(Ordering::SeqCst), 1);
    assert_eq!(*provider_calls.lock().unwrap(), 3);
    assert_eq!(hooks.prepare.load(Ordering::SeqCst), 2);
    let completed = hooks.completed.lock().unwrap();
    assert_eq!(completed.len(), 3);
    let rejected = &completed[0].tool_results;
    assert_eq!(rejected.len(), 2);
    for (index, result) in rejected.iter().enumerate() {
        assert_eq!(result.tool_call_id, format!("partial-{index}"));
        assert!(result.is_error);
        assert!(!result.truncated, "the synthetic failure text is complete");
        assert!(result.content.iter().any(|content| matches!(
            content,
            OutputContent::Text { text } if text.contains("output token limit") && text.contains("not executed")
        )));
    }
    assert!(!completed[1].tool_results[0].is_error);
    let batch_events: Vec<String> = events
        .lock()
        .unwrap()
        .iter()
        .filter_map(|event| match event {
            AgentEvent::ToolExecutionStart { tool_call_id, .. }
                if tool_call_id.starts_with("partial-") =>
            {
                Some(format!("start:{tool_call_id}"))
            }
            AgentEvent::ToolExecutionEnd {
                tool_call_id,
                is_error,
                ..
            } if tool_call_id.starts_with("partial-") => {
                assert!(*is_error);
                Some(format!("end:{tool_call_id}"))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        batch_events,
        [
            "start:partial-0",
            "end:partial-0",
            "start:partial-1",
            "end:partial-1"
        ]
    );
}

#[tokio::test]
async fn length_sequential_batch_never_executes_complete_looking_calls() {
    check_length_tool_batch(ExecutionMode::Sequential, r#"{"value":"salvaged"}"#).await;
}

#[tokio::test]
async fn length_parallel_batch_never_executes_complete_looking_calls() {
    check_length_tool_batch(ExecutionMode::Parallel, r#"{"value":"salvaged"}"#).await;
}

#[tokio::test]
async fn length_sequential_batch_rejects_earlier_valid_call_with_malformed_tail() {
    check_length_tool_batch(ExecutionMode::Sequential, r#"{"value":"cut"#).await;
}

#[tokio::test]
async fn length_parallel_batch_rejects_earlier_valid_call_with_malformed_tail() {
    check_length_tool_batch(ExecutionMode::Parallel, r#"{"value":"cut"#).await;
}

#[tokio::test]
async fn length_without_tools_does_not_create_continuation() {
    let provider = ScriptedProvider::new(vec![length_response(&[])]);
    let provider_calls = provider.call_count.clone();
    let hooks = LengthProbeHooks::default();
    let result = opi_agent::agent_loop(
        length_context(
            provider,
            ExecutionMode::Parallel,
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
        ),
        AgentLoopConfig::default(),
        &hooks,
        Box::new(|_| {}),
        CancellationToken::new(),
    )
    .await;
    assert!(result.error().is_none(), "{result:?}");
    assert_eq!(*provider_calls.lock().unwrap(), 1);
    assert_eq!(hooks.prepare.load(Ordering::SeqCst), 0);
    assert!(hooks.completed.lock().unwrap()[0].tool_results.is_empty());
    assert!(
        matches!(result.state().context.last(), Some(AgentMessage::Llm(Message::Assistant(message))) if message.stop_reason == StopReason::Length)
    );
}

#[tokio::test]
async fn length_tool_failures_reach_stop_hook_before_queues_and_preparation() {
    use std::collections::VecDeque;

    let provider = ScriptedProvider::new(vec![length_response(&[r#"{"value":"salvaged"}"#])]);
    let provider_calls = provider.call_count.clone();
    let executions = Arc::new(AtomicUsize::new(0));
    let authorizations = Arc::new(AtomicUsize::new(0));
    let mut context = length_context(
        provider,
        ExecutionMode::Parallel,
        executions.clone(),
        authorizations.clone(),
    );
    let follow_up = Arc::new(Mutex::new(VecDeque::from(["queued".into()])));
    context.follow_up_queue = Some(follow_up.clone());
    let hooks = LengthProbeHooks {
        stop: true,
        ..LengthProbeHooks::default()
    };
    let result = opi_agent::agent_loop(
        context,
        AgentLoopConfig::default(),
        &hooks,
        Box::new(|_| {}),
        CancellationToken::new(),
    )
    .await;
    assert!(result.error().is_none(), "{result:?}");
    assert_eq!(executions.load(Ordering::SeqCst), 0);
    assert_eq!(authorizations.load(Ordering::SeqCst), 0);
    assert_eq!(hooks.before.load(Ordering::SeqCst), 0);
    assert_eq!(hooks.after.load(Ordering::SeqCst), 0);
    assert_eq!(hooks.prepare.load(Ordering::SeqCst), 0);
    assert_eq!(*provider_calls.lock().unwrap(), 1);
    assert_eq!(follow_up.lock().unwrap().len(), 1);
    let completed = hooks.completed.lock().unwrap();
    assert_eq!(completed.len(), 1);
    assert!(completed[0].tool_results[0].is_error);
}
