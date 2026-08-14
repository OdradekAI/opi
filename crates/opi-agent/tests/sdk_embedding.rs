//! SDK embedding surface tests (task 4.2).
//!
//! Tests drive the SDK with MockProvider through prompt, continue,
//! session, model, thinking, compaction, and cancellation flows.
//! No live provider network access required.

mod common;

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use opi_agent::Agent;
use opi_agent::event::AgentEvent;
use opi_agent::hooks::AgentHooks;
use opi_agent::loop_types::{AgentError, AgentLoopConfig, InferenceConfig};
use opi_agent::sdk::{SDK_SCHEMA_VERSION, SdkCommand, SdkResponse, agent_event_to_value};
use opi_agent::tool::{ExecutionMode, Tool, ToolError, ToolResult};
use opi_ai::message::{OutputContent, ToolDef};
use opi_ai::test_support::{
    MockProvider, single_route_collection, text_response, tool_call_response,
};
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

struct NoopTool;

impl Tool for NoopTool {
    fn definition(&self) -> ToolDef {
        serde_json::from_value(serde_json::json!({
            "name": "noop",
            "description": "does nothing",
            "input_schema": { "type": "object", "properties": {} }
        }))
        .unwrap()
    }

    fn execute(
        &self,
        _call_id: &str,
        _arguments: serde_json::Value,
        _signal: CancellationToken,
        _on_update: Option<opi_agent::tool::UpdateCallback>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send>> {
        Box::pin(async {
            Ok(ToolResult {
                content: vec![OutputContent::Text { text: "ok".into() }],
                details: None,
                is_error: false,
                terminate: false,
                truncated: false,
                diagnostics: vec![],
            })
        })
    }

    fn execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Parallel
    }
}

struct NoopHooks;

impl AgentHooks for NoopHooks {
    fn convert_to_llm(
        &self,
        messages: &[opi_agent::message::AgentMessage],
    ) -> Result<Vec<opi_ai::message::Message>, AgentError> {
        // Passthrough: forward all Llm messages, drop internal variants.
        Ok(messages
            .iter()
            .filter_map(|m| match m {
                opi_agent::message::AgentMessage::Llm(msg) => Some(msg.clone()),
                _ => None,
            })
            .collect())
    }
}

fn make_agent(responses: Vec<Vec<opi_ai::stream::AssistantStreamEvent>>) -> Agent {
    let provider = MockProvider::new("mock", responses);
    Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        common::registrations_from(vec![Box::new(NoopTool)]),
        Some(common::permissive_authorizer()),
        "mock:mock-model".into(),
        Some("test system prompt".into()),
        InferenceConfig::default(),
        AgentLoopConfig::default(),
        Box::new(NoopHooks),
    )
    .expect("agent")
}

fn collect_events(agent: &mut Agent) -> Arc<Mutex<Vec<AgentEvent>>> {
    let events: Arc<Mutex<Vec<AgentEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let ev_clone = events.clone();
    agent.subscribe(Box::new(move |e: &AgentEvent| {
        ev_clone.lock().unwrap().push(e.clone());
    }));
    events
}

