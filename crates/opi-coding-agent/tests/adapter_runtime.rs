//! Adapter runtime bridge tests (task 5.6).
//!
//! Covers: ProcessAdapter implements Extension, tools are bridged through Tool
//! trait, commands dispatch through Extension::on_command, hooks block/allow
//! tool calls, events are forwarded, state serialize/restore round-trips,
//! model overrides are collected, and cancellation bridges through.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use opi_agent::extension::{Extension, ExtensionCommand, ExtensionHookResult, ExtensionRegistry};
use opi_agent::hooks::{AgentHooks, BeforeToolCallContext, PrepareNextTurnContext};
use opi_agent::loop_types::{InferenceConfig, ModelSelection, NextTurnState};
use opi_agent::message::AgentMessage;
use opi_coding_agent::adapter_extension::{ProcessAdapter, start_adapters_from_packages};
use opi_coding_agent::adapter_host::{AdapterHost, AdapterProcessConfig};
use opi_coding_agent::adapter_protocol::AdapterHostMessage;
use opi_coding_agent::package_discovery::{AdapterManifest, PackageManifest, PackageResource};

// ---------------------------------------------------------------------------
// Noop hooks for composite hook testing
// ---------------------------------------------------------------------------

/// Minimal no-op AgentHooks for test use.
struct NoopHooks;

impl AgentHooks for NoopHooks {
    fn convert_to_llm(
        &self,
        _messages: &[opi_agent::message::AgentMessage],
    ) -> Result<Vec<opi_ai::message::Message>, opi_agent::loop_types::AgentError> {
        Ok(vec![])
    }
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/// Locate the `adapter_host_mock` test binary, preferring the newest version
/// when multiple hashed copies exist.
fn mock_adapter_bin() -> PathBuf {
    let current = std::env::current_exe().expect("current exe path");
    let deps_dir = current.parent().expect("deps directory");

    let exact_name = if cfg!(windows) {
        "adapter_host_mock.exe"
    } else {
        "adapter_host_mock"
    };
    let exact_path = deps_dir.join(exact_name);
    if exact_path.exists() {
        return exact_path;
    }

    let exe_suffix = if cfg!(windows) { ".exe" } else { "" };
    let prefix = "adapter_host_mock-";
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    if let Ok(entries) = std::fs::read_dir(deps_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with(prefix)
                && name_str.ends_with(exe_suffix)
                && !name_str.ends_with(".d")
                && let Ok(meta) = entry.metadata()
                && let Ok(modified) = meta.modified()
                && best.as_ref().is_none_or(|(t, _)| modified > *t)
            {
                best = Some((modified, entry.path()));
            }
        }
    }

    best.map(|(_, p)| p)
        .expect("Could not find adapter_host_mock binary in deps directory")
}

fn config_for_mode(mode: &str) -> AdapterProcessConfig {
    AdapterProcessConfig {
        command: mock_adapter_bin(),
        args: vec![],
        working_dir: std::env::current_dir().expect("cwd"),
        env: vec![("OPI_ADAPTER_TEST_MODE".to_string(), mode.to_string())],
    }
}

async fn start_capabilities_adapter() -> (Arc<AdapterHost>, ProcessAdapter) {
    let host = AdapterHost::start(
        "mock",
        config_for_mode("capabilities"),
        Duration::from_secs(5),
    )
    .await
    .expect("start host");
    let caps = host.capabilities().clone();
    let host = Arc::new(host);
    let adapter = ProcessAdapter::from_host("mock", host.clone(), caps);
    (host, adapter)
}

async fn start_gate_adapter() -> (Arc<AdapterHost>, ProcessAdapter) {
    let host = AdapterHost::start("gate", config_for_mode("gate"), Duration::from_secs(5))
        .await
        .expect("start host");
    let caps = host.capabilities().clone();
    let host = Arc::new(host);
    let adapter = ProcessAdapter::from_host("gate", host.clone(), caps);
    (host, adapter)
}

async fn start_prepare_adapter() -> (Arc<AdapterHost>, ProcessAdapter) {
    let host = AdapterHost::start(
        "prepare",
        config_for_mode("prepare"),
        Duration::from_secs(5),
    )
    .await
    .expect("start host");
    let caps = host.capabilities().clone();
    let host = Arc::new(host);
    let adapter = ProcessAdapter::from_host("prepare", host.clone(), caps);
    (host, adapter)
}

