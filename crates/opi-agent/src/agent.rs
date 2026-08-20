//! Stateful Agent wrapper around the agent loop.
//!
//! Provides `prompt`, `continue_`, `abort`, `subscribe`, `steer`, and
//! `follow_up` methods. The Agent is the sole durable owner of the complete
//! mutable [`NextTurnState`] (conversation context, canonical provider:model
//! selection, and inference configuration) and dispatches every logical model
//! call through the registered [`opi_ai::ProviderCollection`] via one prepared call.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use opi_ai::ProviderCollection;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::ThinkingConfig;
use tokio_util::sync::CancellationToken;

use crate::diagnostic_sink::DiagnosticSink;
use crate::event::{AgentEvent, AgentEventSink};
use crate::hooks::AgentHooks;
use crate::loop_types::{
    AgentError, AgentLoopConfig, AgentLoopContext, InferenceConfig, ModelSelection, NextTurnState,
    validate_next_turn_candidate,
};
use crate::message::AgentMessage;

// -- Agent -------------------------------------------------------------------

type EventSubscriber = Box<dyn Fn(&AgentEvent) + Send + Sync>;
type SharedEventSubscriber = Arc<dyn Fn(&AgentEvent) + Send + Sync>;

#[derive(Default)]
struct PublicEventFanout {
    subscribers: Mutex<Vec<SharedEventSubscriber>>,
}

impl PublicEventFanout {
    fn subscribe(&self, subscriber: EventSubscriber) {
        self.subscribers.lock().unwrap().push(Arc::from(subscriber));
    }

    fn emit(&self, event: AgentEvent) {
        let public_event = event.redacted_for_public();
        let subscribers = self.subscribers.lock().unwrap().clone();
        for subscriber in subscribers {
            subscriber(&public_event);
        }
    }
}

#[cfg(test)]
mod event_fanout_tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn callback_runs_without_holding_the_subscriber_mutex() {
        let fanout = Arc::new(PublicEventFanout::default());
        let weak_fanout = Arc::downgrade(&fanout);
        let lock_was_available = Arc::new(AtomicBool::new(false));
        fanout.subscribe(Box::new({
            let lock_was_available = lock_was_available.clone();
            move |_| {
                let fanout = weak_fanout.upgrade().expect("fan-out remains alive");
                lock_was_available.store(fanout.subscribers.try_lock().is_ok(), Ordering::SeqCst);
            }
        }));

        fanout.emit(AgentEvent::AgentStart);

        assert!(
            lock_was_available.load(Ordering::SeqCst),
            "callbacks must run after the subscriber snapshot releases its lock"
        );
    }
}

/// Core-owned result of one public Agent operation.
///
/// The actual terminal execution outcome and final evidence-health snapshot
/// remain available together even when the operation failed. The result also
/// owns the run-local evidence lifecycle used by post-loop compaction and
/// manifest finalization. Dropping any result whose lifecycle is still active
/// performs a synchronous fail-closed
/// [`crate::evidence::EvidenceSink::abandon_run`] attempt. Because `Drop` cannot
/// return an error, callers that need to observe abandonment failure must call
/// [`AgentRunResult::finalize_evidence`] or [`AgentRunResult::abandon_evidence`],
/// as appropriate, and inspect the retained result.
/// [`AgentRunResult::into_execution_result`] performs required abandonment
/// before consumption, but preserves an owning execution error instead of
/// returning a separate cleanup failure.
#[must_use = "inspect execution outcome and evidence health before discarding a run result"]
pub struct AgentRunResult {
    messages: Vec<AgentMessage>,
    error: Option<AgentError>,
    terminal_outcome: crate::evidence::TerminalOutcome,
    evidence_health: crate::evidence::EvidenceHealth,
    evidence_cleanup_error: Option<crate::evidence::EvidenceError>,
    identities: crate::evidence::IdentityAllocator,
    evidence_sink: Option<Arc<dyn crate::evidence::EvidenceSink>>,
    lifecycle: AgentRunLifecycle,
}

impl std::fmt::Debug for AgentRunResult {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AgentRunResult")
            .field("messages", &self.messages)
            .field("error", &self.error)
            .field("terminal_outcome", &self.terminal_outcome)
            .field("evidence_health", &self.evidence_health)
            .field("evidence_cleanup_error", &self.evidence_cleanup_error)
            .field("lifecycle_phase", &self.lifecycle_phase())
            .finish_non_exhaustive()
    }
}