fn assert_has_event(events: &[AgentEvent], predicate: impl Fn(&AgentEvent) -> bool) {
    assert!(
        events.iter().any(predicate),
        "expected event not found in: {:?}",
        events
            .iter()
            .map(|e| match e {
                AgentEvent::AgentStart => "AgentStart",
                AgentEvent::AgentEnd { .. } => "AgentEnd",
                AgentEvent::TurnStart => "TurnStart",
                AgentEvent::TurnEnd { .. } => "TurnEnd",
                AgentEvent::MessageStart { .. } => "MessageStart",
                AgentEvent::MessageUpdate { .. } => "MessageUpdate",
                AgentEvent::MessageEnd { .. } => "MessageEnd",
                AgentEvent::ToolExecutionStart { .. } => "ToolExecutionStart",
                AgentEvent::ToolExecutionEnd { .. } => "ToolExecutionEnd",
                AgentEvent::QueueUpdate { .. } => "QueueUpdate",
                AgentEvent::AutoRetryStart { .. } => "AutoRetryStart",
                AgentEvent::AutoRetryEnd { .. } => "AutoRetryEnd",
                AgentEvent::CompactionStart { .. } => "CompactionStart",
                AgentEvent::CompactionEnd { .. } => "CompactionEnd",
                AgentEvent::SessionPersistError { .. } => "SessionPersistError",
                AgentEvent::ToolExecutionUpdate { .. } => "ToolExecutionUpdate",
                _ => "Unknown",
            })
            .collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Command parsing tests
// ---------------------------------------------------------------------------

#[test]
fn sdk_command_parse_prompt() {
    let json = r#"{"type":"prompt","message":"hello","id":"42"}"#;
    let cmd: SdkCommand = serde_json::from_str(json).unwrap();
    assert!(matches!(cmd, SdkCommand::prompt { .. }));
    assert_eq!(cmd.id().unwrap(), "42");
    assert_eq!(cmd.command_name(), "prompt");
}

#[test]
fn sdk_command_parse_continue() {
    let json = r#"{"type":"continue","message":"more"}"#;
    let cmd: SdkCommand = serde_json::from_str(json).unwrap();
    assert!(matches!(cmd, SdkCommand::continue_ { .. }));
    assert_eq!(cmd.command_name(), "continue");
}

#[test]
fn sdk_command_parse_abort() {
    let json = r#"{"type":"abort","id":"1"}"#;
    let cmd: SdkCommand = serde_json::from_str(json).unwrap();
    assert!(matches!(cmd, SdkCommand::abort { .. }));
    assert_eq!(cmd.command_name(), "abort");
}

#[test]
fn sdk_command_parse_set_model() {
    let json = r#"{"type":"set_model","model":"anthropic:claude-sonnet"}"#;
    let cmd: SdkCommand = serde_json::from_str(json).unwrap();
    assert!(matches!(cmd, SdkCommand::set_model { .. }));
    assert_eq!(cmd.command_name(), "set_model");
}

#[test]
fn sdk_command_parse_set_thinking_level() {
    let json = r#"{"type":"set_thinking_level","level":"high"}"#;
    let cmd: SdkCommand = serde_json::from_str(json).unwrap();
    assert!(matches!(cmd, SdkCommand::set_thinking_level { .. }));
    assert_eq!(cmd.command_name(), "set_thinking_level");
}

#[test]
fn sdk_command_parse_compact() {
    let json = r#"{"type":"compact"}"#;
    let cmd: SdkCommand = serde_json::from_str(json).unwrap();
    assert!(matches!(cmd, SdkCommand::compact { .. }));
    assert_eq!(cmd.command_name(), "compact");
}

#[test]
fn sdk_command_parse_session_info() {
    let json = r#"{"type":"session_info","id":"s1"}"#;
    let cmd: SdkCommand = serde_json::from_str(json).unwrap();
    assert!(matches!(cmd, SdkCommand::session_info { .. }));
    assert_eq!(cmd.command_name(), "session_info");
}

#[test]
fn sdk_command_parse_quit() {
    let json = r#"{"type":"quit"}"#;
    let cmd: SdkCommand = serde_json::from_str(json).unwrap();
    assert!(cmd.is_quit());
    assert_eq!(cmd.command_name(), "quit");
}

#[test]
fn sdk_command_parse_steer() {
    let json = r#"{"type":"steer","message":"redirect"}"#;
    let cmd: SdkCommand = serde_json::from_str(json).unwrap();
    assert!(matches!(cmd, SdkCommand::steer { .. }));
    assert_eq!(cmd.command_name(), "steer");
}

#[test]
fn sdk_command_parse_follow_up() {
    let json = r#"{"type":"follow_up","message":"then do this"}"#;
    let cmd: SdkCommand = serde_json::from_str(json).unwrap();
    assert!(matches!(cmd, SdkCommand::follow_up { .. }));
    assert_eq!(cmd.command_name(), "follow_up");
}

#[test]
fn sdk_command_roundtrip_all_variants() {
    let commands = vec![
        serde_json::json!({"type":"prompt","message":"hi","id":"a"}),
        serde_json::json!({"type":"continue","message":"more"}),
        serde_json::json!({"type":"steer","message":"s"}),
        serde_json::json!({"type":"follow_up","message":"f"}),
        serde_json::json!({"type":"abort"}),
        serde_json::json!({"type":"set_model","model":"m"}),
        serde_json::json!({"type":"set_thinking_level","level":"low"}),
        serde_json::json!({"type":"compact"}),
        serde_json::json!({"type":"session_info"}),
        serde_json::json!({"type":"quit"}),
    ];
    for json in commands {
        let cmd: SdkCommand = serde_json::from_value(json.clone()).unwrap();
        let serialized = serde_json::to_value(&cmd).unwrap();
        assert_eq!(
            json,
            serialized,
            "roundtrip failed for {:?}",
            cmd.command_name()
        );
    }
}

#[test]
fn sdk_command_invalid_type_rejected() {
    let json = r#"{"type":"unknown_command"}"#;
    assert!(serde_json::from_str::<SdkCommand>(json).is_err());
}

#[test]
fn sdk_command_missing_required_field_rejected() {
    // prompt without message field
    let json = r#"{"type":"prompt"}"#;
    assert!(serde_json::from_str::<SdkCommand>(json).is_err());
}

// ---------------------------------------------------------------------------
// Response tests
// ---------------------------------------------------------------------------

#[test]
fn sdk_response_success_serializes() {
    let resp = SdkResponse::success(Some("42"), "prompt");
    let val = serde_json::to_value(&resp).unwrap();
    assert_eq!(val["type"], "response");
    assert_eq!(val["command"], "prompt");
    assert_eq!(val["success"], true);
    assert_eq!(val["id"], "42");
    assert!(val.get("error").is_none());
    assert!(val.get("data").is_none());
}

#[test]
fn sdk_response_success_without_id() {
    let resp = SdkResponse::success(None, "compact");
    let val = serde_json::to_value(&resp).unwrap();
    assert_eq!(val["type"], "response");
    assert!(val.get("id").is_none());
}

#[test]
fn sdk_response_success_with_data() {
    let data = serde_json::json!({"model": "test", "session_id": "abc"});
    let resp = SdkResponse::success_with_data(Some("1"), "session_info", data.clone());
    let val = serde_json::to_value(&resp).unwrap();
    assert_eq!(val["type"], "response");
    assert_eq!(val["success"], true);
    assert_eq!(val["data"], data);
}

#[test]
fn sdk_response_roundtrips_through_json() {
    let data = serde_json::json!({"model": "test", "session_id": "abc"});
    let resp = SdkResponse::success_with_data(Some("1"), "session_info", data.clone());
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: SdkResponse = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.command, "session_info");
    assert!(parsed.success);
    assert_eq!(parsed.id.as_deref(), Some("1"));
    assert_eq!(parsed.data, Some(data));
}

#[test]
fn sdk_response_error_serializes() {
    let resp = SdkResponse::error(Some("1"), "set_model", "cannot change while running");
    let val = serde_json::to_value(&resp).unwrap();
    assert_eq!(val["type"], "response");
    assert_eq!(val["success"], false);
    assert_eq!(val["error"], "cannot change while running");
}

#[test]
fn sdk_response_error_without_id() {
    let resp = SdkResponse::error(None, "parse", "invalid json");
    let val = serde_json::to_value(&resp).unwrap();
    assert!(val.get("id").is_none());
}

// ---------------------------------------------------------------------------
// Schema version
// ---------------------------------------------------------------------------

#[test]
fn sdk_schema_version_matches_rpc() {
    assert_eq!(SDK_SCHEMA_VERSION, 3);
}

// ---------------------------------------------------------------------------
// Agent driving tests (SDK flows with MockProvider)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sdk_prompt_flow() {
    let mut agent = make_agent(vec![text_response("hello world")]);
    let events = collect_events(&mut agent);

    let result = agent.prompt("test prompt").await;
    assert!(result.is_ok());

    let ev = events.lock().unwrap();
    assert_has_event(&ev, |e| matches!(e, AgentEvent::AgentStart));
    assert_has_event(&ev, |e| matches!(e, AgentEvent::AgentEnd { .. }));
    assert_has_event(&ev, |e| matches!(e, AgentEvent::TurnStart));
    assert_has_event(&ev, |e| matches!(e, AgentEvent::TurnEnd { .. }));
}

