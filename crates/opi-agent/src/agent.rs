//! Stateful Agent wrapper around the agent loop (S8.2, Phase 17.2).
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
};
use crate::message::AgentMessage;

// -- Agent -------------------------------------------------------------------

type EventSubscriber = Box<dyn Fn(&AgentEvent) + Send + Sync>;

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

/// Stateful wrapper around `agent_loop` with conversation state, cancellation,
/// event subscription, and message queue management.
///
/// Phase 17.2: the Agent durably owns one complete [`NextTurnState`] and one
/// dispatchable [`opi_ai::ProviderCollection`]. Public piecemeal setters for model,
/// inference, or messages are replaced by [`Agent::replace_state`], the one
/// validated idle-state replacement operation.
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
    subscribers: Arc<Mutex<Vec<EventSubscriber>>>,
    steering_queue: Arc<Mutex<VecDeque<String>>>,
    follow_up_queue: Arc<Mutex<VecDeque<String>>>,
    diagnostic_sink: Option<Arc<dyn DiagnosticSink>>,
    trace_collector: Option<Arc<crate::trace::TraceCollector>>,
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
        let model_selection = ModelSelection::parse_spec(&model_spec).ok_or_else(|| {
            AgentError::InvalidNextTurnCandidate(format!(
                "model spec '{model_spec}' is not a canonical provider:model selection"
            ))
        })?;
        let registry = Arc::new(
            crate::authority::ToolRegistry::from_tools(registrations)
                .map_err(|e| AgentError::InvalidToolRegistration(e.to_string()))?,
        );
        Ok(Self {
            collection,
            registry,
            authorizer,
            state: NextTurnState::new(Vec::new(), model_selection, inference),
            system,
            config,
            hooks,
            cancel: CancellationToken::new(),
            subscribers: Arc::new(Mutex::new(Vec::new())),
            steering_queue: Arc::new(Mutex::new(VecDeque::new())),
            follow_up_queue: Arc::new(Mutex::new(VecDeque::new())),
            diagnostic_sink: None,
            trace_collector: None,
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

    /// Install a trace collector that receives versioned redacted trace
    /// records during the next `prompt`/`continue_` run. `None` (the default)
    /// runs untraced with no behavior change. The collector MUST be prepared
    /// by the caller before the run (fail-closed): the agent loop does not
    /// call `prepare`/`finish`, only emits records (fail-open).
    pub fn set_trace_collector(&mut self, collector: Option<Arc<crate::trace::TraceCollector>>) {
        self.trace_collector = collector;
    }

    /// Install an evidence sink that receives the run's call-graph lifecycle
    /// (stable run/turn/call identities, ordered records, terminal manifest)
    /// during the next `prompt`/`continue_`/`retry_last_turn` run (Phase 17.6).
    /// `None` (the default) is the capture-disabled no-op: no identities are
    /// minted and execution behavior is unchanged. Non-breaking: callers that
    /// do not set a sink are unaffected. The Reference Product remains on its
    /// existing `TraceSink` capture path; this substrate adds an independent,
    /// additive evidence channel.
    pub fn set_evidence_sink(&mut self, sink: Option<Arc<dyn crate::evidence::EvidenceSink>>) {
        self.evidence_sink = sink;
    }

    /// Atomically replace the complete durable state with a validated
    /// candidate, the one public idle-state replacement operation (Phase
    /// 17.2). The candidate's model selection must resolve to a route in the
    /// provider collection; otherwise the prior state is preserved and the
    /// error is returned. Must be called while the agent is idle.
    pub fn replace_state(&mut self, candidate: NextTurnState) -> Result<(), AgentError> {
        self.validate_state(&candidate)?;
        self.state = candidate;
        Ok(())
    }

    fn validate_state(&self, candidate: &NextTurnState) -> Result<(), AgentError> {
        if self
            .collection
            .resolve(&candidate.model_selection.to_spec())
            .is_err()
        {
            return Err(AgentError::InvalidNextTurnCandidate(format!(
                "model selection '{}' does not resolve to a route in the provider collection",
                candidate.model_selection.to_spec()
            )));
        }
        Ok(())
    }

    // -- Narrow convenience mutators (Phase 17.2) ---------------------------
    // Each delegates to [`Agent::replace_state`] so no mutation bypasses
    // candidate validation. They keep call sites concise; the one validated
    // idle-state replacement operation remains `replace_state`. A bare model is
    // normalized against the current provider id before validation.

    /// Change the model used by subsequent calls. `model` may be a canonical
    /// `provider:model` spec or a bare model id (normalized against the current
    /// provider). Delegates to [`Agent::replace_state`]; panics if the result is
    /// not a dispatchable route (callers validate first).
    pub fn set_model(&mut self, model: String) {
        let spec = if model.contains(':') {
            model
        } else {
            format!("{}:{model}", self.provider_id())
        };
        let selection = ModelSelection::parse_spec(&spec)
            .expect("normalized model spec must parse as provider:model");
        let mut candidate = self.state_snapshot();
        candidate.model_selection = selection;
        self.replace_state(candidate)
            .expect("model change must keep a dispatchable route");
    }

    /// Change the maximum output tokens. Delegates to [`Agent::replace_state`].
    pub fn set_max_tokens(&mut self, max_tokens: Option<u64>) {
        let mut candidate = self.state_snapshot();
        candidate.inference.max_tokens = max_tokens;
        let _ = self.replace_state(candidate);
    }

    /// Change the thinking configuration. Delegates to [`Agent::replace_state`].
    pub fn set_thinking_config(&mut self, thinking: Option<ThinkingConfig>) {
        let mut candidate = self.state_snapshot();
        candidate.inference.thinking = thinking.unwrap_or_default();
        let _ = self.replace_state(candidate);
    }

    /// Set the initial conversation context (for session resume). Delegates to
    /// [`Agent::replace_state`].
    pub fn set_initial_messages(&mut self, messages: Vec<AgentMessage>) {
        let mut candidate = self.state_snapshot();
        candidate.context = messages;
        let _ = self.replace_state(candidate);
    }

    /// Inject a single message into the conversation context. Delegates to
    /// [`Agent::replace_state`].
    pub fn inject_message(&mut self, message: AgentMessage) {
        let mut candidate = self.state_snapshot();
        candidate.context.push(message);
        let _ = self.replace_state(candidate);
    }

    /// Replace the entire conversation context. Delegates to
    /// [`Agent::replace_state`].
    pub fn replace_messages(&mut self, messages: Vec<AgentMessage>) {
        let mut candidate = self.state_snapshot();
        candidate.context = messages;
        let _ = self.replace_state(candidate);
    }

    /// Drop trailing context beyond `len`. Delegates to [`Agent::replace_state`].
    pub fn rewind_to(&mut self, len: usize) {
        let mut candidate = self.state_snapshot();
        candidate.context.truncate(len);
        let _ = self.replace_state(candidate);
    }

    /// Send a user message and run the agent loop.
    ///
    /// Resets the cancellation state if the agent was previously aborted,
    /// allowing a fresh conversation turn.
    pub async fn prompt(
        &mut self,
        text: impl Into<String>,
    ) -> Result<Vec<AgentMessage>, AgentError> {
        self.maybe_reset_cancel();
        let token = self.cancel.child_token();
        self.push_user_text(text.into());
        self.run_with_token(token).await
    }

    /// Send a user message with arbitrary content (text + images) and run the
    /// agent loop.
    pub async fn prompt_with_content(
        &mut self,
        content: Vec<InputContent>,
    ) -> Result<Vec<AgentMessage>, AgentError> {
        self.maybe_reset_cancel();
        let token = self.cancel.child_token();
        self.state
            .context
            .push(AgentMessage::Llm(Message::User(UserMessage {
                content,
                timestamp_ms: opi_ai::time::now_ms(),
            })));
        self.run_with_token(token).await
    }

    /// Continue the conversation with an additional user message.
    ///
    /// Requires the last context message to be a user message or tool result.
    pub async fn continue_(
        &mut self,
        text: impl Into<String>,
    ) -> Result<Vec<AgentMessage>, AgentError> {
        self.maybe_reset_cancel();

        if self.state.context.is_empty() {
            return Err(AgentError::Hook("cannot continue: no messages".into()));
        }

        let token = self.cancel.child_token();
        self.push_user_text(text.into());
        self.run_with_token(token).await
    }

    /// Re-run the agent loop with the current state, without pushing a new
    /// user message. Used by the harness to retry after a `CredentialNeeded`
    /// error is resolved via interactive login — the user message from the
    /// original `prompt`/`continue_` call is already in the context, so
    /// pushing another would duplicate it in the session.
    pub async fn retry_last_turn(&mut self) -> Result<Vec<AgentMessage>, AgentError> {
        self.maybe_reset_cancel();
        let token = self.cancel.child_token();
        self.run_with_token(token).await
    }

    /// Cancel the current operation.
    ///
    /// Equivalent to the first Ctrl+C. The running `prompt` or `continue_`
    /// call will return `AgentError::Cancelled`.
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
        let subs = self.subscribers.lock().unwrap();
        for sub in subs.iter() {
            sub(&event);
        }
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

    /// Register an event subscriber that receives all `AgentEvent`s.
    pub fn subscribe(&mut self, callback: EventSubscriber) {
        self.subscribers.lock().unwrap().push(callback);
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
        let subscribers = self.subscribers.clone();
        Box::new(move |event: AgentEvent| {
            let subs = subscribers.lock().unwrap();
            for sub in subs.iter() {
                sub(&event);
            }
        })
    }

    async fn run_with_token(
        &mut self,
        cancel: CancellationToken,
    ) -> Result<Vec<AgentMessage>, AgentError> {
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
            trace: self.trace_collector.clone(),
            session_id: self.session_id.clone(),
        };

        let sink = self.build_event_sink();
        let final_state =
            crate::agent_loop(context, self.config.clone(), &*self.hooks, sink, cancel).await?;

        // The Agent is the sole durable owner: persist the loop's final
        // complete state before the public operation settles.
        self.state = final_state;
        Ok(self.state.context.clone())
    }
}
