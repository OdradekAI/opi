//! E2E tests for interactive CLI wiring with MockProvider (task 1.14).
//!
//! DoD: "runs against mock provider"
//!
//! Tests exercise the full path: CodingHarness → Agent → MockProvider,
//! verifying tool wiring, system prompt construction, hooks, and multi-turn.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::StreamExt;
use futures_util::stream;
use opi_agent::event::AgentEvent;
use opi_agent::message::AgentMessage;
use opi_agent::tool::{Tool, ToolError, ToolResult};
use opi_ai::message::{InputContent, Message};
use opi_ai::provider::{EventStream, Provider, ProviderError, Request};
use opi_ai::stream::AssistantStreamEvent;
use opi_ai::test_support::{self, MockProvider};
use opi_coding_agent::config::OpiConfig;
use opi_coding_agent::harness::CodingHarness;
use serde_json::json;
use std::pin::Pin;
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A mock tool that records its invocations for assertion.
struct RecordTool {
    name: String,
    call_log: Arc<Mutex<Vec<serde_json::Value>>>,
}

impl RecordTool {
    fn new(name: &str, call_log: Arc<Mutex<Vec<serde_json::Value>>>) -> Self {
        Self {
            name: name.to_owned(),
            call_log,
        }
    }
}

impl Tool for RecordTool {
    fn definition(&self) -> opi_ai::message::ToolDef {
        opi_ai::message::ToolDef {
            name: self.name.clone(),
            description: format!("Record tool: {}", self.name),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "arg": { "type": "string" }
                },
                "required": ["arg"]
            }),
        }
    }

    fn execute(
        &self,
        _call_id: &str,
        arguments: serde_json::Value,
        _signal: CancellationToken,
        _on_update: Option<opi_agent::tool::UpdateCallback>,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<ToolResult, ToolError>> + Send>> {
        let log = self.call_log.clone();
        log.lock().unwrap().push(arguments.clone());
        let text = arguments
            .get("arg")
            .and_then(|v| v.as_str())
            .unwrap_or("mock-result")
            .to_owned();
        Box::pin(async move {
            Ok(ToolResult {
                content: vec![opi_ai::message::OutputContent::Text {
                    text: format!("tool-result: {text}"),
                }],
                details: None,
                is_error: false,
                terminate: false,
                truncated: false,
                diagnostics: vec![],
            })
        })
    }
}

fn event_name(event: &AgentEvent) -> &'static str {
    use AgentEvent::*;
    match event {
        AgentStart => "AgentStart",
        AgentEnd { .. } => "AgentEnd",
        TurnStart => "TurnStart",
        TurnEnd { .. } => "TurnEnd",
        MessageStart { .. } => "MessageStart",
        MessageUpdate { .. } => "MessageUpdate",
        MessageEnd { .. } => "MessageEnd",
        ToolExecutionStart { .. } => "ToolExecutionStart",
        ToolExecutionEnd { is_error, .. } => {
            if *is_error {
                "ToolExecutionEnd(error)"
            } else {
                "ToolExecutionEnd(ok)"
            }
        }
        _ => "Other",
    }
}

