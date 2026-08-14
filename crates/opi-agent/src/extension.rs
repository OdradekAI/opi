//! Extension system for agent customization.
//!
//! Provides the [`Extension`] trait for registering lifecycle hooks, custom
//! tools, custom commands, custom agent messages, and scoped extension state.
//!
//! # Lifecycle Ordering
//!
//! Extension hooks are called in registration order after the base hooks:
//!
//! 1. Base [`AgentHooks::before_tool_call`] then extension
//!    [`Extension::on_before_tool_call`] for each extension (in registration
//!    order).
//! 2. Base [`AgentHooks::after_tool_call`] then extension
//!    [`Extension::on_after_tool_call`] for each extension (in registration
//!    order).
//! 3. Base [`AgentHooks::prepare_next_turn`] then extension
//!    [`Extension::prepare_next_turn`] for each extension (in registration
//!    order). Extra messages are appended in that order.
//!
//! If any hook in the chain returns a deny/block result, the chain stops and
//! the denial propagates. Extensions cannot override a denial from the base
//! hooks or from an earlier extension.
//!
//! # Hook Error/Blocking Semantics
//!
//! - `on_before_tool_call` returning [`ExtensionHookResult::Block`] prevents
//!   the tool from executing. The block reason is returned to the agent loop
//!   as a tool error.
//! - `on_after_tool_call` is an observer callback; it cannot modify tool
//!   results. The base hook retains full control over result replacement via
//!   [`AfterToolCallResult::Replace`].
//! - `on_command` errors propagate to the caller via
//!   [`ExtensionError::CommandError`]. If an extension returns an error, the
//!   dispatch stops and the error is returned.
//!
//! # State Serialization
//!
//! Each extension can serialize and restore its own state. Extension states
//! are keyed by extension name in the serialized map produced by
//! [`ExtensionRegistry::serialize_states`]. Extensions that don't need state
//! persistence can use the default (no-op) implementations of
//! [`Extension::serialize_state`] and [`Extension::restore_state`].
//!
//! # Custom Tools
//!
//! Extensions provide tools via the [`Extension::tools`] method. Trusted
//! assembly collects them with registration-owned origin through
//! [`ExtensionRegistry::collect_tools_with_origin`]. Extension tools follow
//! the same [`Tool`] trait contract and validation rules as built-in tools.
//!
//! # Custom Commands
//!
//! Extensions handle custom commands via [`Extension::on_command`]. Commands
//! are dispatched via [`ExtensionRegistry::dispatch_command`] to extensions in
//! registration order. The first extension that returns `Ok(Some(value))`
//! claims the command.
//!
//! # Unstable
//!
//! This module is part of the **unstable 0.x extension API**. Breaking changes
//! may occur between minor versions without a major version bump.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde_json::Value;
use tokio_util::sync::CancellationToken;

use opi_ai::provider::{ModelInfo, Provider};

use crate::event::AgentEvent;
use crate::hooks::{
    AfterToolCallContext, AfterToolCallResult, AgentHooks, BeforeToolCallContext,
    BeforeToolCallResult, PrepareNextTurnContext, ShouldStopAfterTurnContext,
};
use crate::loop_types::{AgentError, NextTurnState};
use crate::message::AgentMessage;
use crate::tool::{Tool, ToolResult};

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors from extension operations.
#[derive(Debug, thiserror::Error)]
pub enum ExtensionError {
    /// An extension with the same name is already registered.
    #[error("duplicate extension name: {0}")]
    DuplicateName(String),
    /// The registry has already been shared with hooks or an event sink.
    #[error("cannot register extensions after registry has been shared")]
    RegistryLocked,
    /// Extension state serialization failed.
    #[error("state serialization failed for extension '{name}': {reason}")]
    StateSerialization { name: String, reason: String },
    /// Extension state restoration failed.
    #[error("state restoration failed for extension '{name}': {reason}")]
    StateRestoration { name: String, reason: String },
    /// An extension command returned an error.
    #[error("extension command error: {0}")]
    CommandError(String),
    /// An extension lifecycle hook returned an error.
    #[error("extension hook error in {name}: {reason}")]
    HookError { name: String, reason: String },
    /// A generic extension error.
    #[error("{0}")]
    Other(String),
}

// ---------------------------------------------------------------------------
// Hook result
// ---------------------------------------------------------------------------

