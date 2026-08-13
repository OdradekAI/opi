//! Bridge between adapter process host and the opi-agent extension system.
//!
//! [`ProcessAdapter`] wraps an [`AdapterHost`] and
//! implements the [`Extension`] trait,
//! exposing adapter capabilities (tools, commands, hooks, events, state,
//! model overrides) through the standard extension contracts.
//!
//! [`ProcessAdapterTool`] wraps an individual tool capability advertised by the
//! adapter and implements the [`Tool`] trait, delegating
//! execution to the adapter host with cancellation bridging.
//!
//! # Bridge Mappings
//!
//! | Adapter Protocol | Extension System |
//! |-----------------|-----------------|
//! | `AdapterToolCapability` | `Extension::tools()` → `Tool::definition()` |
//! | `AdapterCommandCapability` | `Extension::on_command()` |
//! | `AdapterModelOverride` | `Extension::model_overrides()` |
//! | `Hook "before_tool_call"` | `Extension::on_before_tool_call()` |
//! | `Hook "after_tool_call"` | `Extension::on_after_tool_call()` |
//! | `Hook "transform_context"` | `Extension::transform_context()` |
//! | `Hook "prepare_next_turn"` | `Extension::prepare_next_turn()` |
//! | Hook `"event"` | `Extension::on_event()` |
//! | `StateSerialize`/`StateRestore` | `Extension::serialize_state_async()`/`restore_state_async()` |
//! | Tool call cancellation | `Tool::execute()` → `tokio::select!` with cancel signal |
//!
//! # Hook Filtering
//!
//! Hooks are only dispatched to adapters that declared them in their
//! `capabilities.hooks` list. If an adapter does not declare
//! `"before_tool_call"`, its `on_before_tool_call` implementation returns
//! `Continue` without sending a message to the adapter process.
//!
//! # Failure Semantics
//!
//! - Tool call timeout → `ToolError::ExecutionFailed`
//! - Hook timeout on `before_tool_call` → `ExtensionHookResult::Block`
//!   (fail closed)
//! - Hook timeout on `after_tool_call` → continue (fail open)
//! - Event delivery under backpressure → dropped and recorded as an adapter diagnostic
//! - State serialization failure → `ExtensionError::StateSerialization`
//!
//! # Unstable
//!
//! This module is part of the **unstable 0.x extension API**. Breaking changes
//! may occur between minor versions without a major version bump.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use opi_agent::Diagnostic;
use opi_agent::event::AgentEvent;
use opi_agent::extension::{
    Extension, ExtensionCommand, ExtensionError, ExtensionHookResult, ExtensionRegistry,
};
use opi_agent::hooks::PrepareNextTurnContext;
use opi_agent::loop_types::{AgentError, NextTurnState};
use opi_agent::message::AgentMessage;
use opi_agent::tool::{ExecutionMode, Tool, ToolError, ToolResult};
use opi_ai::WireApi;
use opi_ai::message::{OutputContent, ToolDef};
use opi_ai::provider::ModelInfo;
use opi_ai::registry::ModelCapabilities;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::adapter_host::{AdapterCapabilities, AdapterHost, AdapterProcessConfig};
use crate::adapter_protocol::{AdapterHostMessage, AdapterProcessMessage};
use crate::diagnostic_bridge::{
    diagnostic_for_adapter_command_invalid, diagnostic_for_adapter_registration_failed,
    diagnostic_for_adapter_startup_failed, diagnostic_for_unsupported_adapter_kind,
    diagnostic_for_unsupported_adapter_protocol,
};

/// Default timeout for adapter tool calls and command dispatch.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

// ---------------------------------------------------------------------------
// ProcessAdapterTool
// ---------------------------------------------------------------------------

/// A tool backed by an adapter process.
///
/// Implements [`Tool`] by delegating `execute()` to the adapter host via the
/// JSONL protocol. Cancellation is bridged through `tokio::select!`.
pub struct ProcessAdapterTool {
    name: String,
    description: String,
    schema: Value,
    host: Arc<AdapterHost>,
}

impl std::fmt::Debug for ProcessAdapterTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProcessAdapterTool")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl ProcessAdapterTool {
    fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        schema: Value,
        host: Arc<AdapterHost>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            schema,
            host,
        }
    }
}

