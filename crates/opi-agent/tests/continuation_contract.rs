//! Completed-turn decisions and preparation at the next-dispatch boundary.

mod common;

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use opi_agent::event::AgentEvent;
use opi_agent::hooks::{
    AfterToolCallContext, AfterToolCallResult, AgentHooks, PrepareNextTurnContext,
    ShouldStopAfterTurnContext,
};
use opi_agent::loop_types::{InferenceConfig, ModelSelection, NextTurnState};
use opi_agent::{AgentError, AgentLoopConfig, AgentLoopContext, AgentMessage};
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::test_support::{
    MockProvider, single_route_collection, text_response, tool_call_response,
};
use tokio_util::sync::CancellationToken;

enum HookFailure {
    Transform,
    Convert,
}

struct FailHookOnce {
    at: HookFailure,
    pending: AtomicBool,
}

impl AgentHooks for FailHookOnce {
    fn transform_context(
        &self,
        messages: Vec<AgentMessage>,
        _signal: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<AgentMessage>, AgentError>> + Send>> {
        let fail =
            matches!(self.at, HookFailure::Transform) && self.pending.swap(false, Ordering::SeqCst);
        Box::pin(async move {
            if fail {
                Err(AgentError::Hook("transform failed before dispatch".into()))
            } else {
                Ok(messages)
            }
        })
    }

    fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
        if matches!(self.at, HookFailure::Convert) && self.pending.swap(false, Ordering::SeqCst) {
            Err(AgentError::Hook("conversion failed before dispatch".into()))
        } else {
            Hooks::default().convert_to_llm(messages)
        }
    }
}

struct RequireCredentialsOnce(AtomicBool);

impl opi_ai::auth::AuthResolver for RequireCredentialsOnce {
    fn resolve<'a>(
        &'a self,
    ) -> opi_ai::credential::BoxAuthFuture<
        'a,
        Result<opi_ai::auth::ResolvedAuth, opi_ai::provider::ProviderError>,
    > {
        let missing = self.0.swap(false, Ordering::SeqCst);
        Box::pin(async move {
            if missing {
                Err(opi_ai::provider::ProviderError::CredentialNeeded {
                    provider_id: "mock".into(),
                })
            } else {
                Ok(opi_ai::test_support::resolved_auth())
            }
        })
    }
}