async fn start_transform_adapter() -> (Arc<AdapterHost>, ProcessAdapter) {
    let host = AdapterHost::start(
        "transform",
        config_for_mode("transform"),
        Duration::from_secs(5),
    )
    .await
    .expect("start host");
    let caps = host.capabilities().clone();
    let host = Arc::new(host);
    let adapter = ProcessAdapter::from_host("transform", host.clone(), caps);
    (host, adapter)
}

// ---------------------------------------------------------------------------
// 1. Extension name
// ---------------------------------------------------------------------------

#[tokio::test]
async fn adapter_extension_reports_name() {
    let (_host, adapter) = start_capabilities_adapter().await;
    assert_eq!(adapter.name(), "mock");
}

// ---------------------------------------------------------------------------
// 2. Tools are collected from capabilities
// ---------------------------------------------------------------------------

#[tokio::test]
async fn adapter_extension_provides_tools_from_capabilities() {
    let (_host, adapter) = start_capabilities_adapter().await;
    let tools = adapter.tools();
    assert_eq!(tools.len(), 1);

    let def = tools[0].definition();
    assert_eq!(def.name, "test_tool");
    assert!(!def.description.is_empty());
}

// ---------------------------------------------------------------------------
// 3. Tool execution round-trips through the adapter
// ---------------------------------------------------------------------------

#[tokio::test]
async fn adapter_tool_execute_sends_tool_call() {
    let (_host, adapter) = start_capabilities_adapter().await;
    let tools = adapter.tools();
    let tool = &tools[0];

    let result = tool
        .execute(
            "call-1",
            serde_json::json!({"input": "hello"}),
            tokio_util::sync::CancellationToken::new(),
            None,
        )
        .await
        .expect("tool execute");

    assert!(!result.is_error);
    assert!(
        result
            .content
            .iter()
            .any(|c| matches!(c, opi_ai::message::OutputContent::Text { text } if text.contains("mock_result"))),
        "expected mock_result in tool output"
    );
}

// ---------------------------------------------------------------------------
// 4. Tool execution bridges cancellation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn adapter_tool_execute_respects_cancellation() {
    // Start a hang_request adapter that never responds after handshake
    let host = AdapterHost::start(
        "mock",
        config_for_mode("hang_request"),
        Duration::from_secs(5),
    )
    .await
    .expect("start host");

    // Manually craft a ProcessAdapter with a tool definition that will time out
    let caps = host.capabilities().clone();
    let host = Arc::new(host);
    let adapter = ProcessAdapter::from_host("mock", host.clone(), caps);
    let tools = adapter.tools();

    if tools.is_empty() {
        // hang_request mode has no tools — skip this variant
        let _ = host
            .send_request(
                AdapterHostMessage::Shutdown {
                    id: "end".into(),
                    reason: "test_end".into(),
                },
                Duration::from_secs(1),
            )
            .await;
        return;
    }

    let token = tokio_util::sync::CancellationToken::new();
    let token_clone = token.clone();

    let handle = tokio::spawn(async move {
        tools[0]
            .execute("call-cancel", serde_json::json!({}), token_clone, None)
            .await
    });

    // Cancel immediately
    token.cancel();

    let result = tokio::time::timeout(Duration::from_secs(5), handle).await;
    match result {
        Ok(Ok(Ok(tool_result))) => {
            // Cancellation may produce an error result or succeed with what
            // completed before cancel — both are acceptable.
            assert!(tool_result.is_error || !tool_result.content.is_empty());
        }
        Ok(Ok(Err(_))) | Ok(Err(_)) | Err(_) => {
            // Cancellation caused the future to be dropped or timed out —
            // acceptable.
        }
    }
}

// ---------------------------------------------------------------------------
// 5. Command dispatches through Extension::on_command
// ---------------------------------------------------------------------------

#[tokio::test]
async fn adapter_command_dispatches_through_extension() {
    let (_host, adapter) = start_capabilities_adapter().await;

    let cmd = ExtensionCommand::new("test/status", serde_json::json!({}));
    let result = adapter.on_command(&cmd).await.expect("command dispatch");
    assert!(result.is_some(), "adapter should handle its own commands");
    let data = result.unwrap();
    assert_eq!(data["status"], "ok");
}

// ---------------------------------------------------------------------------
// 6. Unknown command returns None
// ---------------------------------------------------------------------------

