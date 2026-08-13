//! Regression tests for agent loop semantics:
//!   H3 — batch parallel vs serial execution
//!   H4 — terminate flag (all vs partial)
//!   H5 — prepare_next_turn message injection
//!   M1 — should_stop_after_turn receives current-turn tool_results only

mod common;

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::StreamExt;
use futures_util::stream;
use opi_agent::event::AgentEvent;
use opi_agent::hooks::{
    AfterToolCallContext, AfterToolCallResult, AgentHooks, BeforeToolCallContext,
    BeforeToolCallResult, PrepareNextTurnContext, ShouldStopAfterTurnContext,
};
use opi_agent::loop_types::{
    AgentError, AgentLoopConfig, AgentLoopContext, InferenceConfig, ModelSelection, NextTurnState,
};
use opi_agent::message::AgentMessage;
use opi_agent::tool::{ExecutionMode, Tool, ToolError, ToolResult};
use opi_ai::message::{
    AssistantContent, AssistantMessage, InputContent, Message, OutputContent, ToolCall, ToolDef,
};
use opi_ai::provider::{EventStream, Provider, ProviderError, Request};
use opi_ai::stream::{AssistantStreamEvent, StopReason, Usage};
use opi_ai::test_support::single_route_collection;
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Recording mock provider
// ---------------------------------------------------------------------------

struct RecordingProvider {
    responses: Arc<Mutex<Vec<Vec<AssistantStreamEvent>>>>,
    call_count: Arc<Mutex<usize>>,
}

impl RecordingProvider {
    fn new(responses: Vec<Vec<AssistantStreamEvent>>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses)),
            call_count: Arc::new(Mutex::new(0)),
        }
    }
}

impl Provider for RecordingProvider {
    fn id(&self) -> &str {
        "recording"
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

// ---------------------------------------------------------------------------
// Order-logging tool (configurable execution mode and terminate flag)
// ---------------------------------------------------------------------------

struct OrderTool {
    name: String,
    log: Arc<Mutex<Vec<String>>>,
    mode: ExecutionMode,
    terminate: bool,
    delay_ms: Duration,
}

impl OrderTool {
    fn new(name: &str, log: Arc<Mutex<Vec<String>>>, mode: ExecutionMode, terminate: bool) -> Self {
        Self {
            name: name.into(),
            log,
            mode,
            terminate,
            delay_ms: Duration::from_millis(5),
        }
    }

    /// Like [`OrderTool::new`] but with an explicit per-call delay, used to make
    /// completion order diverge from assistant source order deterministically.
    fn new_with_delay(
        name: &str,
        log: Arc<Mutex<Vec<String>>>,
        mode: ExecutionMode,
        terminate: bool,
        delay_ms: Duration,
    ) -> Self {
        Self {
            delay_ms,
            ..Self::new(name, log, mode, terminate)
        }
    }
}

impl Tool for OrderTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: self.name.clone(),
            description: format!("order tool: {}", self.name),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": { "arg": { "type": "string" } },
                "required": ["arg"]
            }),
        }
    }

    fn execute(
        &self,
        _call_id: &str,
        _args: serde_json::Value,
        _signal: CancellationToken,
        _on_update: Option<opi_agent::tool::UpdateCallback>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send>> {
        let log = self.log.clone();
        let name = self.name.clone();
        let terminate = self.terminate;
        let delay_ms = self.delay_ms;
        Box::pin(async move {
            log.lock().unwrap().push(format!("{name}:start"));
            tokio::time::sleep(delay_ms).await;
            log.lock().unwrap().push(format!("{name}:end"));
            Ok(ToolResult {
                content: vec![OutputContent::Text {
                    text: format!("{name}:ok"),
                }],
                details: None,
                is_error: false,
                terminate,
                truncated: false,
                diagnostics: vec![],
            })
        })
    }

    fn execution_mode(&self) -> ExecutionMode {
        self.mode
    }
}

// ---------------------------------------------------------------------------
// Simple echo tool
// ---------------------------------------------------------------------------

struct EchoTool;

impl Tool for EchoTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "echo".into(),
            description: "echo".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": { "arg": { "type": "string" } },
                "required": ["arg"]
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
        let text = args["arg"].as_str().unwrap_or_default().to_owned();
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
}

// ---------------------------------------------------------------------------
// Hook implementations
// ---------------------------------------------------------------------------

struct MinimalHooks;

impl AgentHooks for MinimalHooks {
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
        Box::pin(async { BeforeToolCallResult::Continue })
    }

    fn after_tool_call(
        &self,
        _: AfterToolCallContext,
    ) -> Pin<Box<dyn Future<Output = AfterToolCallResult> + Send>> {
        Box::pin(async { AfterToolCallResult::Keep })
    }
}

struct RecordingStopHooks {
    stop_contexts: Arc<Mutex<Vec<ShouldStopAfterTurnContext>>>,
}

