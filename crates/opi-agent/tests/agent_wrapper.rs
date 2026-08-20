//! Behavioral tests for the Agent wrapper (task 1.7).
//!
//! DoD: "prompt, continue, abort, subscribe tested"

mod common;

#[test]
fn complete_state_is_the_only_public_next_turn_mutation_surface() {
    let agent_source = include_str!("../src/agent.rs");
    for removed in [
        "pub fn set_model(",
        "pub fn set_max_tokens(",
        "pub fn set_thinking_config(",
        "pub fn set_initial_messages(",
        "pub fn inject_message(",
        "pub fn replace_messages(",
        "pub fn rewind_to(",
    ] {
        assert!(
            !agent_source.contains(removed),
            "piecemeal state mutator remains public: {removed}"
        );
    }
    let lib_source = include_str!("../src/lib.rs");
    assert!(!lib_source.contains("pub mod state;"));
    assert!(!lib_source.contains("pub use state::AgentState;"));
}

use std::pin::Pin;
use std::sync::{Arc, Mutex};

use futures_util::StreamExt;
use futures_util::stream;
use opi_agent::agent::Agent;
use opi_agent::event::AgentEvent;
use opi_agent::hooks::{
    AgentHooks, BeforeToolCallContext, BeforeToolCallResult, ShouldStopAfterTurnContext,
};
use opi_agent::loop_types::{
    AgentError, AgentLoopConfig, InferenceConfig, InvalidNextTurnReason, ModelSelection,
    NextTurnState,
};
use opi_agent::message::AgentMessage;
use opi_ai::message::{AssistantContent, AssistantMessage, InputContent, Message, UserMessage};
use opi_ai::provider::{EventStream, Provider, ProviderError, Request, ThinkingConfig};
use opi_ai::stream::{AssistantStreamEvent, StopReason, Usage};
use opi_ai::test_support::single_route_collection;
use opi_ai::{ModelCapabilities, ModelInfo, ThinkingLevel, WireApi};

// ---------------------------------------------------------------------------
// Mock provider (reused from agent_loop_mock)
// ---------------------------------------------------------------------------

struct MockProvider {
    id: String,
    responses: Arc<Mutex<Vec<Vec<AssistantStreamEvent>>>>,
}

impl MockProvider {
    fn new(id: &str, responses: Vec<Vec<AssistantStreamEvent>>) -> Self {
        Self {
            id: id.to_owned(),
            responses: Arc::new(Mutex::new(responses)),
        }
    }
}

