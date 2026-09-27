//! Validated preparation, external persistence, and cancellation boundaries.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use opi_agent::hooks::{AgentHooks, InitialTurnContext, PreparationCommit, PreparationEvidence};
use opi_agent::loop_types::{InferenceConfig, ModelSelection, NextTurnState};
use opi_agent::{Agent, AgentError, AgentLoopConfig, AgentMessage};
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::test_support::{MockProvider, MockResponse, single_route_collection, text_response};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy)]
enum PreparationCase {
    InvalidRoute,
    InvalidProductState,
    CancelBeforeCommit,
    CancelDuringCommit,
    LaterTransformFailure,
    LaterProviderFailure,
    DropSelectedInput,
}

struct TransactionHooks {
    case: PreparationCase,
    commits: Arc<AtomicUsize>,
    ordering: Arc<Mutex<Vec<&'static str>>>,
}

impl AgentHooks for TransactionHooks {
    fn owns_preparation_commit(&self) -> bool {
        true
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

    fn prepare_initial_turn<'a>(
        &'a self,
        ctx: InitialTurnContext,
        cancel: CancellationToken,
        _evidence: PreparationEvidence<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<Option<NextTurnState>, AgentError>> + Send + 'a>> {
        Box::pin(async move {
            if self.commits.load(Ordering::SeqCst) > 0 {
                return Ok(None);
            }
            self.ordering.lock().unwrap().push("prepare");
            let mut candidate = ctx.state;
            candidate.context.remove(0);
            candidate.inference.max_tokens = Some(321);
            if matches!(self.case, PreparationCase::InvalidRoute) {
                candidate.model_selection = ModelSelection::new("missing", "missing");
            }
            if matches!(self.case, PreparationCase::CancelBeforeCommit) {
                cancel.cancel();
            }
            if matches!(self.case, PreparationCase::DropSelectedInput) {
                candidate.context.pop();
            }
            Ok(Some(candidate))
        })
    }

    fn validate_prepared_state(&self, _: &NextTurnState) -> Result<(), AgentError> {
        self.ordering.lock().unwrap().push("validate");
        if matches!(self.case, PreparationCase::InvalidProductState) {
            Err(AgentError::Hook("product candidate rejected".into()))
        } else {
            Ok(())
        }
    }

    fn commit_prepared_state<'a>(
        &'a self,
        candidate: NextTurnState,
        cancel: CancellationToken,
        _evidence: PreparationEvidence<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<PreparationCommit, AgentError>> + Send + 'a>> {
        Box::pin(async move {
            self.ordering.lock().unwrap().push("commit-start");
            assert_eq!(candidate.inference.max_tokens, Some(321));
            if matches!(self.case, PreparationCase::CancelDuringCommit) {
                cancel.cancel();
                tokio::task::yield_now().await;
            }
            self.commits.fetch_add(1, Ordering::SeqCst);
            self.ordering.lock().unwrap().push("commit-end");
            Ok(PreparationCommit::ExternalStateCommitted)
        })
    }

    fn transform_context(
        &self,
        messages: Vec<AgentMessage>,
        _: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<AgentMessage>, AgentError>> + Send>> {
        self.ordering.lock().unwrap().push("transform");
        let fail = matches!(self.case, PreparationCase::LaterTransformFailure);
        Box::pin(async move {
            if fail {
                Err(AgentError::Hook("later transform failed".into()))
            } else {
                Ok(messages)
            }
        })
    }
}

fn user(text: &str) -> AgentMessage {
    AgentMessage::Llm(Message::User(UserMessage {
        content: vec![InputContent::Text { text: text.into() }],
        timestamp_ms: 1,
    }))
}