impl RecordingStopHooks {
    fn new() -> Self {
        Self {
            stop_contexts: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl AgentHooks for RecordingStopHooks {
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
        ctx: ShouldStopAfterTurnContext,
    ) -> Pin<Box<dyn Future<Output = bool> + Send>> {
        let stop_contexts = self.stop_contexts.clone();
        Box::pin(async move {
            stop_contexts.lock().unwrap().push(ctx);
            false
        })
    }

    fn before_tool_call(
        &self,
        _: BeforeToolCallContext,
    ) -> Pin<Box<dyn Future<Output = BeforeToolCallResult> + Send>> {
        Box::pin(async { BeforeToolCallResult::Continue })
    }

    fn after_tool_call(
        &self,
        _: AfterToolCallContext,
    ) -> Pin<Box<dyn Future<Output = AfterToolCallResult> + Send>> {
        Box::pin(async { AfterToolCallResult::Keep })
    }
}

struct InjectingHooks {
    inject_on_turn: u32,
    inject_text: String,
}

impl InjectingHooks {
    fn new(inject_on_turn: u32, inject_text: String) -> Self {
        Self {
            inject_on_turn,
            inject_text,
        }
    }
}

impl AgentHooks for InjectingHooks {
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
        Box::pin(async { BeforeToolCallResult::Continue })
    }

    fn prepare_next_turn(
        &self,
        ctx: PrepareNextTurnContext,
    ) -> Pin<Box<dyn Future<Output = Result<Option<NextTurnState>, AgentError>> + Send>> {
        let text = self.inject_text.clone();
        let inject_on_turn = self.inject_on_turn;
        Box::pin(async move {
            if ctx.turn == inject_on_turn {
                let mut state = ctx.state.clone();
                state.context.push(AgentMessage::Llm(Message::User(
                    opi_ai::message::UserMessage {
                        content: vec![InputContent::Text { text }],
                        timestamp_ms: 0,
                    },
                )));
                Ok(Some(state))
            } else {
                Ok(None)
            }
        })
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn base_msg() -> AssistantMessage {
    AssistantMessage {
        content: vec![],
        api: opi_ai::ApiKind::Anthropic,
        provider: "recording".into(),
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

fn multi_tool_call_response(calls: Vec<(&str, &str, &str)>) -> Vec<AssistantStreamEvent> {
    let mut partial = base_msg();
    for (id, name, args) in &calls {
        let tc = ToolCall {
            id: id.to_string(),
            name: name.to_string(),
            arguments: args.to_string(),
        };
        partial
            .content
            .push(AssistantContent::ToolCall { tool_call: tc });
    }
    partial.stop_reason = StopReason::ToolUse;

    let mut events = vec![AssistantStreamEvent::Start {
        partial: base_msg(),
    }];
    for (id, name, args) in &calls {
        let tc = ToolCall {
            id: id.to_string(),
            name: name.to_string(),
            arguments: args.to_string(),
        };
        events.push(AssistantStreamEvent::ToolCallEnd {
            content_index: 0,
            tool_call: tc,
            partial: partial.clone(),
        });
    }
    events.push(AssistantStreamEvent::Done {
        reason: StopReason::ToolUse,
        message: partial,
    });
    events
}

fn make_context(provider: Box<dyn Provider>, tools: Vec<Box<dyn Tool>>) -> AgentLoopContext {
    let model_spec = format!("{}:mock", provider.id());
    let messages = vec![AgentMessage::Llm(Message::User(
        opi_ai::message::UserMessage {
            content: vec![InputContent::Text {
                text: "test".into(),
            }],
            timestamp_ms: 0,
        },
    ))];
    AgentLoopContext {
        collection: Arc::new(single_route_collection(provider)),
        registry: common::test_registry(tools),
        authorizer: Some(common::permissive_authorizer()),
        evidence_health: opi_agent::evidence::EvidenceHealth::healthy(),
        state: NextTurnState::new(
            messages,
            ModelSelection::parse_spec(&model_spec).unwrap(),
            InferenceConfig::default(),
        ),
        system: None,
        steering_queue: None,
        follow_up_queue: None,
        diagnostic_sink: None,
        trace: None,
        session_id: None,
        evidence_sink: None,
    }
}

fn noop_sink() -> Box<dyn Fn(AgentEvent) + Send + Sync> {
    Box::new(|_| {})
}

// ---------------------------------------------------------------------------
// H3: Batch parallel execution
// ---------------------------------------------------------------------------

#[tokio::test]
async fn h3_parallel_tools_both_execute() {
    let log = Arc::new(Mutex::new(Vec::new()));

    let provider = RecordingProvider::new(vec![
        multi_tool_call_response(vec![
            ("c1", "tool_a", r#"{"arg":"a"}"#),
            ("c2", "tool_b", r#"{"arg":"b"}"#),
        ]),
        text_response("done"),
    ]);

    let tools: Vec<Box<dyn Tool>> = vec![
        Box::new(OrderTool::new(
            "tool_a",
            log.clone(),
            ExecutionMode::Parallel,
            false,
        )),
        Box::new(OrderTool::new(
            "tool_b",
            log.clone(),
            ExecutionMode::Parallel,
            false,
        )),
    ];

    let context = make_context(Box::new(provider), tools);
    let hooks = MinimalHooks;

    opi_agent::agent_loop(
        context,
        AgentLoopConfig::default(),
        &hooks,
        noop_sink(),
        CancellationToken::new(),
    )
    .await
    .unwrap();

    let entries = log.lock().unwrap();
    assert!(
        entries.contains(&"tool_a:start".to_string()),
        "tool_a should execute"
    );
    assert!(
        entries.contains(&"tool_b:start".to_string()),
        "tool_b should execute"
    );
}

#[tokio::test]
async fn h3_sequential_tool_forces_serial_execution() {
    let log = Arc::new(Mutex::new(Vec::new()));

    let provider = RecordingProvider::new(vec![
        multi_tool_call_response(vec![
            ("c1", "tool_a", r#"{"arg":"a"}"#),
            ("c2", "tool_b", r#"{"arg":"b"}"#),
        ]),
        text_response("done"),
    ]);

    let tools: Vec<Box<dyn Tool>> = vec![
        Box::new(OrderTool::new(
            "tool_a",
            log.clone(),
            ExecutionMode::Sequential,
            false,
        )),
        Box::new(OrderTool::new(
            "tool_b",
            log.clone(),
            ExecutionMode::Parallel,
            false,
        )),
    ];

    let context = make_context(Box::new(provider), tools);
    let hooks = MinimalHooks;

    opi_agent::agent_loop(
        context,
        AgentLoopConfig::default(),
        &hooks,
        noop_sink(),
        CancellationToken::new(),
    )
    .await
    .unwrap();

    let entries = log.lock().unwrap();
    let a_end = entries
        .iter()
        .position(|e| e == "tool_a:end")
        .expect("tool_a should end");
    let b_start = entries
        .iter()
        .position(|e| e == "tool_b:start")
        .expect("tool_b should start");
    assert!(
        a_end < b_start,
        "sequential batch: tool_a must complete before tool_b starts, got: {entries:?}"
    );
}

// ---------------------------------------------------------------------------
// Phase 11.8 S3: consolidated tool-scheduling contract.
//
// Pins the agent_loop batch classification (batch_is_sequential): any
// Sequential tool in a batch forces serial execution; an all-Parallel batch
// runs concurrently. Detailed per-case coverage lives in the phase8_* tests
// below; this is the named scenario anchor that pins both invariants with
// observable per-tool ordering. Real async sleeps make join_all concurrency
// observable via interleaving rather than submission order.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn phase8_tool_scheduling_contract() {
    use std::time::Duration;

    // (1) A batch containing a Sequential tool forces serial execution: the
    // sequential tool completes before the parallel one starts.
    let log = Arc::new(Mutex::new(Vec::new()));
    let provider = RecordingProvider::new(vec![
        multi_tool_call_response(vec![
            ("s1", "seq_a", r#"{"arg":"a"}"#),
            ("s2", "par_a", r#"{"arg":"b"}"#),
        ]),
        text_response("done"),
    ]);
    let tools: Vec<Box<dyn Tool>> = vec![
        Box::new(OrderTool::new(
            "seq_a",
            log.clone(),
            ExecutionMode::Sequential,
            false,
        )),
        Box::new(OrderTool::new(
            "par_a",
            log.clone(),
            ExecutionMode::Parallel,
            false,
        )),
    ];
    opi_agent::agent_loop(
        make_context(Box::new(provider), tools),
        AgentLoopConfig::default(),
        &MinimalHooks,
        noop_sink(),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    {
        let entries = log.lock().unwrap();
        let seq_end = entries
            .iter()
            .position(|e| e == "seq_a:end")
            .expect("seq_a should end");
        let par_start = entries
            .iter()
            .position(|e| e == "par_a:start")
            .expect("par_a should start");
        assert!(
            seq_end < par_start,
            "sequential batch: seq_a must complete before par_a starts: {entries:?}"
        );
    }

    // (2) An all-Parallel batch runs concurrently: the short tool starts before
    // the long tool ends (interleaving impossible under serial execution).
    let log2 = Arc::new(Mutex::new(Vec::new()));
    let provider2 = RecordingProvider::new(vec![
        multi_tool_call_response(vec![
            ("p1", "par_long", r#"{"arg":"a"}"#),
            ("p2", "par_short", r#"{"arg":"b"}"#),
        ]),
        text_response("done"),
    ]);
    let tools2: Vec<Box<dyn Tool>> = vec![
        Box::new(OrderTool::new_with_delay(
            "par_long",
            log2.clone(),
            ExecutionMode::Parallel,
            false,
            Duration::from_millis(60),
        )),
        Box::new(OrderTool::new_with_delay(
            "par_short",
            log2.clone(),
            ExecutionMode::Parallel,
            false,
            Duration::from_millis(5),
        )),
    ];
    opi_agent::agent_loop(
        make_context(Box::new(provider2), tools2),
        AgentLoopConfig::default(),
        &MinimalHooks,
        noop_sink(),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    {
        let entries = log2.lock().unwrap();
        let long_end = entries
            .iter()
            .position(|e| e == "par_long:end")
            .expect("par_long should end");
        let short_start = entries
            .iter()
            .position(|e| e == "par_short:start")
            .expect("par_short should start");
        assert!(
            short_start < long_end,
            "parallel batch: par_short must start before par_long ends (concurrent): {entries:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// H4: Terminate flags
// ---------------------------------------------------------------------------

#[tokio::test]
async fn h4_all_terminate_stops_early() {
    let log = Arc::new(Mutex::new(Vec::new()));

    let provider = RecordingProvider::new(vec![multi_tool_call_response(vec![
        ("c1", "tool_a", r#"{"arg":"a"}"#),
        ("c2", "tool_b", r#"{"arg":"b"}"#),
    ])]);
    let call_count = provider.call_count.clone();

    let tools: Vec<Box<dyn Tool>> = vec![
        Box::new(OrderTool::new(
            "tool_a",
            log.clone(),
            ExecutionMode::Parallel,
            true,
        )),
        Box::new(OrderTool::new(
            "tool_b",
            log.clone(),
            ExecutionMode::Parallel,
            true,
        )),
    ];

    let context = make_context(Box::new(provider), tools);
    let hooks = MinimalHooks;

    let result = opi_agent::agent_loop(
        context,
        AgentLoopConfig::default(),
        &hooks,
        noop_sink(),
        CancellationToken::new(),
    )
    .await
    .unwrap();

    assert_eq!(
        *call_count.lock().unwrap(),
        1,
        "provider should be called once when all tools terminate"
    );
    assert!(
        result.context.len() >= 3,
        "should have user + assistant + tool results"
    );
}

#[tokio::test]
async fn h4_partial_terminate_continues() {
    let log = Arc::new(Mutex::new(Vec::new()));

    let provider = RecordingProvider::new(vec![
        multi_tool_call_response(vec![
            ("c1", "tool_a", r#"{"arg":"a"}"#),
            ("c2", "tool_b", r#"{"arg":"b"}"#),
        ]),
        text_response("done"),
    ]);
    let call_count = provider.call_count.clone();

    let tools: Vec<Box<dyn Tool>> = vec![
        Box::new(OrderTool::new(
            "tool_a",
            log.clone(),
            ExecutionMode::Parallel,
            true,
        )),
        Box::new(OrderTool::new(
            "tool_b",
            log.clone(),
            ExecutionMode::Parallel,
            false,
        )),
    ];

    let context = make_context(Box::new(provider), tools);
    let hooks = MinimalHooks;

    opi_agent::agent_loop(
        context,
        AgentLoopConfig::default(),
        &hooks,
        noop_sink(),
        CancellationToken::new(),
    )
    .await
    .unwrap();

    assert_eq!(
        *call_count.lock().unwrap(),
        2,
        "provider should be called twice when only some tools terminate"
    );
}

// ---------------------------------------------------------------------------
// H5: prepare_next_turn injection
// ---------------------------------------------------------------------------

#[tokio::test]
async fn h5_prepare_next_turn_injects_and_continues() {
    let provider = RecordingProvider::new(vec![text_response("first"), text_response("second")]);
    let call_count = provider.call_count.clone();

    let hooks = InjectingHooks::new(1, "injected context".into());

    let context = make_context(Box::new(provider), vec![]);

    let result = opi_agent::agent_loop(
        context,
        AgentLoopConfig::default(),
        &hooks,
        noop_sink(),
        CancellationToken::new(),
    )
    .await
    .unwrap();

    assert_eq!(
        *call_count.lock().unwrap(),
        2,
        "provider should be called twice after prepare_next_turn injection"
    );

    let has_injected = result.context.iter().any(|m| match m {
        AgentMessage::Llm(Message::User(u)) => u
            .content
            .iter()
            .any(|c| matches!(c, InputContent::Text { text } if text == "injected context")),
        _ => false,
    });
    assert!(has_injected, "injected message should appear in result");
}

// ---------------------------------------------------------------------------
// M1: should_stop_after_turn receives current-turn tool_results only
// ---------------------------------------------------------------------------

#[tokio::test]
async fn m1_tool_results_scoped_to_current_turn() {
    let provider = RecordingProvider::new(vec![
        multi_tool_call_response(vec![("c1", "echo", r#"{"arg":"first"}"#)]),
        multi_tool_call_response(vec![("c2", "echo", r#"{"arg":"second"}"#)]),
        text_response("done"),
    ]);

    let hooks = RecordingStopHooks::new();
    let stop_contexts = hooks.stop_contexts.clone();

    let context = make_context(Box::new(provider), vec![Box::new(EchoTool)]);

    opi_agent::agent_loop(
        context,
        AgentLoopConfig::default(),
        &hooks,
        noop_sink(),
        CancellationToken::new(),
    )
    .await
    .unwrap();

    let contexts = stop_contexts.lock().unwrap();
    let with_tools: Vec<_> = contexts
        .iter()
        .filter(|c| !c.tool_results.is_empty())
        .collect();

    assert!(
        with_tools.len() >= 2,
        "at least 2 turns should have tool_results"
    );

    // Turn 1: only c1
    assert_eq!(
        with_tools[0].tool_results.len(),
        1,
        "turn 1 should have exactly 1 tool_result"
    );
    assert_eq!(with_tools[0].tool_results[0].tool_call_id, "c1");

    // Turn 2: only c2 (not accumulated from turn 1)
    assert_eq!(
        with_tools[1].tool_results.len(),
        1,
        "turn 2 should have exactly 1 tool_result (current turn only)"
    );
    assert_eq!(with_tools[1].tool_results[0].tool_call_id, "c2");
}

// ---------------------------------------------------------------------------
// Phase 8: runtime event order and queue-polling contract (task 8.1)
//
// Pins the documented agent_start -> agent_end order through
// opi_agent::agent_loop: AgentStart/AgentEnd bracket the run; per turn the
// order is TurnStart -> assistant message events -> tool execution -> TurnEnd
// -> should_stop_after_turn -> prepare_next_turn -> steering/follow-up polling.
// ---------------------------------------------------------------------------

/// Reduce an `AgentEvent` to a stable kind label for sequence assertions.
fn event_kind(e: &AgentEvent) -> &'static str {
    match e {
        AgentEvent::AgentStart => "agent_start",
        AgentEvent::AgentEnd { .. } => "agent_end",
        AgentEvent::TurnStart => "turn_start",
        AgentEvent::TurnEnd { .. } => "turn_end",
        AgentEvent::MessageStart { .. } => "message_start",
        AgentEvent::MessageUpdate { .. } => "message_update",
        AgentEvent::MessageEnd { .. } => "message_end",
        AgentEvent::ToolExecutionStart { .. } => "tool_execution_start",
        AgentEvent::ToolExecutionUpdate { .. } => "tool_execution_update",
        AgentEvent::ToolExecutionEnd { .. } => "tool_execution_end",
        AgentEvent::QueueUpdate { .. } => "queue_update",
        AgentEvent::AutoRetryStart { .. } => "auto_retry_start",
        AgentEvent::AutoRetryEnd { .. } => "auto_retry_end",
        AgentEvent::CompactionStart { .. } => "compaction_start",
        AgentEvent::CompactionEnd { .. } => "compaction_end",
        AgentEvent::SessionPersistError { .. } => "session_persist_error",
        // AgentEvent is #[non_exhaustive]; future variants collapse to "other".
        _ => "other",
    }
}

/// Build an event sink that records the kind of every emitted `AgentEvent`.
fn recording_sink(log: Arc<Mutex<Vec<String>>>) -> Box<dyn Fn(AgentEvent) + Send + Sync> {
    Box::new(move |e| {
        log.lock().unwrap().push(event_kind(&e).to_string());
    })
}

/// Position of the first occurrence of `kind` in the recorded sequence.
fn event_pos(seq: &[String], kind: &str) -> usize {
    seq.iter()
        .position(|s| s == kind)
        .unwrap_or_else(|| panic!("event `{kind}` not present in sequence: {seq:?}"))
}

/// True if `m` is a user text message equal to `target`.
fn matches_user_text(m: &AgentMessage, target: &str) -> bool {
    matches!(
        m,
        AgentMessage::Llm(Message::User(u))
            if u.content.iter().any(|c| matches!(
                c,
                InputContent::Text { text } if text == target
            ))
    )
}

/// Hooks that terminate the run on the first `should_stop_after_turn` and
/// record every `prepare_next_turn` invocation. Models a compaction coordinator
/// that requests a stop before the next turn.
struct TerminalStopHooks {
    prepare_calls: Arc<Mutex<Vec<u32>>>,
}

impl AgentHooks for TerminalStopHooks {
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
        Box::pin(async { true })
    }

    fn before_tool_call(
        &self,
        _: BeforeToolCallContext,
    ) -> Pin<Box<dyn Future<Output = BeforeToolCallResult> + Send>> {
        Box::pin(async { BeforeToolCallResult::Continue })
    }

    fn after_tool_call(
        &self,
        _: AfterToolCallContext,
    ) -> Pin<Box<dyn Future<Output = AfterToolCallResult> + Send>> {
        Box::pin(async { AfterToolCallResult::Keep })
    }

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

// DoD: no-tool run emits the documented order with no tool events.
#[tokio::test]
async fn phase8_event_order_no_tool_run() {
    let provider = RecordingProvider::new(vec![text_response("done")]);
    let call_count = provider.call_count.clone();

    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = recording_sink(events.clone());

    let context = make_context(Box::new(provider), vec![]);
    let hooks = MinimalHooks;

    opi_agent::agent_loop(
        context,
        AgentLoopConfig::default(),
        &hooks,
        sink,
        CancellationToken::new(),
    )
    .await
    .unwrap();

    let seq = events.lock().unwrap();
    assert_eq!(
        seq.first().map(String::as_str),
        Some("agent_start"),
        "AgentStart must be first: {seq:?}"
    );
    assert_eq!(
        seq.last().map(String::as_str),
        Some("agent_end"),
        "AgentEnd must be last: {seq:?}"
    );
    assert_eq!(
        *call_count.lock().unwrap(),
        1,
        "no-tool run makes exactly one provider call"
    );
    assert!(
        !seq.contains(&"tool_execution_start".to_string()),
        "no tool events in a no-tool run: {seq:?}"
    );
    assert!(!seq.contains(&"tool_execution_end".to_string()));
    assert_eq!(
        seq.iter().filter(|s| s.as_str() == "turn_start").count(),
        1,
        "no-tool run is a single turn"
    );
    assert_eq!(seq.iter().filter(|s| s.as_str() == "turn_end").count(), 1);
    // turn_start < message_end < turn_end within the only turn
    assert!(event_pos(&seq, "turn_start") < event_pos(&seq, "message_end"));
    assert!(event_pos(&seq, "message_end") < event_pos(&seq, "turn_end"));
}

// DoD: one-tool run brackets tool execution inside the first turn and runs a
// second turn for the final assistant response.
#[tokio::test]
async fn phase8_event_order_one_tool_run() {
    let provider = RecordingProvider::new(vec![
        multi_tool_call_response(vec![("c1", "echo", r#"{"arg":"x"}"#)]),
        text_response("done"),
    ]);
    let call_count = provider.call_count.clone();

    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = recording_sink(events.clone());

    let context = make_context(Box::new(provider), vec![Box::new(EchoTool)]);
    let hooks = MinimalHooks;

    opi_agent::agent_loop(
        context,
        AgentLoopConfig::default(),
        &hooks,
        sink,
        CancellationToken::new(),
    )
    .await
    .unwrap();

    let seq = events.lock().unwrap();
    assert_eq!(seq.first().map(String::as_str), Some("agent_start"));
    assert_eq!(seq.last().map(String::as_str), Some("agent_end"));
    assert_eq!(
        *call_count.lock().unwrap(),
        2,
        "one-tool run makes two provider calls"
    );
    assert_eq!(
        seq.iter().filter(|s| s.as_str() == "turn_start").count(),
        2,
        "one-tool run spans two turns"
    );
    assert_eq!(seq.iter().filter(|s| s.as_str() == "turn_end").count(), 2);
    assert_eq!(
        seq.iter()
            .filter(|s| s.as_str() == "tool_execution_start")
            .count(),
        1
    );
    assert_eq!(
        seq.iter()
            .filter(|s| s.as_str() == "tool_execution_end")
            .count(),
        1
    );
    // Tool execution sits inside the first turn: after its assistant
    // message_end and before the first turn_end.
    let first_turn_end = event_pos(&seq, "turn_end");
    assert!(event_pos(&seq, "message_end") < event_pos(&seq, "tool_execution_start"));
    assert!(event_pos(&seq, "tool_execution_start") < first_turn_end);
    assert!(event_pos(&seq, "tool_execution_end") < first_turn_end);
}

// DoD: prepare_next_turn injection is applied before follow-up polling. With a
// turn-1 injection and a queued follow-up, the injected message reaches the
// provider strictly before the follow-up message.
#[tokio::test]
async fn phase8_event_order_prepare_next_turn_injection() {
    let provider = RecordingProvider::new(vec![
        text_response("first"),
        text_response("second"),
        text_response("third"),
    ]);
    let call_count = provider.call_count.clone();

    let follow_up_queue: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::from(vec![
        "follow-up-msg".to_string(),
    ])));

    let model_spec = format!("{}:mock", provider.id());
    let seed_messages = vec![AgentMessage::Llm(Message::User(
        opi_ai::message::UserMessage {
            content: vec![InputContent::Text {
                text: "seed".into(),
            }],
            timestamp_ms: 0,
        },
    ))];
    let context = AgentLoopContext {
        collection: Arc::new(single_route_collection(Box::new(provider))),
        registry: common::test_registry(vec![]),
        authorizer: Some(common::permissive_authorizer()),
        evidence_health: opi_agent::evidence::EvidenceHealth::healthy(),
        state: NextTurnState::new(
            seed_messages,
            ModelSelection::parse_spec(&model_spec).unwrap(),
            InferenceConfig::default(),
        ),
        system: None,
        steering_queue: None,
        follow_up_queue: Some(follow_up_queue),
        diagnostic_sink: None,
        trace: None,
        session_id: None,
        evidence_sink: None,
    };
    let hooks = InjectingHooks::new(1, "injected-context".into());

    let result = opi_agent::agent_loop(
        context,
        AgentLoopConfig::default(),
        &hooks,
        noop_sink(),
        CancellationToken::new(),
    )
    .await
    .unwrap();

    assert_eq!(
        *call_count.lock().unwrap(),
        3,
        "injection then follow-up yield three provider calls"
    );
    let injected_at = result
        .context
        .iter()
        .position(|m| matches_user_text(m, "injected-context"))
        .expect("injected message must be present");
    let follow_up_at = result
        .context
        .iter()
        .position(|m| matches_user_text(m, "follow-up-msg"))
        .expect("follow-up message must be present");
    assert!(
        injected_at < follow_up_at,
        "prepare_next_turn injection must precede follow-up delivery"
    );
}

// DoD (Phase 17.2): a terminal should_stop_after_turn stops the run after
// prepare_next_turn has applied the candidate state, before any queue is polled.
#[tokio::test]
async fn phase8_event_order_terminal_stop_runs_prepare_then_stops() {
    let provider = RecordingProvider::new(vec![text_response("only")]);
    let call_count = provider.call_count.clone();

    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = recording_sink(events.clone());

    let hooks = TerminalStopHooks {
        prepare_calls: Arc::new(Mutex::new(Vec::new())),
    };
    let prepare_calls = hooks.prepare_calls.clone();

    let context = make_context(Box::new(provider), vec![]);

    opi_agent::agent_loop(
        context,
        AgentLoopConfig::default(),
        &hooks,
        sink,
        CancellationToken::new(),
    )
    .await
    .unwrap();

    let seq = events.lock().unwrap();
    assert_eq!(seq.first().map(String::as_str), Some("agent_start"));
    assert_eq!(seq.last().map(String::as_str), Some("agent_end"));
    assert_eq!(
        *call_count.lock().unwrap(),
        1,
        "terminal stop makes exactly one provider call"
    );
    // Phase 17.2: prepare_next_turn runs BEFORE should_stop (applies the
    // candidate state that stop observes), so it is invoked once; the terminal
    // stop then ends the run without polling queues.
    assert_eq!(
        prepare_calls.lock().unwrap().len(),
        1,
        "prepare_next_turn runs once before a terminal stop ends the run"
    );
    assert!(
        !seq.contains(&"queue_update".to_string()),
        "no queue polling after a terminal should_stop_after_turn: {seq:?}"
    );
}

// ---------------------------------------------------------------------------
// Phase 8: tool scheduling and termination contract (task 8.3)
//
// Pins the documented scheduler rules through opi_agent::agent_loop: global
// default Parallel mode, per-tool Sequential override, mixed-batch
// sequential-forces-batch, source-ordered persisted tool results (independent
// of completion order), one ToolExecutionEnd per tool, and early termination
// only when every finalized result sets terminate.
// ---------------------------------------------------------------------------

/// Collect persisted `ToolResult` tool-call ids in message order.
fn persisted_tool_result_ids(messages: &[AgentMessage]) -> Vec<String> {
    messages
        .iter()
        .filter_map(|m| match m {
            AgentMessage::Llm(Message::ToolResult(trm)) => Some(trm.tool_call_id.clone()),
            _ => None,
        })
        .collect()
}

/// Event sink that records the `tool_call_id` of every `ToolExecutionEnd`.
fn tool_end_sink(ids: Arc<Mutex<Vec<String>>>) -> Box<dyn Fn(AgentEvent) + Send + Sync> {
    Box::new(move |e| {
        if let AgentEvent::ToolExecutionEnd { tool_call_id, .. } = e {
            ids.lock().unwrap().push(tool_call_id);
        }
    })
}

// DoD: a parallel batch executes every tool call.
#[tokio::test]
async fn phase8_tool_scheduling_parallel_batch_executes_all() {
    let log = Arc::new(Mutex::new(Vec::new()));

    let provider = RecordingProvider::new(vec![
        multi_tool_call_response(vec![
            ("c1", "tool_a", r#"{"arg":"a"}"#),
            ("c2", "tool_b", r#"{"arg":"b"}"#),
        ]),
        text_response("done"),
    ]);

    let tools: Vec<Box<dyn Tool>> = vec![
        Box::new(OrderTool::new(
            "tool_a",
            log.clone(),
            ExecutionMode::Parallel,
            false,
        )),
        Box::new(OrderTool::new(
            "tool_b",
            log.clone(),
            ExecutionMode::Parallel,
            false,
        )),
    ];

    let context = make_context(Box::new(provider), tools);
    let hooks = MinimalHooks;

    opi_agent::agent_loop(
        context,
        AgentLoopConfig::default(),
        &hooks,
        noop_sink(),
        CancellationToken::new(),
    )
    .await
    .unwrap();

    let entries = log.lock().unwrap();
    assert!(
        entries.contains(&"tool_a:start".to_string()),
        "tool_a executes: {entries:?}"
    );
    assert!(
        entries.contains(&"tool_b:start".to_string()),
        "tool_b executes: {entries:?}"
    );
}

// DoD: a fully-sequential batch runs strictly serially in source order.
#[tokio::test]
async fn phase8_tool_scheduling_sequential_batch_runs_serially() {
    let log = Arc::new(Mutex::new(Vec::new()));

    let provider = RecordingProvider::new(vec![
        multi_tool_call_response(vec![
            ("c1", "tool_a", r#"{"arg":"a"}"#),
            ("c2", "tool_b", r#"{"arg":"b"}"#),
        ]),
        text_response("done"),
    ]);

    let tools: Vec<Box<dyn Tool>> = vec![
        Box::new(OrderTool::new(
            "tool_a",
            log.clone(),
            ExecutionMode::Sequential,
            false,
        )),
        Box::new(OrderTool::new(
            "tool_b",
            log.clone(),
            ExecutionMode::Sequential,
            false,
        )),
    ];

    let context = make_context(Box::new(provider), tools);
    let hooks = MinimalHooks;

    opi_agent::agent_loop(
        context,
        AgentLoopConfig::default(),
        &hooks,
        noop_sink(),
        CancellationToken::new(),
    )
    .await
    .unwrap();

    let entries = log.lock().unwrap();
    let a_end = entries
        .iter()
        .position(|e| e == "tool_a:end")
        .expect("tool_a ends");
    let b_start = entries
        .iter()
        .position(|e| e == "tool_b:start")
        .expect("tool_b starts");
    assert!(
        a_end < b_start,
        "sequential batch: tool_a must finish before tool_b starts, got: {entries:?}"
    );
}

// DoD: a single Sequential tool in a mixed batch forces the whole batch
// sequential even when the other tool is Parallel.
#[tokio::test]
async fn phase8_tool_scheduling_mixed_batch_forces_sequential() {
    let log = Arc::new(Mutex::new(Vec::new()));

    let provider = RecordingProvider::new(vec![
        multi_tool_call_response(vec![
            ("c1", "tool_a", r#"{"arg":"a"}"#),
            ("c2", "tool_b", r#"{"arg":"b"}"#),
        ]),
        text_response("done"),
    ]);

    let tools: Vec<Box<dyn Tool>> = vec![
        Box::new(OrderTool::new(
            "tool_a",
            log.clone(),
            ExecutionMode::Sequential,
            false,
        )),
        Box::new(OrderTool::new(
            "tool_b",
            log.clone(),
            ExecutionMode::Parallel,
            false,
        )),
    ];

    let context = make_context(Box::new(provider), tools);
    let hooks = MinimalHooks;

    opi_agent::agent_loop(
        context,
        AgentLoopConfig::default(),
        &hooks,
        noop_sink(),
        CancellationToken::new(),
    )
    .await
    .unwrap();

    let entries = log.lock().unwrap();
    let a_end = entries
        .iter()
        .position(|e| e == "tool_a:end")
        .expect("tool_a ends");
    let b_start = entries
        .iter()
        .position(|e| e == "tool_b:start")
        .expect("tool_b starts");
    assert!(
        a_end < b_start,
        "mixed batch with one Sequential tool must run serially, got: {entries:?}"
    );
}

// DoD: persisted tool-result messages follow assistant source order even when
// completion order diverges (tool_a is slower than tool_b but listed first).
// join_all preserves input order, so persistence is source order, not
// completion order.
#[tokio::test]
async fn phase8_tool_scheduling_persisted_results_in_source_order() {
    let log = Arc::new(Mutex::new(Vec::new()));

    let provider = RecordingProvider::new(vec![
        multi_tool_call_response(vec![
            ("c1", "tool_a", r#"{"arg":"a"}"#),
            ("c2", "tool_b", r#"{"arg":"b"}"#),
        ]),
        text_response("done"),
    ]);

    let tools: Vec<Box<dyn Tool>> = vec![
        // tool_a is first in source order (c1) but much slower, so it completes
        // after tool_b. Persistence must still be [c1, c2].
        Box::new(OrderTool::new_with_delay(
            "tool_a",
            log.clone(),
            ExecutionMode::Parallel,
            false,
            Duration::from_millis(40),
        )),
        Box::new(OrderTool::new_with_delay(
            "tool_b",
            log.clone(),
            ExecutionMode::Parallel,
            false,
            Duration::from_millis(2),
        )),
    ];

    let context = make_context(Box::new(provider), tools);
    let hooks = MinimalHooks;

    let messages = opi_agent::agent_loop(
        context,
        AgentLoopConfig::default(),
        &hooks,
        noop_sink(),
        CancellationToken::new(),
    )
    .await
    .unwrap();

    // Sanity: completion order genuinely diverged (tool_b finished first).
    let entries = log.lock().unwrap();
    let a_end = entries
        .iter()
        .position(|e| e == "tool_a:end")
        .expect("tool_a ends");
    let b_end = entries
        .iter()
        .position(|e| e == "tool_b:end")
        .expect("tool_b ends");
    assert!(
        b_end < a_end,
        "test setup: tool_b must complete before tool_a, got: {entries:?}"
    );

    // Yet persisted results follow assistant source order.
    let ids = persisted_tool_result_ids(&messages.context);
    assert_eq!(
        ids,
        vec!["c1".to_string(), "c2".to_string()],
        "persisted tool results must follow assistant source order, got: {ids:?}"
    );
}

// DoD: a parallel batch emits one ToolExecutionEnd per tool. The current
// runtime realizes this in source order (join_all preserves input order); the
// contract permits completion-order emission, so per-tool coverage and the
// current realization are what is asserted.
#[tokio::test]
async fn phase8_tool_scheduling_completion_events_one_per_tool() {
    let provider = RecordingProvider::new(vec![
        multi_tool_call_response(vec![
            ("c1", "tool_a", r#"{"arg":"a"}"#),
            ("c2", "tool_b", r#"{"arg":"b"}"#),
        ]),
        text_response("done"),
    ]);

    let log = Arc::new(Mutex::new(Vec::new()));
    let tools: Vec<Box<dyn Tool>> = vec![
        Box::new(OrderTool::new(
            "tool_a",
            log.clone(),
            ExecutionMode::Parallel,
            false,
        )),
        Box::new(OrderTool::new(
            "tool_b",
            log.clone(),
            ExecutionMode::Parallel,
            false,
        )),
    ];

    let end_ids = Arc::new(Mutex::new(Vec::new()));
    let sink = tool_end_sink(end_ids.clone());

    let context = make_context(Box::new(provider), tools);
    let hooks = MinimalHooks;

    opi_agent::agent_loop(
        context,
        AgentLoopConfig::default(),
        &hooks,
        sink,
        CancellationToken::new(),
    )
    .await
    .unwrap();

    let ids = end_ids.lock().unwrap();
    assert_eq!(ids.len(), 2, "one ToolExecutionEnd per tool in the batch");
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(
        sorted,
        vec!["c1".to_string(), "c2".to_string()],
        "end events cover both tool calls"
    );
    assert_eq!(
        *ids,
        vec!["c1".to_string(), "c2".to_string()],
        "current runtime emits end events in source order; contract permits completion order"
    );
}

// DoD: early termination applies only when every finalized tool result sets
// terminate. Both terminate -> single provider call, early stop, both results
// persisted.
#[tokio::test]
async fn phase8_tool_scheduling_all_terminate_stops_early() {
    let log = Arc::new(Mutex::new(Vec::new()));

    let provider = RecordingProvider::new(vec![multi_tool_call_response(vec![
        ("c1", "tool_a", r#"{"arg":"a"}"#),
        ("c2", "tool_b", r#"{"arg":"b"}"#),
    ])]);
    let call_count = provider.call_count.clone();

    let tools: Vec<Box<dyn Tool>> = vec![
        Box::new(OrderTool::new(
            "tool_a",
            log.clone(),
            ExecutionMode::Parallel,
            true,
        )),
        Box::new(OrderTool::new(
            "tool_b",
            log.clone(),
            ExecutionMode::Parallel,
            true,
        )),
    ];

    let context = make_context(Box::new(provider), tools);
    let hooks = MinimalHooks;

    let messages = opi_agent::agent_loop(
        context,
        AgentLoopConfig::default(),
        &hooks,
        noop_sink(),
        CancellationToken::new(),
    )
    .await
    .unwrap();

    assert_eq!(
        *call_count.lock().unwrap(),
        1,
        "all-terminate batch stops the run after a single provider call"
    );
    assert_eq!(
        persisted_tool_result_ids(&messages.context),
        vec!["c1".to_string(), "c2".to_string()],
        "both finalized results are persisted before the early stop"
    );
}

// DoD: a single non-terminating result prevents early termination and the run
// continues to the next turn.
#[tokio::test]
async fn phase8_tool_scheduling_partial_terminate_continues() {
    let log = Arc::new(Mutex::new(Vec::new()));

    let provider = RecordingProvider::new(vec![
        multi_tool_call_response(vec![
            ("c1", "tool_a", r#"{"arg":"a"}"#),
            ("c2", "tool_b", r#"{"arg":"b"}"#),
        ]),
        text_response("done"),
    ]);
    let call_count = provider.call_count.clone();

    let tools: Vec<Box<dyn Tool>> = vec![
        Box::new(OrderTool::new(
            "tool_a",
            log.clone(),
            ExecutionMode::Parallel,
            true,
        )),
        Box::new(OrderTool::new(
            "tool_b",
            log.clone(),
            ExecutionMode::Parallel,
            false,
        )),
    ];

    let context = make_context(Box::new(provider), tools);
    let hooks = MinimalHooks;

    opi_agent::agent_loop(
        context,
        AgentLoopConfig::default(),
        &hooks,
        noop_sink(),
        CancellationToken::new(),
    )
    .await
    .unwrap();

    assert_eq!(
        *call_count.lock().unwrap(),
        2,
        "partial-terminate batch continues to a second provider call"
    );
}

// ---------------------------------------------------------------------------
// Phase 8: cancellation and finalized-state contract (task 8.4)
//
// Pins the observable cancellation contract through opi_agent::agent_loop: a
// cancelled run emits a terminal AgentEnd event and returns
// Err(AgentError::Cancelled), the provider is not called when cancellation
// arrives before the first turn, and a run cancelled mid-stream discards the
// partial assistant message so the terminal payload carries only finalized
// messages.
// ---------------------------------------------------------------------------

/// A provider whose stream emits `Start` and a partial `TextDelta`, then never
/// completes. Used to cancel a run mid-stream and assert the partial content is
/// discarded: the `Done` event that would finalize the assistant message never
/// arrives, so nothing is pushed to the message buffer.
struct HangingStreamProvider {
    call_count: Arc<Mutex<usize>>,
}

impl Provider for HangingStreamProvider {
    fn id(&self) -> &str {
        "hanging"
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
        let mut partial = base_msg();
        partial.content.push(AssistantContent::Text {
            text: "partial".into(),
        });
        let events: Vec<Result<AssistantStreamEvent, ProviderError>> = vec![
            Ok(AssistantStreamEvent::Start {
                partial: base_msg(),
            }),
            Ok(AssistantStreamEvent::TextDelta {
                content_index: 0,
                delta: "partial".into(),
                partial,
            }),
        ];
        // Emit the partial events, then hang forever so the cancel is observed
        // by the loop's `select!` during streaming rather than on stream end.
        Box::pin(
            stream::iter(events)
                .chain(stream::pending::<Result<AssistantStreamEvent, ProviderError>>()),
        )
    }
}

/// Sink that records event kinds and captures the messages carried by the one
/// terminal `AgentEnd` event.
fn agent_end_sink(
    kinds: Arc<Mutex<Vec<String>>>,
    end_messages: Arc<Mutex<Option<Vec<AgentMessage>>>>,
) -> Box<dyn Fn(AgentEvent) + Send + Sync> {
    Box::new(move |e| {
        kinds.lock().unwrap().push(event_kind(&e).to_string());
        if let AgentEvent::AgentEnd { messages } = e {
            *end_messages.lock().unwrap() = Some(messages);
        }
    })
}

// DoD: a run cancelled before its first turn never calls the provider, emits a
// terminal AgentEnd, and returns Err(AgentError::Cancelled).
#[tokio::test]
async fn phase8_cancellation_contract_before_turn_emits_agent_end_and_returns_cancelled() {
    let provider = HangingStreamProvider {
        call_count: Arc::new(Mutex::new(0)),
    };
    let call_count = provider.call_count.clone();

    let cancel = CancellationToken::new();
    cancel.cancel();

    let kinds = Arc::new(Mutex::new(Vec::new()));
    let end_messages = Arc::new(Mutex::new(None));
    let sink = agent_end_sink(kinds.clone(), end_messages.clone());

    let context = make_context(Box::new(provider), vec![]);
    let hooks = MinimalHooks;

    let result =
        opi_agent::agent_loop(context, AgentLoopConfig::default(), &hooks, sink, cancel).await;
    assert!(
        matches!(result, Err(AgentError::Cancelled)),
        "cancelled-before-turn run returns Err(Cancelled): {result:?}"
    );
    assert_eq!(
        *call_count.lock().unwrap(),
        0,
        "provider must not be called when cancellation arrives before the first turn"
    );

    let seq = kinds.lock().unwrap();
    assert_eq!(seq.first().map(String::as_str), Some("agent_start"));
    assert_eq!(
        seq.iter().filter(|s| s.as_str() == "agent_end").count(),
        1,
        "cancelled run emits exactly one terminal AgentEnd: {seq:?}"
    );
    assert!(
        !seq.contains(&"turn_start".to_string()),
        "no turn work runs when cancelled before turn 0: {seq:?}"
    );

    // Only the seed message is reflected in the terminal payload.
    let end = end_messages.lock().unwrap();
    let end = end.as_ref().expect("AgentEnd payload captured");
    assert_eq!(end.len(), 1, "only the seed message is finalized: {end:?}");
    assert!(matches!(end[0], AgentMessage::Llm(Message::User(_))));
}

// DoD: a run cancelled mid-stream emits a terminal AgentEnd, returns
// Err(AgentError::Cancelled), and discards the partial assistant message — the
// terminal payload carries only the seed user message, never the in-flight
// assistant content.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn phase8_cancellation_contract_during_stream_discards_partial_and_emits_agent_end() {
    let provider = HangingStreamProvider {
        call_count: Arc::new(Mutex::new(0)),
    };
    let call_count = provider.call_count.clone();

    let cancel = CancellationToken::new();
    let cancel_for_task = cancel.clone();

    let kinds = Arc::new(Mutex::new(Vec::new()));
    let end_messages = Arc::new(Mutex::new(None));
    let sink = agent_end_sink(kinds.clone(), end_messages.clone());

    let context = make_context(Box::new(provider), vec![]);
    let hooks = MinimalHooks;

    let handle = tokio::spawn(async move {
        opi_agent::agent_loop(
            context,
            AgentLoopConfig::default(),
            &hooks,
            sink,
            cancel_for_task,
        )
        .await
    });

    // Let the provider emit Start + the partial TextDelta and enter its hang.
    tokio::time::sleep(Duration::from_millis(120)).await;
    cancel.cancel();

    let result = handle.await.expect("agent_loop task panicked");
    assert!(
        matches!(result, Err(AgentError::Cancelled)),
        "mid-stream cancel returns Err(Cancelled): {result:?}"
    );
    assert_eq!(
        *call_count.lock().unwrap(),
        1,
        "provider was called once before the cancel was observed"
    );

    let seq = kinds.lock().unwrap();
    assert_eq!(seq.first().map(String::as_str), Some("agent_start"));
    assert_eq!(
        seq.iter().filter(|s| s.as_str() == "agent_end").count(),
        1,
        "cancelled run emits exactly one terminal AgentEnd: {seq:?}"
    );
    assert!(
        !seq.contains(&"message_end".to_string()),
        "partial assistant message must not be finalized (Done never arrived): {seq:?}"
    );
    drop(seq);

    let end = end_messages.lock().unwrap();
    let end = end.as_ref().expect("AgentEnd payload captured");
    assert!(
        end.iter()
            .all(|m| !matches!(m, AgentMessage::Llm(Message::Assistant(_)))),
        "terminal payload must not carry a finalized assistant message for the cancelled turn: {end:?}"
    );
    assert!(
        end.iter()
            .any(|m| matches!(m, AgentMessage::Llm(Message::User(_)))),
        "terminal payload still carries the seed user message: {end:?}"
    );
}