impl Provider for MockProvider {
    fn id(&self) -> &str {
        &self.id
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

struct InvalidMetadataProvider {
    models: Vec<ModelInfo>,
}

impl InvalidMetadataProvider {
    fn new() -> Self {
        Self {
            models: vec![
                ModelInfo::new(
                    "mock-model",
                    "Mock Model",
                    WireApi::OpenAiCompletions,
                    ModelCapabilities::new(100_000, 4_096),
                ),
                ModelInfo::new(
                    "invalid-model",
                    "Invalid Model",
                    WireApi::OpenAiCompletions,
                    ModelCapabilities::new(0, 4_096),
                ),
            ],
        }
    }
}

impl Provider for InvalidMetadataProvider {
    fn id(&self) -> &str {
        "invalid-metadata"
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    fn stream_prepared(&self, _request: Request, _auth: opi_ai::auth::ResolvedAuth) -> EventStream {
        Box::pin(stream::empty())
    }
}

// ---------------------------------------------------------------------------
// Default hooks for testing
// ---------------------------------------------------------------------------

struct TestHooks;

impl AgentHooks for TestHooks {
    fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn base_assistant() -> AssistantMessage {
    AssistantMessage {
        content: vec![],
        api: opi_ai::ApiKind::Anthropic,
        provider: "mock".into(),
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

fn assert_same_state(actual: &NextTurnState, expected: &NextTurnState) {
    assert_eq!(
        serde_json::to_value(&actual.context).unwrap(),
        serde_json::to_value(&expected.context).unwrap()
    );
    assert_eq!(actual.model_selection, expected.model_selection);
    assert_eq!(actual.inference.max_tokens, expected.inference.max_tokens);
    assert_eq!(actual.inference.temperature, expected.inference.temperature);
    assert_eq!(
        actual.inference.thinking.enabled,
        expected.inference.thinking.enabled
    );
    assert_eq!(
        actual.inference.thinking.budget_tokens,
        expected.inference.thinking.budget_tokens
    );
    assert_eq!(
        actual.inference.thinking.level,
        expected.inference.thinking.level
    );
}

fn contains_user_text(request: &Request, expected: &str) -> bool {
    request.messages.iter().any(|message| match message {
        Message::User(user) => user
            .content
            .iter()
            .any(|content| matches!(content, InputContent::Text { text } if text == expected)),
        _ => false,
    })
}

fn recording_agent(responses: Vec<Vec<AssistantStreamEvent>>) -> (Agent, Arc<Mutex<Vec<Request>>>) {
    let provider = opi_ai::test_support::MockProvider::new("mock", responses);
    let calls = provider.call_log_handle();
    let agent = Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        common::registrations_from(vec![]),
        Some(common::permissive_authorizer()),
        "mock:mock-model".into(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig::default(),
        Box::new(TestHooks),
    )
    .expect("agent");
    (agent, calls)
}

#[tokio::test]
async fn armed_run_rejects_foreign_owner_before_mutation_or_dispatch() {
    let (mut agent_a, calls_a) = recording_agent(vec![text_response("agent-a-valid")]);
    let (mut agent_b, calls_b) = recording_agent(vec![text_response("agent-b-valid")]);
    let run_a = agent_a.arm_run();
    let run_b = agent_b.arm_run();
    let state_a = agent_a.state_snapshot();
    let state_b = agent_b.state_snapshot();

    assert!(matches!(
        agent_a.control_handle_for_run(&run_b),
        Err(AgentError::InvalidArmedRun)
    ));
    assert!(matches!(
        agent_b.control_handle_for_run(&run_a),
        Err(AgentError::InvalidArmedRun)
    ));
    assert!(agent_a.control_handle_for_run(&run_a).is_ok());
    assert!(agent_b.control_handle_for_run(&run_b).is_ok());

    run_b.cancel_token().cancel();
    let foreign_a = agent_a.prompt_armed("must not enter agent A", run_b).await;
    assert!(matches!(
        foreign_a.error(),
        Some(AgentError::InvalidArmedRun)
    ));
    assert_same_state(&agent_a.state_snapshot(), &state_a);
    assert!(calls_a.lock().unwrap().is_empty());

    let foreign_b = agent_b.prompt_armed("must not enter agent B", run_a).await;
    assert!(matches!(
        foreign_b.error(),
        Some(AgentError::InvalidArmedRun)
    ));
    assert_same_state(&agent_b.state_snapshot(), &state_b);
    assert!(calls_b.lock().unwrap().is_empty());

    agent_a
        .prompt("agent A direct prompt")
        .await
        .into_execution_result()
        .unwrap();
    agent_b
        .prompt("agent B direct prompt")
        .await
        .into_execution_result()
        .unwrap();
    assert_eq!(calls_a.lock().unwrap().len(), 1);
    assert_eq!(calls_b.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn armed_run_rejects_stale_generation_before_mutation_or_dispatch() {
    let (mut agent, calls) = recording_agent(vec![text_response("latest-valid")]);
    let stale = agent.arm_run();
    let latest = agent.arm_run();
    let state = agent.state_snapshot();

    assert!(matches!(
        agent.control_handle_for_run(&stale),
        Err(AgentError::InvalidArmedRun)
    ));
    assert!(agent.control_handle_for_run(&latest).is_ok());

    let stale_result = agent.prompt_armed("must not enter stale run", stale).await;
    assert!(matches!(
        stale_result.error(),
        Some(AgentError::InvalidArmedRun)
    ));
    assert_same_state(&agent.state_snapshot(), &state);
    assert!(calls.lock().unwrap().is_empty());

    agent
        .prompt_armed("latest run", latest)
        .await
        .into_execution_result()
        .unwrap();
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert!(contains_user_text(&calls[0], "latest run"));
    assert!(!contains_user_text(&calls[0], "must not enter stale run"));
}

#[test]
fn constructor_rejects_unsupported_initial_thinking() {
    let result = Agent::new(
        Arc::new(single_route_collection(Box::new(MockProvider::new(
            "mock",
            vec![],
        )))),
        common::registrations_from(vec![]),
        Some(common::permissive_authorizer()),
        "mock:mock-model".into(),
        None,
        InferenceConfig {
            thinking: ThinkingConfig {
                enabled: true,
                budget_tokens: Some(2048),
                level: ThinkingLevel::High,
            },
            ..Default::default()
        },
        AgentLoopConfig::default(),
        Box::new(TestHooks),
    );

    assert!(matches!(
        result,
        Err(AgentError::InvalidNextTurnCandidate(
            InvalidNextTurnReason::UnsupportedThinking { .. }
        ))
    ));
}

#[test]
fn constructor_rejects_enabled_none_thinking_on_non_thinking_model() {
    let result = Agent::new(
        Arc::new(single_route_collection(Box::new(MockProvider::new(
            "mock",
            vec![],
        )))),
        common::registrations_from(vec![]),
        Some(common::permissive_authorizer()),
        "mock:mock-model".into(),
        None,
        InferenceConfig {
            thinking: ThinkingConfig {
                enabled: true,
                budget_tokens: Some(2048),
                level: ThinkingLevel::None,
            },
            ..Default::default()
        },
        AgentLoopConfig::default(),
        Box::new(TestHooks),
    );

    assert!(matches!(
        result,
        Err(AgentError::InvalidNextTurnCandidate(
            InvalidNextTurnReason::UnsupportedThinking { .. }
        ))
    ));
}

#[test]
fn constructor_rejects_noncanonical_whitespace_model_identity() {
    let result = Agent::new(
        Arc::new(single_route_collection(Box::new(MockProvider::new(
            "mock",
            vec![],
        )))),
        common::registrations_from(vec![]),
        Some(common::permissive_authorizer()),
        " mock:mock-model".into(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig::default(),
        Box::new(TestHooks),
    );

    assert!(matches!(
        result,
        Err(AgentError::InvalidNextTurnCandidate(
            InvalidNextTurnReason::NonCanonicalModelSelection { .. }
        ))
    ));
}

#[test]
fn constructor_rejects_invalid_resolved_model_constraints() {
    let result = Agent::new(
        Arc::new(single_route_collection(Box::new(
            InvalidMetadataProvider::new(),
        ))),
        common::registrations_from(vec![]),
        Some(common::permissive_authorizer()),
        "invalid-metadata:invalid-model".into(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig::default(),
        Box::new(TestHooks),
    );

    assert!(matches!(
        result,
        Err(AgentError::InvalidNextTurnCandidate(
            InvalidNextTurnReason::InvalidModelConstraints { .. }
        ))
    ));
}

#[test]
fn constructor_rejects_every_non_finite_initial_temperature() {
    for temperature in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let result = Agent::new(
            Arc::new(single_route_collection(Box::new(MockProvider::new(
                "mock",
                vec![],
            )))),
            common::registrations_from(vec![]),
            Some(common::permissive_authorizer()),
            "mock:mock-model".into(),
            None,
            InferenceConfig {
                temperature: Some(temperature),
                ..Default::default()
            },
            AgentLoopConfig::default(),
            Box::new(TestHooks),
        );

        assert!(matches!(
            result,
            Err(AgentError::InvalidNextTurnCandidate(
                InvalidNextTurnReason::NonFiniteTemperature
            ))
        ));
    }
}

// ---------------------------------------------------------------------------
// Test 1: prompt sends user message and returns result
// ---------------------------------------------------------------------------

#[tokio::test]
async fn prompt_sends_user_message_and_returns_result() {
    let provider = MockProvider::new("mock", vec![text_response("Hello!")]);

    let mut agent = Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        common::registrations_from(vec![]),
        Some(common::permissive_authorizer()),
        "mock:mock-model".into(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig::default(),
        Box::new(TestHooks),
    )
    .expect("agent");

    let result = agent
        .prompt("Hi there")
        .await
        .into_execution_result()
        .unwrap();

    // Should contain: user message + assistant message
    assert!(
        result.len() >= 2,
        "expected at least 2 messages, got {}",
        result.len()
    );

    // First message should be the user message
    if let AgentMessage::Llm(Message::User(msg)) = &result[0] {
        match &msg.content[0] {
            InputContent::Text { text } => assert_eq!(text, "Hi there"),
            _ => panic!("expected text content"),
        }
    } else {
        panic!("first message should be user message");
    }
}

// ---------------------------------------------------------------------------
// Test 2: prompt accumulates state across calls
// ---------------------------------------------------------------------------

#[tokio::test]
async fn prompt_accumulates_state_across_calls() {
    let provider = MockProvider::new(
        "mock",
        vec![text_response("First"), text_response("Second")],
    );

    let mut agent = Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        common::registrations_from(vec![]),
        Some(common::permissive_authorizer()),
        "mock:mock-model".into(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig::default(),
        Box::new(TestHooks),
    )
    .expect("agent");

    let r1 = agent.prompt("Hello").await.into_execution_result().unwrap();
    assert!(r1.len() >= 2);

    let r2 = agent.prompt("World").await.into_execution_result().unwrap();
    // Second call should include messages from first call
    // r2 includes: [user1, assistant1, user2, assistant2]
    assert!(
        r2.len() >= 4,
        "expected at least 4 messages after two prompts, got {}",
        r2.len()
    );
}

// ---------------------------------------------------------------------------
// Test 3: continue appends and runs loop
// ---------------------------------------------------------------------------

#[tokio::test]
async fn continue_appends_message_and_runs_loop() {
    let provider = MockProvider::new(
        "mock",
        vec![text_response("First"), text_response("Continued")],
    );

    let mut agent = Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        common::registrations_from(vec![]),
        Some(common::permissive_authorizer()),
        "mock:mock-model".into(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig::default(),
        Box::new(TestHooks),
    )
    .expect("agent");

    let r1 = agent.prompt("Hello").await.into_execution_result().unwrap();
    assert!(r1.len() >= 2);

    let r2 = agent
        .continue_("Tell me more")
        .await
        .into_execution_result()
        .unwrap();
    assert!(
        r2.len() >= 4,
        "expected at least 4 messages after prompt+continue, got {}",
        r2.len()
    );
}

// ---------------------------------------------------------------------------
// Test 4: abort cancels running loop
// ---------------------------------------------------------------------------

#[tokio::test]
async fn abort_cancels_running_loop() {
    // Provider that yields Start then blocks until cancelled
    struct BlockingProvider;

    impl Provider for BlockingProvider {
        fn id(&self) -> &str {
            "blocking"
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

        fn stream_prepared(
            &self,
            request: Request,
            _auth: opi_ai::auth::ResolvedAuth,
        ) -> EventStream {
            let cancel = request.cancel;
            // Yield Start event, then wait for cancellation to end the stream
            Box::pin(
                futures_util::stream::once(async move {
                    Ok(AssistantStreamEvent::Start {
                        partial: base_assistant(),
                    })
                })
                .chain(futures_util::stream::unfold((), move |()| {
                    let cancel = cancel.clone();
                    async move {
                        cancel.cancelled().await;
                        None // end the stream when cancelled
                    }
                })),
            )
        }
    }

    let mut agent = Agent::new(
        Arc::new(single_route_collection(Box::new(BlockingProvider))),
        common::registrations_from(vec![]),
        Some(common::permissive_authorizer()),
        "blocking:mock-model".into(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig::default(),
        Box::new(TestHooks),
    )
    .expect("agent");

    // Get cancel token before moving agent into spawned task
    let token = agent.cancel_token();

    let handle = tokio::spawn(async move { agent.prompt("Hello").await });

    // Let the prompt start and enter the stream loop
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Cancel via the external token
    token.cancel();

    let result = handle.await.unwrap();

    assert!(
        matches!(result.error(), Some(AgentError::Cancelled)),
        "expected Cancelled error, got {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Test 5: subscribe receives events
// ---------------------------------------------------------------------------

#[tokio::test]
async fn subscribe_receives_events() {
    let provider = MockProvider::new("mock", vec![text_response("Response")]);

    let collected: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let collected_clone = collected.clone();

    let mut agent = Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        common::registrations_from(vec![]),
        Some(common::permissive_authorizer()),
        "mock:mock-model".into(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig::default(),
        Box::new(TestHooks),
    )
    .expect("agent");

    agent.subscribe(Box::new(move |event| {
        let name = match event {
            AgentEvent::AgentStart => "AgentStart",
            AgentEvent::AgentEnd { .. } => "AgentEnd",
            AgentEvent::TurnStart => "TurnStart",
            AgentEvent::TurnEnd { .. } => "TurnEnd",
            AgentEvent::MessageStart { .. } => "MessageStart",
            AgentEvent::MessageUpdate { .. } => "MessageUpdate",
            AgentEvent::MessageEnd { .. } => "MessageEnd",
            AgentEvent::ToolExecutionStart { .. } => "ToolExecutionStart",
            AgentEvent::ToolExecutionUpdate { .. } => "ToolExecutionUpdate",
            AgentEvent::ToolExecutionEnd { .. } => "ToolExecutionEnd",
            _ => "Unknown",
        };
        collected_clone.lock().unwrap().push(name.to_owned());
    }));

    let result = agent.prompt("Hello").await.into_execution_result().unwrap();
    assert!(result.len() >= 2);

    let events = collected.lock().unwrap();
    assert!(
        events.contains(&"AgentStart".to_owned()),
        "subscriber should receive AgentStart, got {:?}",
        *events
    );
    assert!(
        events.contains(&"AgentEnd".to_owned()),
        "subscriber should receive AgentEnd, got {:?}",
        *events
    );
}

// ---------------------------------------------------------------------------
// Phase 17.2 — P17-A04: Agent persists the complete NextTurnState across calls.
// ---------------------------------------------------------------------------

struct PrepareInferenceHooks;
impl AgentHooks for PrepareInferenceHooks {
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
        ctx: opi_agent::hooks::PrepareNextTurnContext,
    ) -> Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<Option<opi_agent::loop_types::NextTurnState>, AgentError>,
                > + Send,
        >,
    > {
        Box::pin(async move {
            if ctx.turn == 1 {
                // Apply an inference change through the complete-state transition.
                let mut next = ctx.state.clone();
                next.inference.max_tokens = Some(9999);
                Ok(Some(next))
            } else {
                Ok(None)
            }
        })
    }
    fn should_stop_after_turn(
        &self,
        _ctx: ShouldStopAfterTurnContext,
    ) -> Pin<Box<dyn std::future::Future<Output = bool> + Send>> {
        Box::pin(async { true })
    }
    fn before_tool_call(
        &self,
        _ctx: BeforeToolCallContext,
    ) -> Pin<Box<dyn std::future::Future<Output = BeforeToolCallResult> + Send>> {
        Box::pin(async { BeforeToolCallResult::Continue })
    }
}

#[tokio::test]
async fn phase17_agent_persists_complete_next_turn_state() {
    // P17-A04: the complete NextTurnState (context + model + inference) durably
    // persists across consecutive Agent::prompt operations. A recording mock
    // captures each provider Request; a prepare_next_turn hook applies an
    // inference change (max_tokens = 9999) during the first prompt, and the
    // second prompt's provider Request must carry that applied value — proving
    // the complete state survived the public Agent operation boundary.
    let provider = opi_ai::test_support::MockProvider::new(
        "mock",
        vec![text_response("first"), text_response("second")],
    );
    let call_log = provider.call_log_handle();
    let mut agent = Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        common::registrations_from(vec![]),
        Some(common::permissive_authorizer()),
        "mock:mock-model".into(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig::default(),
        Box::new(PrepareInferenceHooks),
    )
    .expect("agent");

    agent
        .prompt("turn one")
        .await
        .into_execution_result()
        .unwrap();
    agent
        .prompt("turn two")
        .await
        .into_execution_result()
        .unwrap();

    let log = call_log.lock().unwrap();
    assert_eq!(log.len(), 2, "two provider calls across two prompts");
    // First call (turn one) ran before prepare applied the inference change.
    assert_eq!(log[0].max_tokens, None);
    // Second call (turn two) carries max_tokens == 9999 — the prepare-applied
    // inference persisted across the Agent::prompt boundary.
    assert_eq!(
        log[1].max_tokens,
        Some(9999),
        "prepare-applied inference (max_tokens) must persist across Agent::prompt"
    );
    // Context persistence: the second call observed the first turn's user msg.
    assert!(
        log[1].messages.iter().any(|m| matches!(
            m,
            Message::User(u) if u.content.iter().any(|c| matches!(
                c, InputContent::Text { text } if text == "turn one"
            ))
        )),
        "first turn's user message must persist into the second call"
    );
    // Model preserved across calls.
    assert_eq!(agent.model(), "mock-model");
}

#[tokio::test]
async fn idle_replacement_rejects_enabled_none_thinking_atomically_without_consuming_queues() {
    let provider = opi_ai::test_support::MockProvider::new(
        "mock",
        vec![
            text_response("initial"),
            text_response("steering"),
            text_response("follow-up"),
        ],
    );
    let call_log = provider.call_log_handle();
    let mut agent = Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        common::registrations_from(vec![]),
        Some(common::permissive_authorizer()),
        "mock:mock-model".into(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig::default(),
        Box::new(TestHooks),
    )
    .expect("agent");

    let mut baseline = agent.state_snapshot();
    baseline
        .context
        .push(AgentMessage::Llm(Message::Assistant(base_assistant())));
    baseline.inference.max_tokens = Some(1234);
    baseline.inference.temperature = Some(0.25);
    agent.replace_state(baseline.clone()).unwrap();
    agent.steer("queued steering".into());
    agent.follow_up("queued follow-up".into());

    let mut candidate = baseline.clone();
    candidate
        .context
        .push(AgentMessage::Llm(Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "candidate-only context".into(),
            }],
            timestamp_ms: 0,
        })));
    candidate.inference.max_tokens = Some(9999);
    candidate.inference.temperature = Some(0.75);
    candidate.inference.thinking = ThinkingConfig {
        enabled: true,
        budget_tokens: Some(2048),
        level: ThinkingLevel::None,
    };

    let error = agent
        .replace_state(candidate)
        .expect_err("thinking on a non-thinking model must be rejected");
    assert!(matches!(
        error,
        AgentError::InvalidNextTurnCandidate(InvalidNextTurnReason::UnsupportedThinking { .. })
    ));
    assert_same_state(&agent.state_snapshot(), &baseline);

    agent
        .prompt("after rejection")
        .await
        .into_execution_result()
        .unwrap();
    let requests = call_log.lock().unwrap();
    assert_eq!(requests.len(), 3);
    assert!(
        contains_user_text(&requests[1], "queued steering"),
        "steering must not be consumed by invalid idle replacement"
    );
    assert!(
        contains_user_text(&requests[2], "queued follow-up"),
        "follow-up must not be consumed by invalid idle replacement"
    );
    assert_eq!(
        requests[0]
            .messages
            .first()
            .and_then(|message| match message {
                Message::Assistant(message) => Some(message.stop_reason),
                _ => None,
            }),
        Some(StopReason::Stop),
        "the prior context's stop reason must remain unchanged"
    );
}

#[test]
fn idle_replacement_rejects_noncanonical_whitespace_identity_without_partial_apply() {
    let provider = MockProvider::new("mock", vec![]);
    let mut agent = Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        common::registrations_from(vec![]),
        Some(common::permissive_authorizer()),
        "mock:mock-model".into(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig::default(),
        Box::new(TestHooks),
    )
    .expect("agent");
    let mut baseline = agent.state_snapshot();
    baseline
        .context
        .push(AgentMessage::Llm(Message::Assistant(base_assistant())));
    baseline.inference.max_tokens = Some(1234);
    baseline.inference.temperature = Some(0.25);
    agent.replace_state(baseline.clone()).unwrap();

    let mut candidate = baseline.clone();
    candidate.model_selection = ModelSelection::new(" mock ", "mock-model");
    candidate.inference.max_tokens = Some(9999);
    candidate
        .context
        .push(AgentMessage::Llm(Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "candidate-only context".into(),
            }],
            timestamp_ms: 0,
        })));

    let error = agent
        .replace_state(candidate)
        .expect_err("noncanonical route identity must be rejected");
    assert!(matches!(
        error,
        AgentError::InvalidNextTurnCandidate(
            InvalidNextTurnReason::NonCanonicalModelSelection { .. }
        )
    ));
    assert_same_state(&agent.state_snapshot(), &baseline);
}

#[test]
fn idle_replacement_rejects_invalid_model_constraints_without_partial_apply() {
    let mut agent = Agent::new(
        Arc::new(single_route_collection(Box::new(
            InvalidMetadataProvider::new(),
        ))),
        common::registrations_from(vec![]),
        Some(common::permissive_authorizer()),
        "invalid-metadata:mock-model".into(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig::default(),
        Box::new(TestHooks),
    )
    .expect("valid initial route");
    let mut baseline = agent.state_snapshot();
    baseline
        .context
        .push(AgentMessage::Llm(Message::Assistant(base_assistant())));
    baseline.inference.max_tokens = Some(1234);
    baseline.inference.temperature = Some(0.25);
    agent.replace_state(baseline.clone()).unwrap();

    let mut candidate = baseline.clone();
    candidate.model_selection = ModelSelection::new("invalid-metadata", "invalid-model");
    candidate.inference.max_tokens = Some(9999);
    candidate
        .context
        .push(AgentMessage::Llm(Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "candidate-only context".into(),
            }],
            timestamp_ms: 0,
        })));

    let error = agent
        .replace_state(candidate)
        .expect_err("invalid model constraints must be rejected");
    assert!(matches!(
        error,
        AgentError::InvalidNextTurnCandidate(InvalidNextTurnReason::InvalidModelConstraints { .. })
    ));
    assert_same_state(&agent.state_snapshot(), &baseline);
}

#[test]
fn idle_replacement_rejects_every_non_finite_temperature_without_partial_apply() {
    let provider = MockProvider::new("mock", vec![]);
    let mut agent = Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        common::registrations_from(vec![]),
        Some(common::permissive_authorizer()),
        "mock:mock-model".into(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig::default(),
        Box::new(TestHooks),
    )
    .expect("agent");
    let baseline = agent.state_snapshot();

    for temperature in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut candidate = baseline.clone();
        candidate.model_selection = ModelSelection::new("mock", "mock-model");
        candidate.inference.max_tokens = Some(7777);
        candidate.inference.temperature = Some(temperature);
        candidate
            .context
            .push(AgentMessage::Llm(Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "must not apply".into(),
                }],
                timestamp_ms: 0,
            })));

        let error = agent
            .replace_state(candidate)
            .expect_err("non-finite JSON scalar must be rejected");
        assert!(matches!(
            error,
            AgentError::InvalidNextTurnCandidate(InvalidNextTurnReason::NonFiniteTemperature)
        ));
        assert_same_state(&agent.state_snapshot(), &baseline);
    }
}