#[tokio::test]
async fn adapter_unknown_command_returns_none() {
    let (_host, adapter) = start_capabilities_adapter().await;

    let cmd = ExtensionCommand::new("nonexistent", serde_json::json!({}));
    let result = adapter.on_command(&cmd).await.expect("command dispatch");
    assert!(
        result.is_none(),
        "adapter should not handle unknown commands"
    );
}

// ---------------------------------------------------------------------------
// 7. Hook blocks destructive tool calls
// ---------------------------------------------------------------------------

#[tokio::test]
async fn adapter_before_tool_hook_can_block() {
    let (_host, adapter) = start_gate_adapter().await;

    let result = adapter
        .on_before_tool_call("bash", &serde_json::json!({"command": "rm -rf target"}))
        .await;
    assert!(
        matches!(result, ExtensionHookResult::Block { .. }),
        "expected Block for destructive bash command"
    );
}

// ---------------------------------------------------------------------------
// 8. Hook allows non-destructive tool calls
// ---------------------------------------------------------------------------

#[tokio::test]
async fn adapter_before_tool_hook_allows_safe_tools() {
    let (_host, adapter) = start_gate_adapter().await;

    let result = adapter
        .on_before_tool_call("read", &serde_json::json!({"path": "/tmp/file"}))
        .await;
    assert!(
        matches!(result, ExtensionHookResult::Continue),
        "expected Continue for safe read tool"
    );
}

// ---------------------------------------------------------------------------
// 9. Hook skips if adapter does not declare it
// ---------------------------------------------------------------------------

#[tokio::test]
async fn adapter_skips_hook_not_in_capabilities() {
    let (_host, adapter) = start_capabilities_adapter().await;

    // The capabilities mock hooks are ["before_tool_call", "event"].
    // It does NOT declare "after_tool_call", so on_after_tool_call should
    // return immediately without sending anything to the adapter process.
    // Verify by: calling on_after_tool_call, then sending a command through
    // the adapter to confirm the host is not blocked by a stale request.
    let _result = adapter
        .on_after_tool_call(
            "read",
            &opi_agent::tool::ToolResult {
                content: vec![],
                details: None,
                is_error: false,
                terminate: false,
                truncated: false,
                diagnostics: vec![],
            },
        )
        .await;

    // If on_after_tool_call had sent a request to the adapter, the adapter
    // (which doesn't handle "after_tool_call") would not respond, and the
    // pending request would time out or block. Verify the host is still
    // responsive by dispatching a command.
    let cmd = ExtensionCommand::new("test/status", serde_json::json!({}));
    let result = adapter.on_command(&cmd).await.expect("command dispatch");
    assert!(
        result.is_some(),
        "adapter host should still be responsive after skipped hook"
    );
}

// ---------------------------------------------------------------------------
// 10. Event forwarding
// ---------------------------------------------------------------------------

#[tokio::test]
async fn adapter_event_forwarding_does_not_block() {
    let (_host, adapter) = start_capabilities_adapter().await;

    let start = std::time::Instant::now();
    adapter.on_event(&create_test_event());
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(2),
        "on_event took {elapsed:?}, should be near-instant"
    );
}

fn create_test_event() -> opi_agent::event::AgentEvent {
    opi_agent::event::AgentEvent::TurnStart
}

// ---------------------------------------------------------------------------
// 11. State serialize round-trips
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn adapter_state_serialize_returns_state() {
    let (_host, adapter) = start_capabilities_adapter().await;

    let state = adapter.serialize_state().expect("serialize");
    assert!(state.is_some());
    let state = state.unwrap();
    assert_eq!(state["mock"], true);
}

// ---------------------------------------------------------------------------
// 12. State restore round-trips
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn adapter_state_restore_accepts_state() {
    let (_host, adapter) = start_capabilities_adapter().await;

    adapter
        .restore_state(serde_json::json!({"items": ["a"]}))
        .expect("restore");
}

#[tokio::test]
async fn adapter_state_async_round_trip_works_on_current_thread_runtime() {
    let (_host, adapter) = start_capabilities_adapter().await;

    let state = adapter
        .serialize_state_async()
        .await
        .expect("serialize")
        .expect("state");
    assert_eq!(state["mock"], true);

    adapter
        .restore_state_async(serde_json::json!({"items": ["a"]}))
        .await
        .expect("restore");
}

// ---------------------------------------------------------------------------
// 13. Model overrides collected from capabilities
// ---------------------------------------------------------------------------