/// Post-loop evidence lifecycle phase owned by [`AgentRunResult`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRunLifecyclePhase {
    /// The run may begin compaction or be finalized.
    Active,
    /// One compaction start was emitted and requires one matching terminal.
    CompactionPending,
    /// The run was finalized or its incomplete evidence was abandoned.
    FinalizedOrAbandoned,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompactionIdentity {
    run: crate::evidence::RunId,
    call: Option<crate::evidence::CallId>,
    trigger: crate::evidence::CompactionTrigger,
}

#[derive(Debug)]
enum AgentRunLifecycle {
    Active,
    CompactionPending(CompactionIdentity),
    FinalizedOrAbandoned,
}

/// Opaque proof that compaction start evidence completed before mutation.
/// Passing this value to [`AgentRunResult::finish_compaction`] records the
/// terminal outcome on the same evidence call. Reusing it is rejected after
/// the first terminal attempt.
///
/// ```compile_fail
/// #![deny(unused_must_use)]
/// use opi_agent::PendingCompaction;
/// fn token() -> PendingCompaction { unimplemented!() }
/// fn discard() { token(); }
/// ```
#[must_use = "a started compaction must be completed with finish_compaction"]
#[derive(Debug)]
pub struct PendingCompaction {
    identity: CompactionIdentity,
}

impl AgentRunResult {
    /// Actual messages retained at the loop boundary, including terminal tool
    /// results that preceded a partial or cleanup-unknown failure.
    pub fn messages(&self) -> &[AgentMessage] {
        &self.messages
    }

    /// The owning execution error, if the run did not complete normally.
    pub fn error(&self) -> Option<&AgentError> {
        self.error.as_ref()
    }

    /// Exact terminal execution outcome used by the finalized manifest.
    pub fn terminal_outcome(&self) -> &crate::evidence::TerminalOutcome {
        &self.terminal_outcome
    }

    /// Final core-owned evidence-health snapshot.
    pub fn evidence_health(&self) -> &crate::evidence::EvidenceHealth {
        &self.evidence_health
    }

    /// Cleanup failure retained separately from the owning execution or
    /// evidence-finalization error.
    pub fn evidence_cleanup_error(&self) -> Option<&crate::evidence::EvidenceError> {
        self.evidence_cleanup_error.as_ref()
    }

    /// Opaque run identity required to assemble the matching final manifest.
    pub fn run_id(&self) -> crate::evidence::RunId {
        self.identities.run_id()
    }

    /// Current post-loop compaction/finalization phase.
    pub fn lifecycle_phase(&self) -> AgentRunLifecyclePhase {
        match self.lifecycle {
            AgentRunLifecycle::Active => AgentRunLifecyclePhase::Active,
            AgentRunLifecycle::CompactionPending(_) => AgentRunLifecyclePhase::CompactionPending,
            AgentRunLifecycle::FinalizedOrAbandoned => AgentRunLifecyclePhase::FinalizedOrAbandoned,
        }
    }

    /// Consume the lifecycle result into the traditional execution result.
    /// Evidence health and terminal classification should be inspected before
    /// consuming when the caller performs evidence finalization. Consumption
    /// synchronously abandons any lifecycle the caller did not finalize.
    pub fn into_execution_result(mut self) -> Result<Vec<AgentMessage>, AgentError> {
        let pending_compaction = matches!(self.lifecycle, AgentRunLifecycle::CompactionPending(_));
        let active_capture =
            matches!(self.lifecycle, AgentRunLifecycle::Active) && self.evidence_sink.is_some();
        let cleanup_error = self.abandon_unfinalized(
            "run result was consumed before evidence finalization or abandonment",
        );
        if let Some(error) = self.error.take() {
            return Err(error);
        }
        if pending_compaction {
            let detail = if cleanup_error.is_some() {
                "pending compaction was abandoned but cleanup could not be confirmed"
            } else {
                "pending compaction was abandoned before execution result consumption"
            };
            return Err(AgentError::EvidenceFinalization(detail.to_owned()));
        }
        if cleanup_error.is_some() {
            return Err(AgentError::EvidenceFinalization(
                "active evidence run was abandoned but cleanup could not be confirmed".to_owned(),
            ));
        }
        if active_capture {
            return Err(AgentError::EvidenceFinalization(
                "active evidence run was abandoned before execution result consumption".to_owned(),
            ));
        }
        Ok(std::mem::take(&mut self.messages))
    }