#[tokio::test]
async fn sdk_continue_flow() {
    let mut agent = make_agent(vec![text_response("first"), text_response("second")]);

    let first = agent.prompt("first prompt").await;
    assert!(first.is_ok());

    let second = agent.continue_("continue prompt").await;
    assert!(second.is_ok());
}

#[tokio::test]
async fn sdk_continue_without_prior_prompt_errors() {
    let mut agent = make_agent(vec![]);
    let result = agent.continue_("orphan continue").await;
    assert!(matches!(result, Err(AgentError::Hook(_))));
}

#[tokio::test]
async fn sdk_abort_cancels_running_agent() {
    let mut agent = make_agent(vec![text_response("should not finish")]);
    let events = collect_events(&mut agent);

    let cancel = agent.cancel_token();
    let cancel_clone = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        cancel_clone.cancel();
    });

    let result = agent.prompt("test").await;
    match result {
        Ok(_) => {}
        Err(AgentError::Cancelled) => {}
        Err(e) => panic!("unexpected error: {:?}", e),
    }

    let ev = events.lock().unwrap();
    assert_has_event(&ev, |e| matches!(e, AgentEvent::AgentStart));
}

#[tokio::test]
async fn sdk_set_model_changes_model() {
    // Phase 17.2: set_model validates that the new selection resolves to a
    // route in the provider collection, so the provider must advertise the
    // target model. Switch the model within the same provider.
    let provider = MockProvider::new_with_models(
        "mock",
        vec![
            opi_ai::provider::ModelInfo::new(
                "mock-model",
                "Mock Model",
                opi_ai::WireApi::OpenAiCompletions,
                opi_ai::ModelCapabilities::new(100_000, 4_096),
            ),
            opi_ai::provider::ModelInfo::new(
                "other-model",
                "Other Model",
                opi_ai::WireApi::OpenAiCompletions,
                opi_ai::ModelCapabilities::new(100_000, 4_096),
            ),
        ],
        vec![text_response("response")],
    );
    let mut agent = Agent::new(
        std::sync::Arc::new(single_route_collection(Box::new(provider))),
        common::registrations_from(vec![Box::new(NoopTool)]),
        Some(common::permissive_authorizer()),
        "mock:mock-model".into(),
        Some("test system prompt".into()),
        InferenceConfig::default(),
        AgentLoopConfig::default(),
        Box::new(NoopHooks),
    )
    .expect("agent");
    assert_eq!(agent.model(), "mock-model");
    let mut candidate = agent.state_snapshot();
    candidate.model_selection = opi_agent::loop_types::ModelSelection::new("mock", "other-model");
    agent.replace_state(candidate).unwrap();
    assert_eq!(agent.model(), "other-model");
}