// ---------------------------------------------------------------------------
// Phase 17.2 — P17-NXT-006: the next provider call routes from the APPLIED
// next-turn state. A prepare_next_turn model-selection change reaches a
// different registered route on the next turn.
// ---------------------------------------------------------------------------

use std::sync::atomic::{AtomicU32, Ordering};

struct RerouteHooks {
    switch_to: opi_agent::loop_types::ModelSelection,
    switched: Arc<Mutex<bool>>,
    stop_calls: AtomicU32,
}

impl AgentHooks for RerouteHooks {
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
        ctx: opi_agent::hooks::PrepareNextTurnContext,
    ) -> Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<Option<opi_agent::loop_types::NextTurnState>, AgentError>,
                > + Send,
        >,
    > {
        let switch_to = self.switch_to.clone();
        let switched = self.switched.clone();
        Box::pin(async move {
            if ctx.turn == 1 {
                let mut next = ctx.state.clone();
                next.model_selection = switch_to;
                *switched.lock().unwrap() = true;
                Ok(Some(next))
            } else {
                Ok(None)
            }
        })
    }
    fn should_stop_after_turn(
        &self,
        _ctx: ShouldStopAfterTurnContext,
    ) -> Pin<Box<dyn std::future::Future<Output = bool> + Send>> {
        // Stop only after the rerouted (second) turn, so the run reaches provider B.
        let n = self.stop_calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move { n > 0 })
    }
    fn before_tool_call(
        &self,
        _ctx: BeforeToolCallContext,
    ) -> Pin<Box<dyn std::future::Future<Output = BeforeToolCallResult> + Send>> {
        Box::pin(async { BeforeToolCallResult::Continue })
    }
}