    /// Emit a typed compaction-start record before any compaction mutation.
    /// A failed emission returns no token, so a caller cannot honestly report a
    /// terminal compaction for a mutation that was never authorized to start.
    pub fn begin_compaction(
        &mut self,
        trigger: crate::evidence::CompactionTrigger,
    ) -> Result<PendingCompaction, crate::evidence::EvidenceError> {
        if !matches!(self.lifecycle, AgentRunLifecycle::Active) {
            return Err(crate::evidence::EvidenceError::Emission {
                detail: "run lifecycle cannot begin compaction in its current phase".to_owned(),
            });
        }
        if !self.evidence_health.is_healthy() {
            return Err(crate::evidence::EvidenceError::Emission {
                detail: "incomplete evidence prohibits compaction start".to_owned(),
            });
        }
        let run = self.identities.run_id();
        let identity = if let Some(sink) = self.evidence_sink.as_ref() {
            let call = self.identities.next_call();
            let record = crate::evidence::EvidenceRecord {
                run,
                turn: None,
                call,
                parent: None,
                sequence: self.identities.next_sequence(),
                kind: crate::evidence::CallKind::Compaction,
                payload: crate::evidence::EvidencePayload::Compaction(
                    crate::evidence::CompactionEvidenceFacts::started(trigger),
                ),
            };
            if let Err(error) = sink.emit(&record) {
                self.evidence_health
                    .advance_on_failure(error.failure_code());
                return Err(error);
            }
            CompactionIdentity {
                run,
                call: Some(call),
                trigger,
            }
        } else {
            CompactionIdentity {
                run,
                call: None,
                trigger,
            }
        };
        self.lifecycle = AgentRunLifecycle::CompactionPending(identity.clone());
        Ok(PendingCompaction { identity })
    }

    /// Record the terminal outcome of a started compaction. The outcome updates
    /// the run before emission, so a terminal sink failure cannot erase the
    /// actual partial-side-effect or cleanup-unknown state.
    pub fn finish_compaction(
        &mut self,
        pending: &PendingCompaction,
        outcome: crate::evidence::CompactionOutcome,
    ) -> Result<(), crate::evidence::EvidenceError> {
        let AgentRunLifecycle::CompactionPending(expected) = &self.lifecycle else {
            return Err(crate::evidence::EvidenceError::Emission {
                detail: "run lifecycle has no pending compaction".to_owned(),
            });
        };
        if expected != &pending.identity {
            return Err(crate::evidence::EvidenceError::Emission {
                detail: "compaction token does not match the pending operation".to_owned(),
            });
        }
        self.retain_compaction_outcome(outcome);
        let result = match (self.evidence_sink.as_ref(), pending.identity.call) {
            (Some(sink), Some(call)) => sink.emit(&crate::evidence::EvidenceRecord {
                run: pending.identity.run,
                turn: None,
                call,
                parent: None,
                sequence: self.identities.next_sequence(),
                kind: crate::evidence::CallKind::Compaction,
                payload: crate::evidence::EvidencePayload::Compaction(
                    crate::evidence::CompactionEvidenceFacts::terminal(
                        pending.identity.trigger,
                        outcome,
                    ),
                ),
            }),
            _ => Ok(()),
        };
        // The lower-boundary compaction has a terminal truth after this call,
        // even when recording that truth failed. A failed emission advances
        // health and prevents manifest publication rather than leaving a token
        // that could emit a conflicting second terminal.
        self.lifecycle = AgentRunLifecycle::Active;
        if let Err(error) = result {
            self.evidence_health
                .advance_on_failure(error.failure_code());
            return Err(error);
        }
        Ok(())
    }

    /// Finalize this run through its bound sink. Any failure advances the
    /// core-owned health and abandons provisional sink state. If abandonment
    /// also fails, cleanup becomes unknown while the original owning error is
    /// returned unchanged.
    pub fn finalize_evidence(
        &mut self,
        manifest: &crate::evidence::FinalizedManifest,
    ) -> Result<(), crate::evidence::EvidenceError> {
        if matches!(self.lifecycle, AgentRunLifecycle::FinalizedOrAbandoned) {
            return Err(crate::evidence::EvidenceError::Finalization {
                detail: "run evidence lifecycle is already finalized or abandoned".to_owned(),
            });
        }
        let sink = self.evidence_sink.clone();
        if matches!(self.lifecycle, AgentRunLifecycle::CompactionPending(_)) {
            let error = crate::evidence::EvidenceError::Finalization {
                detail: "pending compaction has no terminal outcome".to_owned(),
            };
            let _ = self.abandon_pending_compaction(error.failure_code());
            return Err(error);
        }
        let result = if !self.evidence_health.is_healthy() {
            Err(crate::evidence::EvidenceError::Finalization {
                detail: "evidence is incomplete; manifest finalization withheld".to_owned(),
            })
        } else if manifest.facts().correlation.run != self.identities.run_id() {
            Err(crate::evidence::EvidenceError::Finalization {
                detail: "manifest run does not match the run lifecycle".to_owned(),
            })
        } else if manifest.facts().outcome != self.terminal_outcome {
            Err(crate::evidence::EvidenceError::Finalization {
                detail: "manifest terminal outcome does not match the run lifecycle".to_owned(),
            })
        } else if let Some(sink) = sink.as_ref() {
            sink.finalize_run(manifest)
        } else {
            Ok(())
        };
        if let Err(error) = result {
            let _ = self.abandon_evidence(&error);
            return Err(error);
        }
        self.lifecycle = AgentRunLifecycle::FinalizedOrAbandoned;
        Ok(())
    }