fn texts(messages: &[AgentMessage]) -> Vec<&str> {
    messages
        .iter()
        .filter_map(|message| match message {
            AgentMessage::Llm(Message::User(message)) => match message.content.first() {
                Some(InputContent::Text { text }) => Some(text.as_str()),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

async fn run_case(
    case: PreparationCase,
) -> (Agent, opi_agent::AgentRunResult, usize, Vec<&'static str>) {
    let provider = MockProvider::new_with_errors(
        "mock",
        vec![if matches!(case, PreparationCase::LaterProviderFailure) {
            MockResponse::Error(opi_ai::provider::ProviderError::Timeout)
        } else {
            MockResponse::Events(text_response("done"))
        }],
    );
    let calls = provider.call_log_handle();
    let commits = Arc::new(AtomicUsize::new(0));
    let ordering = Arc::new(Mutex::new(Vec::new()));
    let mut agent = Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        vec![],
        None,
        "mock:mock-model".into(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig::default(),
        Box::new(TransactionHooks {
            case,
            commits: commits.clone(),
            ordering: ordering.clone(),
        }),
    )
    .unwrap();
    agent
        .replace_state(NextTurnState::new(
            vec![user("old history")],
            ModelSelection::new("mock", "mock-model"),
            InferenceConfig::default(),
        ))
        .unwrap();
    agent.steer("selected queue".into());
    let result = agent.prompt("new prompt").await;
    if result.error().is_some() && !matches!(case, PreparationCase::LaterProviderFailure) {
        assert!(
            calls.lock().unwrap().is_empty(),
            "failed transition must not dispatch"
        );
    }
    let commit_count = commits.load(Ordering::SeqCst);
    let order = ordering.lock().unwrap().clone();
    (agent, result, commit_count, order)
}

#[tokio::test]
async fn invalid_intrinsic_candidate_never_commits() {
    let (agent, result, commits, order) = run_case(PreparationCase::InvalidRoute).await;
    assert!(matches!(
        result.error(),
        Some(AgentError::InvalidNextTurnCandidate(_))
    ));
    assert_eq!(commits, 0);
    assert_eq!(order, ["prepare"]);
    assert_eq!(
        texts(&agent.state_snapshot().context),
        ["old history", "new prompt"]
    );
}

#[tokio::test]
async fn product_candidate_validation_precedes_commit() {
    let (_, result, commits, order) = run_case(PreparationCase::InvalidProductState).await;
    assert!(matches!(result.error(), Some(AgentError::Hook(_))));
    assert_eq!(commits, 0);
    assert_eq!(order, ["prepare", "validate"]);
}

#[tokio::test]
async fn cancellation_before_commit_writes_nothing() {
    let (agent, result, commits, _) = run_case(PreparationCase::CancelBeforeCommit).await;
    assert!(matches!(result.error(), Some(AgentError::Cancelled)));
    assert_eq!(commits, 0);
    assert_eq!(
        texts(&agent.state_snapshot().context),
        ["old history", "new prompt"]
    );
}

#[tokio::test]
async fn cancellation_during_commit_settles_and_retains_committed_baseline() {
    let (mut agent, result, commits, order) = run_case(PreparationCase::CancelDuringCommit).await;
    assert!(matches!(result.error(), Some(AgentError::Cancelled)));
    assert_eq!(commits, 1);
    assert_eq!(order, ["prepare", "validate", "commit-start", "commit-end"]);
    let state = agent.state_snapshot();
    assert_eq!(texts(&state.context), ["new prompt"]);
    assert_eq!(state.inference.max_tokens, Some(321));
    let retried = agent.retry_last_turn().await;
    assert!(retried.error().is_none(), "{:?}", retried.error());
    assert_eq!(texts(retried.messages()), ["new prompt", "selected queue"]);
}

#[tokio::test]
async fn later_failure_keeps_external_commit_without_undelivered_queue_suffix() {
    let (agent, result, commits, order) = run_case(PreparationCase::LaterTransformFailure).await;
    assert!(matches!(result.error(), Some(AgentError::Hook(_))));
    assert_eq!(commits, 1);
    assert_eq!(
        order,
        [
            "prepare",
            "validate",
            "commit-start",
            "commit-end",
            "transform"
        ]
    );
    assert_eq!(texts(&agent.state_snapshot().context), ["new prompt"]);
    assert_eq!(texts(result.messages()), ["new prompt"]);
}

#[tokio::test]
async fn later_provider_failure_keeps_last_external_commit() {
    let (agent, result, commits, _) = run_case(PreparationCase::LaterProviderFailure).await;
    assert!(matches!(result.error(), Some(AgentError::Provider(_))));
    assert_eq!(commits, 1);
    assert_eq!(agent.state_snapshot().inference.max_tokens, Some(321));
    assert!(!texts(&agent.state_snapshot().context).contains(&"old history"));
}

struct NoOpHooks;

impl AgentHooks for NoOpHooks {
    fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
        Ok(messages
            .iter()
            .filter_map(|message| match message {
                AgentMessage::Llm(message) => Some(message.clone()),
                _ => None,
            })
            .collect())
    }
}

#[tokio::test]
async fn manual_preparation_has_no_task_dispatch_or_fabricated_turn_end() {
    let provider = MockProvider::new("mock", vec![text_response("must not dispatch")]);
    let calls = provider.call_log_handle();
    let mut agent = Agent::new(
        Arc::new(single_route_collection(Box::new(provider))),
        vec![],
        None,
        "mock:mock-model".into(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig::default(),
        Box::new(NoOpHooks),
    )
    .unwrap();
    let events = Arc::new(Mutex::new(Vec::new()));
    agent.subscribe(Box::new({
        let events = events.clone();
        move |event| events.lock().unwrap().push(event.clone())
    }));
    let run = agent.arm_run();
    let result = agent.prepare_context_armed(run).await;
    assert!(result.error().is_none());
    assert!(
        calls.lock().unwrap().is_empty(),
        "manual preparation cannot dispatch a task"
    );
    assert!(
        !events
            .lock()
            .unwrap()
            .iter()
            .any(|event| matches!(event, opi_agent::AgentEvent::TurnEnd { .. }))
    );
    assert!(
        !events
            .lock()
            .unwrap()
            .iter()
            .any(|event| matches!(event, opi_agent::AgentEvent::TurnStart))
    );
}

#[tokio::test]
async fn initial_replacement_cannot_discard_selected_input_before_delivery() {
    let (_, result, commits, _) = run_case(PreparationCase::DropSelectedInput).await;
    assert!(matches!(result.error(), Some(AgentError::Hook(_))));
    assert_eq!(commits, 0);
}

struct SummaryHooks {
    collection: Arc<opi_ai::ProviderCollection>,
    pending: Mutex<Option<opi_agent::hooks::PendingPreparationCompaction>>,
    next_turns: Arc<Mutex<Vec<u64>>>,
    completed: Arc<AtomicUsize>,
    cancel_after_start: bool,
    sink: Arc<opi_agent::evidence::InMemoryEvidenceSink>,
    projection_path: std::path::PathBuf,
    fail_terminal: bool,
    outcome: opi_agent::evidence::CompactionOutcome,
}

impl SummaryHooks {
    async fn summarize(
        &self,
        mut state: NextTurnState,
        cancel: CancellationToken,
        mut evidence: PreparationEvidence<'_>,
    ) -> Result<Option<NextTurnState>, AgentError> {
        use opi_agent::evidence::{CompactionOutcome, CompactionTrigger};
        let pending = evidence
            .begin_compaction(CompactionTrigger::Manual)
            .map_err(|_| AgentError::Hook("start evidence failed".into()))?;
        if self.cancel_after_start {
            cancel.cancel();
            tokio::task::yield_now().await;
            return Err(AgentError::Cancelled);
        }
        let summary = evidence
            .run_provider_call(
                &self.collection,
                opi_ai::provider::Request {
                    model: state.model_selection.to_spec(),
                    system: Some("fixture summary instruction".into()),
                    messages: vec![match user("summary request") {
                        AgentMessage::Llm(message) => message,
                        _ => unreachable!(),
                    }],
                    tools: vec![],
                    max_tokens: Some(256),
                    temperature: None,
                    thinking: Default::default(),
                    stop_sequences: vec![],
                    metadata: None,
                    cancel: cancel.clone(),
                    timeout: None,
                    extra_headers: vec![],
                    cache_retention: opi_ai::provider::CacheRetention::None,
                    session_id: None,
                },
                &pending,
                cancel,
            )
            .await;
        let summary = match summary {
            Ok(summary) => summary,
            Err(error) => {
                evidence
                    .finish_compaction(pending, CompactionOutcome::Failed)
                    .unwrap();
                return Err(error);
            }
        };
        let summary = summary
            .content
            .iter()
            .find_map(|content| match content {
                opi_ai::message::AssistantContent::Text { text } => Some(text.clone()),
                _ => None,
            })
            .unwrap();
        state.context.insert(
            0,
            AgentMessage::CompactionSummary(opi_agent::message::CompactionSummaryMessage {
                summary,
                first_kept_entry_id: "fixture-entry".into(),
                tokens_before: 12,
                tokens_after: 4,
            }),
        );
        *self.pending.lock().unwrap() = Some(pending);
        Ok(Some(state))
    }
}

impl AgentHooks for SummaryHooks {
    fn owns_preparation_commit(&self) -> bool {
        true
    }
    fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
        NoOpHooks.convert_to_llm(messages)
    }

    fn prepare_initial_turn<'a>(
        &'a self,
        ctx: InitialTurnContext,
        cancel: CancellationToken,
        evidence: PreparationEvidence<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<Option<NextTurnState>, AgentError>> + Send + 'a>> {
        Box::pin(self.summarize(ctx.state, cancel, evidence))
    }

    fn prepare_next_turn_transaction<'a>(
        &'a self,
        ctx: opi_agent::hooks::PrepareNextTurnContext,
        cancel: CancellationToken,
        evidence: PreparationEvidence<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<Option<NextTurnState>, AgentError>> + Send + 'a>> {
        self.next_turns.lock().unwrap().push(ctx.turn);
        Box::pin(self.summarize(ctx.state, cancel, evidence))
    }

    fn commit_prepared_state<'a>(
        &'a self,
        candidate: NextTurnState,
        _: CancellationToken,
        mut evidence: PreparationEvidence<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<PreparationCommit, AgentError>> + Send + 'a>> {
        Box::pin(async move {
            std::fs::write(
                &self.projection_path,
                serde_json::to_vec(&candidate.context).unwrap(),
            )
            .unwrap();
            if self.fail_terminal {
                self.sink
                    .inject_failure(opi_agent::evidence::EvidenceError::Emission {
                        detail: "fixture terminal emission failure after committed projection"
                            .into(),
                    });
            }
            let pending = self.pending.lock().unwrap().take().unwrap();
            // A completed durable write remains acknowledged even when its
            // subsequent evidence emission fails; Core owns the health gate.
            let _ = evidence.finish_compaction(pending, self.outcome);
            Ok(PreparationCommit::ExternalStateCommitted)
        })
    }

    fn should_stop_after_turn(
        &self,
        _: opi_agent::hooks::ShouldStopAfterTurnContext,
    ) -> Pin<Box<dyn Future<Output = bool> + Send>> {
        self.completed.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { false })
    }
}