fn public_agent(
    collection: opi_ai::ProviderCollection,
    hooks: Box<dyn AgentHooks>,
) -> opi_agent::Agent {
    opi_agent::Agent::new(
        Arc::new(collection),
        vec![],
        None,
        "mock:mock-model".into(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig::default(),
        hooks,
    )
    .unwrap()
}

#[tokio::test]
async fn public_retry_retains_initial_steering_after_hook_failure() {
    for at in [HookFailure::Transform, HookFailure::Convert] {
        let provider = MockProvider::new("mock", vec![text_response("recovered")]);
        let calls = provider.call_log_handle();
        let mut agent = public_agent(
            single_route_collection(Box::new(provider)),
            Box::new(FailHookOnce {
                at,
                pending: AtomicBool::new(true),
            }),
        );
        agent.steer("must keep".into());
        let failed = agent.prompt("start").await;
        assert!(matches!(failed.error(), Some(AgentError::Hook(_))));
        assert!(calls.lock().unwrap().is_empty());
        agent
            .retry_last_turn()
            .await
            .into_execution_result()
            .unwrap();
        assert_eq!(
            texts(&calls.lock().unwrap()[0].messages),
            ["start", "must keep"]
        );
    }
}

#[tokio::test]
async fn public_retry_retains_initial_steering_after_auth_preparation_failure() {
    let provider = MockProvider::new("mock", vec![text_response("recovered")]);
    let calls = provider.call_log_handle();
    let mut collection = opi_ai::ProviderCollection::new();
    collection
        .register_route(
            Box::new(provider),
            Arc::new(RequireCredentialsOnce(AtomicBool::new(true))),
            opi_ai::AuthProvenanceSource::Static,
            opi_ai::CompatMetadata::default(),
        )
        .unwrap();
    let mut agent = public_agent(collection, Box::new(Hooks::default()));
    agent.steer("must keep".into());
    let failed = agent.prompt("start").await;
    assert!(
        matches!(
            failed.error(),
            Some(AgentError::Provider(failure))
                if matches!(failure.provider_error(), opi_ai::provider::ProviderError::CredentialNeeded { .. })
        ),
        "{:?}",
        failed.error()
    );
    assert!(calls.lock().unwrap().is_empty());
    agent
        .retry_last_turn()
        .await
        .into_execution_result()
        .unwrap();
    assert_eq!(
        texts(&calls.lock().unwrap()[0].messages),
        ["start", "must keep"]
    );
}

#[tokio::test]
async fn public_retry_retains_initial_steering_after_turn_start_cancellation() {
    let provider = MockProvider::new("mock", vec![text_response("recovered")]);
    let calls = provider.call_log_handle();
    let mut agent = public_agent(
        single_route_collection(Box::new(provider)),
        Box::new(Hooks::default()),
    );
    let control = agent.control_handle();
    let cancel_once = AtomicBool::new(true);
    agent.subscribe(Box::new(move |event| {
        if matches!(event, AgentEvent::TurnStart) && cancel_once.swap(false, Ordering::SeqCst) {
            control.abort();
        }
    }));
    agent.steer("must keep".into());
    let failed = agent.prompt("start").await;
    assert!(matches!(failed.error(), Some(AgentError::Cancelled)));
    assert!(calls.lock().unwrap().is_empty());
    agent
        .retry_last_turn()
        .await
        .into_execution_result()
        .unwrap();
    assert_eq!(
        texts(&calls.lock().unwrap()[0].messages),
        ["start", "must keep"]
    );
}

#[tokio::test]
async fn public_retry_does_not_requeue_steering_after_a_dispatched_provider_failure() {
    use opi_ai::test_support::MockResponse;
    let provider = MockProvider::new_with_errors(
        "mock",
        vec![
            MockResponse::Error(opi_ai::provider::ProviderError::Timeout),
            MockResponse::Events(text_response("recovered")),
        ],
    );
    let calls = provider.call_log_handle();
    let mut agent = public_agent(
        single_route_collection(Box::new(provider)),
        Box::new(Hooks::default()),
    );
    agent.steer("already sent".into());
    let failed = agent.prompt("start").await;
    assert!(matches!(failed.error(), Some(AgentError::Provider(_))));
    assert_eq!(calls.lock().unwrap().len(), 1);
    assert_eq!(
        texts(&calls.lock().unwrap()[0].messages),
        ["start", "already sent"]
    );
    agent
        .retry_last_turn()
        .await
        .into_execution_result()
        .unwrap();
    assert_eq!(texts(&calls.lock().unwrap()[1].messages), ["start"]);
}

#[derive(Default)]
struct Hooks {
    stop: bool,
    prepare_calls: AtomicUsize,
    stop_contexts: Mutex<Vec<NextTurnState>>,
    prepare_contexts: Mutex<Vec<PrepareNextTurnContext>>,
    steering_during_prepare: Option<Arc<Mutex<VecDeque<String>>>>,
    queue_observed_empty: Mutex<Vec<bool>>,
    inspected_queue: Option<Arc<Mutex<VecDeque<String>>>>,
    terminate_tools: bool,
}

impl AgentHooks for Hooks {
    fn after_tool_call(
        &self,
        mut ctx: AfterToolCallContext,
    ) -> Pin<Box<dyn Future<Output = AfterToolCallResult> + Send>> {
        ctx.result.terminate = self.terminate_tools;
        Box::pin(async move { AfterToolCallResult::Replace(ctx.result) })
    }
    fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
        Ok(messages
            .iter()
            .filter_map(|message| match message {
                AgentMessage::Llm(message) => Some(message.clone()),
                _ => None,
            })
            .collect())
    }

    fn should_stop_after_turn(
        &self,
        ctx: ShouldStopAfterTurnContext,
    ) -> Pin<Box<dyn Future<Output = bool> + Send>> {
        self.stop_contexts.lock().unwrap().push(ctx.state);
        let stop = self.stop;
        Box::pin(async move { stop })
    }

    fn prepare_next_turn(
        &self,
        ctx: PrepareNextTurnContext,
    ) -> Pin<Box<dyn Future<Output = Result<Option<NextTurnState>, AgentError>> + Send>> {
        self.prepare_calls.fetch_add(1, Ordering::SeqCst);
        self.prepare_contexts.lock().unwrap().push(ctx.clone());
        if let Some(queue) = &self.inspected_queue {
            self.queue_observed_empty
                .lock()
                .unwrap()
                .push(queue.lock().unwrap().is_empty());
        }
        if let Some(queue) = &self.steering_during_prepare {
            queue.lock().unwrap().push_back("during preparation".into());
        }
        Box::pin(async move {
            let mut state = ctx.state;
            state.context.push(user_message("prepared"));
            Ok(Some(state))
        })
    }
}