    /// Mark an unfinalizable run incomplete and abandon its provisional sink
    /// state without replacing the owning lifecycle failure.
    ///
    /// The returned error, when present, is only the cleanup failure from
    /// [`crate::evidence::EvidenceSink::abandon_run`]. The caller retains the
    /// supplied `failure` as the owning error and can observe cleanup
    /// uncertainty through [`Self::terminal_outcome`] and
    /// [`Self::evidence_health`].
    pub fn abandon_evidence(
        &mut self,
        failure: &crate::evidence::EvidenceError,
    ) -> Result<(), crate::evidence::EvidenceError> {
        if matches!(self.lifecycle, AgentRunLifecycle::FinalizedOrAbandoned) {
            return Err(crate::evidence::EvidenceError::Finalization {
                detail: "run evidence lifecycle is already finalized or abandoned".to_owned(),
            });
        }
        if matches!(self.lifecycle, AgentRunLifecycle::CompactionPending(_)) {
            return self
                .abandon_pending_compaction(failure.failure_code())
                .map_or(Ok(()), Err);
        }
        self.evidence_health
            .advance_on_failure(failure.failure_code());
        let cleanup = self
            .evidence_sink
            .as_ref()
            .map_or(Ok(()), |sink| sink.abandon_run(&self.terminal_outcome));
        self.lifecycle = AgentRunLifecycle::FinalizedOrAbandoned;
        if let Err(error) = cleanup {
            self.evidence_health
                .advance_on_failure(error.failure_code());
            self.terminal_outcome = crate::evidence::TerminalOutcome::CleanupUnknown;
            self.evidence_cleanup_error = Some(error.clone());
            return Err(error);
        }
        Ok(())
    }

    fn abandon_pending_compaction(
        &mut self,
        failure_code: crate::evidence::EvidenceFailureCode,
    ) -> Option<crate::evidence::EvidenceError> {
        if !matches!(self.lifecycle, AgentRunLifecycle::CompactionPending(_)) {
            return None;
        }
        self.terminal_outcome = crate::evidence::TerminalOutcome::CleanupUnknown;
        self.evidence_health.advance_on_failure(failure_code);
        let cleanup_error = self
            .evidence_sink
            .as_ref()
            .and_then(|sink| sink.abandon_run(&self.terminal_outcome).err());
        if let Some(error) = cleanup_error.as_ref() {
            self.evidence_health
                .advance_on_failure(error.failure_code());
            self.evidence_cleanup_error = Some(error.clone());
        }
        self.lifecycle = AgentRunLifecycle::FinalizedOrAbandoned;
        cleanup_error
    }

    fn abandon_unfinalized(&mut self, detail: &str) -> Option<crate::evidence::EvidenceError> {
        match self.lifecycle {
            AgentRunLifecycle::FinalizedOrAbandoned => None,
            AgentRunLifecycle::CompactionPending(_) => {
                self.abandon_pending_compaction(crate::evidence::EvidenceFailureCode::Finalization)
            }
            AgentRunLifecycle::Active => {
                if self.evidence_sink.is_none() {
                    self.lifecycle = AgentRunLifecycle::FinalizedOrAbandoned;
                    return None;
                }
                let failure = crate::evidence::EvidenceError::Finalization {
                    detail: detail.to_owned(),
                };
                self.abandon_evidence(&failure).err()
            }
        }
    }

    fn retain_compaction_outcome(&mut self, outcome: crate::evidence::CompactionOutcome) {
        use crate::evidence::{CompactionOutcome, TerminalOutcome};
        let candidate = match outcome {
            CompactionOutcome::Succeeded => return,
            CompactionOutcome::Aborted => TerminalOutcome::Cancelled,
            CompactionOutcome::Failed => TerminalOutcome::Failed,
            CompactionOutcome::PartialSideEffect => TerminalOutcome::PartialSideEffect,
            CompactionOutcome::CleanupUnknown => TerminalOutcome::CleanupUnknown,
        };
        let rank = |outcome: &TerminalOutcome| match outcome {
            TerminalOutcome::Success => 0,
            TerminalOutcome::Failed | TerminalOutcome::Cancelled => 1,
            TerminalOutcome::PartialSideEffect => 2,
            TerminalOutcome::CleanupUnknown => 3,
        };
        if rank(&candidate) > rank(&self.terminal_outcome) {
            self.terminal_outcome = candidate;
        }
    }
}