/// Result of an extension lifecycle hook.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum ExtensionHookResult {
    /// Continue processing. The next hook in the chain (or the default
    /// behavior) is invoked.
    Continue,
    /// Block further processing. For `on_before_tool_call`, this denies the
    /// tool execution. The block reason is returned to the agent loop as a
    /// tool error.
    Block { reason: String },
}

// ---------------------------------------------------------------------------
// Extension command
// ---------------------------------------------------------------------------

/// A custom command dispatched to extensions.
#[derive(Debug, Clone)]
pub struct ExtensionCommand {
    /// The command name (e.g. `"todo/add"`).
    pub name: String,
    /// Optional ID for response correlation.
    pub id: Option<String>,
    /// Command arguments.
    pub args: Value,
}

impl ExtensionCommand {
    /// Create a new extension command.
    pub fn new(name: impl Into<String>, args: Value) -> Self {
        Self {
            name: name.into(),
            id: None,
            args,
        }
    }

    /// Add an ID for response correlation.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
}

// ---------------------------------------------------------------------------
// Extension trait
// ---------------------------------------------------------------------------

/// Extension trait for registering lifecycle hooks, custom tools,
/// custom commands, and scoped extension state.
///
/// See the [module-level documentation](self) for lifecycle ordering,
/// error/blocking semantics, and state serialization contracts.
///
/// # Unstable
///
/// This trait is part of the **unstable 0.x extension API**. Breaking changes
/// may occur between minor versions without a major version bump.
pub trait Extension: Send + Sync {
    /// Unique name for this extension. Must be non-empty and unique within
    /// the registry.
    fn name(&self) -> &str;

    /// Tools provided by this extension.
    ///
    /// Called once during [`ExtensionRegistry::collect_tools_with_origin`] to
    /// gather extension tools for trusted registration assembly.
    fn tools(&self) -> Vec<Box<dyn Tool>> {
        vec![]
    }

    /// Custom providers provided by this extension.
    ///
    /// Called during [`ExtensionRegistry::collect_providers`] to gather
    /// providers for registration with the provider registry. Extensions
    /// should return new provider instances on each call since `Box<dyn
    /// Provider>` is not `Clone`.
    ///
    /// Provider breadth should arrive through registration rather than core
    /// provider additions.
    fn providers(&self) -> Vec<Box<dyn Provider>> {
        vec![]
    }

    /// Additional models to register for existing providers.
    ///
    /// Called during [`ExtensionRegistry::collect_model_overrides`] to gather
    /// model metadata that supplements or overrides the models declared by
    /// built-in providers. Each entry is `(provider_id, ModelInfo)`.
    fn model_overrides(&self) -> Vec<(String, ModelInfo)> {
        vec![]
    }

    /// Called before a tool is executed (after the base hook, in registration
    /// order).
    ///
    /// Return [`ExtensionHookResult::Block`] to deny the tool execution.
    fn on_before_tool_call(
        &self,
        tool_name: &str,
        args: &Value,
    ) -> Pin<Box<dyn Future<Output = ExtensionHookResult> + Send>> {
        let _ = (tool_name, args);
        Box::pin(async { ExtensionHookResult::Continue })
    }

    /// Called after a tool has been executed (after the base hook, in
    /// registration order).
    ///
    /// This is an observer callback; it cannot modify the tool result.
    fn on_after_tool_call(
        &self,
        tool_name: &str,
        result: &ToolResult,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let _ = (tool_name, result);
        Box::pin(async {})
    }