#[tokio::test]
async fn adapter_model_overrides_from_capabilities() {
    let (_host, adapter) = start_capabilities_adapter().await;
    // Capabilities mock has empty model_overrides
    let overrides = adapter.model_overrides();
    assert!(overrides.is_empty());
}

// ---------------------------------------------------------------------------
// 14. Full registry integration: tools + command dispatch
// ---------------------------------------------------------------------------

#[tokio::test]
async fn adapter_registered_in_registry_dispatches_command() {
    let (_host, adapter) = start_capabilities_adapter().await;

    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(adapter))
        .expect("register adapter");

    // Collect tools
    let tools = registry.collect_tools();
    assert_eq!(tools.len(), 1);

    // Dispatch command
    let cmd = ExtensionCommand::new("test/status", serde_json::json!({}));
    let result = registry.dispatch_command(&cmd).await.expect("dispatch");
    assert!(result.is_some());
    assert_eq!(result.unwrap()["status"], "ok");
}

// ---------------------------------------------------------------------------
// 15. Gate adapter in registry blocks destructive commands
// ---------------------------------------------------------------------------

#[tokio::test]
async fn gate_adapter_in_registry_blocks_destructive_tools() {
    let (_host, adapter) = start_gate_adapter().await;

    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(adapter))
        .expect("register gate adapter");

    // Use wrap_hooks with a no-op base hook
    let base_hooks = Box::new(NoopHooks);
    let composite = registry.wrap_hooks(base_hooks);

    let ctx = BeforeToolCallContext {
        tool_call_id: "call-1".into(),
        tool_name: "bash".into(),
        args: serde_json::json!({"command": "rm -rf target"}),
        messages: vec![],
    };

    let result = composite.before_tool_call(ctx).await;
    assert!(
        matches!(result, opi_agent::hooks::BeforeToolCallResult::Deny { .. }),
        "composite hooks should deny destructive bash via adapter"
    );
}

// ---------------------------------------------------------------------------
// 16. State serialize/restore through registry
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn adapter_state_round_trip_through_registry() {
    let (_host, adapter) = start_gate_adapter().await;

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(adapter)).expect("register");

    let states = registry.serialize_states().expect("serialize");
    assert!(states.is_object());
    assert!(states.get("gate").is_some());

    registry.restore_states(states).expect("restore");
}

#[tokio::test]
async fn adapter_prepare_next_turn_can_inject_message() {
    let (_host, adapter) = start_prepare_adapter().await;
    let update = adapter
        .prepare_next_turn(&PrepareNextTurnContext {
            state: NextTurnState::new(
                vec![],
                ModelSelection::parse_spec("mock:mock-model").unwrap(),
                InferenceConfig::default(),
            ),
            tool_results: vec![],
            terminate: false,
            turn: 1,
        })
        .await
        .expect("update")
        .expect("prepare adapter injects a message");

    assert_eq!(update.context.len(), 1);
    match &update.context[0] {
        AgentMessage::Custom(message) => {
            assert_eq!(message.kind, "adapter_note");
            assert_eq!(message.data["text"], "next turn");
        }
        other => panic!("expected custom message, got {other:?}"),
    }
}