impl Drop for AgentRunResult {
    fn drop(&mut self) {
        let _ = self.abandon_unfinalized(
            "run result was dropped before evidence finalization or abandonment",
        );
    }
}

/// Clonable control surface for an agent turn owned elsewhere.
#[derive(Clone)]
pub struct AgentControl {
    cancel: CancellationToken,
    steering_queue: Arc<Mutex<VecDeque<String>>>,
    follow_up_queue: Arc<Mutex<VecDeque<String>>>,
}

impl AgentControl {
    /// Cancel the currently running agent operation.
    pub fn abort(&self) {
        self.cancel.cancel();
    }

    /// Queue a steering message for the next provider request.
    pub fn steer(&self, message: String) {
        self.steering_queue.lock().unwrap().push_back(message);
    }

    /// Queue a follow-up message for when the agent would otherwise stop.
    pub fn follow_up(&self, message: String) {
        self.follow_up_queue.lock().unwrap().push_back(message);
    }
}

/// Opaque cancellation generation armed for exactly one Agent operation.
///
/// The Reference Product arms this value before its awaited preflight and then
/// passes the same value into the Agent operation. This prevents an interrupt
/// observed during preflight from being replaced by a later cancellation reset.
#[must_use = "an armed run must be passed to an Agent operation"]
pub struct ArmedAgentRun {
    owner: Arc<ArmedRunOwner>,
    generation: Arc<ArmedRunGeneration>,
    cancel: CancellationToken,
}

struct ArmedRunOwner;
struct ArmedRunGeneration;

impl ArmedAgentRun {
    /// Clone the cancellation token for this run generation.
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel.clone()
    }
}

/// Stateful wrapper around `agent_loop` with conversation state, cancellation,
/// event subscription, and message queue management.
///
/// The Agent durably owns one complete [`NextTurnState`] and one dispatchable
/// [`opi_ai::ProviderCollection`]. [`Agent::replace_state`] is the single
/// validated idle-state replacement operation for model, inference, and
/// messages together.
pub struct Agent {
    collection: Arc<ProviderCollection>,
    registry: Arc<crate::authority::ToolRegistry>,
    authorizer: Option<Arc<dyn crate::authority::ToolAuthorizer>>,
    state: NextTurnState,
    system: Option<String>,
    session_id: Option<String>,
    config: AgentLoopConfig,
    hooks: Box<dyn AgentHooks>,
    cancel: CancellationToken,
    armed_run_owner: Arc<ArmedRunOwner>,
    latest_armed_generation: Option<Arc<ArmedRunGeneration>>,
    event_fanout: Arc<PublicEventFanout>,
    steering_queue: Arc<Mutex<VecDeque<String>>>,
    follow_up_queue: Arc<Mutex<VecDeque<String>>>,
    diagnostic_sink: Option<Arc<dyn DiagnosticSink>>,
    evidence_sink: Option<Arc<dyn crate::evidence::EvidenceSink>>,
}

impl Agent {
    /// Create a new Agent over a dispatchable provider collection.
    ///
    /// `model_spec` is the canonical `provider:model` selection for the first
    /// call; `inference` is the initial per-request inference configuration.
    /// Both live in the durable [`NextTurnState`] and may be replaced atomically
    /// via [`Agent::replace_state`] between runs.
    #[allow(clippy::too_many_arguments)] // Agent construction seam (Phase 17.4 adds the authorizer binding)
    pub fn new(
        collection: Arc<ProviderCollection>,
        registrations: Vec<crate::authority::RegisteredTool>,
        authorizer: Option<Arc<dyn crate::authority::ToolAuthorizer>>,
        model_spec: String,
        system: Option<String>,
        inference: InferenceConfig,
        config: AgentLoopConfig,
        hooks: Box<dyn AgentHooks>,
    ) -> Result<Self, AgentError> {
        let (provider_id, model_id) =
            model_spec
                .split_once(':')
                .ok_or_else(|| AgentError::InvalidModelSpec {
                    spec: model_spec.clone(),
                })?;
        opi_ai::registry::parse_model_spec(&model_spec).map_err(|_| {
            AgentError::InvalidModelSpec {
                spec: model_spec.clone(),
            }
        })?;
        let model_selection = ModelSelection::new(provider_id, model_id);
        let state = NextTurnState::new(Vec::new(), model_selection, inference);
        validate_next_turn_candidate(&collection, &state)?;
        let registry = Arc::new(
            crate::authority::ToolRegistry::from_tools(registrations)
                .map_err(|e| AgentError::InvalidToolRegistration(e.to_string()))?,
        );
        Ok(Self {
            collection,
            registry,
            authorizer,
            state,
            system,
            config,
            hooks,
            cancel: CancellationToken::new(),
            armed_run_owner: Arc::new(ArmedRunOwner),
            latest_armed_generation: None,
            event_fanout: Arc::new(PublicEventFanout::default()),
            steering_queue: Arc::new(Mutex::new(VecDeque::new())),
            follow_up_queue: Arc::new(Mutex::new(VecDeque::new())),
            diagnostic_sink: None,
            evidence_sink: None,
            session_id: None,
        })
    }