#[tokio::test]
async fn sdk_steer_flow() {
    let mut agent = make_agent(vec![text_response("first"), text_response("steered")]);
    agent.steer("steer this".into());
    let result = agent.prompt("initial").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn sdk_follow_up_flow() {
    let mut agent = make_agent(vec![text_response("first"), text_response("followed")]);
    agent.follow_up("follow up".into());
    let result = agent.prompt("initial").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn sdk_messages_snapshot_after_prompt() {
    let mut agent = make_agent(vec![text_response("response text")]);
    let result = agent.prompt("hello").await;
    assert!(result.is_ok());
    let snapshot = agent.messages_snapshot();
    // Should have at least user message + assistant message
    assert!(snapshot.len() >= 2);
}

#[tokio::test]
async fn sdk_cancel_token_is_clonable() {
    let agent = make_agent(vec![text_response("ok")]);
    let t1 = agent.cancel_token();
    let t2 = t1.clone();
    assert!(!t1.is_cancelled());
    assert!(!t2.is_cancelled());
}

// ---------------------------------------------------------------------------
// Event conversion tests
// ---------------------------------------------------------------------------

#[test]
fn sdk_agent_event_to_value_agent_start() {
    let event = AgentEvent::AgentStart;
    let val = agent_event_to_value(&event);
    assert_eq!(val["type"], "AgentStart");
}

#[test]
fn sdk_agent_event_to_value_turn_start() {
    let event = AgentEvent::TurnStart;
    let val = agent_event_to_value(&event);
    assert_eq!(val["type"], "TurnStart");
}

#[test]
fn sdk_agent_event_to_value_session_persist_error() {
    let event = AgentEvent::SessionPersistError {
        message: "disk full".into(),
    };
    let val = agent_event_to_value(&event);
    assert_eq!(val["type"], "SessionPersistError");
    assert_eq!(val["message"], "disk full");
}

// ---------------------------------------------------------------------------
// SDK command → response correlation tests
// ---------------------------------------------------------------------------

#[test]
fn sdk_command_id_correlation() {
    let json = r#"{"type":"prompt","message":"hi","id":"corr-42"}"#;
    let cmd: SdkCommand = serde_json::from_str(json).unwrap();
    assert_eq!(cmd.id(), Some("corr-42"));

    // Build a response with the same id
    let resp = SdkResponse::success(cmd.id(), cmd.command_name());
    let val = serde_json::to_value(&resp).unwrap();
    assert_eq!(val["id"], "corr-42");
    assert_eq!(val["command"], "prompt");
}

// ---------------------------------------------------------------------------
// Documentation / unstable 0.x marker test
// ---------------------------------------------------------------------------

#[test]
fn sdk_types_are_documented_unstable() {
    // This test exists as a behavioral assertion that the SDK module
    // documentation mentions "unstable" and "0.x". The actual doc
    // check is in the module-level doc comment.
    let module_doc = include_str!("../src/sdk.rs");
    assert!(
        module_doc.contains("unstable"),
        "SDK module must document unstable 0.x status"
    );
    assert!(
        module_doc.contains("0.x"),
        "SDK module must document unstable 0.x status"
    );
}

// ---------------------------------------------------------------------------
// Phase 8 (task 8.5): SDK and RPC command-state contract.
//
// The SdkCommand/SdkResponse types are the 0.x command contract shared by the
// RPC JSONL runner and embedders. These tests pin the documented command
// enumeration, the structured unsupported-command error shape, and that
// AgentControl (the clonable handle RpcRunner stores as its control surface)
// routes steer/follow_up into the agent's queues and abort into its shared
// cancellation token.
// ---------------------------------------------------------------------------

/// The full documented 0.x command set parses to its canonical name and echoes
/// its correlation id.
#[test]
fn phase8_sdk_command_contract_all_documented_commands_parse_and_name() {
    let cases: &[(&str, &str, &str)] = &[
        (
            r#"{"type":"prompt","message":"hi","id":"c1"}"#,
            "prompt",
            "c1",
        ),
        (
            r#"{"type":"continue","message":"more","id":"c2"}"#,
            "continue",
            "c2",
        ),
        (r#"{"type":"steer","message":"s","id":"c3"}"#, "steer", "c3"),
        (
            r#"{"type":"follow_up","message":"f","id":"c4"}"#,
            "follow_up",
            "c4",
        ),
        (r#"{"type":"abort","id":"c5"}"#, "abort", "c5"),
        (
            r#"{"type":"set_model","model":"m","id":"c6"}"#,
            "set_model",
            "c6",
        ),
        (
            r#"{"type":"set_thinking_level","level":"low","id":"c7"}"#,
            "set_thinking_level",
            "c7",
        ),
        (r#"{"type":"compact","id":"c8"}"#, "compact", "c8"),
        (r#"{"type":"session_info","id":"c9"}"#, "session_info", "c9"),
        (
            r#"{"type":"extension_command","name":"n","args":{},"id":"c10"}"#,
            "extension_command",
            "c10",
        ),
        (r#"{"type":"trace","id":"c11"}"#, "trace", "c11"),
        (r#"{"type":"quit","id":"c12"}"#, "quit", "c12"),
    ];
    for (json, name, id) in cases {
        let cmd: SdkCommand =
            serde_json::from_str(json).unwrap_or_else(|e| panic!("parse {json}: {e}"));
        assert_eq!(cmd.command_name(), *name, "command_name for {json}");
        assert_eq!(cmd.id(), Some(*id), "correlated id for {json}");
    }
}

/// A structured unsupported-command response carries a stable machine-readable
/// `error_code`; a plain error response carries none.
#[test]
fn phase8_sdk_command_contract_unsupported_response_carries_error_code() {
    let resp = SdkResponse::error_with_code(
        Some("t1"),
        "trace",
        "unsupported_trace_request",
        "trace is not enabled",
    );
    let val = serde_json::to_value(&resp).unwrap();
    assert_eq!(val["type"], "response");
    assert_eq!(val["command"], "trace");
    assert_eq!(val["success"], false);
    assert_eq!(val["id"], "t1");
    assert_eq!(val["error"], "trace is not enabled");
    assert_eq!(
        val["error_code"], "unsupported_trace_request",
        "structured unsupported-command errors carry a stable error_code"
    );

    // The error_code field is additive: plain errors omit it entirely.
    let plain = SdkResponse::error(Some("e1"), "set_model", "bad model");
    let pval = serde_json::to_value(&plain).unwrap();
    assert_eq!(pval["success"], false);
    assert!(
        pval.get("error_code").is_none(),
        "plain errors must not carry an error_code"
    );
}

/// Build an agent backed by a MockProvider whose request log is observable.
fn agent_with_call_log(
    responses: Vec<Vec<opi_ai::stream::AssistantStreamEvent>>,
) -> (Agent, Arc<Mutex<Vec<opi_ai::provider::Request>>>) {
    let provider = MockProvider::new("mock", responses);
    let call_log = provider.call_log_handle();
    let agent = Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        common::registrations_from(vec![Box::new(NoopTool)]),
        Some(common::permissive_authorizer()),
        "mock:mock-model".into(),
        Some("test system prompt".into()),
        InferenceConfig::default(),
        AgentLoopConfig::default(),
        Box::new(NoopHooks),
    )
    .expect("agent");
    (agent, call_log)
}

/// True when provider call `call_index` contains a user text message equal to
/// `text`.
fn provider_call_contains_user_text(
    call_log: &Arc<Mutex<Vec<opi_ai::provider::Request>>>,
    call_index: usize,
    text: &str,
) -> bool {
    let calls = call_log.lock().unwrap();
    let Some(request) = calls.get(call_index) else {
        return false;
    };
    request.messages.iter().any(|message| match message {
        opi_ai::message::Message::User(user) => user.content.iter().any(|content| {
            matches!(
                content,
                opi_ai::message::InputContent::Text { text: t } if t == text
            )
        }),
        _ => false,
    })
}

/// AgentControl (the clonable handle RpcRunner stores) routes steer and
/// follow_up into the agent's queues — observable as a second provider request
/// carrying the queued text — and routes abort into the agent's shared token.
#[tokio::test]
async fn phase8_sdk_command_contract_agent_control_routes_steer_follow_up_abort() {
    // Steer via the control handle is delivered before the next provider
    // request. A tool-call first turn forces a continuation so the steering
    // message is deterministically observed in the second call.
    let (mut agent, steer_log) = agent_with_call_log(vec![
        tool_call_response("tc-steer", "noop", "{}"),
        text_response("steered"),
    ]);
    let steer_control = agent.control_handle();
    steer_control.steer("steer-via-control".into());
    let result = agent.prompt("initial").await;
    assert!(result.is_ok(), "steered prompt should complete: {result:?}");
    assert_eq!(
        steer_log.lock().unwrap().len(),
        2,
        "queued steering must trigger a second provider request"
    );
    assert!(
        provider_call_contains_user_text(&steer_log, 1, "steer-via-control"),
        "second provider request must include the steering text"
    );

    // Follow-up via the control handle is delivered when the agent would
    // otherwise stop, triggering a continuation turn.
    let (mut agent, follow_up_log) =
        agent_with_call_log(vec![text_response("first"), text_response("followed")]);
    let follow_up_control = agent.control_handle();
    follow_up_control.follow_up("followup-via-control".into());
    let result = agent.prompt("initial").await;
    assert!(
        result.is_ok(),
        "follow-up prompt should complete: {result:?}"
    );
    assert_eq!(
        follow_up_log.lock().unwrap().len(),
        2,
        "queued follow-up must trigger a second provider request"
    );
    assert!(
        provider_call_contains_user_text(&follow_up_log, 1, "followup-via-control"),
        "second provider request must include the follow-up text"
    );

    // Abort via the control handle cancels the agent's shared token.
    let agent = make_agent(vec![text_response("ok")]);
    let abort_control = agent.control_handle();
    assert!(
        !agent.cancel_token().is_cancelled(),
        "agent starts uncancelled"
    );
    abort_control.abort();
    assert!(
        agent.cancel_token().is_cancelled(),
        "AgentControl::abort must cancel the agent's shared cancellation token"
    );
}