    /// Transform the agent message buffer after the base hook and before LLM
    /// conversion.
    fn transform_context(
        &self,
        messages: Vec<AgentMessage>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<AgentMessage>, ExtensionError>> + Send>> {
        Box::pin(async move { Ok(messages) })
    }

    /// Prepare the complete next-turn state before it is atomically applied.
    ///
    /// Phase 17.2: preparation returns a complete replacement state (not a
    /// message append). The base hook is authoritative; an extension returning
    /// `Ok(Some(replacement))` replaces the current candidate (last-wins, in
    /// registration order); `Ok(None)` contributes nothing; `Err(_)` propagates
    /// and leaves the prior state intact.
    fn prepare_next_turn(
        &self,
        _ctx: &PrepareNextTurnContext,
    ) -> Pin<Box<dyn Future<Output = Result<Option<NextTurnState>, AgentError>> + Send>> {
        Box::pin(async { Ok(None) })
    }

    /// Called for every agent event.
    fn on_event(&self, _event: &AgentEvent) {}

    /// Handle a custom command.
    ///
    /// Return `Ok(Some(value))` if the command was handled, `Ok(None)` if
    /// the command is not recognized by this extension.
    fn on_command(
        &self,
        _command: &ExtensionCommand,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Value>, ExtensionError>> + Send>> {
        Box::pin(async { Ok(None) })
    }

    /// Serialize extension state for session persistence.
    ///
    /// Return `Ok(Some(value))` with the serialized state, or `Ok(None)` if
    /// the extension has no state to persist.
    fn serialize_state(&self) -> Result<Option<Value>, ExtensionError> {
        Ok(None)
    }

    /// Async variant of [`serialize_state`](Self::serialize_state).
    ///
    /// Process-backed extensions can override this to avoid blocking an async
    /// runtime while preserving the synchronous API for in-process extensions.
    fn serialize_state_async(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Value>, ExtensionError>> + Send + '_>> {
        Box::pin(async { self.serialize_state() })
    }

    /// Restore extension state from session persistence.
    fn restore_state(&self, _state: Value) -> Result<(), ExtensionError> {
        Ok(())
    }

    /// Async variant of [`restore_state`](Self::restore_state).
    fn restore_state_async(
        &self,
        state: Value,
    ) -> Pin<Box<dyn Future<Output = Result<(), ExtensionError>> + Send + '_>> {
        Box::pin(async move { self.restore_state(state) })
    }
}

// ---------------------------------------------------------------------------
// ExtensionRegistry
// ---------------------------------------------------------------------------

/// Registry that manages extensions and provides integration wrappers.
///
/// Extensions are registered before the agent loop starts. Once hooks or event
/// sinks are wrapped via [`wrap_hooks`](Self::wrap_hooks) or
/// [`wrap_event_sink`](Self::wrap_event_sink), the registry should not be
/// modified further.
pub struct ExtensionRegistry {
    extensions: Arc<Vec<Box<dyn Extension>>>,
}

/// One extension tool paired with the registration-owned extension identity
/// from the registry traversal that produced it. The tool definition cannot
/// replace this origin, even when its provider-visible name collides with a
/// built-in name.
pub struct CollectedExtensionTool {
    extension_id: String,
    tool: Box<dyn Tool>,
}

impl CollectedExtensionTool {
    /// The owning extension identity assigned by [`ExtensionRegistry`].
    pub fn extension_id(&self) -> &str {
        &self.extension_id
    }

    /// The contributed tool definition.
    pub fn definition(&self) -> opi_ai::message::ToolDef {
        self.tool.definition()
    }