    /// Set the opaque session identifier carried into every provider Request.
    /// The harness calls this on startup, resume, and fork so the active
    /// session id flows through the agent loop to provider affinity headers.
    pub fn set_session_id(&mut self, session_id: Option<String>) {
        self.session_id = session_id;
    }

    /// Install a diagnostic sink that receives observations emitted from
    /// runtime failure paths during `prompt`/`continue_` (retry, cancellation,
    /// provider/tool failures). `None` (the default) disables emission without
    /// changing any other runtime behavior. Non-breaking: callers that do not
    /// set a sink are unaffected.
    pub fn set_diagnostic_sink(&mut self, sink: Option<Arc<dyn DiagnosticSink>>) {
        self.diagnostic_sink = sink;
    }

    /// Install an evidence sink that receives the run's call-graph lifecycle
    /// (stable run/turn/call identities and ordered records) during the next
    /// `prompt`/`continue_`/`retry_last_turn` run. The returned
    /// [`AgentRunResult`] owns post-loop compaction and manifest finalization.
    /// `None` (the default) disables record emission; typed identities remain
    /// available to authorization. The Reference Product binds its recording
    /// adapter here; the minimal runtime leaves it unset.
    pub fn set_evidence_sink(&mut self, sink: Option<Arc<dyn crate::evidence::EvidenceSink>>) {
        self.evidence_sink = sink;
    }

    /// Atomically replace the complete durable state with a validated
    /// candidate. This is the single public idle-state replacement operation.
    /// The candidate's model selection must resolve to a route in the provider
    /// collection; otherwise the prior state is preserved and the error is
    /// returned. Must be called while the agent is idle.
    pub fn replace_state(&mut self, candidate: NextTurnState) -> Result<(), AgentError> {
        validate_next_turn_candidate(&self.collection, &candidate)?;
        self.state = candidate;
        Ok(())
    }

    /// Send a user message and run the agent loop.
    ///
    /// Resets the cancellation state if the agent was previously aborted,
    /// allowing a fresh conversation turn.
    pub async fn prompt(&mut self, text: impl Into<String>) -> AgentRunResult {
        let run = self.arm_run();
        self.prompt_armed(text, run).await
    }

    /// Run a prompt with the cancellation generation already armed by a caller
    /// that performs awaited preflight before entering the Agent loop.
    #[doc(hidden)]
    pub async fn prompt_armed(
        &mut self,
        text: impl Into<String>,
        run: ArmedAgentRun,
    ) -> AgentRunResult {
        if let Err(error) = self.validate_armed_run(&run) {
            return self.preflight_failure(error);
        }
        self.push_user_text(text.into());
        self.run_with_token(run.cancel).await
    }

    /// Send a user message with arbitrary content (text + images) and run the
    /// agent loop.
    pub async fn prompt_with_content(&mut self, content: Vec<InputContent>) -> AgentRunResult {
        let run = self.arm_run();
        self.prompt_with_content_armed(content, run).await
    }

    /// Run a content prompt with a cancellation generation armed before caller
    /// preflight.
    #[doc(hidden)]
    pub async fn prompt_with_content_armed(
        &mut self,
        content: Vec<InputContent>,
        run: ArmedAgentRun,
    ) -> AgentRunResult {
        if let Err(error) = self.validate_armed_run(&run) {
            return self.preflight_failure(error);
        }
        self.state
            .context
            .push(AgentMessage::Llm(Message::User(UserMessage {
                content,
                timestamp_ms: opi_ai::time::now_ms(),
            })));
        self.run_with_token(run.cancel).await
    }

    /// Continue the conversation with an additional user message.
    ///
    /// Requires the last context message to be a user message or tool result.
    pub async fn continue_(&mut self, text: impl Into<String>) -> AgentRunResult {
        let run = self.arm_run();
        self.continue_armed(text, run).await
    }