struct SummaryFixture {
    agent: Agent,
    calls: Arc<Mutex<Vec<opi_ai::provider::Request>>>,
    sink: Arc<opi_agent::evidence::InMemoryEvidenceSink>,
    binding: opi_agent::evidence::RuntimeInputBinding,
    next_turns: Arc<Mutex<Vec<u64>>>,
    completed: Arc<AtomicUsize>,
    projection_dir: tempfile::TempDir,
}

fn summary_agent(
    responses: Vec<Vec<opi_ai::stream::AssistantStreamEvent>>,
    cancel_after_start: bool,
    fail_terminal: bool,
) -> SummaryFixture {
    summary_agent_with_outcome(
        responses,
        cancel_after_start,
        fail_terminal,
        opi_agent::evidence::CompactionOutcome::Succeeded,
    )
}

fn summary_agent_with_outcome(
    responses: Vec<Vec<opi_ai::stream::AssistantStreamEvent>>,
    cancel_after_start: bool,
    fail_terminal: bool,
    outcome: opi_agent::evidence::CompactionOutcome,
) -> SummaryFixture {
    use opi_agent::evidence::{
        AssemblyIdentity, ContentDigest, EvidenceSink, InMemoryEvidenceSink, RuntimeInputBinding,
    };
    let provider = MockProvider::new("mock", responses);
    let calls = provider.call_log_handle();
    let collection = Arc::new(single_route_collection(Box::new(provider)));
    let next_turns = Arc::new(Mutex::new(Vec::new()));
    let completed = Arc::new(AtomicUsize::new(0));
    let sink = Arc::new(InMemoryEvidenceSink::new());
    let projection_dir = tempfile::tempdir().unwrap();
    let mut agent = Agent::new(
        collection.clone(),
        vec![],
        None,
        "mock:mock-model".into(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig::default(),
        Box::new(SummaryHooks {
            collection,
            pending: Mutex::new(None),
            next_turns: next_turns.clone(),
            completed: completed.clone(),
            cancel_after_start,
            sink: sink.clone(),
            projection_path: projection_dir.path().join("projection.json"),
            fail_terminal,
            outcome,
        }),
    )
    .unwrap();
    let binding = RuntimeInputBinding::direct(
        ContentDigest::from_hex("1".repeat(64)).unwrap(),
        AssemblyIdentity::new("preparation-fixture").unwrap(),
    );
    sink.setup(&binding).unwrap();
    agent.set_evidence_sink(Some(sink.clone()));
    SummaryFixture {
        agent,
        calls,
        sink,
        binding,
        next_turns,
        completed,
        projection_dir,
    }
}

#[tokio::test]
async fn manual_summary_is_real_correlated_evidence_and_finalizes_without_task_events() {
    use opi_agent::evidence::*;
    let SummaryFixture {
        mut agent,
        calls,
        sink,
        binding,
        completed,
        projection_dir: _projection_dir,
        ..
    } = summary_agent(vec![text_response("actual summary")], false, false);
    let events = Arc::new(Mutex::new(Vec::new()));
    agent.subscribe(Box::new({
        let events = events.clone();
        move |event| events.lock().unwrap().push(event.clone())
    }));
    let armed = agent.arm_run();
    let mut result = agent.prepare_context_armed(armed).await;
    assert!(result.error().is_none(), "{:?}", result.error());
    assert_eq!(calls.lock().unwrap().len(), 1);
    assert_eq!(calls.lock().unwrap()[0].max_tokens, Some(256));
    assert_eq!(completed.load(Ordering::SeqCst), 0);
    assert!(!events.lock().unwrap().iter().any(|event| matches!(
        event,
        opi_agent::AgentEvent::TurnStart | opi_agent::AgentEvent::TurnEnd { .. }
    )));
    let records = sink.records();
    assert_eq!(records.len(), 4);
    assert_eq!(
        records.iter().map(|record| record.kind).collect::<Vec<_>>(),
        [
            CallKind::Compaction,
            CallKind::Provider,
            CallKind::Provider,
            CallKind::Compaction
        ]
    );
    assert!(
        records
            .iter()
            .all(|record| record.run == records[0].run && record.turn == records[0].turn)
    );
    assert!(
        records
            .windows(2)
            .all(|pair| pair[0].sequence < pair[1].sequence)
    );
    assert_eq!(records[1].parent, Some(records[0].call));
    assert_eq!(records[2].call, records[1].call);
    assert_eq!(records[3].call, records[0].call);
    let EvidencePayload::Provider(provider) = &records[2].payload else {
        panic!("actual provider terminal")
    };
    assert!(!matches!(
        provider.route.actual(),
        ActualRoute::Unknown { .. }
    ));
    let terminal = records.last().unwrap();
    let digest = || ContentDigest::from_hex("2".repeat(64)).unwrap();
    let unknown = || Measurement::Unknown {
        reason: UnknownReason::NotReported,
    };
    let manifest = ManifestCandidate {
        correlation: ManifestCorrelation {
            run: terminal.run,
            turn: terminal.turn,
            call: Some(terminal.call),
            parent: terminal.parent,
            sequence: terminal.sequence,
        },
        outcome: TerminalOutcome::Success,
        session: SessionBinding::NoSession,
        binding: binding.clone(),
        config: ConfigIdentity {
            harness_digest: digest(),
            runtime_digest: digest(),
            adapter_digest: digest(),
            material_digest: digest(),
        },
        provider: ProviderInvocationFacts::applicable(
            provider.route.clone(),
            provider.provenance.clone(),
        ),
        policy: UserPolicyFacts {
            policy_digest: digest(),
            capability: None,
            permission_ref: None,
            permission_scope: None,
            scoped_grant_ref: None,
        },
        input_identity: InputIdentity {
            prompt_digest: digest(),
            system_digest: None,
            tool_schema_digests: vec![],
        },
        environment: EnvironmentFacts {
            budget: unknown(),
            trigger: ExecutionTrigger::Compaction {
                reason: CompactionTrigger::Manual,
            },
            time: unknown(),
            platform: PlatformIdentity::new("fixture"),
        },
        usage: UsageFacts {
            input_tokens: unknown(),
            output_tokens: unknown(),
        },
        artifacts: vec![],
        completeness: EvidenceCompleteness::Complete,
    }
    .validate(EvidenceRunObservation::new(&binding, &records, &[]))
    .unwrap();
    result.finalize_evidence(&manifest).unwrap();
    assert!(sink.completed_manifest().is_some());
}

#[tokio::test]
async fn summary_calls_share_one_run_without_counting_as_completed_task_turns() {
    let SummaryFixture {
        mut agent,
        calls,
        sink,
        next_turns,
        completed,
        projection_dir: _projection_dir,
        ..
    } = summary_agent(
        vec![
            text_response("initial summary"),
            opi_ai::test_support::tool_call_response("work", "unknown", "{}"),
            text_response("next summary"),
            text_response("done"),
        ],
        false,
        false,
    );
    let result = agent.prompt("task").await;
    assert!(result.error().is_none(), "{:?}", result.error());
    assert_eq!(calls.lock().unwrap().len(), 4);
    assert_eq!(completed.load(Ordering::SeqCst), 2);
    assert_eq!(*next_turns.lock().unwrap(), [1]);
    let records = sink.records();
    assert!(records.iter().all(|record| record.run == records[0].run));
    assert_eq!(
        records
            .iter()
            .filter(|record| record.parent.is_some()
                && record.kind == opi_agent::evidence::CallKind::Provider)
            .map(|record| record.call)
            .collect::<std::collections::HashSet<_>>()
            .len(),
        2
    );
}

#[tokio::test]
async fn preparation_cancel_closes_started_compaction_once_without_provider_or_commit() {
    let SummaryFixture {
        mut agent,
        calls,
        sink,
        projection_dir: _projection_dir,
        ..
    } = summary_agent(vec![], true, false);
    let armed = agent.arm_run();
    let result = agent.prepare_context_armed(armed).await;
    assert!(matches!(result.error(), Some(AgentError::Cancelled)));
    assert!(calls.lock().unwrap().is_empty());
    let records: Vec<_> = sink
        .records()
        .into_iter()
        .filter(|record| record.kind == opi_agent::evidence::CallKind::Compaction)
        .collect();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].call, records[1].call);
    assert!(
        matches!(&records[1].payload, opi_agent::evidence::EvidencePayload::Compaction(facts) if facts.outcome() == Some(opi_agent::evidence::CompactionOutcome::Aborted))
    );
}