#[tokio::test]
async fn phase17_next_call_routes_from_applied_state_nxt006() {
    use opi_agent::loop_types::ModelSelection;
    use opi_ai::auth::AuthResolver;
    use opi_ai::test_support::{CountingAuthResolver, MockProvider};
    use opi_ai::{AuthProvenanceSource, CompatMetadata, ProviderCollection};

    let provider_a = MockProvider::new("a", vec![text_response("from-a")]);
    let provider_b = MockProvider::new("b", vec![text_response("from-b")]);
    let log_a = provider_a.call_log_handle();
    let log_b = provider_b.call_log_handle();

    let resolver: Arc<dyn AuthResolver> =
        Arc::new(CountingAuthResolver::new(Arc::new(AtomicU32::new(0))));
    let mut collection = ProviderCollection::new();
    collection
        .register_route(
            Box::new(provider_a),
            resolver.clone(),
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .unwrap();
    collection
        .register_route(
            Box::new(provider_b),
            resolver.clone(),
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .unwrap();

    let hooks = RerouteHooks {
        switch_to: ModelSelection::new("b", "mock-model"),
        switched: Arc::new(Mutex::new(false)),
        stop_calls: AtomicU32::new(0),
    };
    let switched = hooks.switched.clone();

    let mut agent = Agent::new(
        Arc::new(collection),
        common::registrations_from(vec![]),
        Some(common::permissive_authorizer()),
        "a:mock-model".into(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig {
            max_turns: 5,
            ..Default::default()
        },
        Box::new(hooks),
    )
    .expect("agent");

    agent
        .prompt("route me")
        .await
        .into_execution_result()
        .unwrap();

    assert!(
        *switched.lock().unwrap(),
        "prepare_next_turn must have switched the model selection"
    );
    // Provider A served the first turn; the applied (rerouted) state made the
    // next turn resolve provider B — proving next-call routing from applied state.
    assert_eq!(
        log_a.lock().unwrap().len(),
        1,
        "provider A called once on turn 1"
    );
    assert_eq!(
        log_b.lock().unwrap().len(),
        1,
        "provider B called on turn 2 via the applied (rerouted) state"
    );
}