impl Tool for ProcessAdapterTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: self.name.clone(),
            description: self.description.clone(),
            input_schema: self.schema.clone(),
        }
    }

    fn execute(
        &self,
        call_id: &str,
        arguments: Value,
        signal: CancellationToken,
        _on_update: Option<opi_agent::tool::UpdateCallback>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ToolResult, ToolError>> + Send>>
    {
        let id = format!("{}_{}", call_id, self.host.next_id());
        let host = self.host.clone();
        let tool_name = self.name.clone();

        Box::pin(async move {
            let request = AdapterHostMessage::ToolCall {
                id: id.clone(),
                tool: tool_name,
                args: arguments,
            };

            let result = tokio::select! {
                result = host.send_request(request, REQUEST_TIMEOUT) => result,
                _ = signal.cancelled() => {
                    // Best-effort cancel
                    let _ = host.cancel(&id, "tool_cancelled").await;
                    return Err(ToolError::Cancelled);
                }
            };

            match result {
                Ok(msg) => match msg {
                    crate::adapter_protocol::AdapterProcessMessage::ToolResult {
                        content,
                        is_error,
                        ..
                    } => {
                        let output_content: Vec<OutputContent> = content
                            .into_iter()
                            .filter_map(|c| {
                                if c["type"].as_str() == Some("text") {
                                    Some(OutputContent::Text {
                                        text: c["text"].as_str().unwrap_or_default().to_string(),
                                    })
                                } else {
                                    None
                                }
                            })
                            .collect();

                        Ok(ToolResult {
                            content: output_content,
                            details: None,
                            is_error,
                            terminate: false,
                            truncated: false,
                            diagnostics: vec![],
                        })
                    }
                    crate::adapter_protocol::AdapterProcessMessage::Error { message, .. } => {
                        Err(ToolError::ExecutionFailed(message))
                    }
                    other => Err(ToolError::ExecutionFailed(format!(
                        "unexpected response type for tool call: {:?}",
                        other_type(&other)
                    ))),
                },
                Err(e) => Err(ToolError::ExecutionFailed(e.to_string())),
            }
        })
    }

    fn execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Sequential
    }
}

// ---------------------------------------------------------------------------
// ProcessAdapter
// ---------------------------------------------------------------------------

/// Bridge between an adapter process host and the opi-agent extension system.
///
/// Wraps an [`AdapterHost`] and exposes adapter capabilities (tools, commands,
/// hooks, events, state, model overrides) through the [`Extension`] trait.
///
/// This is the coding-agent product adapter boundary defined by Phase 10
/// Workstream 10.4: process adapter protocol parsing and hosting stay in
/// `opi-coding-agent`, and this bridge composes through the generic
/// `opi_agent::extension::ExtensionRegistry::wrap_hooks` composite (base hook
/// first, then extensions in registration order, `Block`/`Deny` short-circuit)
/// rather than bypassing it.
///
/// # Construction
///
/// Created via [`ProcessAdapter::from_host`] after the adapter host has
/// completed the initialize/capabilities handshake.
pub struct ProcessAdapter {
    name: String,
    host: Arc<AdapterHost>,
    tools_defs: Vec<crate::adapter_protocol::AdapterToolCapability>,
    commands: BTreeSet<String>,
    hooks: BTreeSet<String>,
    model_overrides: Vec<crate::adapter_protocol::AdapterModelOverride>,
}

impl std::fmt::Debug for ProcessAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProcessAdapter")
            .field("name", &self.name)
            .field("tools", &self.tools_defs.len())
            .field("commands", &self.commands.len())
            .field("hooks", &self.hooks)
            .finish_non_exhaustive()
    }
}

impl ProcessAdapter {
    /// Create a `ProcessAdapter` from a running adapter host and its
    /// capabilities.
    ///
    /// The host must have already completed the initialize/capabilities
    /// handshake successfully.
    pub fn from_host(
        name: impl Into<String>,
        host: Arc<AdapterHost>,
        capabilities: AdapterCapabilities,
    ) -> Self {
        let tools_defs = capabilities.tools.clone();
        let commands = capabilities
            .commands
            .iter()
            .map(|c| c.name.clone())
            .collect();
        let hooks = capabilities.hooks.iter().cloned().collect();
        let model_overrides = capabilities.model_overrides.clone();

        Self {
            name: name.into(),
            host,
            tools_defs,
            commands,
            hooks,
            model_overrides,
        }
    }
}

