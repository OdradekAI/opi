//! Types for the agent loop (S6.1, S8.2).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::diagnostic_sink::DiagnosticSink;
use crate::message::AgentMessage;
use crate::tool::Tool;
use crate::trace::TraceCollector;
use opi_ai::provider::{Provider, ThinkingConfig};

/// Errors that can occur during the agent loop.
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("provider error: {0}")]
    Provider(String),
    #[error("authentication failed: {0}")]
    AuthFailed(String),
    #[error("credential needed for '{provider_id}': run /login {provider_id}")]
    CredentialNeeded { provider_id: String },
    #[error("credential revoked for '{provider_id}': login required")]
    CredentialRevoked { provider_id: String },
    #[error("tool error: {0}")]
    Tool(String),
    #[error("hook error: {0}")]
    Hook(String),
    #[error("cancelled")]
    Cancelled,
    #[error("max turns exceeded ({0})")]
    MaxTurnsExceeded(u32),
    /// A requested trace could not be prepared before the run (fail-closed):
    /// the trace file could not be created/opened. The run is aborted so it
    /// never runs untraced when tracing was explicitly requested.
    #[error("trace setup failed: {0}")]
    TraceSetup(String),
}

/// Input context for the agent loop.
pub struct AgentLoopContext {
    /// The LLM provider.
    pub provider: Box<dyn Provider>,
    /// Available tools.
    pub tools: Vec<Box<dyn Tool>>,
    /// Initial conversation messages.
    pub messages: Vec<AgentMessage>,
    /// Model identifier to use.
    pub model: String,
    /// Optional system prompt.
    pub system: Option<String>,
    /// Steering queue (high-priority user messages injected before next turn).
    pub steering_queue: Option<Arc<Mutex<VecDeque<String>>>>,
    /// Follow-up queue (messages injected when agent would otherwise stop).
    pub follow_up_queue: Option<Arc<Mutex<VecDeque<String>>>>,
    /// Optional sink receiving diagnostics emitted from runtime failure paths
    /// (retry, cancellation, provider/tool failures). `None` disables emission
    /// without changing any other runtime behavior.
    pub diagnostic_sink: Option<Arc<dyn DiagnosticSink>>,
    /// Optional trace collector receiving versioned redacted trace records
    /// (run/turn/provider/tool/diagnostic-linked) during the run. `None`
    /// (the default) runs untraced with no behavior change. Tracing is opt-in
    /// and fail-open: a trace sink write failure never aborts the run.
    pub trace: Option<Arc<TraceCollector>>,
}

/// Configuration for the agent loop.
#[derive(Debug, Clone)]
pub struct AgentLoopConfig {
    /// Maximum number of turns before stopping.
    pub max_turns: u32,
    /// Maximum output tokens per request.
    pub max_tokens: Option<u64>,
    /// Sampling temperature.
    pub temperature: Option<f64>,
    /// Retry configuration for retryable provider errors.
    pub retry: Option<opi_ai::retry::RetryConfig>,
    /// Thinking/reasoning configuration for extended thinking models.
    pub thinking: Option<ThinkingConfig>,
}

impl Default for AgentLoopConfig {
    fn default() -> Self {
        Self {
            max_turns: 50,
            max_tokens: None,
            temperature: None,
            retry: None,
            thinking: None,
        }
    }
}

/// Update returned by `prepare_next_turn` to modify the next turn.
pub struct AgentLoopTurnUpdate {
    pub extra_messages: Vec<AgentMessage>,
}
