//! Real agent consumers distinguish abandonment from explicit cancellation.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use futures_util::Stream;
use opi_agent::hooks::{AgentHooks, InitialTurnContext, PreparationEvidence};
use opi_agent::loop_types::NextTurnState;
use opi_agent::{Agent, AgentError, AgentLoopConfig, AgentMessage, AgentRunResult};
use opi_ai::message::Message;
use opi_ai::provider::{EventStream, Provider, ProviderError, Request};
use opi_ai::stream::{AssistantStreamEvent, StopReason};
use opi_ai::test_support::{base_assistant, single_route_collection};
use tokio::sync::{Notify, mpsc, oneshot};
use tokio_util::sync::CancellationToken;

type Item = Result<AssistantStreamEvent, ProviderError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Turn,
    InitialSummary,
    NextSummary,
}

struct ControlledSource {
    events: mpsc::Receiver<Item>,
    released: Option<oneshot::Sender<bool>>,
    terminal_seen: bool,
}

impl Stream for ControlledSource {
    type Item = Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Item>> {
        let this = self.get_mut();
        let item = this.events.poll_recv(cx);
        if matches!(&item, Poll::Ready(Some(Ok(event))) if event.is_terminal()) {
            this.terminal_seen = true;
        }
        item
    }
}

impl Drop for ControlledSource {
    fn drop(&mut self) {
        let _ = self.released.take().unwrap().send(self.terminal_seen);
    }
}

struct ControlledProvider {
    source: Mutex<Option<EventStream>>,
    dispatched: Arc<Notify>,
    models: Vec<opi_ai::ModelInfo>,
    first_turn: AtomicBool,
}

impl Provider for ControlledProvider {
    fn id(&self) -> &str {
        "controlled"
    }

    fn models(&self) -> &[opi_ai::ModelInfo] {
        &self.models
    }

    fn stream_prepared(&self, _request: Request, _auth: opi_ai::auth::ResolvedAuth) -> EventStream {
        if self.first_turn.swap(false, Ordering::SeqCst) {
            return Box::pin(futures_util::stream::iter([Ok(
                AssistantStreamEvent::Done {
                    reason: StopReason::Stop,
                    message: base_assistant(),
                },
            )]));
        }
        // Deliberately ignores the token: explicit agent cancellation must
        // still release the source without requiring runtime shutdown.
        self.dispatched.notify_one();
        self.source.lock().unwrap().take().expect("one dispatch")
    }
}

struct Hooks {
    mode: Mode,
    collection: Arc<opi_ai::Models>,
}

impl AgentHooks for Hooks {
    fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
        Ok(messages
            .iter()
            .filter_map(|message| match message {
                AgentMessage::Llm(message) => Some(message.clone()),
                _ => None,
            })
            .collect())
    }

    fn prepare_initial_turn<'a>(
        &'a self,
        ctx: InitialTurnContext,
        cancel: CancellationToken,
        mut evidence: PreparationEvidence<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<Option<NextTurnState>, AgentError>> + Send + 'a>> {
        Box::pin(async move {
            if self.mode == Mode::InitialSummary {
                let pending = evidence
                    .begin_compaction(opi_agent::evidence::CompactionTrigger::Manual)
                    .unwrap();
                let result = evidence
                    .run_provider_call(
                        &self.collection,
                        Request {
                            model: ctx.state.model_selection.to_spec(),
                            context: opi_ai::transcript::normalize_context(
                                opi_ai::transcript::Context {
                                    system_prompt: Some("controlled summary".into()),
                                    messages: vec![],
                                    tools: vec![],
                                },
                            )
                            .unwrap(),
                            cancel: cancel.clone(),
                            projection: Default::default(),
                            transport: Default::default(),
                            max_tokens: None,
                            temperature: None,
                            sampling_params: Default::default(),
                            thinking: Default::default(),
                            stop_sequences: vec![],
                            metadata: None,
                            timeout: None,
                            extra_headers: vec![],
                            cache_retention: opi_ai::provider::CacheRetention::None,
                            session_id: None,
                            telemetry_context: None,
                            callbacks: Default::default(),
                        },
                        &pending,
                        cancel,
                    )
                    .await;
                evidence
                    .finish_compaction(
                        pending,
                        if result.is_ok() {
                            opi_agent::evidence::CompactionOutcome::Succeeded
                        } else {
                            opi_agent::evidence::CompactionOutcome::Failed
                        },
                    )
                    .unwrap();
                result?;
            }
            Ok(None)
        })
    }

    fn prepare_next_turn_transaction<'a>(
        &'a self,
        ctx: opi_agent::hooks::PrepareNextTurnContext,
        cancel: CancellationToken,
        mut evidence: PreparationEvidence<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<Option<NextTurnState>, AgentError>> + Send + 'a>> {
        Box::pin(async move {
            assert_eq!(self.mode, Mode::NextSummary);
            // Exercise the same borrowed execution ownership through nested
            // hooks, as product preparation does.
            let nested = Hooks {
                mode: Mode::InitialSummary,
                collection: self.collection.clone(),
            };
            nested
                .prepare_initial_turn(
                    InitialTurnContext { state: ctx.state },
                    cancel,
                    evidence.reborrow(),
                )
                .await
        })
    }
}