impl Extension for ProcessAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn tools(&self) -> Vec<Box<dyn Tool>> {
        self.tools_defs
            .iter()
            .map(|cap| {
                Box::new(ProcessAdapterTool::new(
                    &cap.name,
                    &cap.description,
                    cap.input_schema.clone(),
                    self.host.clone(),
                )) as Box<dyn Tool>
            })
            .collect()
    }

    /// Note: The current implementation maps `AdapterModelOverride.model` as
    /// both the provider_id and the model id. This is a simplification for the
    /// Phase 5 MVP where adapter model overrides are not yet exercised. A
    /// future iteration should resolve the provider from the model spec.
    fn model_overrides(&self) -> Vec<(String, ModelInfo)> {
        self.model_overrides
            .iter()
            .map(|mo| {
                (
                    mo.model.clone(),
                    ModelInfo::new(
                        &mo.model,
                        &mo.model,
                        WireApi::OpenAiCompletions,
                        ModelCapabilities::new(128_000, 16_384).with_streaming(true),
                    ),
                )
            })
            .collect()
    }

    fn on_before_tool_call(
        &self,
        tool_name: &str,
        args: &Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ExtensionHookResult> + Send>> {
        if !self.hooks.contains("before_tool_call") {
            return Box::pin(async { ExtensionHookResult::Continue });
        }

        let id = self.host.next_id();
        let host = self.host.clone();
        let tool = tool_name.to_string();
        let args = args.clone();

        Box::pin(async move {
            let request = AdapterHostMessage::Hook {
                id,
                hook: "before_tool_call".to_string(),
                payload: serde_json::json!({
                    "tool": tool,
                    "args": args,
                }),
            };

            // Fail closed: timeout → block
            match host.send_request(request, REQUEST_TIMEOUT).await {
                Ok(msg) => match msg {
                    crate::adapter_protocol::AdapterProcessMessage::HookResult {
                        action,
                        data,
                        ..
                    } => {
                        if action == "block" {
                            let reason = data
                                .and_then(|d| d["reason"].as_str().map(String::from))
                                .unwrap_or_else(|| "blocked by adapter".to_string());
                            ExtensionHookResult::Block { reason }
                        } else {
                            ExtensionHookResult::Continue
                        }
                    }
                    crate::adapter_protocol::AdapterProcessMessage::Error { message, .. } => {
                        // Adapter error during hook → fail closed
                        ExtensionHookResult::Block {
                            reason: format!("adapter hook error: {message}"),
                        }
                    }
                    other => ExtensionHookResult::Block {
                        reason: format!("unexpected hook response: {:?}", other_type(&other)),
                    },
                },
                Err(e) => {
                    // Timeout or unavailable → fail closed
                    ExtensionHookResult::Block {
                        reason: format!("adapter before_tool_call failed: {e}"),
                    }
                }
            }
        })
    }

    fn on_after_tool_call(
        &self,
        tool_name: &str,
        result: &ToolResult,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        if !self.hooks.contains("after_tool_call") {
            return Box::pin(async {});
        }

        let id = self.host.next_id();
        let host = self.host.clone();
        let tool = tool_name.to_string();
        let is_error = result.is_error;
        let content_summary: Vec<String> = result
            .content
            .iter()
            .map(|c| match c {
                opi_ai::message::OutputContent::Text { text } => text.clone(),
                _ => "[content]".to_string(),
            })
            .collect();

        Box::pin(async move {
            let request = AdapterHostMessage::Hook {
                id,
                hook: "after_tool_call".to_string(),
                payload: serde_json::json!({
                    "tool": tool,
                    "is_error": is_error,
                    "content": content_summary,
                }),
            };

            // Fail open: ignore timeout/errors for after hooks
            let _ = host.send_request(request, REQUEST_TIMEOUT).await;
        })
    }

    fn transform_context(
        &self,
        messages: Vec<AgentMessage>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<AgentMessage>, ExtensionError>> + Send>,
    > {
        if !self.hooks.contains("transform_context") {
            return Box::pin(async move { Ok(messages) });
        }

        let id = self.host.next_id();
        let host = self.host.clone();
        let adapter_name = self.name.clone();

        Box::pin(async move {
            let request = AdapterHostMessage::Hook {
                id,
                hook: "transform_context".to_string(),
                payload: serde_json::json!({
                    "messages": messages,
                }),
            };

            match host.send_request(request, REQUEST_TIMEOUT).await {
                Ok(AdapterProcessMessage::HookResult { data, .. }) => {
                    let Some(data) = data else {
                        return Err(ExtensionError::HookError {
                            name: adapter_name,
                            reason: "missing transform_context data".to_string(),
                        });
                    };
                    serde_json::from_value(data["messages"].clone()).map_err(|e| {
                        ExtensionError::HookError {
                            name: adapter_name,
                            reason: format!("invalid transform_context messages: {e}"),
                        }
                    })
                }
                Ok(AdapterProcessMessage::Error { message, .. }) => {
                    Err(ExtensionError::HookError {
                        name: adapter_name,
                        reason: message,
                    })
                }
                Ok(other) => Err(ExtensionError::HookError {
                    name: adapter_name,
                    reason: format!(
                        "unexpected transform_context response: {:?}",
                        other_type(&other)
                    ),
                }),
                Err(e) => Err(ExtensionError::HookError {
                    name: adapter_name,
                    reason: e.to_string(),
                }),
            }
        })
    }

    fn prepare_next_turn(
        &self,
        ctx: &PrepareNextTurnContext,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Option<NextTurnState>, AgentError>> + Send>,
    > {
        if !self.hooks.contains("prepare_next_turn") {
            return Box::pin(async { Ok(None) });
        }

        let id = self.host.next_id();
        let host = self.host.clone();
        let turn = ctx.turn;
        let state = ctx.state.clone();

        Box::pin(async move {
            let request = AdapterHostMessage::Hook {
                id,
                hook: "prepare_next_turn".to_string(),
                payload: serde_json::json!({
                    "turn": turn,
                    "messages": state.context,
                }),
            };

            // Phase 17.2: the adapter returns extra messages; fold them into a
            // complete candidate built from the current state (complete-state
            // replacement, not an append-only update).
            let extra_messages: Vec<AgentMessage> =
                match host.send_request(request, REQUEST_TIMEOUT).await {
                    Ok(AdapterProcessMessage::HookResult { data, .. }) => data
                        .and_then(|d| d.get("extra_messages").cloned())
                        .and_then(|value| serde_json::from_value(value).ok())
                        .unwrap_or_default(),
                    _ => return Ok(None),
                };
            if extra_messages.is_empty() {
                Ok(None)
            } else {
                let mut candidate = state;
                candidate.context.extend(extra_messages);
                Ok(Some(candidate))
            }
        })
    }

    fn on_event(&self, event: &AgentEvent) {
        if !self.hooks.contains("event") {
            return;
        }

        // Fire-and-forget: serialize and drop under backpressure
        let event_json = match serde_json::to_value(event) {
            Ok(v) => v,
            Err(_) => return,
        };

        // Since Extension::on_event is sync, try to spawn an async task
        // for fire-and-forget event delivery. If no runtime is available,
        // the event is dropped (consistent with backpressure semantics).
        let host = self.host.clone();
        let _ = tokio::runtime::Handle::try_current().map(|handle| {
            handle.spawn(async move {
                host.send_event(event_json).await;
            });
        });
    }

    fn on_command(
        &self,
        command: &ExtensionCommand,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Option<Value>, ExtensionError>> + Send>,
    > {
        if !self.commands.contains(&command.name) {
            return Box::pin(async { Ok(None) });
        }

        let id = self.host.next_id();
        let host = self.host.clone();
        let name = command.name.clone();
        let args = command.args.clone();

        Box::pin(async move {
            let request = AdapterHostMessage::Command { id, name, args };

            match host.send_request(request, REQUEST_TIMEOUT).await {
                Ok(msg) => match msg {
                    crate::adapter_protocol::AdapterProcessMessage::CommandResult {
                        data, ..
                    } => Ok(Some(data)),
                    crate::adapter_protocol::AdapterProcessMessage::Error { message, .. } => {
                        Err(ExtensionError::CommandError(message))
                    }
                    other => Err(ExtensionError::CommandError(format!(
                        "unexpected command response: {:?}",
                        other_type(&other)
                    ))),
                },
                Err(e) => Err(ExtensionError::CommandError(e.to_string())),
            }
        })
    }

    /// Serialize adapter state for session persistence.
    ///
    /// **Runtime requirement:** This synchronous bridge uses
    /// `tokio::task::block_in_place` internally and requires a multi-threaded
    /// Tokio runtime. Runtime code should prefer
    /// [`Extension::serialize_state_async`] so current-thread runtimes can
    /// await the adapter host directly.
    fn serialize_state(&self) -> Result<Option<Value>, ExtensionError> {
        // State serialization needs to be synchronous but the host is async.
        // Use tokio::runtime::Handle to block_on.
        let id = self.host.next_id();
        let host = self.host.clone();

        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                host.send_request(AdapterHostMessage::StateSerialize { id }, REQUEST_TIMEOUT)
                    .await
            })
        });

        match result {
            Ok(msg) => match msg {
                crate::adapter_protocol::AdapterProcessMessage::StateResult { state, .. } => {
                    Ok(Some(state))
                }
                crate::adapter_protocol::AdapterProcessMessage::Error { message, .. } => {
                    Err(ExtensionError::StateSerialization {
                        name: self.name.clone(),
                        reason: message,
                    })
                }
                other => Err(ExtensionError::StateSerialization {
                    name: self.name.clone(),
                    reason: format!("unexpected state response: {:?}", other_type(&other)),
                }),
            },
            Err(e) => Err(ExtensionError::StateSerialization {
                name: self.name.clone(),
                reason: e.to_string(),
            }),
        }
    }

    fn serialize_state_async(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Option<Value>, ExtensionError>> + Send + '_>,
    > {
        let id = self.host.next_id();
        let host = self.host.clone();
        let name = self.name.clone();
        Box::pin(async move {
            match host
                .send_request(AdapterHostMessage::StateSerialize { id }, REQUEST_TIMEOUT)
                .await
            {
                Ok(msg) => match msg {
                    crate::adapter_protocol::AdapterProcessMessage::StateResult {
                        state, ..
                    } => Ok(Some(state)),
                    crate::adapter_protocol::AdapterProcessMessage::Error { message, .. } => {
                        Err(ExtensionError::StateSerialization {
                            name,
                            reason: message,
                        })
                    }
                    other => Err(ExtensionError::StateSerialization {
                        name,
                        reason: format!("unexpected state response: {:?}", other_type(&other)),
                    }),
                },
                Err(e) => Err(ExtensionError::StateSerialization {
                    name,
                    reason: e.to_string(),
                }),
            }
        })
    }

    /// Restore adapter state from session persistence.
    ///
    /// **Runtime requirement:** Same as [`serialize_state`](Self::serialize_state) —
    /// requires a multi-threaded Tokio runtime. Runtime code should prefer
    /// [`Extension::restore_state_async`].
    fn restore_state(&self, state: Value) -> Result<(), ExtensionError> {
        let id = self.host.next_id();
        let host = self.host.clone();

        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                host.send_request(
                    AdapterHostMessage::StateRestore { id, state },
                    REQUEST_TIMEOUT,
                )
                .await
            })
        });

        match result {
            Ok(msg) => match msg {
                crate::adapter_protocol::AdapterProcessMessage::StateResult { .. } => Ok(()),
                crate::adapter_protocol::AdapterProcessMessage::Error { message, .. } => {
                    Err(ExtensionError::StateRestoration {
                        name: self.name.clone(),
                        reason: message,
                    })
                }
                other => Err(ExtensionError::StateRestoration {
                    name: self.name.clone(),
                    reason: format!("unexpected state response: {:?}", other_type(&other)),
                }),
            },
            Err(e) => Err(ExtensionError::StateRestoration {
                name: self.name.clone(),
                reason: e.to_string(),
            }),
        }
    }

    fn restore_state_async(
        &self,
        state: Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ExtensionError>> + Send + '_>>
    {
        let id = self.host.next_id();
        let host = self.host.clone();
        let name = self.name.clone();
        Box::pin(async move {
            match host
                .send_request(
                    AdapterHostMessage::StateRestore { id, state },
                    REQUEST_TIMEOUT,
                )
                .await
            {
                Ok(msg) => match msg {
                    crate::adapter_protocol::AdapterProcessMessage::StateResult { .. } => Ok(()),
                    crate::adapter_protocol::AdapterProcessMessage::Error { message, .. } => {
                        Err(ExtensionError::StateRestoration {
                            name,
                            reason: message,
                        })
                    }
                    other => Err(ExtensionError::StateRestoration {
                        name,
                        reason: format!("unexpected state response: {:?}", other_type(&other)),
                    }),
                },
                Err(e) => Err(ExtensionError::StateRestoration {
                    name,
                    reason: e.to_string(),
                }),
            }
        })
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Get the type name of an AdapterProcessMessage for error messages.
fn other_type(msg: &crate::adapter_protocol::AdapterProcessMessage) -> &'static str {
    match msg {
        crate::adapter_protocol::AdapterProcessMessage::Capabilities { .. } => "capabilities",
        crate::adapter_protocol::AdapterProcessMessage::ToolResult { .. } => "tool_result",
        crate::adapter_protocol::AdapterProcessMessage::CommandResult { .. } => "command_result",
        crate::adapter_protocol::AdapterProcessMessage::HookResult { .. } => "hook_result",
        crate::adapter_protocol::AdapterProcessMessage::StateResult { .. } => "state_result",
        crate::adapter_protocol::AdapterProcessMessage::Error { .. } => "error",
    }
}

// ---------------------------------------------------------------------------
// Adapter startup from packages
// ---------------------------------------------------------------------------

/// Default timeout for adapter startup handshake.
const DEFAULT_ADAPTER_TIMEOUT: Duration = Duration::from_secs(30);

/// Start adapter processes from discovered packages.
///
/// Takes packages that have `[adapter]` manifests, starts each adapter process
/// in deterministic order (ascending by layer precedence, then package name),
/// and registers successfully started adapters into the provided registry.
/// Returns the registry and any diagnostics from startup failures.
///
/// Packages without adapter manifests are silently skipped.
///
/// # Command Resolution
///
/// The adapter command from the manifest is resolved as follows:
/// - Absolute paths are used as-is.
/// - Relative paths (containing `/` or `\` or starting with `.`) are resolved
///   against the package root directory.
/// - Bare names are left for OS PATH lookup.
///
/// # Protocol Validation
///
/// Only `kind = "process-jsonl"` and `protocol = "opi-extension-jsonl-v1"` are
/// supported. Other values produce diagnostics and the package is skipped.
///
/// # Deterministic Order
///
/// Adapters are started in ascending order by `(layer_precedence, name)`. This
/// ensures reproducible tool/hook composition across sessions.
pub async fn start_adapters_from_packages(
    packages: &[crate::package_discovery::PackageResource],
    working_dir: &Path,
    mut registry: ExtensionRegistry,
) -> (ExtensionRegistry, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();

    // Filter and sort packages with adapter manifests
    let mut adapter_packages: Vec<&crate::package_discovery::PackageResource> = packages
        .iter()
        .filter(|p| p.manifest.adapter.is_some())
        .collect();
    adapter_packages.sort_by(|a, b| {
        a.layer_precedence
            .cmp(&b.layer_precedence)
            .then_with(|| a.manifest.name.cmp(&b.manifest.name))
    });

    for package in adapter_packages {
        let adapter = package.manifest.adapter.as_ref().expect("filtered above");

        // Validate protocol
        if adapter.protocol != "opi-extension-jsonl-v1" {
            diagnostics.push(diagnostic_for_unsupported_adapter_protocol(
                &package.manifest.name,
                &adapter.protocol,
                &adapter.command,
            ));
            continue;
        }

        // Validate kind
        if adapter.kind != "process-jsonl" {
            diagnostics.push(diagnostic_for_unsupported_adapter_kind(
                &package.manifest.name,
                &adapter.kind,
                &adapter.command,
            ));
            continue;
        }

        // Resolve command path
        let command =
            match crate::package_discovery::resolve_adapter_command_checked(adapter, &package.path)
            {
                Ok(command) => command,
                Err(e) => {
                    diagnostics.push(diagnostic_for_adapter_command_invalid(
                        &package.manifest.name,
                        &adapter.command,
                        e,
                    ));
                    continue;
                }
            };

        let config = AdapterProcessConfig {
            command,
            args: adapter.args.clone(),
            working_dir: working_dir.to_path_buf(),
            env: vec![],
        };

        let timeout = adapter
            .timeout_ms
            .map(Duration::from_millis)
            .unwrap_or(DEFAULT_ADAPTER_TIMEOUT);

        match AdapterHost::start(&package.manifest.name, config, timeout).await {
            Ok(host) => {
                let caps = host.capabilities().clone();
                for diagnostic in host.take_diagnostics() {
                    diagnostics.push(diagnostic);
                }
                let host = Arc::new(host);
                let process_adapter = ProcessAdapter::from_host(&package.manifest.name, host, caps);
                if let Err(e) = registry.register(Box::new(process_adapter)) {
                    diagnostics.push(diagnostic_for_adapter_registration_failed(
                        &package.manifest.name,
                        e,
                    ));
                }
            }
            Err(e) => {
                diagnostics.push(diagnostic_for_adapter_startup_failed(
                    &package.manifest.name,
                    &adapter.command,
                    e,
                ));
            }
        }
    }

    (registry, diagnostics)
}