    /// Continue with a cancellation generation armed before caller preflight.
    #[doc(hidden)]
    pub async fn continue_armed(
        &mut self,
        text: impl Into<String>,
        run: ArmedAgentRun,
    ) -> AgentRunResult {
        if let Err(error) = self.validate_armed_run(&run) {
            return self.preflight_failure(error);
        }
        if self.state.context.is_empty() {
            return self.preflight_failure(AgentError::Hook("cannot continue: no messages".into()));
        }

        self.push_user_text(text.into());
        self.run_with_token(run.cancel).await
    }

    /// Re-run the agent loop with the current state, without pushing a new
    /// user message. Used by the harness to retry after a `CredentialNeeded`
    /// error is resolved via interactive login — the user message from the
    /// original `prompt`/`continue_` call is already in the context, so
    /// pushing another would duplicate it in the session.
    pub async fn retry_last_turn(&mut self) -> AgentRunResult {
        let run = self.arm_run();
        self.retry_last_turn_armed(run).await
    }

    /// Retry with a cancellation generation armed before caller preflight.
    #[doc(hidden)]
    pub async fn retry_last_turn_armed(&mut self, run: ArmedAgentRun) -> AgentRunResult {
        if let Err(error) = self.validate_armed_run(&run) {
            return self.preflight_failure(error);
        }
        self.run_with_token(run.cancel).await
    }

    /// Cancel the current operation.
    ///
    /// Equivalent to the first Ctrl+C. The running `prompt` or `continue_`
    /// call returns an [`AgentRunResult`] whose error is
    /// `Some(AgentError::Cancelled)`.
    pub fn abort(&self) {
        self.cancel.cancel();
    }

    /// Return the active model id (the canonical selection's model half).
    pub fn model(&self) -> &str {
        &self.state.model_selection.model_id
    }

    /// Return the active provider id (the canonical selection's provider half).
    pub fn provider_id(&self) -> &str {
        &self.state.model_selection.provider_id
    }

    /// Return the canonical `provider:model` spec for the active selection.
    ///
    /// This is the full canonical form (both halves), for callers that need to
    /// persist or report the durable model identity rather than the model half
    /// alone. Distinct from [`Agent::model`](Self::model), which returns only
    /// the model half.
    pub fn model_spec(&self) -> String {
        self.state.model_selection.to_spec()
    }

    /// Return the thinking configuration used by subsequent provider requests.
    pub fn thinking_config(&self) -> ThinkingConfig {
        self.state.inference.thinking.clone()
    }

    /// Emit an `AgentEvent` to all subscribers outside of the agent loop.
    ///
    /// Used by callers (e.g. harness) to surface lifecycle events that occur
    /// between loop invocations, such as compaction start/end.
    pub fn emit_event(&self, event: AgentEvent) {
        self.event_fanout.emit(event);
    }

    /// Snapshot the current conversation context.
    ///
    /// The harness uses this after a turn (and any subsequent compaction) to
    /// compute the next `turn_offset` and return the post-compaction message
    /// list to callers.
    pub fn messages_snapshot(&self) -> Vec<AgentMessage> {
        self.state.context.clone()
    }

    /// Snapshot the complete durable state (context, model selection, inference).
    ///
    /// The harness builds a modified candidate from this snapshot and applies it
    /// through [`Agent::replace_state`] — the one validated idle-state
    /// replacement operation — for model changes, inference changes, failed-turn
    /// rewind, and compaction.
    pub fn state_snapshot(&self) -> NextTurnState {
        self.state.clone()
    }

    /// Snapshot the exact provider-visible tool definitions projected from the
    /// immutable trusted registry, in registration order.
    pub fn tool_definitions_snapshot(&self) -> Vec<opi_ai::message::ToolDef> {
        self.registry.definitions()
    }

    /// Register an event subscriber that receives all `AgentEvent`s.
    pub fn subscribe(&mut self, callback: EventSubscriber) {
        self.event_fanout.subscribe(callback);
    }