#[tokio::test]
async fn terminating_tool_batch_still_checks_stop_and_delivers_follow_up() {
    let provider = MockProvider::new(
        "mock",
        vec![
            tool_call_response("call", "work", "{}"),
            text_response("done"),
        ],
    );
    let calls = provider.call_log_handle();
    let mut ctx = context(provider);
    ctx.follow_up_queue = Some(Arc::new(Mutex::new(VecDeque::from(["follow-up".into()]))));
    let hooks = Hooks {
        terminate_tools: true,
        ..Hooks::default()
    };
    let result = opi_agent::agent_loop(
        ctx,
        AgentLoopConfig::default(),
        &hooks,
        Box::new(|_| {}),
        CancellationToken::new(),
    )
    .await;
    assert!(result.error().is_none());
    assert_eq!(calls.lock().unwrap().len(), 2);
    assert_eq!(hooks.stop_contexts.lock().unwrap().len(), 2);
    assert!(hooks.prepare_contexts.lock().unwrap()[0].terminate);
    assert_eq!(
        texts(&calls.lock().unwrap()[1].messages),
        ["start", "prepared", "follow-up"]
    );
}

fn user_message(text: &str) -> AgentMessage {
    AgentMessage::Llm(Message::User(UserMessage {
        content: vec![InputContent::Text { text: text.into() }],
        timestamp_ms: 0,
    }))
}

fn context(provider: MockProvider) -> AgentLoopContext {
    AgentLoopContext {
        collection: Arc::new(single_route_collection(Box::new(provider))),
        registry: common::test_registry(vec![Box::new(common::RecordingTool::new(
            "work",
            Arc::new(AtomicUsize::new(0)),
        ))]),
        authorizer: Some(common::permissive_authorizer()),
        evidence_health: opi_agent::evidence::EvidenceHealth::healthy(),
        evidence_sink: None,
        state: NextTurnState::new(
            vec![user_message("start")],
            ModelSelection::new("mock", "mock-model"),
            InferenceConfig::default(),
        ),
        system: None,
        steering_queue: None,
        follow_up_queue: None,
        diagnostic_sink: None,
        session_id: None,
    }
}