// ---------------------------------------------------------------------------
// Test 1: text prompt through CodingHarness with MockProvider
// ---------------------------------------------------------------------------

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn harness_text_prompt_with_mock() {
    let _lock = session_lock();
    let response = test_support::text_response("Hello from harness!");
    let provider = MockProvider::new("mock", vec![response]);

    let mut harness = CodingHarness::new(
        Box::new(provider),
        "mock-model".into(),
        OpiConfig::default(),
        std::env::current_dir().unwrap(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    let events: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let ev = events.clone();
    harness.subscribe(Box::new(move |event| {
        ev.lock().unwrap().push(event_name(event).to_owned());
    }));

    let result = harness.prompt("Hi there").await.unwrap();

    // Should have user message + assistant response
    assert!(
        result.len() >= 2,
        "expected >= 2 messages, got {}",
        result.len()
    );

    // First message is user
    if let AgentMessage::Llm(Message::User(user)) = &result[0] {
        let text = &user.content[0];
        assert!(
            matches!(text, InputContent::Text { text } if text == "Hi there"),
            "first message should be the user prompt"
        );
    } else {
        panic!("first message should be a User message");
    }

    // Should have assistant message
    let has_assistant = result
        .iter()
        .any(|m| matches!(m, AgentMessage::Llm(Message::Assistant(_))));
    assert!(has_assistant, "should have at least one Assistant message");

    // Lifecycle events
    let ev_lock = events.lock().unwrap();
    assert!(
        ev_lock.contains(&"AgentStart".to_owned()),
        "missing AgentStart"
    );
    assert!(ev_lock.contains(&"AgentEnd".to_owned()), "missing AgentEnd");
}

// ---------------------------------------------------------------------------
// Test 2: tool call through CodingHarness with MockProvider
// ---------------------------------------------------------------------------

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn harness_tool_call_with_mock() {
    let _lock = session_lock();
    let tool_call_log: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(Vec::new()));

    let first = test_support::tool_call_response("tc-1", "record_tool", r#"{"arg":"hello"}"#);
    let second = test_support::text_response("Tool executed!");

    let provider = MockProvider::new("mock", vec![first, second]);

    let mut harness = CodingHarness::new(
        Box::new(provider),
        "mock-model".into(),
        OpiConfig::default(),
        std::env::current_dir().unwrap(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    // Add the record tool alongside built-in tools
    let record_tool = RecordTool::new("record_tool", tool_call_log.clone());
    harness.add_tool(Box::new(record_tool));

    let result = harness.prompt("Use the record tool").await.unwrap();

    // Tool should have been called
    let log = tool_call_log.lock().unwrap();
    assert_eq!(log.len(), 1, "tool should have been called exactly once");
    assert_eq!(log[0]["arg"], "hello");

    // Should have: user → assistant(tool_call) → tool_result → assistant(text)
    assert!(
        result.len() >= 4,
        "expected >= 4 messages, got {}",
        result.len()
    );
}

// ---------------------------------------------------------------------------
// Test 3: system prompt includes built-in tool descriptions
// ---------------------------------------------------------------------------

#[tokio::test]
async fn harness_system_prompt_includes_tools() {
    let _lock = session_lock();
    let response = test_support::text_response("ok");
    let provider = MockProvider::new("mock", vec![response]);

    let harness = CodingHarness::new(
        Box::new(provider),
        "mock-model".into(),
        OpiConfig::default(),
        std::env::current_dir().unwrap(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    // Use the system_prompt() accessor to verify tool descriptions
    let system_prompt = harness.system_prompt();
    assert!(
        system_prompt.contains("Available tools:"),
        "system prompt should include tool section header"
    );
    assert!(
        system_prompt.contains("read"),
        "system prompt should mention read tool"
    );
    assert!(
        system_prompt.contains("bash"),
        "system prompt should mention bash tool"
    );
}

// ---------------------------------------------------------------------------
// Test 4: multi-turn conversation through CodingHarness
// ---------------------------------------------------------------------------

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn harness_multi_turn_with_mock() {
    let _lock = session_lock();
    let first = test_support::text_response("First response");
    let second = test_support::text_response("Second response");

    let provider = MockProvider::new("mock", vec![first, second]);

    let mut harness = CodingHarness::new(
        Box::new(provider),
        "mock-model".into(),
        OpiConfig::default(),
        std::env::current_dir().unwrap(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    let result1 = harness.prompt("Hello").await.unwrap();
    assert!(result1.len() >= 2, "first turn should have >= 2 messages");

    let result2 = harness.continue_("Tell me more").await.unwrap();

    // After two turns: user1 + asst1 + user2 + asst2
    assert!(
        result2.len() >= 4,
        "expected >= 4 messages after two turns, got {}",
        result2.len()
    );
}

// ---------------------------------------------------------------------------
// Test 5: harness respects config max_iterations
// ---------------------------------------------------------------------------

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn harness_respects_max_iterations_config() {
    let _lock = session_lock();
    let responses = (0..50)
        .map(|index| {
            test_support::tool_call_response(
                &format!("tc-{index}"),
                "record_tool",
                r#"{"arg":"keep-going"}"#,
            )
        })
        .collect();
    let provider = MockProvider::new("mock", responses);
    let provider_calls = provider.call_log_handle();
    let tool_calls = Arc::new(Mutex::new(Vec::new()));

    let mut config = OpiConfig::default();
    config.defaults.max_iterations = 3;

    let mut harness = CodingHarness::new(
        Box::new(provider),
        "mock-model".into(),
        config,
        std::env::current_dir().unwrap(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );
    harness.add_tool(Box::new(RecordTool::new(
        "record_tool",
        Arc::clone(&tool_calls),
    )));

    let result = harness.prompt("Keep using the record tool").await;

    assert!(
        matches!(
            result,
            Err(opi_agent::loop_types::AgentError::MaxTurnsExceeded(3))
        ),
        "the configured boundary must terminate the production loop: {result:?}"
    );
    assert_eq!(
        provider_calls.lock().unwrap().len(),
        3,
        "max_iterations=3 must stop before a fourth provider turn"
    );
    assert_eq!(
        tool_calls.lock().unwrap().len(),
        3,
        "each permitted provider turn must execute exactly one tool call"
    );
}

// ---------------------------------------------------------------------------
// Phase 8: interactive abort/shutdown cancellation contract (task 8.4)
//
// Interactive abort and shutdown reduce to the shared cancellation primitive
// exposed by the harness (cancel_token / cancel — the same handle the TUI abort
// keybinding and the exit/quit path use). An aborted run must emit a terminal
// AgentEnd, return Err(AgentError::Cancelled), leave no run pending, and let
// the harness return to idle so a subsequent prompt is accepted and completes.
// ---------------------------------------------------------------------------

/// Provider whose first stream hangs mid-stream (so a prompt can be aborted in
/// flight) and whose subsequent streams complete normally (so the harness can
/// be shown to return to idle and accept a new prompt after the abort).
struct HangingThenCompleteProvider {
    calls: Arc<Mutex<usize>>,
}

impl HangingThenCompleteProvider {
    fn new() -> Self {
        Self {
            calls: Arc::new(Mutex::new(0)),
        }
    }
}

impl Provider for HangingThenCompleteProvider {
    fn id(&self) -> &str {
        "mock"
    }
    fn models(&self) -> &[opi_ai::provider::ModelInfo] {
        static MODELS: std::sync::OnceLock<Vec<opi_ai::provider::ModelInfo>> =
            std::sync::OnceLock::new();
        MODELS
            .get_or_init(|| {
                vec![opi_ai::provider::ModelInfo::new(
                    "mock-model",
                    "Mock Model",
                    opi_ai::WireApi::OpenAiCompletions,
                    opi_ai::ModelCapabilities::new(100_000, 4_096),
                )]
            })
            .as_slice()
    }
    fn stream(&self, _request: Request) -> EventStream {
        let mut count = self.calls.lock().unwrap();
        *count += 1;
        let first = *count == 1;
        if first {
            // Emit Start, then hang: the run is in flight but never finalizes,
            // so aborting discards the partial assistant content.
            Box::pin(
                stream::iter([Ok::<_, ProviderError>(AssistantStreamEvent::Start {
                    partial: test_support::base_assistant(),
                })])
                .chain(stream::pending::<Result<AssistantStreamEvent, ProviderError>>()),
            )
        } else {
            let events = test_support::text_response("recovered");
            Box::pin(stream::iter(events.into_iter().map(Ok::<_, ProviderError>)))
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::await_holding_lock)]
async fn phase8_interactive_abort_shutdown_contract() {
    let _lock = session_lock();
    let provider = HangingThenCompleteProvider::new();
    let mut harness = CodingHarness::new(
        Box::new(provider),
        "mock-model".into(),
        OpiConfig::default(),
        std::env::current_dir().unwrap(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );
    let token = harness.cancel_token();

    let events: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let ev = events.clone();
    harness.subscribe(Box::new(move |event| {
        ev.lock().unwrap().push(event_name(event).to_owned());
    }));

    // Spawn the prompt (in-flight run). The interactive TUI abort keybinding
    // cancels the active run via this same token.
    let handle = tokio::spawn(async move {
        let result = harness.prompt("hang").await;
        (harness, result)
    });

    // Let the provider emit Start and enter its hang before aborting.
    tokio::time::sleep(Duration::from_millis(150)).await;
    token.cancel();

    let (mut harness, result) = handle.await.expect("prompt task panicked");
    assert!(
        matches!(result, Err(opi_agent::loop_types::AgentError::Cancelled)),
        "aborted run returns Err(Cancelled)"
    );

    {
        let seq = events.lock().unwrap();
        assert!(
            seq.contains(&"AgentStart".to_owned()),
            "AgentStart emitted: {seq:?}"
        );
        assert_eq!(
            seq.iter().filter(|s| s.as_str() == "AgentEnd").count(),
            1,
            "aborted run emits exactly one terminal AgentEnd: {seq:?}"
        );
        assert!(
            !seq.contains(&"MessageEnd".to_owned()),
            "partial assistant message must not be finalized (Done never arrived): {seq:?}"
        );
    }

    // The harness returns to idle: after resetting the token, a new prompt is
    // accepted and runs to completion (provider's second stream completes).
    harness.reset_cancel_if_cancelled();
    let result2 = harness.prompt("again").await;
    assert!(
        result2.is_ok(),
        "harness accepts a new prompt after abort (idle): {result2:?}"
    );
}

// ---------------------------------------------------------------------------
// Phase 13.4: session metadata slash commands (/name, /label, /unlabel,
// /session info) parse into typed commands and dispatch through the harness
// coordinator to durable JSONL entries. Bare /session stays the resume picker.
// ---------------------------------------------------------------------------

/// Set env var safely (edition 2024 requires unsafe for set_var).
fn set_sessions_dir(dir: &std::path::Path) {
    // SAFETY: test-only env var mutation; tests acquire SESSION_MUTEX first.
    unsafe {
        std::env::set_var("OPI_SESSIONS_DIR", dir);
    }
}

/// Remove env var safely (edition 2024 requires unsafe for remove_var).
fn clear_sessions_dir() {
    // SAFETY: test-only env var mutation; tests acquire SESSION_MUTEX first.
    unsafe {
        std::env::remove_var("OPI_SESSIONS_DIR");
    }
}

fn session_lock() -> std::sync::MutexGuard<'static, ()> {
    static SESSION_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());
    // The lock guards both process-global env var mutation and harness/session
    // creation while another test temporarily points OPI_SESSIONS_DIR at a
    // fixture directory. It is held for the whole test including awaits.
    // No re-entrant acquisition across tests.
    SESSION_MUTEX
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn phase13_session_picker_command_remains_resume_picker() {
    //! Phase 13.4 acceptance: the metadata-command parser must NOT consume
    //! bare `/session` (the resume picker trigger) or the other existing
    //! slash commands. Only `/name <name>`, `/label <label>`, `/unlabel
    //! <label>`, and the exact form `/session info` parse to a metadata
    //! command. This is the parser half of DoD scenario
    //! `phase13-session-name-label-commands`.

    use opi_coding_agent::interactive::{SessionMetadataCommand, parse_session_metadata_command};

    // Metadata commands parse to their typed variants.
    assert_eq!(
        parse_session_metadata_command("/name demo-session"),
        Some(SessionMetadataCommand::Name("demo-session".into()))
    );
    assert_eq!(
        parse_session_metadata_command("/label bug"),
        Some(SessionMetadataCommand::Label("bug".into()))
    );
    assert_eq!(
        parse_session_metadata_command("/unlabel bug"),
        Some(SessionMetadataCommand::Unlabel("bug".into()))
    );
    assert_eq!(
        parse_session_metadata_command("/session info"),
        Some(SessionMetadataCommand::Info)
    );

    // Bare `/session` is the resume picker trigger — parser returns None so
    // the existing picker path stays intact (non-regression).
    assert_eq!(parse_session_metadata_command("/session"), None);
    // Other existing slash commands are unaffected.
    assert_eq!(parse_session_metadata_command("/model"), None);
    assert_eq!(parse_session_metadata_command("/branch"), None);
    assert_eq!(parse_session_metadata_command("/fork"), None);

    // Empty / whitespace-only args parse to local usage hints instead of
    // falling through to the LLM as user prompts.
    assert_eq!(
        parse_session_metadata_command("/name "),
        Some(SessionMetadataCommand::Usage("usage: /name <name>".into()))
    );
    assert_eq!(
        parse_session_metadata_command("/label "),
        Some(SessionMetadataCommand::Usage(
            "usage: /label <label>".into()
        ))
    );
    assert_eq!(
        parse_session_metadata_command("/unlabel "),
        Some(SessionMetadataCommand::Usage(
            "usage: /unlabel <label>".into()
        ))
    );

    // Non-slash input is left for the agent.
    assert_eq!(parse_session_metadata_command("hello world"), None);

    // `/session info extra` is NOT the metadata command (exact form only),
    // so it falls through to the agent like any other prompt.
    assert_eq!(parse_session_metadata_command("/session info extra"), None);
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn phase13_name_label_and_session_info_commands_persist_typed_entries() {
    //! Phase 13.4 acceptance: dispatching parsed `/name`, `/label`, `/unlabel`,
    //! and `/session info` commands through `apply_session_metadata_command`
    //! persists typed `session_info` and `label` entries to the session JSONL
    //! and the info command surfaces the live metadata. This is the dispatch
    //! half of DoD scenario `phase13-session-name-label-commands`.

    use opi_agent::session::{LabelAction, SessionEntry, SessionReader};
    use opi_coding_agent::interactive::{
        apply_session_metadata_command, parse_session_metadata_command,
    };

    let _lock = session_lock();
    let workspace = tempfile::tempdir().unwrap();
    let sessions = tempfile::tempdir().unwrap();
    set_sessions_dir(sessions.path());

    let provider = MockProvider::new("mock", vec![test_support::text_response("ok")]);
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .build();
    harness.prompt("seed turn").await.unwrap();

    let jsonl_path = {
        let session = harness.session().expect("session exists after prompt");
        let id = session.session_id().to_owned();
        sessions.path().join(format!("{id}.jsonl"))
    };

    // Dispatch each metadata command via the parsed form, the way the TUI
    // dispatcher would.
    let name_msg = apply_session_metadata_command(
        &mut harness,
        parse_session_metadata_command("/name demo").unwrap(),
    );
    assert!(
        name_msg.contains("demo") && !name_msg.contains("failed"),
        "/name dispatch should report success: {name_msg}"
    );
    let label_msg = apply_session_metadata_command(
        &mut harness,
        parse_session_metadata_command("/label bug").unwrap(),
    );
    assert!(
        label_msg.contains("bug") && !label_msg.contains("failed"),
        "/label dispatch should report success: {label_msg}"
    );
    let unlabel_msg = apply_session_metadata_command(
        &mut harness,
        parse_session_metadata_command("/unlabel bug").unwrap(),
    );
    assert!(
        unlabel_msg.contains("bug") && !unlabel_msg.contains("failed"),
        "/unlabel dispatch should report success: {unlabel_msg}"
    );

    let info_msg = apply_session_metadata_command(
        &mut harness,
        parse_session_metadata_command("/session info").unwrap(),
    );
    assert!(
        info_msg.contains("demo") && info_msg.contains("name"),
        "/session info should surface the live name: {info_msg}"
    );
    let metadata = harness
        .session_metadata()
        .expect("metadata commands keep session metadata available");
    assert_eq!(metadata.name.as_deref(), Some("demo"));
    assert!(
        metadata.labels.is_empty(),
        "/unlabel dispatch should remove the label from the live aggregate"
    );

    // Read back the JSONL and confirm typed entries were persisted.
    let (_h, entries) = SessionReader::read_all(&jsonl_path).unwrap();
    let session_info = entries.iter().find_map(|e| match e {
        SessionEntry::SessionInfo(s) => Some(s.name.clone()),
        _ => None,
    });
    assert_eq!(session_info, Some("demo".to_string()));

    let label_entries: Vec<_> = entries
        .iter()
        .filter_map(|e| match e {
            SessionEntry::Label(l) => Some((l.label.clone(), l.action)),
            _ => None,
        })
        .collect();
    assert!(
        label_entries
            .iter()
            .any(|(l, a)| l == "bug" && *a == LabelAction::Add),
        "Add label entry persisted: {label_entries:?}"
    );
    assert!(
        label_entries
            .iter()
            .any(|(l, a)| l == "bug" && *a == LabelAction::Remove),
        "Remove label entry persisted: {label_entries:?}"
    );

    clear_sessions_dir();
}
