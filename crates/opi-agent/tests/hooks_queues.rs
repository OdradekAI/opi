//! Behavioral tests for hooks and queues (task 1.8).
//!
//! DoD: "before/after, should-stop, steering, follow-up tested"

mod common;

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use futures_util::stream;
use opi_agent::agent::Agent;
use opi_agent::event::AgentEvent;
use opi_agent::hooks::{
    AfterToolCallContext, AfterToolCallResult, AgentHooks, BeforeToolCallContext,
    BeforeToolCallResult, PrepareNextTurnContext, ShouldStopAfterTurnContext,
};
use opi_agent::loop_types::{
    AgentError, AgentLoopConfig, InferenceConfig, ModelSelection, NextTurnState,
};
use opi_agent::message::AgentMessage;
use opi_agent::tool::{ExecutionMode, Tool, ToolError, ToolResult};
use opi_ai::message::{
    AssistantContent, AssistantMessage, InputContent, Message, OutputContent, ToolCall, ToolDef,
    UserMessage,
};
use opi_ai::provider::{EventStream, Provider, ProviderError, Request, ThinkingConfig};
use opi_ai::stream::{AssistantStreamEvent, StopReason, Usage};
use opi_ai::test_support::single_route_collection;
use opi_ai::{ModelCapabilities, ModelInfo, WireApi};
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Recording mock provider
// ---------------------------------------------------------------------------

struct RecordingProvider {
    responses: Arc<Mutex<Vec<Vec<AssistantStreamEvent>>>>,
    received_messages: Arc<Mutex<Vec<Vec<Message>>>>,
    models: Vec<ModelInfo>,
}

impl RecordingProvider {
    fn new(responses: Vec<Vec<AssistantStreamEvent>>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses)),
            received_messages: Arc::new(Mutex::new(Vec::new())),
            models: vec![
                ModelInfo::new(
                    "mock-model",
                    "Mock Model",
                    WireApi::OpenAiCompletions,
                    ModelCapabilities::new(100_000, 4_096),
                ),
                ModelInfo::new(
                    "alt-model",
                    "Alternate Model",
                    WireApi::OpenAiCompletions,
                    ModelCapabilities::new(100_000, 4_096),
                ),
            ],
        }
    }
}

impl Provider for RecordingProvider {
    fn id(&self) -> &str {
        "recording"
    }

    fn models(&self) -> &[opi_ai::provider::ModelInfo] {
        &self.models
    }

    fn stream_prepared(&self, request: Request, _auth: opi_ai::auth::ResolvedAuth) -> EventStream {
        self.received_messages
            .lock()
            .unwrap()
            .push(request.messages);
        let events = self.responses.lock().unwrap().remove(0);
        Box::pin(stream::iter(events.into_iter().map(Ok::<_, ProviderError>)))
    }
}

// ---------------------------------------------------------------------------
// Echo tool
// ---------------------------------------------------------------------------

struct EchoTool;