    /// Return a clonable cancellation token for external cancellation.
    ///
    /// Cancelling this token cancels the currently running loop operation.
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel.clone()
    }

    /// Return a clonable handle for cancellation and message queues.
    pub fn control_handle(&self) -> AgentControl {
        AgentControl {
            cancel: self.cancel.clone(),
            steering_queue: self.steering_queue.clone(),
            follow_up_queue: self.follow_up_queue.clone(),
        }
    }

    /// Arm one run-specific cancellation generation.
    ///
    /// Callers that await preflight before invoking the Agent loop must retain
    /// this value and pass it to the matching `*_armed` operation.
    #[doc(hidden)]
    pub fn arm_run(&mut self) -> ArmedAgentRun {
        self.maybe_reset_cancel();
        let generation = Arc::new(ArmedRunGeneration);
        self.latest_armed_generation = Some(generation.clone());
        ArmedAgentRun {
            owner: self.armed_run_owner.clone(),
            generation,
            cancel: self.cancel.child_token(),
        }
    }

    /// Return a control handle targeting an already armed run generation.
    #[doc(hidden)]
    pub fn control_handle_for_run(&self, run: &ArmedAgentRun) -> Result<AgentControl, AgentError> {
        self.validate_armed_run(run)?;
        Ok(AgentControl {
            cancel: run.cancel_token(),
            steering_queue: self.steering_queue.clone(),
            follow_up_queue: self.follow_up_queue.clone(),
        })
    }

    /// Add a steering message to be delivered before the next provider request.
    ///
    /// Steering messages are high-priority and delivered after the current
    /// turn's tool calls complete but before the next provider request.
    pub fn steer(&self, message: String) {
        self.steering_queue.lock().unwrap().push_back(message);
    }

    /// Add a follow-up message to be delivered when the agent would otherwise stop.
    ///
    /// Follow-up messages are only delivered when the agent has no tool calls
    /// pending and no steering messages queued.
    pub fn follow_up(&self, message: String) {
        self.follow_up_queue.lock().unwrap().push_back(message);
    }

    // -- Internal helpers ---------------------------------------------------

    fn push_user_text(&mut self, text: String) {
        self.state
            .context
            .push(AgentMessage::Llm(Message::User(UserMessage {
                content: vec![InputContent::Text { text }],
                timestamp_ms: opi_ai::time::now_ms(),
            })));
    }

    fn validate_armed_run(&self, run: &ArmedAgentRun) -> Result<(), AgentError> {
        let is_owner = Arc::ptr_eq(&self.armed_run_owner, &run.owner);
        let is_latest = self
            .latest_armed_generation
            .as_ref()
            .is_some_and(|generation| Arc::ptr_eq(generation, &run.generation));
        if !is_owner || !is_latest {
            return Err(AgentError::InvalidArmedRun);
        }
        Ok(())
    }

    fn maybe_reset_cancel(&mut self) {
        if self.cancel.is_cancelled() {
            self.cancel = CancellationToken::new();
        }
    }

    /// Reset cancellation state before handing out a control handle for a new turn.
    pub fn reset_cancel_if_cancelled(&mut self) {
        self.maybe_reset_cancel();
    }

    fn build_event_sink(&self) -> AgentEventSink {
        let event_fanout = self.event_fanout.clone();
        Box::new(move |event: AgentEvent| event_fanout.emit(event))
    }

    async fn run_with_token(&mut self, cancel: CancellationToken) -> AgentRunResult {
        let context = AgentLoopContext {
            collection: self.collection.clone(),
            registry: self.registry.clone(),
            authorizer: self.authorizer.clone(),
            evidence_health: crate::evidence::EvidenceHealth::healthy(),
            evidence_sink: self.evidence_sink.clone(),
            state: self.state.clone(),
            system: self.system.clone(),
            steering_queue: Some(self.steering_queue.clone()),
            follow_up_queue: Some(self.follow_up_queue.clone()),
            diagnostic_sink: self.diagnostic_sink.clone(),
            session_id: self.session_id.clone(),
        };

        let sink = self.build_event_sink();
        let run = crate::agent_loop::agent_loop_with_identities(
            context,
            self.config.clone(),
            &*self.hooks,
            sink,
            cancel,
        )
        .await;

        // Preserve the existing rollback contract for ordinary failures and
        // cancellation. A partial side effect or cleanup uncertainty is
        // different: its terminal tool result is actual lower-boundary truth
        // and must remain in the durable state instead of being discarded.
        let messages = run.state.context.clone();
        if run.error.is_none()
            || matches!(
                run.terminal_outcome,
                crate::evidence::TerminalOutcome::PartialSideEffect
                    | crate::evidence::TerminalOutcome::CleanupUnknown
            )
        {
            self.state = run.state;
        }
        AgentRunResult {
            messages,
            error: run.error,
            terminal_outcome: run.terminal_outcome,
            evidence_health: run.evidence_health,
            evidence_cleanup_error: None,
            identities: run.identities,
            evidence_sink: self.evidence_sink.clone(),
            lifecycle: AgentRunLifecycle::Active,
        }
    }

    fn preflight_failure(&self, error: AgentError) -> AgentRunResult {
        AgentRunResult {
            messages: self.state.context.clone(),
            terminal_outcome: crate::agent_loop::terminal_outcome_for_error(&error),
            error: Some(error),
            evidence_health: crate::evidence::EvidenceHealth::healthy(),
            evidence_cleanup_error: None,
            identities: crate::evidence::IdentityAllocator::new(),
            evidence_sink: self.evidence_sink.clone(),
            lifecycle: AgentRunLifecycle::Active,
        }
    }
}
