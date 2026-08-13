//! General-purpose agent runtime with tool calling and session management.
//!
//! Provides the foundation for building specialized agents with pluggable
//! tool systems, hooks, queues, and session persistence.

pub mod agent;
pub mod authority;
pub mod compaction;
pub mod diagnostic;
pub mod diagnostic_sink;
pub mod event;
pub mod evidence;
pub mod extension;
pub mod harness;
pub mod hooks;
pub mod loop_types;
pub mod message;
pub mod sdk;
pub mod session;
pub mod session_branch;
pub mod session_context;
pub mod session_event;
pub mod state;
pub mod streaming_proxy;
pub mod tool;
pub mod validation;

mod agent_loop;

pub use agent::Agent;
pub use agent_loop::agent_loop;
pub use diagnostic::{Diagnostic, DiagnosticPayload, RedactionMode, Severity, redact, redact_text};
pub use diagnostic_sink::{DiagnosticSink, NullSink, RecordingSink};
pub use event::{AgentEvent, AgentEventSink};
pub use extension::{
    Extension, ExtensionCommand, ExtensionError, ExtensionHookResult, ExtensionRegistry,
};
pub use hooks::AgentHooks;
pub use loop_types::{AgentError, AgentLoopConfig, AgentLoopContext};
pub use message::AgentMessage;
pub use sdk::{SDK_SCHEMA_VERSION, SdkCommand, SdkResponse};
pub use session_event::AgentSessionEvent;
pub use state::AgentState;
pub use streaming_proxy::{
    ProxyConfig, ProxyEvent, ProxyHandler, SecretRedactor, StreamingProxy, StreamingProxyError,
};
pub use tool::{ExecutionMode, Tool, ToolError, ToolResult};

// Re-export provider-facing types needed at the agent boundary.
pub use opi_ai::message::ToolDef;