impl Tool for EchoTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "echo".into(),
            description: "echoes input".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": { "text": { "type": "string" } },
                "required": ["text"]
            }),
        }
    }

    fn execute(
        &self,
        _call_id: &str,
        args: serde_json::Value,
        _signal: CancellationToken,
        _on_update: Option<opi_agent::tool::UpdateCallback>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send>> {
        let text = args["text"].as_str().unwrap_or_default().to_owned();
        Box::pin(async move {
            Ok(ToolResult {
                content: vec![OutputContent::Text { text }],
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
// Recording hooks (records after_tool_call and should_stop contexts)
// ---------------------------------------------------------------------------

struct RecordingHooks {
    after_calls: Arc<Mutex<Vec<AfterToolCallContext>>>,
    stop_calls: Arc<Mutex<Vec<ShouldStopAfterTurnContext>>>,
    prepare_calls: Arc<Mutex<Vec<u32>>>,
    stop_result: bool,
}

impl RecordingHooks {
    fn new(stop_result: bool) -> Self {
        Self {
            after_calls: Arc::new(Mutex::new(Vec::new())),
            stop_calls: Arc::new(Mutex::new(Vec::new())),
            prepare_calls: Arc::new(Mutex::new(Vec::new())),
            stop_result,
        }
    }
}

impl AgentHooks for RecordingHooks {
    fn convert_to_llm(
        &self,
        messages: &[AgentMessage],
    ) -> Result<Vec<opi_ai::message::Message>, AgentError> {
        let mut result = Vec::new();
        for msg in messages {
            if let AgentMessage::Llm(m) = msg {
                result.push(m.clone());
            }
        }
        Ok(result)
    }

    fn should_stop_after_turn(
        &self,
        ctx: ShouldStopAfterTurnContext,
    ) -> Pin<Box<dyn Future<Output = bool> + Send>> {
        let calls = self.stop_calls.clone();
        let stop = self.stop_result;
        Box::pin(async move {
            calls.lock().unwrap().push(ctx);
            stop
        })
    }

    fn before_tool_call(
        &self,
        _ctx: BeforeToolCallContext,
    ) -> Pin<Box<dyn Future<Output = BeforeToolCallResult> + Send>> {
        Box::pin(async { BeforeToolCallResult::Continue })
    }

    fn after_tool_call(
        &self,
        ctx: AfterToolCallContext,
    ) -> Pin<Box<dyn Future<Output = AfterToolCallResult> + Send>> {
        let calls = self.after_calls.clone();
        Box::pin(async move {
            calls.lock().unwrap().push(ctx);
            AfterToolCallResult::Keep
        })
    }

    // Records each invocation so queue-polling-order tests can prove that a
    // terminal should_stop_after_turn skips prepare_next_turn. Returns None
    // (no injection), preserving the behavior other tests rely on.
    fn prepare_next_turn(
        &self,
        ctx: PrepareNextTurnContext,
    ) -> Pin<Box<dyn Future<Output = Result<Option<NextTurnState>, AgentError>> + Send>> {
        let prepare_calls = self.prepare_calls.clone();
        Box::pin(async move {
            prepare_calls.lock().unwrap().push(ctx.turn);
            Ok(None)
        })
    }
}

// ---------------------------------------------------------------------------
// Replacing hooks (returns AfterToolCallResult::Replace)
// ---------------------------------------------------------------------------

struct ReplacingHooks;

impl AgentHooks for ReplacingHooks {
    fn convert_to_llm(
        &self,
        messages: &[AgentMessage],
    ) -> Result<Vec<opi_ai::message::Message>, AgentError> {
        let mut result = Vec::new();
        for msg in messages {
            if let AgentMessage::Llm(m) = msg {
                result.push(m.clone());
            }
        }
        Ok(result)
    }

    fn should_stop_after_turn(
        &self,
        _ctx: ShouldStopAfterTurnContext,
    ) -> Pin<Box<dyn Future<Output = bool> + Send>> {
        Box::pin(async { false })
    }

    fn before_tool_call(
        &self,
        _ctx: BeforeToolCallContext,
    ) -> Pin<Box<dyn Future<Output = BeforeToolCallResult> + Send>> {
        Box::pin(async { BeforeToolCallResult::Continue })
    }

    fn after_tool_call(
        &self,
        ctx: AfterToolCallContext,
    ) -> Pin<Box<dyn Future<Output = AfterToolCallResult> + Send>> {
        let content_len = ctx.result.content.len();
        Box::pin(async move {
            AfterToolCallResult::Replace(ToolResult {
                content: vec![OutputContent::Text {
                    text: format!("replaced: {content_len}"),
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn base_assistant() -> AssistantMessage {
    AssistantMessage {
        content: vec![],
        api: opi_ai::ApiKind::Anthropic,
        provider: "recording".into(),
        model: "mock-model".into(),
        response_model: None,
        response_id: None,
        usage: Usage::default(),
        stop_reason: StopReason::Stop,
        error_message: None,
        timestamp_ms: 0,
    }
}

fn text_response(text: &str) -> Vec<AssistantStreamEvent> {
    let mut partial = base_assistant();
    partial
        .content
        .push(AssistantContent::Text { text: text.into() });
    vec![
        AssistantStreamEvent::Start {
            partial: base_assistant(),
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

fn tool_call_response(call_id: &str, tool_name: &str, args: &str) -> Vec<AssistantStreamEvent> {
    let tool_call = ToolCall {
        id: call_id.into(),
        name: tool_name.into(),
        arguments: args.into(),
    };
    let mut partial = base_assistant();
    partial.content.push(AssistantContent::ToolCall {
        tool_call: tool_call.clone(),
    });
    partial.stop_reason = StopReason::ToolUse;
    vec![
        AssistantStreamEvent::Start {
            partial: base_assistant(),
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

fn make_agent(
    provider: RecordingProvider,
    tools: Vec<Box<dyn Tool>>,
    hooks: Box<dyn AgentHooks>,
) -> Agent {
    Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        common::registrations_from(tools),
        Some(common::permissive_authorizer()),
        "recording:mock-model".into(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig::default(),
        hooks,
    )
    .expect("agent")
}

fn user_text_in_messages(messages: &[Message], text: &str) -> bool {
    messages.iter().any(|m| match m {
        Message::User(u) => u
            .content
            .iter()
            .any(|c| matches!(c, InputContent::Text { text: t } if t == text)),
        _ => false,
    })
}

// ---------------------------------------------------------------------------
// Test 1: after_tool_call receives AfterToolCallContext
// ---------------------------------------------------------------------------

#[tokio::test]
async fn after_tool_call_receives_context() {
    let provider = RecordingProvider::new(vec![
        tool_call_response("c1", "echo", r#"{"text":"hello"}"#),
        text_response("done"),
    ]);

    let hooks = RecordingHooks::new(false);
    let after_calls = hooks.after_calls.clone();

    let mut agent = make_agent(provider, vec![Box::new(EchoTool)], Box::new(hooks));
    agent.prompt("test").await.unwrap();

    let calls = after_calls.lock().unwrap();
    assert_eq!(calls.len(), 1, "after_tool_call should be called once");
    assert_eq!(calls[0].tool_call_id, "c1");
    assert_eq!(calls[0].tool_name, "echo");
    assert!(!calls[0].result.is_error);
}

// ---------------------------------------------------------------------------
// Test 2: after_tool_call Replace modifies tool result
// ---------------------------------------------------------------------------

#[tokio::test]
async fn after_tool_call_replace_result() {
    let provider = RecordingProvider::new(vec![
        tool_call_response("c1", "echo", r#"{"text":"hello"}"#),
        text_response("done"),
    ]);

    let mut agent = make_agent(provider, vec![Box::new(EchoTool)], Box::new(ReplacingHooks));
    let result = agent.prompt("test").await.unwrap();

    let tool_result = result
        .iter()
        .find_map(|m| match m {
            AgentMessage::Llm(Message::ToolResult(tr)) => Some(tr.clone()),
            _ => None,
        })
        .expect("should have a tool result");

    match &tool_result.content[0] {
        OutputContent::Text { text } => assert_eq!(text, "replaced: 1"),
        _ => panic!("expected text content"),
    }
}

// ---------------------------------------------------------------------------
// Test 3: should_stop_after_turn receives ShouldStopAfterTurnContext
// ---------------------------------------------------------------------------

#[tokio::test]
async fn should_stop_receives_context() {
    let provider = RecordingProvider::new(vec![text_response("hello")]);

    let hooks = RecordingHooks::new(false);
    let stop_calls = hooks.stop_calls.clone();

    let mut agent = make_agent(provider, vec![], Box::new(hooks));
    agent.prompt("test").await.unwrap();

    let calls = stop_calls.lock().unwrap();
    assert!(!calls.is_empty(), "should_stop_after_turn should be called");
    assert!(
        !calls[0].state.context.is_empty(),
        "context should have messages"
    );
}

// ---------------------------------------------------------------------------
// Test 4: steering queue delivers before next request
// ---------------------------------------------------------------------------

#[tokio::test]
async fn steering_queue_delivered_before_next_request() {
    let provider = RecordingProvider::new(vec![
        tool_call_response("c1", "echo", r#"{"text":"hello"}"#),
        text_response("done"),
    ]);
    let received = provider.received_messages.clone();

    let hooks = RecordingHooks::new(false);

    let mut agent = make_agent(provider, vec![Box::new(EchoTool)], Box::new(hooks));
    agent.steer("focus on quality".into());
    agent.prompt("test").await.unwrap();

    let msgs = received.lock().unwrap();
    assert_eq!(msgs.len(), 2, "provider should be called twice");
    assert!(
        user_text_in_messages(&msgs[1], "focus on quality"),
        "second provider call should include steering message"
    );
}

// ---------------------------------------------------------------------------
// Test 5: follow-up queue delivers when agent would stop
// ---------------------------------------------------------------------------

#[tokio::test]
async fn follow_up_queue_delivered_when_would_stop() {
    let provider = RecordingProvider::new(vec![text_response("hello"), text_response("more")]);
    let received = provider.received_messages.clone();

    let hooks = RecordingHooks::new(false);

    let mut agent = make_agent(provider, vec![], Box::new(hooks));
    agent.follow_up("tell me more".into());
    agent.prompt("test").await.unwrap();

    let msgs = received.lock().unwrap();
    assert_eq!(msgs.len(), 2, "provider should be called twice");
    assert!(
        user_text_in_messages(&msgs[1], "tell me more"),
        "second provider call should include follow-up message"
    );
}

// ---------------------------------------------------------------------------
// Test 6: should_stop true prevents queue polling
// ---------------------------------------------------------------------------

#[tokio::test]
async fn should_stop_prevents_queue_polling() {
    let provider = RecordingProvider::new(vec![tool_call_response(
        "c1",
        "echo",
        r#"{"text":"hello"}"#,
    )]);
    let received = provider.received_messages.clone();

    let hooks = RecordingHooks::new(true);

    let mut agent = make_agent(provider, vec![Box::new(EchoTool)], Box::new(hooks));
    agent.steer("should not be delivered".into());
    agent.prompt("test").await.unwrap();

    let msgs = received.lock().unwrap();
    assert_eq!(msgs.len(), 1, "provider should only be called once");
}

// ---------------------------------------------------------------------------
// Test 7: QueueUpdate event emitted
// ---------------------------------------------------------------------------

#[tokio::test]
async fn queue_update_event_emitted() {
    let provider = RecordingProvider::new(vec![
        tool_call_response("c1", "echo", r#"{"text":"hello"}"#),
        text_response("done"),
    ]);

    let hooks = RecordingHooks::new(false);
    type QueueData = (Vec<String>, Vec<String>);
    let queue_events: Arc<Mutex<Vec<QueueData>>> = Arc::new(Mutex::new(Vec::new()));
    let queue_events_clone = queue_events.clone();

    let mut agent = make_agent(provider, vec![Box::new(EchoTool)], Box::new(hooks));
    agent.steer("redirect".into());
    agent.subscribe(Box::new(move |e| {
        if let AgentEvent::QueueUpdate {
            steering,
            follow_up,
        } = e
        {
            queue_events_clone
                .lock()
                .unwrap()
                .push((steering.clone(), follow_up.clone()));
        }
    }));
    agent.prompt("test").await.unwrap();

    let updates = queue_events.lock().unwrap();
    assert!(!updates.is_empty(), "should emit QueueUpdate event");
    assert!(
        updates[0].0.contains(&"redirect".to_owned()),
        "event should contain steering message"
    );
}

// ---------------------------------------------------------------------------
// Phase 8: queue-polling order contract (task 8.1)
//
// Pins the documented order: steering is drained before follow-up, and a
// compaction stop (should_stop_after_turn == true) terminates the run before
// prepare_next_turn runs or any queued message is polled.
// ---------------------------------------------------------------------------

// DoD: when both a steering and a follow-up message are queued, steering is
// delivered to the next provider request strictly before the follow-up.
#[tokio::test]
async fn phase8_queue_polling_order_steering_before_follow_up() {
    let provider = RecordingProvider::new(vec![
        text_response("first"),
        text_response("second"),
        text_response("third"),
    ]);
    let received = provider.received_messages.clone();

    let hooks = RecordingHooks::new(false);

    let mut agent = make_agent(provider, vec![], Box::new(hooks));
    agent.steer("steer-msg".into());
    agent.follow_up("follow-msg".into());
    agent.prompt("test").await.unwrap();

    let msgs = received.lock().unwrap();
    assert_eq!(
        msgs.len(),
        3,
        "steering + follow-up yield three provider calls"
    );

    let steer_call = msgs
        .iter()
        .position(|ms| user_text_in_messages(ms, "steer-msg"))
        .expect("steering message must be delivered");
    let follow_call = msgs
        .iter()
        .position(|ms| user_text_in_messages(ms, "follow-msg"))
        .expect("follow-up message must be delivered");
    assert_eq!(
        steer_call, 1,
        "steering delivered on the second provider call (index 1)"
    );
    assert_eq!(
        follow_call, 2,
        "follow-up delivered on the third provider call (index 2), after steering"
    );
    assert!(
        steer_call < follow_call,
        "steering must be delivered before follow-up"
    );
    // Follow-up is not delivered in the same call as steering.
    assert!(
        !user_text_in_messages(&msgs[1], "follow-msg"),
        "follow-up must not accompany steering in the second call"
    );
}

// DoD: a compaction stop signaled through should_stop_after_turn terminates the
// run at the stop gate, before prepare_next_turn runs and before a queued
// follow-up is polled (no next turn is prepared).
#[tokio::test]
async fn phase8_queue_polling_order_compaction_stop_before_next_turn() {
    let provider = RecordingProvider::new(vec![text_response("only")]);
    let received = provider.received_messages.clone();

    let hooks = RecordingHooks::new(true);
    let prepare_calls = hooks.prepare_calls.clone();

    let mut agent = make_agent(provider, vec![], Box::new(hooks));
    agent.follow_up("must-not-deliver".into());
    agent.prompt("test").await.unwrap();

    let msgs = received.lock().unwrap();
    assert_eq!(
        msgs.len(),
        1,
        "compaction stop: exactly one provider call, no next turn"
    );
    assert!(
        !msgs
            .iter()
            .any(|ms| user_text_in_messages(ms, "must-not-deliver")),
        "follow-up must not be delivered after a compaction stop"
    );
    // Phase 17.2: prepare_next_turn runs BEFORE should_stop (it applies the
    // candidate state that stop then observes), so it is invoked once; the stop
    // then terminates the run without polling the follow-up queue.
    assert_eq!(
        prepare_calls.lock().unwrap().len(),
        1,
        "prepare_next_turn runs once before stop; stop terminates before queue polling"
    );
}

// ---------------------------------------------------------------------------
// Phase 8: hook order and failure-semantics contract (task 8.2).
//
// Pins the documented AgentHooks order and effects: transform_context ->
// convert_to_llm -> (stream) -> before_tool_call -> execute -> after_tool_call
// -> should_stop_after_turn -> prepare_next_turn. before_tool_call runs AFTER
// schema validation and may block; after_tool_call replacement is reflected in
// the final ToolExecutionEnd event and persisted result; prepare_next_turn may
// inject a message into the next provider request; a terminal
// should_stop_after_turn skips prepare_next_turn.
// ---------------------------------------------------------------------------

/// Echo tool that records each execution by call id so tests can prove whether
/// `tool.execute` actually ran.
struct CountingTool {
    calls: Arc<Mutex<Vec<String>>>,
}

impl Tool for CountingTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "echo".into(),
            description: "echoes input".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": { "text": { "type": "string" } },
                "required": ["text"]
            }),
        }
    }

    fn execute(
        &self,
        call_id: &str,
        args: serde_json::Value,
        _signal: CancellationToken,
        _on_update: Option<opi_agent::tool::UpdateCallback>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send>> {
        let text = args["text"].as_str().unwrap_or_default().to_owned();
        let calls = self.calls.clone();
        let call_id = call_id.to_owned();
        Box::pin(async move {
            calls.lock().unwrap().push(call_id);
            Ok(ToolResult {
                content: vec![OutputContent::Text { text }],
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

/// Hooks that record the ordered sequence of every lifecycle entry, for the
/// hook-ordering contract test. `convert_to_llm` and `transform_context` pass
/// messages through so the loop can stream.
struct OrderHooks {
    log: Arc<Mutex<Vec<String>>>,
    stop: bool,
}

impl AgentHooks for OrderHooks {
    fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
        self.log.lock().unwrap().push("convert".into());
        Ok(messages
            .iter()
            .filter_map(|m| match m {
                AgentMessage::Llm(m) => Some(m.clone()),
                _ => None,
            })
            .collect())
    }

    fn transform_context(
        &self,
        messages: Vec<AgentMessage>,
        _signal: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<AgentMessage>, AgentError>> + Send>> {
        let log = self.log.clone();
        Box::pin(async move {
            log.lock().unwrap().push("transform".into());
            Ok(messages)
        })
    }

    fn should_stop_after_turn(
        &self,
        _ctx: ShouldStopAfterTurnContext,
    ) -> Pin<Box<dyn Future<Output = bool> + Send>> {
        let log = self.log.clone();
        let stop = self.stop;
        Box::pin(async move {
            log.lock().unwrap().push("should_stop".into());
            stop
        })
    }

    fn before_tool_call(
        &self,
        _ctx: BeforeToolCallContext,
    ) -> Pin<Box<dyn Future<Output = BeforeToolCallResult> + Send>> {
        let log = self.log.clone();
        Box::pin(async move {
            log.lock().unwrap().push("before".into());
            BeforeToolCallResult::Continue
        })
    }

    fn after_tool_call(
        &self,
        _ctx: AfterToolCallContext,
    ) -> Pin<Box<dyn Future<Output = AfterToolCallResult> + Send>> {
        let log = self.log.clone();
        Box::pin(async move {
            log.lock().unwrap().push("after".into());
            AfterToolCallResult::Keep
        })
    }

    fn prepare_next_turn(
        &self,
        _ctx: PrepareNextTurnContext,
    ) -> Pin<Box<dyn Future<Output = Result<Option<NextTurnState>, AgentError>> + Send>> {
        let log = self.log.clone();
        Box::pin(async move {
            log.lock().unwrap().push("prepare".into());
            Ok(None)
        })
    }
}

/// Hooks that deny one named tool and record every before_tool_call by tool
/// name, for the block-after-validation contract.
struct DenyHooks {
    deny: String,
    before_calls: Arc<Mutex<Vec<String>>>,
}

impl AgentHooks for DenyHooks {
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
        ctx: BeforeToolCallContext,
    ) -> Pin<Box<dyn Future<Output = BeforeToolCallResult> + Send>> {
        let deny = self.deny.clone();
        let before_calls = self.before_calls.clone();
        let tool_name = ctx.tool_name.clone();
        Box::pin(async move {
            let matches = tool_name == deny;
            before_calls.lock().unwrap().push(tool_name);
            if matches {
                BeforeToolCallResult::Deny {
                    reason: "denied by hook".into(),
                }
            } else {
                BeforeToolCallResult::Continue
            }
        })
    }
}

/// Hooks that inject a user message on the first prepare_next_turn, for the
/// injection contract.
struct InjectHooks {
    injected: Arc<Mutex<bool>>,
}

impl AgentHooks for InjectHooks {
    fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
        Ok(messages
            .iter()
            .filter_map(|m| match m {
                AgentMessage::Llm(m) => Some(m.clone()),
                _ => None,
            })
            .collect())
    }

    fn prepare_next_turn(
        &self,
        ctx: PrepareNextTurnContext,
    ) -> Pin<Box<dyn Future<Output = Result<Option<NextTurnState>, AgentError>> + Send>> {
        let injected = self.injected.clone();
        Box::pin(async move {
            // First prepare fires after turn 0 (ctx.turn == 1).
            if ctx.turn == 1 {
                *injected.lock().unwrap() = true;
                let mut state = ctx.state.clone();
                state
                    .context
                    .push(AgentMessage::Llm(Message::User(UserMessage {
                        content: vec![InputContent::Text {
                            text: "injected-from-prepare".into(),
                        }],
                        timestamp_ms: 0,
                    })));
                Ok(Some(state))
            } else {
                Ok(None)
            }
        })
    }
}

// DoD: the six AgentHooks methods fire in the documented order within a turn
// (transform -> convert -> before -> after -> should_stop -> prepare).
#[tokio::test]
async fn phase8_hook_contract_order() {
    let provider = RecordingProvider::new(vec![
        tool_call_response("c1", "echo", r#"{"text":"hello"}"#),
        text_response("done"),
    ]);

    let log = Arc::new(Mutex::new(Vec::<String>::new()));
    let hooks = Box::new(OrderHooks {
        log: log.clone(),
        stop: false,
    });

    let mut agent = make_agent(provider, vec![Box::new(EchoTool)], hooks);
    agent.prompt("test").await.unwrap();

    let recorded = log.lock().unwrap().clone();
    assert!(
        recorded.len() >= 6,
        "expected at least six hook entries, got {recorded:?}"
    );
    assert_eq!(
        &recorded[..6],
        &[
            "transform",
            "convert",
            "before",
            "after",
            "prepare",
            "should_stop",
        ],
        "first-turn hook order must be transform -> convert -> before -> after -> prepare -> should_stop (Phase 17.2: prepare runs before stop observes the applied state)"
    );
}

// DoD (Phase 17.4): before_tool_call runs BEFORE schema validation. An
// invalid-args call still runs the hook (which observes the proposed call) but
// never reaches execute; a valid-args call with a Deny hook runs the hook,
// passes validation, and still does not execute the tool.
#[tokio::test]
async fn phase8_hook_runs_before_schema_validation() {
    let execs = Arc::new(Mutex::new(Vec::<String>::new()));

    // Case 1: invalid arguments -> before_tool_call runs first, then schema
    // validation fails inside execute_tool before tool.execute. The error
    // result does not terminate, so the loop needs a second response to end
    // gracefully.
    let provider = RecordingProvider::new(vec![
        tool_call_response("c-invalid", "echo", r#"{}"#),
        text_response("done"),
    ]);
    let before_calls = Arc::new(Mutex::new(Vec::<String>::new()));
    let hooks = Box::new(DenyHooks {
        deny: "never-matches".into(),
        before_calls: before_calls.clone(),
    });
    let tool = CountingTool {
        calls: execs.clone(),
    };
    let mut agent = make_agent(provider, vec![Box::new(tool)], hooks);
    let result = agent.prompt("test").await.unwrap();

    assert_eq!(
        before_calls.lock().unwrap().len(),
        1,
        "before_tool_call runs before schema validation, so it observes the invalid-args call"
    );
    assert!(
        execs.lock().unwrap().is_empty(),
        "tool.execute must NOT run when schema validation fails"
    );
    let invalid_result = result
        .iter()
        .find_map(|m| match m {
            AgentMessage::Llm(Message::ToolResult(tr)) if tr.tool_call_id == "c-invalid" => {
                Some(tr.clone())
            }
            _ => None,
        })
        .expect("validation-failure tool result must be persisted");
    assert!(
        invalid_result.is_error,
        "invalid-args tool result must be an error"
    );

    // Case 2: valid arguments but the hook denies -> before_tool_call runs,
    // validation passed, but tool.execute still does NOT run. Same as above:
    // the denied result does not terminate, so a second response is needed.
    let provider = RecordingProvider::new(vec![
        tool_call_response("c-deny", "echo", r#"{"text":"hello"}"#),
        text_response("done"),
    ]);
    let before_calls2 = Arc::new(Mutex::new(Vec::<String>::new()));
    let hooks2 = Box::new(DenyHooks {
        deny: "echo".into(),
        before_calls: before_calls2.clone(),
    });
    let tool2 = CountingTool {
        calls: execs.clone(),
    };
    let mut agent = make_agent(provider, vec![Box::new(tool2)], hooks2);
    let result2 = agent.prompt("test").await.unwrap();

    assert_eq!(
        before_calls2.lock().unwrap().as_slice(),
        &["echo".to_string()],
        "before_tool_call runs before validation; it observes the valid-args call"
    );
    assert!(
        execs.lock().unwrap().is_empty(),
        "tool.execute must NOT run when before_tool_call denies"
    );
    let denied = result2
        .iter()
        .find_map(|m| match m {
            AgentMessage::Llm(Message::ToolResult(tr)) if tr.tool_call_id == "c-deny" => {
                Some(tr.clone())
            }
            _ => None,
        })
        .expect("denied tool result must be persisted");
    assert!(denied.is_error, "denied tool result must be an error");
    assert!(
        matches!(&denied.content[0], OutputContent::Text { text } if text == "denied by hook"),
        "denied result must carry the hook reason, got {:?}",
        denied.content
    );
}

// DoD: after_tool_call replacement is reflected in the final ToolExecutionEnd
// event (replacement happens before the event is emitted).
#[tokio::test]
async fn phase8_hook_contract_after_replace_before_events() {
    let provider = RecordingProvider::new(vec![
        tool_call_response("c1", "echo", r#"{"text":"hello"}"#),
        text_response("done"),
    ]);

    let end_results: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(Vec::new()));
    let end_results_clone = end_results.clone();

    let mut agent = make_agent(provider, vec![Box::new(EchoTool)], Box::new(ReplacingHooks));
    agent.subscribe(Box::new(move |e| {
        if let AgentEvent::ToolExecutionEnd { result, .. } = e {
            end_results_clone.lock().unwrap().push(result.clone());
        }
    }));
    agent.prompt("test").await.unwrap();

    let results = end_results.lock().unwrap();
    assert_eq!(results.len(), 1, "one tool execution end event expected");
    let replaced = &results[0];
    assert_eq!(
        replaced[0]["text"], "replaced: 1",
        "ToolExecutionEnd must carry the REPLACED result, proving after_tool_call ran before the event"
    );
}

// DoD: prepare_next_turn may inject a message that reaches the next provider
// request.
#[tokio::test]
async fn phase8_hook_contract_prepare_injection() {
    let provider = RecordingProvider::new(vec![
        tool_call_response("c1", "echo", r#"{"text":"hello"}"#),
        text_response("done"),
    ]);
    let received = provider.received_messages.clone();

    let injected = Arc::new(Mutex::new(false));
    let hooks = Box::new(InjectHooks {
        injected: injected.clone(),
    });

    let mut agent = make_agent(provider, vec![Box::new(EchoTool)], hooks);
    agent.prompt("test").await.unwrap();

    assert!(*injected.lock().unwrap(), "prepare_next_turn must have run");
    let msgs = received.lock().unwrap();
    assert_eq!(msgs.len(), 2, "provider called twice");
    assert!(
        user_text_in_messages(&msgs[1], "injected-from-prepare"),
        "injected prepare message must reach the second provider request"
    );
}

// DoD (Phase 17.2): a terminal should_stop_after_turn terminates the run after
// prepare_next_turn has applied the candidate state; no further turns or queue
// polling follow the stop.
#[tokio::test]
async fn phase8_hook_contract_terminal_stop_terminates_run() {
    let provider = RecordingProvider::new(vec![text_response("only")]);

    let hooks = RecordingHooks::new(true);
    let prepare_calls = hooks.prepare_calls.clone();

    let mut agent = make_agent(provider, vec![], Box::new(hooks));
    agent.prompt("test").await.unwrap();

    // Phase 17.2: prepare_next_turn runs before stop (applies candidate), then
    // the terminal stop ends the run.
    assert_eq!(
        prepare_calls.lock().unwrap().len(),
        1,
        "prepare_next_turn runs once before a terminal stop ends the run"
    );
}

// ---------------------------------------------------------------------------
// Phase 17.2 acceptance — P17-A04 / P17-NXT-003: stop observes the applied
// complete next-turn state (no observer sees a mixed/pre-preparation state).
// ---------------------------------------------------------------------------

fn llm_only(messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
    Ok(messages
        .iter()
        .filter_map(|m| match m {
            AgentMessage::Llm(m) => Some(m.clone()),
            _ => None,
        })
        .collect())
}

struct ObserveAppliedStateHooks {
    observed_max_tokens: Arc<Mutex<Option<u64>>>,
    observed_context_len: Arc<Mutex<Option<usize>>>,
    observed_temperature: Arc<Mutex<Option<f64>>>,
    observed_thinking_budget: Arc<Mutex<Option<u64>>>,
    observed_model: Arc<Mutex<Option<String>>>,
}

impl AgentHooks for ObserveAppliedStateHooks {
    fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
        llm_only(messages)
    }
    fn prepare_next_turn(
        &self,
        ctx: PrepareNextTurnContext,
    ) -> Pin<Box<dyn Future<Output = Result<Option<NextTurnState>, AgentError>> + Send>> {
        Box::pin(async move {
            // Replace the complete state: change EVERY mutable field — context,
            // model selection, thinking, max tokens, and temperature — so stop
            // must observe the complete replacement, never a mixed state.
            let mut next = ctx.state.clone();
            next.model_selection = ModelSelection::new("recording", "alt-model");
            next.inference.max_tokens = Some(9999);
            next.inference.temperature = Some(0.7);
            next.inference.thinking = ThinkingConfig {
                enabled: true,
                budget_tokens: Some(4096),
                ..Default::default()
            };
            next.context
                .push(AgentMessage::Llm(Message::User(UserMessage {
                    content: vec![InputContent::Text {
                        text: "prepared".into(),
                    }],
                    timestamp_ms: 0,
                })));
            Ok(Some(next))
        })
    }
    fn should_stop_after_turn(
        &self,
        ctx: ShouldStopAfterTurnContext,
    ) -> Pin<Box<dyn Future<Output = bool> + Send>> {
        let observed_max_tokens = self.observed_max_tokens.clone();
        let observed_context_len = self.observed_context_len.clone();
        let observed_temperature = self.observed_temperature.clone();
        let observed_thinking_budget = self.observed_thinking_budget.clone();
        let observed_model = self.observed_model.clone();
        Box::pin(async move {
            *observed_max_tokens.lock().unwrap() = ctx.state.inference.max_tokens;
            *observed_context_len.lock().unwrap() = Some(ctx.state.context.len());
            *observed_temperature.lock().unwrap() = ctx.state.inference.temperature;
            *observed_thinking_budget.lock().unwrap() = ctx.state.inference.thinking.budget_tokens;
            *observed_model.lock().unwrap() = Some(ctx.state.model_selection.model_id.clone());
            true
        })
    }
    fn before_tool_call(
        &self,
        _ctx: BeforeToolCallContext,
    ) -> Pin<Box<dyn Future<Output = BeforeToolCallResult> + Send>> {
        Box::pin(async { BeforeToolCallResult::Continue })
    }
}

#[tokio::test]
async fn phase17_stop_observes_complete_next_turn_state() {
    let provider = RecordingProvider::new(vec![text_response("hello")]);
    let observed_max_tokens = Arc::new(Mutex::new(None));
    let observed_context_len = Arc::new(Mutex::new(None));
    let observed_temperature = Arc::new(Mutex::new(None));
    let observed_thinking_budget = Arc::new(Mutex::new(None));
    let observed_model = Arc::new(Mutex::new(None));
    let mut agent = make_agent(
        provider,
        vec![],
        Box::new(ObserveAppliedStateHooks {
            observed_max_tokens: observed_max_tokens.clone(),
            observed_context_len: observed_context_len.clone(),
            observed_temperature: observed_temperature.clone(),
            observed_thinking_budget: observed_thinking_budget.clone(),
            observed_model: observed_model.clone(),
        }),
    );
    agent.prompt("test").await.unwrap();

    // should_stop ran AFTER prepare_next_turn applied the candidate, so it
    // observes ALL FIVE prepared fields together — the complete replacement,
    // never a mixed state (P17-A04: context, model, thinking, max tokens,
    // temperature).
    assert_eq!(
        *observed_max_tokens.lock().unwrap(),
        Some(9999),
        "should_stop must observe the prepared inference, not the pre-preparation value"
    );
    assert_eq!(
        *observed_temperature.lock().unwrap(),
        Some(0.7),
        "should_stop must observe the prepared temperature"
    );
    assert_eq!(
        *observed_thinking_budget.lock().unwrap(),
        Some(4096),
        "should_stop must observe the prepared thinking budget"
    );
    assert_eq!(
        observed_model.lock().unwrap().as_deref(),
        Some("alt-model"),
        "should_stop must observe the prepared model selection"
    );
    let len = observed_context_len
        .lock()
        .unwrap()
        .expect("should_stop must run after a successful prepare");
    assert!(
        len >= 3,
        "should_stop observed the prepared context (user + assistant + prepared): got {len}"
    );
}

// ---------------------------------------------------------------------------
// Phase 17.2 acceptance — P17-A05 / P17-NXT-002/004: a preparation failure
// preserves every mutable field and skips stop + steering/follow-up polling.
// ---------------------------------------------------------------------------

struct FailPrepareHooks {
    stop_calls: Arc<Mutex<u32>>,
}

impl AgentHooks for FailPrepareHooks {
    fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
        llm_only(messages)
    }
    fn prepare_next_turn(
        &self,
        _ctx: PrepareNextTurnContext,
    ) -> Pin<Box<dyn Future<Output = Result<Option<NextTurnState>, AgentError>> + Send>> {
        Box::pin(async { Err(AgentError::Hook("forced prepare failure".into())) })
    }
    fn should_stop_after_turn(
        &self,
        _ctx: ShouldStopAfterTurnContext,
    ) -> Pin<Box<dyn Future<Output = bool> + Send>> {
        let stop_calls = self.stop_calls.clone();
        Box::pin(async move {
            *stop_calls.lock().unwrap() += 1;
            false
        })
    }
    fn before_tool_call(
        &self,
        _ctx: BeforeToolCallContext,
    ) -> Pin<Box<dyn Future<Output = BeforeToolCallResult> + Send>> {
        Box::pin(async { BeforeToolCallResult::Continue })
    }
}

#[tokio::test]
async fn phase17_failed_prepare_preserves_state_and_skips_later_boundaries() {
    let provider = RecordingProvider::new(vec![text_response("hello")]);
    let received = provider.received_messages.clone();
    let stop_calls = Arc::new(Mutex::new(0u32));
    let mut agent = make_agent(
        provider,
        vec![],
        Box::new(FailPrepareHooks {
            stop_calls: stop_calls.clone(),
        }),
    );
    let result = agent.prompt("test").await;

    assert!(result.is_err(), "a failed prepare must surface an error");
    // should_stop was NOT called: prepare failed before the stop gate.
    assert_eq!(
        *stop_calls.lock().unwrap(),
        0,
        "should_stop must not run after a failed prepare"
    );
    // Prior state preserved: only the user message remains (the turn's
    // assistant message is discarded because the candidate never applied).
    assert_eq!(
        agent.messages_snapshot().len(),
        1,
        "prior state preserved: only the user message remains"
    );
    // No steering/follow-up polling: exactly one provider call, no next turn.
    assert_eq!(
        received.lock().unwrap().len(),
        1,
        "no further turns after a failed prepare"
    );
}

// ---------------------------------------------------------------------------
// Phase 17 exit closure — P17-NXT-002 validation leg: a prepare hook that
// returns Some(candidate) whose model selection the collection cannot resolve
// fails the transition with the TYPED InvalidNextTurnCandidate before any
// state applies; prior state, the stop gate, and queue polling are untouched.
// ---------------------------------------------------------------------------

struct InvalidCandidateHooks {
    stop_calls: Arc<Mutex<u32>>,
}

impl AgentHooks for InvalidCandidateHooks {
    fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
        llm_only(messages)
    }
    fn prepare_next_turn(
        &self,
        ctx: PrepareNextTurnContext,
    ) -> Pin<Box<dyn Future<Output = Result<Option<NextTurnState>, AgentError>> + Send>> {
        Box::pin(async move {
            // Canonical two-part shape that the collection cannot resolve: the
            // candidate passes the hook, then fails the loop-side validation.
            let mut next = ctx.state.clone();
            next.model_selection = ModelSelection::new("ghost", "missing-model");
            Ok(Some(next))
        })
    }
    fn should_stop_after_turn(
        &self,
        _ctx: ShouldStopAfterTurnContext,
    ) -> Pin<Box<dyn Future<Output = bool> + Send>> {
        let stop_calls = self.stop_calls.clone();
        Box::pin(async move {
            *stop_calls.lock().unwrap() += 1;
            false
        })
    }
    fn before_tool_call(
        &self,
        _ctx: BeforeToolCallContext,
    ) -> Pin<Box<dyn Future<Output = BeforeToolCallResult> + Send>> {
        Box::pin(async { BeforeToolCallResult::Continue })
    }
}

#[tokio::test]
async fn phase17_invalid_prepare_candidate_preserves_state_with_typed_error() {
    let provider = RecordingProvider::new(vec![text_response("hello")]);
    let received = provider.received_messages.clone();
    let stop_calls = Arc::new(Mutex::new(0u32));
    let mut agent = make_agent(
        provider,
        vec![],
        Box::new(InvalidCandidateHooks {
            stop_calls: stop_calls.clone(),
        }),
    );
    let result = agent.prompt("test").await;

    assert!(
        matches!(result, Err(AgentError::InvalidNextTurnCandidate { .. })),
        "an unresolvable prepare candidate fails with the typed validation error: {result:?}"
    );
    assert_eq!(
        *stop_calls.lock().unwrap(),
        0,
        "the stop gate must not run after an invalid candidate"
    );
    assert_eq!(
        agent.messages_snapshot().len(),
        1,
        "prior state preserved: only the user message remains"
    );
    assert_eq!(
        received.lock().unwrap().len(),
        1,
        "no further turns after an invalid candidate (no queue polling)"
    );
}

struct BlockingPrepareHooks {
    entered: Arc<tokio::sync::Notify>,
    captured: Arc<Mutex<Option<NextTurnState>>>,
}

impl AgentHooks for BlockingPrepareHooks {
    fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
        llm_only(messages)
    }

    fn prepare_next_turn(
        &self,
        ctx: PrepareNextTurnContext,
    ) -> Pin<Box<dyn Future<Output = Result<Option<NextTurnState>, AgentError>> + Send>> {
        let entered = self.entered.clone();
        let captured = self.captured.clone();
        Box::pin(async move {
            *captured.lock().unwrap() = Some(ctx.state);
            entered.notify_one();
            std::future::pending().await
        })
    }

    fn should_stop_after_turn(
        &self,
        _ctx: ShouldStopAfterTurnContext,
    ) -> Pin<Box<dyn Future<Output = bool> + Send>> {
        Box::pin(async { false })
    }

    fn before_tool_call(
        &self,
        _ctx: BeforeToolCallContext,
    ) -> Pin<Box<dyn Future<Output = BeforeToolCallResult> + Send>> {
        Box::pin(async { BeforeToolCallResult::Continue })
    }
}

#[tokio::test]
async fn cancellation_during_prepare_preserves_the_complete_prior_state() {
    let entered = Arc::new(tokio::sync::Notify::new());
    let captured = Arc::new(Mutex::new(None));
    let provider = RecordingProvider::new(vec![text_response("hello")]);
    let mut agent = make_agent(
        provider,
        vec![],
        Box::new(BlockingPrepareHooks {
            entered: entered.clone(),
            captured: captured.clone(),
        }),
    );
    let control = agent.control_handle();

    let prompt = agent.prompt("test");
    let cancel = async move {
        entered.notified().await;
        control.abort();
    };
    let (result, ()) = tokio::join!(prompt, cancel);
    assert!(matches!(result, Err(AgentError::Cancelled)));

    let before = captured.lock().unwrap().clone().expect("captured state");
    let after = agent.state_snapshot();
    let prior_context = &before.context[..before.context.len() - 1];
    assert_eq!(
        serde_json::to_value(&after.context).unwrap(),
        serde_json::to_value(prior_context).unwrap()
    );
    assert_eq!(after.model_selection, before.model_selection);
    assert_eq!(after.inference.max_tokens, before.inference.max_tokens);
    assert_eq!(after.inference.temperature, before.inference.temperature);
    assert_eq!(
        after.inference.thinking.enabled,
        before.inference.thinking.enabled
    );
    assert_eq!(
        after.inference.thinking.budget_tokens,
        before.inference.thinking.budget_tokens
    );
}
