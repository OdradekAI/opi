//! Stateful Agent wrapper around the agent loop (S8.2).
//!
//! Provides `prompt`, `continue_`, `abort`, `subscribe`, `steer`, and
//! `follow_up` methods, managing conversation state, cancellation, event
//! subscribers, and message queues.

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{Provider, ThinkingConfig};
use tokio_util::sync::CancellationToken;

use crate::diagnostic_sink::DiagnosticSink;
use crate::event::{AgentEvent, AgentEventSink};
use crate::hooks::AgentHooks;
use crate::loop_types::{AgentError, AgentLoopConfig, AgentLoopContext};
use crate::message::AgentMessage;
use crate::tool::{ExecutionMode, Tool, ToolError, ToolResult};

// -- Arc wrappers for Provider and Tool reuse across calls ------------------

struct SharedProvider(Arc<dyn Provider>);

impl Provider for SharedProvider {
    fn id(&self) -> &str {
        self.0.id()
    }
    fn models(&self) -> &[opi_ai::provider::ModelInfo] {
        self.0.models()
    }
    fn stream(&self, request: opi_ai::provider::Request) -> opi_ai::provider::EventStream {
        self.0.stream(request)
    }
}

struct SharedTool(Arc<dyn Tool>);

impl Tool for SharedTool {
    fn definition(&self) -> opi_ai::message::ToolDef {
        self.0.definition()
    }

    fn execute(
        &self,
        call_id: &str,
        arguments: serde_json::Value,
        signal: CancellationToken,
        on_update: Option<crate::tool::UpdateCallback>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send>> {
        self.0.execute(call_id, arguments, signal, on_update)
    }

    fn execution_mode(&self) -> ExecutionMode {
        self.0.execution_mode()
    }
}

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
pub struct Agent {
    provider: Arc<dyn Provider>,
    tools: Vec<Arc<dyn Tool>>,
    model: String,
    system: Option<String>,
    session_id: Option<String>,
    config: AgentLoopConfig,
    hooks: Box<dyn AgentHooks>,
    cancel: CancellationToken,
    subscribers: Arc<Mutex<Vec<EventSubscriber>>>,
    messages: Vec<AgentMessage>,
    steering_queue: Arc<Mutex<VecDeque<String>>>,
    follow_up_queue: Arc<Mutex<VecDeque<String>>>,
    diagnostic_sink: Option<Arc<dyn DiagnosticSink>>,
    trace_collector: Option<Arc<crate::trace::TraceCollector>>,
}

impl Agent {
    /// Create a new Agent with the given provider, tools, model, and hooks.
    pub fn new(
        provider: Box<dyn Provider>,
        tools: Vec<Box<dyn Tool>>,
        model: String,
        system: Option<String>,
        config: AgentLoopConfig,
        hooks: Box<dyn AgentHooks>,
    ) -> Self {
        Self {
            provider: Arc::from(provider),
            tools: tools.into_iter().map(Arc::from).collect(),
            model,
            system,
            config,
            hooks,
            cancel: CancellationToken::new(),
            subscribers: Arc::new(Mutex::new(Vec::new())),
            messages: Vec::new(),
            steering_queue: Arc::new(Mutex::new(VecDeque::new())),
            follow_up_queue: Arc::new(Mutex::new(VecDeque::new())),
            diagnostic_sink: None,
            trace_collector: None,
            session_id: None,
        }
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
        self.messages
            .push(AgentMessage::Llm(Message::User(UserMessage {
                content: vec![InputContent::Text { text: text.into() }],
                timestamp_ms: opi_ai::time::now_ms(),
            })));
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
        self.messages
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

        if self.messages.is_empty() {
            return Err(AgentError::Hook("cannot continue: no messages".into()));
        }

        let token = self.cancel.child_token();
        self.messages
            .push(AgentMessage::Llm(Message::User(UserMessage {
                content: vec![InputContent::Text { text: text.into() }],
                timestamp_ms: opi_ai::time::now_ms(),
            })));
        self.run_with_token(token).await
    }

    /// Re-run the agent loop with the current messages, without pushing a new
    /// user message. Used by the harness to retry after a `CredentialNeeded`
    /// error is resolved via interactive login — the user message from the
    /// original `prompt`/`continue_` call is already in `self.messages`, so
    /// pushing another would duplicate it in the session.
    pub async fn retry_last_turn(
        &mut self,
    ) -> Result<Vec<AgentMessage>, AgentError> {
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

    /// Add an additional tool to the agent's tool set.
    pub fn add_tool(&mut self, tool: Box<dyn Tool>) {
        self.tools.push(Arc::from(tool));
    }

    /// Return the active model spec.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Change the model used by subsequent provider requests.
    pub fn set_model(&mut self, model: String) {
        self.model = model;
    }

    /// Return the underlying provider metadata.
    pub fn provider(&self) -> &dyn Provider {
        self.provider.as_ref()
    }

    /// Return the thinking configuration used by subsequent provider requests.
    pub fn thinking_config(&self) -> ThinkingConfig {
        self.config.thinking.clone().unwrap_or_default()
    }

    /// Change the thinking configuration used by subsequent provider requests.
    pub fn set_thinking_config(&mut self, thinking: Option<ThinkingConfig>) {
        self.config.thinking = thinking;
    }

    /// Change the maximum output tokens used by subsequent provider requests.
    pub fn set_max_tokens(&mut self, max_tokens: Option<u64>) {
        self.config.max_tokens = max_tokens;
    }

    /// Set the initial conversation messages (for session resume).
    ///
    /// Must be called before `prompt` or `continue_`. Replaces any
    /// existing messages in the agent's internal buffer.
    pub fn set_initial_messages(&mut self, messages: Vec<AgentMessage>) {
        self.messages = messages;
    }

    /// Inject a single message into the conversation buffer.
    ///
    /// Used after compaction to insert a `CompactionSummary` so subsequent
    /// provider calls include the summary in their context window.
    pub fn inject_message(&mut self, message: AgentMessage) {
        self.messages.push(message);
    }

    /// Replace the entire conversation buffer.
    ///
    /// Used after compaction to install `[summary, ...kept]` so subsequent
    /// provider requests no longer carry the compacted messages.
    pub fn replace_messages(&mut self, messages: Vec<AgentMessage>) {
        self.messages = messages;
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

    /// Snapshot the current conversation buffer.
    ///
    /// The harness uses this after a turn (and any subsequent compaction) to
    /// compute the next `turn_offset` and return the post-compaction message
    /// list to callers.
    pub fn messages_snapshot(&self) -> Vec<AgentMessage> {
        self.messages.clone()
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
            provider: Box::new(SharedProvider(self.provider.clone())),
            tools: self
                .tools
                .iter()
                .map(|t| Box::new(SharedTool(t.clone())) as Box<dyn Tool>)
                .collect(),
            messages: self.messages.clone(),
            model: self.model.clone(),
            system: self.system.clone(),
            steering_queue: Some(self.steering_queue.clone()),
            follow_up_queue: Some(self.follow_up_queue.clone()),
            diagnostic_sink: self.diagnostic_sink.clone(),
            trace: self.trace_collector.clone(),
            session_id: self.session_id.clone(),
        };

        let sink = self.build_event_sink();
        let result =
            crate::agent_loop(context, self.config.clone(), &*self.hooks, sink, cancel).await?;

        self.messages = result.clone();
        Ok(result)
    }
}