    /// Consume the collected value into its trusted owner identity and tool.
    pub fn into_parts(self) -> (String, Box<dyn Tool>) {
        (self.extension_id, self.tool)
    }
}

impl Clone for ExtensionRegistry {
    fn clone(&self) -> Self {
        Self {
            extensions: self.extensions.clone(),
        }
    }
}

impl ExtensionRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            extensions: Arc::new(Vec::new()),
        }
    }

    /// Register an extension.
    ///
    /// Returns an error if an extension with the same name already exists.
    /// Returns [`ExtensionError::RegistryLocked`] if called after
    /// `wrap_hooks()` or `wrap_event_sink()` has been called (i.e., the
    /// extension list is shared).
    pub fn register(&mut self, ext: Box<dyn Extension>) -> Result<(), ExtensionError> {
        let name = ext.name().to_string();
        if self.extensions.iter().any(|e| e.name() == name) {
            return Err(ExtensionError::DuplicateName(name));
        }
        match Arc::get_mut(&mut self.extensions) {
            Some(exts) => {
                exts.push(ext);
            }
            None => {
                return Err(ExtensionError::RegistryLocked);
            }
        }
        Ok(())
    }

    /// Returns true if no extensions are registered.
    pub fn is_empty(&self) -> bool {
        self.extensions.is_empty()
    }

    /// Returns the number of registered extensions.
    pub fn len(&self) -> usize {
        self.extensions.len()
    }

    /// Return extension names in registration order.
    pub fn names(&self) -> Vec<&str> {
        self.extensions.iter().map(|e| e.name()).collect()
    }

    /// Look up an extension by name.
    pub fn get(&self, name: &str) -> Option<&dyn Extension> {
        self.extensions
            .iter()
            .find(|e| e.name() == name)
            .map(|e| e.as_ref())
    }

    /// Collect all tools from all registered extensions.
    pub fn collect_tools(&self) -> Vec<Box<dyn Tool>> {
        self.extensions.iter().flat_map(|e| e.tools()).collect()
    }

    /// Collect extension tools while preserving their registration-owned
    /// extension identity. Trusted tool assembly must use this form whenever
    /// origin or capability is security-relevant; inferring origin from a tool
    /// definition or provider-visible name is not permitted.
    pub fn collect_tools_with_origin(&self) -> Vec<CollectedExtensionTool> {
        self.extensions
            .iter()
            .flat_map(|extension| {
                let extension_id = extension.name().to_owned();
                extension
                    .tools()
                    .into_iter()
                    .map(move |tool| CollectedExtensionTool {
                        extension_id: extension_id.clone(),
                        tool,
                    })
            })
            .collect()
    }

    /// Collect all custom providers from all registered extensions.
    ///
    /// Each extension's [`providers`](Extension::providers) method is called
    /// and the results are concatenated. Extensions should return fresh
    /// provider instances since `Box<dyn Provider>` is not `Clone`.
    pub fn collect_providers(&self) -> Vec<Box<dyn Provider>> {
        self.extensions.iter().flat_map(|e| e.providers()).collect()
    }

    /// Collect all model overrides from all registered extensions.
    ///
    /// Each extension's [`model_overrides`](Extension::model_overrides) method
    /// is called and the results are concatenated.
    pub fn collect_model_overrides(&self) -> Vec<(String, ModelInfo)> {
        self.extensions
            .iter()
            .flat_map(|e| e.model_overrides())
            .collect()
    }

    /// Dispatch an event to all registered extensions.
    pub fn dispatch_event(&self, event: &AgentEvent) {
        for ext in self.extensions.iter() {
            ext.on_event(event);
        }
    }

    /// Dispatch a custom command to extensions in registration order.
    ///
    /// Returns the first `Some` response, or `None` if no extension handled
    /// the command.
    pub async fn dispatch_command(
        &self,
        command: &ExtensionCommand,
    ) -> Result<Option<Value>, ExtensionError> {
        for ext in self.extensions.iter() {
            if let Some(value) = ext.on_command(command).await? {
                return Ok(Some(value));
            }
        }
        Ok(None)
    }

    /// Serialize all extension states into a JSON object keyed by extension
    /// name.
    pub fn serialize_states(&self) -> Result<Value, ExtensionError> {
        let mut map = serde_json::Map::new();
        for ext in self.extensions.iter() {
            match ext.serialize_state() {
                Ok(Some(state)) => {
                    map.insert(ext.name().to_string(), state);
                }
                Ok(None) => {}
                Err(e) => return Err(e),
            }
        }
        Ok(Value::Object(map))
    }

    /// Async variant of [`serialize_states`](Self::serialize_states).
    pub async fn serialize_states_async(&self) -> Result<Value, ExtensionError> {
        let mut map = serde_json::Map::new();
        for ext in self.extensions.iter() {
            match ext.serialize_state_async().await {
                Ok(Some(state)) => {
                    map.insert(ext.name().to_string(), state);
                }
                Ok(None) => {}
                Err(e) => return Err(e),
            }
        }
        Ok(Value::Object(map))
    }

    /// Restore extension states from a JSON object keyed by extension name.
    pub fn restore_states(&self, states: Value) -> Result<(), ExtensionError> {
        let map = match states {
            Value::Object(m) => m,
            _ => return Ok(()),
        };
        for ext in self.extensions.iter() {
            if let Some(state) = map.get(ext.name()) {
                ext.restore_state(state.clone())?;
            }
        }
        Ok(())
    }

    /// Async variant of [`restore_states`](Self::restore_states).
    pub async fn restore_states_async(&self, states: Value) -> Result<(), ExtensionError> {
        let map = match states {
            Value::Object(m) => m,
            _ => return Ok(()),
        };
        for ext in self.extensions.iter() {
            if let Some(state) = map.get(ext.name()) {
                ext.restore_state_async(state.clone()).await?;
            }
        }
        Ok(())
    }

    /// Create a composite [`AgentHooks`] that wraps the base hooks with
    /// extension lifecycle callbacks.
    ///
    /// Extension hooks are called after the base hooks. If any extension
    /// returns [`ExtensionHookResult::Block`], the chain stops and the block
    /// propagates as a denial.
    pub fn wrap_hooks(&self, base: Box<dyn AgentHooks>) -> Box<dyn AgentHooks> {
        Box::new(CompositeHooks {
            base: Arc::from(base),
            extensions: self.extensions.clone(),
        })
    }

    /// Wrap an event sink to dispatch events to all registered extensions
    /// before forwarding to the base sink.
    pub fn wrap_event_sink(
        &self,
        base_sink: crate::event::AgentEventSink,
    ) -> crate::event::AgentEventSink {
        let extensions = self.extensions.clone();
        Box::new(move |event: AgentEvent| {
            for ext in extensions.iter() {
                ext.on_event(&event);
            }
            base_sink(event);
        })
    }
}