struct Fixture {
    agent: Agent,
    events: mpsc::Sender<Item>,
    released: oneshot::Receiver<bool>,
    dispatched: Arc<Notify>,
}

fn fixture(mode: Mode) -> Fixture {
    let (events, rx) = mpsc::channel(1);
    let (released, dropped) = oneshot::channel();
    let dispatched = Arc::new(Notify::new());
    let collection = Arc::new(single_route_collection(Box::new(ControlledProvider {
        source: Mutex::new(Some(Box::pin(ControlledSource {
            events: rx,
            released: Some(released),
            terminal_seen: false,
        }))),
        dispatched: dispatched.clone(),
        first_turn: AtomicBool::new(mode == Mode::NextSummary),
        models: vec![opi_ai::ModelInfo::new(
            "model",
            "Controlled",
            opi_ai::WireApi::OpenAiCompletions,
            opi_ai::ModelCapabilities::new(100_000, 4096),
        )],
    })));
    let agent = Agent::new(
        collection.clone(),
        vec![],
        None,
        "controlled:model".into(),
        None,
        Default::default(),
        AgentLoopConfig::default(),
        Box::new(Hooks { mode, collection }),
    )
    .unwrap();
    if mode == Mode::NextSummary {
        agent.follow_up("continue after the first turn".into());
    }
    Fixture {
        agent,
        events,
        released: dropped,
        dispatched,
    }
}

async fn bounded<T>(future: impl Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(5), future)
        .await
        .expect("controlled request must progress")
}

fn operation(
    agent: &mut Agent,
    mode: Mode,
) -> (
    Pin<Box<dyn Future<Output = AgentRunResult> + Send + '_>>,
    CancellationToken,
) {
    let run = agent.arm_run();
    let cancel = run.cancel_token();
    let future: Pin<Box<dyn Future<Output = AgentRunResult> + Send + '_>> =
        if mode == Mode::InitialSummary {
            Box::pin(agent.prepare_context_armed(run))
        } else {
            Box::pin(agent.prompt_armed("request", run))
        };
    (future, cancel)
}

#[tokio::test]
async fn abandoning_agent_or_summary_waiter_keeps_the_model_execution_alive() {
    for mode in [Mode::Turn, Mode::InitialSummary, Mode::NextSummary] {
        let Fixture {
            mut agent,
            events,
            mut released,
            dispatched,
        } = fixture(mode);
        let (mut operation, cancel) = operation(&mut agent, mode);
        bounded(async {
            tokio::select! {
                _ = dispatched.notified() => {},
                result = &mut operation => panic!("operation ended before dispatch: {result:?}"),
            }
        })
        .await;
        drop(operation);
        assert!(!cancel.is_cancelled());
        assert!(
            matches!(
                released.try_recv(),
                Err(oneshot::error::TryRecvError::Empty)
            ),
            "{mode:?}: dropping the waiter must retain the source"
        );
        for n in 0..129 {
            bounded(events.send(Ok(AssistantStreamEvent::TextDelta {
                content_index: 0,
                delta: n.to_string(),
                partial: base_assistant(),
            })))
            .await
            .unwrap();
        }
        bounded(events.send(Ok(AssistantStreamEvent::Done {
            reason: StopReason::Stop,
            message: base_assistant(),
        })))
        .await
        .unwrap();
        assert!(
            bounded(released).await.unwrap(),
            "source reached its terminal"
        );
        assert!(!cancel.is_cancelled());
    }
}

#[tokio::test]
async fn cancelling_agent_or_summary_joins_the_uncooperative_source() {
    for mode in [Mode::Turn, Mode::InitialSummary, Mode::NextSummary] {
        let Fixture {
            mut agent,
            events,
            mut released,
            dispatched,
        } = fixture(mode);
        let (mut operation, cancel) = operation(&mut agent, mode);
        bounded(async {
            tokio::select! {
                _ = dispatched.notified() => {},
                result = &mut operation => panic!("operation ended before dispatch: {result:?}"),
            }
        })
        .await;
        cancel.cancel();
        let result = bounded(operation).await;
        assert!(
            matches!(result.error(), Some(AgentError::Cancelled)),
            "{result:?}"
        );
        assert_eq!(
            released.try_recv(),
            Ok(false),
            "{mode:?}: cancellation returns after source release"
        );
        assert!(events.is_closed());
        assert_eq!(
            result
                .messages()
                .iter()
                .filter(|message| matches!(message, AgentMessage::Llm(Message::Assistant(_))))
                .count(),
            usize::from(mode == Mode::NextSummary)
        );
    }
}