#[tokio::test]
async fn acknowledged_commit_survives_terminal_emission_failure_without_next_dispatch() {
    let SummaryFixture {
        mut agent,
        calls,
        sink,
        projection_dir,
        ..
    } = summary_agent(
        vec![
            text_response("accepted summary"),
            text_response("must not dispatch"),
        ],
        false,
        true,
    );
    let result = agent.prompt("task").await;
    assert!(matches!(
        result.error(),
        Some(AgentError::EvidenceFinalization(_))
    ));
    assert_eq!(
        calls.lock().unwrap().len(),
        1,
        "only the real summary call dispatched"
    );
    assert!(!result.evidence_health().is_healthy());
    let projection: serde_json::Value = serde_json::from_slice(
        &std::fs::read(projection_dir.path().join("projection.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&agent.state_snapshot().context).unwrap(),
        projection
    );
    assert!(sink.completed_manifest().is_none());
}

#[tokio::test]
async fn acknowledged_preparation_uncertainty_stops_before_next_external_effect() {
    use opi_agent::evidence::{CompactionOutcome, TerminalOutcome};
    for (outcome, expected) in [
        (
            CompactionOutcome::PartialSideEffect,
            TerminalOutcome::PartialSideEffect,
        ),
        (
            CompactionOutcome::CleanupUnknown,
            TerminalOutcome::CleanupUnknown,
        ),
    ] {
        let SummaryFixture {
            mut agent,
            calls,
            projection_dir,
            ..
        } = summary_agent_with_outcome(
            vec![
                text_response("accepted summary"),
                text_response("must not dispatch"),
            ],
            false,
            false,
            outcome,
        );
        let result = agent.prompt("task").await;
        assert!(result.error().is_some());
        assert_eq!(*result.terminal_outcome(), expected);
        assert_eq!(
            calls.lock().unwrap().len(),
            1,
            "known uncertainty forbids the task call"
        );
        let projection: serde_json::Value = serde_json::from_slice(
            &std::fs::read(projection_dir.path().join("projection.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(&agent.state_snapshot().context).unwrap(),
            projection
        );
    }
}