impl Default for ExtensionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// CompositeHooks
// ---------------------------------------------------------------------------

/// Internal type that chains extension hooks after base hooks.
struct CompositeHooks {
    base: Arc<dyn AgentHooks>,
    extensions: Arc<Vec<Box<dyn Extension>>>,
}

impl AgentHooks for CompositeHooks {
    fn convert_to_llm(
        &self,
        messages: &[AgentMessage],
    ) -> Result<Vec<opi_ai::message::Message>, AgentError> {
        self.base.convert_to_llm(messages)
    }

    fn transform_context(
        &self,
        messages: Vec<AgentMessage>,
        signal: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<AgentMessage>, AgentError>> + Send>> {
        let base = self.base.clone();
        let extensions = self.extensions.clone();
        Box::pin(async move {
            let mut messages = base.transform_context(messages, signal).await?;
            for ext in extensions.iter() {
                messages = ext
                    .transform_context(messages)
                    .await
                    .map_err(|e| AgentError::Hook(e.to_string()))?;
            }
            Ok(messages)
        })
    }

    fn should_stop_after_turn(
        &self,
        ctx: ShouldStopAfterTurnContext,
    ) -> Pin<Box<dyn Future<Output = bool> + Send>> {
        self.base.should_stop_after_turn(ctx)
    }

    fn before_tool_call(
        &self,
        ctx: BeforeToolCallContext,
    ) -> Pin<Box<dyn Future<Output = BeforeToolCallResult> + Send>> {
        let base = self.base.clone();
        let extensions = self.extensions.clone();
        let tool_name = ctx.tool_name.clone();
        let args = ctx.args.clone();
        Box::pin(async move {
            // Base hook decides first.
            match base.before_tool_call(ctx).await {
                BeforeToolCallResult::Continue => {}
                BeforeToolCallResult::Deny { reason } => {
                    return BeforeToolCallResult::Deny { reason };
                }
            }

            // Extension hooks in registration order.
            for ext in extensions.iter() {
                match ext.on_before_tool_call(&tool_name, &args).await {
                    ExtensionHookResult::Continue => {}
                    ExtensionHookResult::Block { reason } => {
                        return BeforeToolCallResult::Deny { reason };
                    }
                }
            }

            BeforeToolCallResult::Continue
        })
    }

    fn after_tool_call(
        &self,
        ctx: AfterToolCallContext,
    ) -> Pin<Box<dyn Future<Output = AfterToolCallResult> + Send>> {
        let base = self.base.clone();
        let extensions = self.extensions.clone();
        let tool_name = ctx.tool_name.clone();
        let result_snapshot = ctx.result.clone();
        Box::pin(async move {
            // Base hook decides first (may keep or replace).
            let base_result = base.after_tool_call(ctx).await;

            // Determine the effective result for extension observation.
            let effective: &ToolResult = match &base_result {
                AfterToolCallResult::Keep => &result_snapshot,
                AfterToolCallResult::Replace(r) => r,
            };

            // Notify extension observers (cannot modify result).
            for ext in extensions.iter() {
                ext.on_after_tool_call(&tool_name, effective).await;
            }

            base_result
        })
    }

    fn prepare_next_turn(
        &self,
        ctx: PrepareNextTurnContext,
    ) -> Pin<Box<dyn Future<Output = Result<Option<NextTurnState>, AgentError>> + Send>> {
        let base = self.base.clone();
        let extensions = self.extensions.clone();
        Box::pin(async move {
            // Base hook is authoritative. Phase 17.2 preparation is complete
            // state replacement (not a message append), so each extension that
            // returns Some replaces the current candidate (last-wins, in
            // registration order); an error propagates and preserves prior state.
            let mut candidate = base.prepare_next_turn(ctx.clone()).await?;
            for ext in extensions.iter() {
                if let Some(replacement) = ext.prepare_next_turn(&ctx).await? {
                    candidate = Some(replacement);
                }
            }
            Ok(candidate)
        })
    }
}