fn texts(messages: &[Message]) -> Vec<&str> {
    messages
        .iter()
        .filter_map(|message| match message {
            Message::User(message) => match message.content.first() {
                Some(InputContent::Text { text }) => Some(text.as_str()),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn initial_steering_is_delivered_before_the_first_provider_request() {
    let provider = MockProvider::new("mock", vec![text_response("done"), text_response("extra")]);
    let calls = provider.call_log_handle();
    let mut ctx = context(provider);
    ctx.steering_queue = Some(Arc::new(Mutex::new(VecDeque::from([
        "initial steering".into()
    ]))));
    let hooks = Hooks {
        stop: true,
        ..Hooks::default()
    };
    let result = opi_agent::agent_loop(
        ctx,
        AgentLoopConfig::default(),
        &hooks,
        Box::new(|_| {}),
        CancellationToken::new(),
    )
    .await;
    assert!(result.error().is_none());
    assert_eq!(
        texts(&calls.lock().unwrap()[0].messages),
        ["start", "initial steering"]
    );
    assert_eq!(calls.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn cancelled_entry_preserves_initial_steering_without_dispatch() {
    let provider = MockProvider::new("mock", vec![]);
    let calls = provider.call_log_handle();
    let mut ctx = context(provider);
    let steering = Arc::new(Mutex::new(VecDeque::from(["initial steering".into()])));
    ctx.steering_queue = Some(steering.clone());
    let cancel = CancellationToken::new();
    cancel.cancel();
    let result = opi_agent::agent_loop(
        ctx,
        AgentLoopConfig::default(),
        &Hooks::default(),
        Box::new(|_| {}),
        cancel,
    )
    .await;
    assert!(matches!(result.error(), Some(AgentError::Cancelled)));
    assert!(calls.lock().unwrap().is_empty());
    assert_eq!(steering.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn completed_turn_count_resets_for_each_loop_invocation() {
    let hooks = Hooks::default();
    for _ in 0..2 {
        let provider = MockProvider::new(
            "mock",
            vec![
                tool_call_response("call", "work", "{}"),
                text_response("done"),
            ],
        );
        let result = opi_agent::agent_loop(
            context(provider),
            AgentLoopConfig::default(),
            &hooks,
            Box::new(|_| {}),
            CancellationToken::new(),
        )
        .await;
        assert!(result.error().is_none());
    }
    let turns: Vec<u64> = hooks
        .prepare_contexts
        .lock()
        .unwrap()
        .iter()
        .map(|ctx| ctx.turn)
        .collect();
    assert_eq!(turns, vec![1, 1]);
}

#[test]
fn absent_quota_serializes_as_not_applicable() {
    use opi_agent::evidence::{Measurement, UnknownReason};
    assert_eq!(
        serde_json::to_value(Measurement::Unknown {
            reason: UnknownReason::NotApplicable
        })
        .unwrap(),
        serde_json::json!({"Unknown": {"reason": "not_applicable"}})
    );
}

#[tokio::test]
async fn natural_completion_does_not_prepare_or_create_another_turn() {
    let provider = MockProvider::new("mock", vec![text_response("done")]);
    let calls = provider.call_log_handle();
    let hooks = Hooks::default();
    let result = opi_agent::agent_loop(
        context(provider),
        AgentLoopConfig::default(),
        &hooks,
        Box::new(|_| {}),
        CancellationToken::new(),
    )
    .await;
    assert!(result.error().is_none(), "{result:?}");
    assert_eq!(calls.lock().unwrap().len(), 1);
    assert_eq!(hooks.prepare_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn terminal_stop_observes_completed_state_and_skips_preparation_and_queues() {
    let provider = MockProvider::new("mock", vec![text_response("done")]);
    let mut ctx = context(provider);
    let follow_up = Arc::new(Mutex::new(VecDeque::from(["queued".into()])));
    ctx.follow_up_queue = Some(follow_up.clone());
    let hooks = Hooks {
        stop: true,
        ..Hooks::default()
    };
    let result = opi_agent::agent_loop(
        ctx,
        AgentLoopConfig::default(),
        &hooks,
        Box::new(|_| {}),
        CancellationToken::new(),
    )
    .await;
    assert!(result.error().is_none(), "{result:?}");
    assert_eq!(hooks.prepare_calls.load(Ordering::SeqCst), 0);
    assert_eq!(hooks.stop_contexts.lock().unwrap()[0].context.len(), 2);
    assert_eq!(follow_up.lock().unwrap().len(), 1);
    assert_eq!(result.state().context.len(), 2);
}

#[tokio::test]
async fn tool_continuation_has_no_intrinsic_fifty_turn_limit() {
    let mut responses = (0..51)
        .map(|turn| tool_call_response(&format!("call-{turn}"), "work", "{}"))
        .collect::<Vec<_>>();
    responses.push(text_response("done"));
    let provider = MockProvider::new("mock", responses);
    let calls = provider.call_log_handle();
    let hooks = Hooks::default();
    let result = opi_agent::agent_loop(
        context(provider),
        AgentLoopConfig::default(),
        &hooks,
        Box::new(|_| {}),
        CancellationToken::new(),
    )
    .await;
    assert!(result.error().is_none(), "{result:?}");
    assert_eq!(calls.lock().unwrap().len(), 52);
    assert_eq!(hooks.prepare_calls.load(Ordering::SeqCst), 51);
    assert_eq!(
        hooks.prepare_contexts.lock().unwrap().last().unwrap().turn,
        51
    );
}

#[tokio::test]
async fn follow_up_is_selected_before_preparation_and_applied_to_replacement() {
    let provider = MockProvider::new("mock", vec![text_response("first"), text_response("done")]);
    let calls = provider.call_log_handle();
    let mut ctx = context(provider);
    let follow_up = Arc::new(Mutex::new(VecDeque::from(["follow-up".into()])));
    ctx.follow_up_queue = Some(follow_up.clone());
    let hooks = Hooks {
        inspected_queue: Some(follow_up),
        ..Hooks::default()
    };
    let result = opi_agent::agent_loop(
        ctx,
        AgentLoopConfig::default(),
        &hooks,
        Box::new(|_| {}),
        CancellationToken::new(),
    )
    .await;
    assert!(result.error().is_none(), "{result:?}");
    assert_eq!(*hooks.queue_observed_empty.lock().unwrap(), vec![true]);
    assert_eq!(
        texts(&calls.lock().unwrap()[1].messages),
        ["start", "prepared", "follow-up"]
    );
}

#[tokio::test]
async fn preparation_picks_up_steering_when_no_message_was_pending() {
    let provider = MockProvider::new(
        "mock",
        vec![
            tool_call_response("call", "work", "{}"),
            text_response("done"),
        ],
    );
    let calls = provider.call_log_handle();
    let mut ctx = context(provider);
    let steering = Arc::new(Mutex::new(VecDeque::new()));
    ctx.steering_queue = Some(steering.clone());
    let hooks = Hooks {
        steering_during_prepare: Some(steering),
        ..Hooks::default()
    };
    let result = opi_agent::agent_loop(
        ctx,
        AgentLoopConfig::default(),
        &hooks,
        Box::new(|_| {}),
        CancellationToken::new(),
    )
    .await;
    assert!(result.error().is_none(), "{result:?}");
    assert_eq!(
        texts(&calls.lock().unwrap()[1].messages),
        ["start", "prepared", "during preparation"]
    );
    assert_eq!(hooks.prepare_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn preparation_does_not_dequeue_more_steering_when_one_batch_is_pending() {
    for follow_up_pending in [false, true] {
        let provider = MockProvider::new(
            "mock",
            vec![
                text_response("first"),
                text_response("second"),
                text_response("done"),
            ],
        );
        let calls = provider.call_log_handle();
        let mut ctx = context(provider);
        let steering = Arc::new(Mutex::new(VecDeque::new()));
        ctx.steering_queue = Some(steering.clone());
        let queued = if follow_up_pending {
            let follow_up = Arc::new(Mutex::new(VecDeque::new()));
            ctx.follow_up_queue = Some(follow_up.clone());
            follow_up
        } else {
            steering.clone()
        };
        let hooks = Hooks {
            steering_during_prepare: Some(steering),
            ..Hooks::default()
        };
        let cancel = CancellationToken::new();
        let stop_after_second = cancel.clone();
        let ends = Arc::new(AtomicUsize::new(0));
        let result = opi_agent::agent_loop(
            ctx,
            AgentLoopConfig::default(),
            &hooks,
            Box::new(move |event| {
                if matches!(event, AgentEvent::TurnEnd { .. }) {
                    if ends.fetch_add(1, Ordering::SeqCst) == 0 {
                        queued.lock().unwrap().push_back("earlier input".into());
                    } else {
                        stop_after_second.cancel();
                    }
                }
            }),
            cancel,
        )
        .await;
        assert!(matches!(result.error(), Some(AgentError::Cancelled)));
        assert_eq!(
            texts(&calls.lock().unwrap()[1].messages),
            ["start", "prepared", "earlier input"]
        );
        assert_eq!(
            hooks
                .steering_during_prepare
                .as_ref()
                .unwrap()
                .lock()
                .unwrap()
                .len(),
            1
        );
    }
}