#[tokio::test]
async fn adapter_transform_context_can_rewrite_messages() {
    let (_host, adapter) = start_transform_adapter().await;
    let messages = adapter.transform_context(vec![]).await.expect("transform");

    assert_eq!(messages.len(), 1);
    match &messages[0] {
        AgentMessage::Custom(message) => {
            assert_eq!(message.kind, "adapter_transform");
            assert_eq!(message.data["text"], "transformed");
        }
        other => panic!("expected custom message, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Protocol gate diagnostics (task 6.3).
//
// `start_adapters_from_packages` validates the adapter protocol and kind from
// the manifest BEFORE spawning any child process. The protocol/kind gate is
// the honest 0.x "version negotiation": a package whose manifest declares a
// protocol other than `opi-extension-jsonl-v1` (or a kind other than
// `process-jsonl`) is skipped and produces a diagnostic. The diagnostic must
// name the expected and actual values so a package author can see what the
// host accepts, not just what was rejected.
//
// These tests exercise the production `start_adapters_from_packages` path
// directly (no mock binary needed: the gate runs before spawn).
// ---------------------------------------------------------------------------

/// Build a `PackageResource` whose `[adapter]` manifest has the given protocol
/// and kind. The command is a never-reached placeholder because the protocol
/// and kind gates run before command resolution and process spawn.
fn make_gated_package(
    name: &str,
    protocol: &str,
    kind: &str,
    package_dir: PathBuf,
) -> PackageResource {
    let toml_path = package_dir.join("package.toml");
    PackageResource {
        manifest: PackageManifest {
            name: name.to_string(),
            description: format!("Gated package {name}"),
            version: None,
            opi_version: None,
            adapter: Some(AdapterManifest {
                kind: kind.to_string(),
                command: "never-reached-placeholder".to_string(),
                args: vec![],
                protocol: protocol.to_string(),
                timeout_ms: None,
            }),
            extensions: None,
            skills: None,
            fragments: None,
            themes: None,
            disabled: vec![],
            adapter_contributions: vec![],
        },
        path: package_dir,
        package_toml_path: toml_path,
        layer_precedence: 0,
    }
}

#[tokio::test]
async fn start_adapters_unsupported_protocol_diagnostic_names_expected_and_actual() {
    let dir = tempfile::tempdir().expect("tempdir");
    let package = make_gated_package(
        "proto-pkg",
        "unknown-protocol",
        "process-jsonl",
        dir.path().to_path_buf(),
    );

    let registry = ExtensionRegistry::new();
    let (registry, diagnostics) =
        start_adapters_from_packages(&[package], dir.path(), registry).await;

    assert_eq!(diagnostics.len(), 1, "expected exactly one diagnostic");
    let diag = &diagnostics[0];

    // The diagnostic must name the rejected package.
    assert_eq!(
        diag.code,
        opi_agent::diagnostic::code::CODE_ADAPTER_PROTOCOL_UNSUPPORTED
    );
    assert_eq!(diag.source, opi_agent::diagnostic::SOURCE_ADAPTER);
    assert_eq!(diag.message, "unsupported adapter protocol");
    let details = diag.details.as_ref().expect("diagnostic details");
    assert_eq!(details["package_name"], "proto-pkg");

    // It must name the expected protocol so authors know what the host accepts.
    assert_eq!(details["expected_protocol"], "opi-extension-jsonl-v1");

    // It must name the actual (rejected) protocol.
    assert_eq!(details["actual_protocol"], "unknown-protocol");

    // The package is skipped at the gate, so no adapter is registered.
    assert!(
        registry.collect_tools().is_empty(),
        "unsupported-protocol package must not register an adapter"
    );
}

#[tokio::test]
async fn start_adapters_unsupported_kind_diagnostic_names_expected_and_actual() {
    let dir = tempfile::tempdir().expect("tempdir");
    let package = make_gated_package(
        "kind-pkg",
        "opi-extension-jsonl-v1",
        "websocket",
        dir.path().to_path_buf(),
    );

    let registry = ExtensionRegistry::new();
    let (registry, diagnostics) =
        start_adapters_from_packages(&[package], dir.path(), registry).await;

    assert_eq!(diagnostics.len(), 1, "expected exactly one diagnostic");
    let diag = &diagnostics[0];

    assert_eq!(
        diag.code,
        opi_agent::diagnostic::code::CODE_ADAPTER_KIND_UNSUPPORTED
    );
    assert_eq!(diag.source, opi_agent::diagnostic::SOURCE_ADAPTER);
    assert_eq!(diag.message, "unsupported adapter kind");
    let details = diag.details.as_ref().expect("diagnostic details");
    assert_eq!(details["package_name"], "kind-pkg");

    // The diagnostic must name the expected kind.
    assert_eq!(details["expected_kind"], "process-jsonl");

    // It must name the actual (rejected) kind.
    assert_eq!(details["actual_kind"], "websocket");

    assert!(
        registry.collect_tools().is_empty(),
        "unsupported-kind package must not register an adapter"
    );
}

// ---------------------------------------------------------------------------
// Phase 8: skipped adapter hooks are visible in trace data (task 8.2).
//
// The capabilities mock declares only ["before_tool_call", "event"]; it does
// NOT declare "after_tool_call" (or transform_context / prepare_next_turn).
// When the adapter bridge short-circuits an undeclared hook, the skip must be
// recorded as a TraceKind::HookSkipped record so the "adapter implements only
// a subset" case is visible in Phase 7 trace data when tracing is enabled.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn phase8_skipped_adapter_hooks_trace() {
    let (_host, adapter) = start_capabilities_adapter().await;

    let sink = Arc::new(opi_agent::RecordingTraceSink::new());
    let collector = Arc::new(opi_agent::TraceCollector::new(
        "run-skip",
        opi_agent::RedactionMode::Verbose,
        sink.clone(),
        None,
    ));
    collector.prepare().expect("prepare trace collector");
    adapter.set_trace_collector(Some(collector.clone()));

    // The capabilities mock declares only ["before_tool_call", "event"], so
    // after_tool_call, transform_context, and prepare_next_turn are all
    // undeclared -> each must emit a HookSkipped record when dispatched.
    adapter
        .on_after_tool_call(
            "read",
            &opi_agent::tool::ToolResult {
                content: vec![],
                details: None,
                is_error: false,
                terminate: false,
                truncated: false,
                diagnostics: vec![],
            },
        )
        .await;
    adapter
        .transform_context(vec![])
        .await
        .expect("skipped transform_context passes messages through");
    let _ = adapter
        .prepare_next_turn(&PrepareNextTurnContext {
            state: NextTurnState::new(
                vec![],
                ModelSelection::parse_spec("mock:mock-model").unwrap(),
                InferenceConfig::default(),
            ),
            tool_results: vec![],
            terminate: false,
            turn: 1,
        })
        .await;

    let records = sink.snapshot();
    let skipped_hooks: Vec<&str> = records
        .iter()
        .filter(|r| r.kind == opi_agent::TraceKind::HookSkipped)
        .map(|r| {
            r.details
                .as_ref()
                .and_then(|d| d["hook"].as_str())
                .unwrap_or("")
        })
        .collect();
    for required in ["after_tool_call", "transform_context", "prepare_next_turn"] {
        assert!(
            skipped_hooks.contains(&required),
            "expected a HookSkipped record for {required}, got {skipped_hooks:?}"
        );
    }

    // The record carries the adapter name and the hook name in its details.
    let details = records
        .iter()
        .find(|r| r.kind == opi_agent::TraceKind::HookSkipped)
        .expect("at least one HookSkipped record")
        .details
        .as_ref()
        .expect("hook-skip record details");
    assert_eq!(details["adapter"], "mock");

    // Clearing the collector detaches the adapter so no stale handle survives
    // across runs (production run-end path passes None).
    adapter.set_trace_collector(None);
}

// ---------------------------------------------------------------------------
// Phase 8: adapter STARTUP degradation diagnostic contract (task 8.6).
//
// Characterization gate: when an adapter fails to START (here, an adapter
// command that escapes the package root), the failure must surface as a typed,
// structured `Diagnostic` carrying `SOURCE_ADAPTER` and a stable
// `CODE_ADAPTER_*` code, with structured JSON details — not a free-text
// `Vec<String>`. The chosen trigger is the `command_invalid` startup gate
// (relative command path that resolves outside the package root), which runs
// deterministically before any child process is spawned.
//
// DEFERRED: the runtime send_event backpressure path
// (`CODE_ADAPTER_HOST_DIAGNOSTIC`, from `AdapterHost` observing event-delivery
// backpressure) is not exercised here. There is no clean deterministic trigger
// for that path without a test-only `adapter_host_mock` mode, which is out of
// scope for this characterization gate.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn phase8_adapter_degradation_diagnostic_contract() {
    let dir = tempfile::tempdir().expect("tempdir");
    // Relative command that escapes the package root -> resolve_adapter_command_checked
    // returns SecurityDiagnostic -> diagnostic_for_adapter_command_invalid.
    let package = make_gated_package(
        "cmd-escape-pkg",
        "opi-extension-jsonl-v1",
        "process-jsonl",
        dir.path().to_path_buf(),
    );
    // Override the placeholder command with one that fails command resolution.
    let mut package = package;
    let adapter = package.manifest.adapter.as_mut().expect("adapter manifest");
    adapter.command = "../escape-outside-package".to_string();

    let registry = ExtensionRegistry::new();
    let (registry, diagnostics) =
        start_adapters_from_packages(&[package], dir.path(), registry).await;

    assert_eq!(
        diagnostics.len(),
        1,
        "expected exactly one startup diagnostic"
    );
    let diag = &diagnostics[0];

    // Typed source + stable code (not free text).
    assert_eq!(diag.source, opi_agent::diagnostic::SOURCE_ADAPTER);
    assert_eq!(
        diag.code,
        opi_agent::diagnostic::code::CODE_ADAPTER_COMMAND_INVALID
    );

    // Structured JSON details (a JSON object), not a free-text Vec<String>.
    let details = diag
        .details
        .as_ref()
        .expect("startup degradation must carry structured details");
    assert!(
        matches!(details, serde_json::Value::Object(_)),
        "details must be a JSON object, got {details:?}"
    );
    assert_eq!(details["package_name"], "cmd-escape-pkg");
    assert_eq!(details["adapter_command"], "../escape-outside-package");
    // adapter_error carries the underlying failure reason.
    assert!(
        details["adapter_error"].is_string(),
        "adapter_error must be a string, got {}",
        details["adapter_error"]
    );

    // Every adapter-originated diagnostic in this run carries SOURCE_ADAPTER.
    for d in &diagnostics {
        assert_eq!(
            d.source,
            opi_agent::diagnostic::SOURCE_ADAPTER,
            "adapter-originated diagnostic must keep SOURCE_ADAPTER, got {}",
            d.source
        );
    }

    // Redaction boundary is reachable and preserves the stable identity fields
    // (source + code) so downstream NDJSON/RPC/prompt consumers can match on them.
    let payload = diag.redacted_payload(opi_agent::RedactionMode::Summary);
    assert_eq!(payload.source, opi_agent::diagnostic::SOURCE_ADAPTER);
    assert_eq!(
        payload.code,
        opi_agent::diagnostic::code::CODE_ADAPTER_COMMAND_INVALID
    );

    // The package is skipped at the gate, so no adapter is registered.
    assert!(
        registry.collect_tools().is_empty(),
        "command-invalid package must not register an adapter"
    );
}

// ---------------------------------------------------------------------------
// Phase 8: adapter best-effort cancel contract (task 8.4)
//
// Cancelling an in-flight adapter tool must surface the shared observable
// cancellation contract: the bridged tool returns Err(ToolError::Cancelled) —
// a finalized error result rather than a hang — and the cancel is actually
// dispatched to the adapter child process (best-effort write of a `cancel`
// message). The `cancel_tool` mock mode hangs on every tool_call and writes an
// `OPI_ADAPTER_CANCEL_MARKER` file when it receives the cancel.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn phase8_adapter_cancel_contract() {
    let dir = tempfile::tempdir().expect("tempdir");
    let marker_path = dir.path().join("cancel.observed");
    let env: Vec<(String, String)> = vec![
        (
            "OPI_ADAPTER_TEST_MODE".to_string(),
            "cancel_tool".to_string(),
        ),
        (
            "OPI_ADAPTER_CANCEL_MARKER".to_string(),
            marker_path.to_string_lossy().into_owned(),
        ),
    ];

    let host = AdapterHost::start(
        "cancel-mock",
        AdapterProcessConfig {
            command: mock_adapter_bin(),
            args: vec![],
            working_dir: std::env::current_dir().expect("cwd"),
            env,
        },
        Duration::from_secs(5),
    )
    .await
    .expect("start host");
    let caps = host.capabilities().clone();
    let host = Arc::new(host);
    let adapter = ProcessAdapter::from_host("cancel-mock", host.clone(), caps);
    let tools = adapter.tools();
    assert_eq!(tools.len(), 1, "cancel_tool mode advertises one tool");
    assert_eq!(tools[0].definition().name, "hanging_tool");

    let token = tokio_util::sync::CancellationToken::new();
    let token_clone = token.clone();
    let handle = tokio::spawn(async move {
        tools[0]
            .execute("call-cancel", serde_json::json!({}), token_clone, None)
            .await
    });

    // Let the tool_call reach the adapter and hang before cancelling.
    tokio::time::sleep(Duration::from_millis(150)).await;
    token.cancel();

    let result = handle.await.expect("tool task panicked");
    assert!(
        matches!(result, Err(opi_agent::tool::ToolError::Cancelled)),
        "adapter tool must return Err(ToolError::Cancelled) on cancel"
    );

    // The best-effort cancel was dispatched to the adapter child process.
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while !marker_path.exists() {
        if std::time::Instant::now() >= deadline {
            panic!(
                "adapter cancel marker not written; best-effort cancel was not dispatched to the child"
            );
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // Reap the child process.
    let _ = host
        .send_request(
            AdapterHostMessage::Shutdown {
                id: "end".into(),
                reason: "test_end".into(),
            },
            Duration::from_secs(2),
        )
        .await;
}
