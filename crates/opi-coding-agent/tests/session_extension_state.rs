use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use opi_agent::extension::{ExtensionCommand, ExtensionRegistry};
use opi_agent::session::{
    ExtensionStateEntry, LeafEntry, MessageEntry, SessionEntry, SessionHeader, SessionReader,
    SessionWriter,
};
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::test_support::{self, MockProvider};
use opi_coding_agent::adapter_extension::ProcessAdapter;
use opi_coding_agent::adapter_host::{AdapterHost, AdapterProcessConfig};
use opi_coding_agent::config::OpiConfig;
use opi_coding_agent::harness::{CodingHarness, ResumeInfo};

fn make_header(id: &str, cwd: &str) -> SessionHeader {
    SessionHeader::new(id.into(), "2026-06-09T00:00:00Z".into(), cwd.into(), None)
}

fn user_entry(id: &str, parent_id: Option<&str>, text: &str) -> SessionEntry {
    SessionEntry::Message(MessageEntry {
        id: id.into(),
        parent_id: parent_id.map(str::to_owned),
        timestamp: "2026-06-09T00:00:00Z".into(),
        message: Message::User(UserMessage {
            content: vec![InputContent::Text { text: text.into() }],
            timestamp_ms: 0,
        }),
    })
}

#[test]
fn latest_extension_state_selects_state_for_active_branch_tip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("session.jsonl");
    let header = make_header("sess-branch", &dir.path().display().to_string());

    let mut writer = SessionWriter::create(&path, header).unwrap();
    writer.append(&user_entry("msg-1", None, "root")).unwrap();
    writer
        .append(&user_entry("msg-2a", Some("msg-1"), "old branch"))
        .unwrap();
    writer
        .append(&SessionEntry::ExtensionState(ExtensionStateEntry {
            id: "state-old".to_string(),
            parent_id: Some("msg-2a".to_string()),
            timestamp: "2026-06-09T00:00:01Z".to_string(),
            state: serde_json::json!({"todo": {"items": []}}),
        }))
        .unwrap();
    writer
        .append(&user_entry("msg-2b", Some("msg-1"), "new branch"))
        .unwrap();
    writer
        .append(&SessionEntry::ExtensionState(ExtensionStateEntry {
            id: "state-new".to_string(),
            parent_id: Some("msg-2b".to_string()),
            timestamp: "2026-06-09T00:00:02Z".to_string(),
            state: serde_json::json!({"todo": {"items": [{"id": "todo-1"}]}}),
        }))
        .unwrap();
    writer
        .append(&SessionEntry::Leaf(LeafEntry {
            id: "leaf-new".to_string(),
            parent_id: Some("msg-2b".to_string()),
            timestamp: "2026-06-09T00:00:03Z".to_string(),
            entry_id: "msg-2b".to_string(),
        }))
        .unwrap();
    drop(writer);

    let (_, entries) = SessionReader::read_all(&path).unwrap();
    let state = opi_coding_agent::session_coordinator::latest_extension_state(&entries);

    assert_eq!(state.unwrap()["todo"]["items"][0]["id"], "todo-1");
}

fn example_adapter_bin() -> PathBuf {
    let current = std::env::current_exe().expect("current exe path");
    let deps_dir = current.parent().expect("deps directory");

    let exact_name = if cfg!(windows) {
        "package_adapter_example.exe"
    } else {
        "package_adapter_example"
    };
    let exact_path = deps_dir.join(exact_name);
    if exact_path.exists() {
        return exact_path;
    }

    let exe_suffix = if cfg!(windows) { ".exe" } else { "" };
    let prefix = "package_adapter_example-";
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
        .expect("Could not find package_adapter_example binary in deps directory")
}

async fn start_todo_registry() -> (Arc<AdapterHost>, ExtensionRegistry) {
    let config = AdapterProcessConfig {
        command: example_adapter_bin(),
        args: vec!["todo".to_string()],
        working_dir: std::env::current_dir().expect("cwd"),
        env: vec![],
    };
    let host = AdapterHost::start("todo", config, Duration::from_secs(10))
        .await
        .expect("start adapter");
    let caps = host.capabilities().clone();
    let host = Arc::new(host);
    let adapter = ProcessAdapter::from_host("todo", host.clone(), caps);
    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(adapter)).expect("register");
    (host, registry)
}

#[tokio::test(flavor = "multi_thread")]
async fn adapter_state_restores_from_latest_session_extension_state() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("session.jsonl");
    let header = make_header("sess-adapter-state", &dir.path().display().to_string());

    let (_host, registry) = start_todo_registry().await;
    let add = ExtensionCommand::new(
        "todo/add",
        serde_json::json!({"title": "resume me", "description": "state"}),
    );
    registry.dispatch_command(&add).await.expect("add todo");
    let state = registry.serialize_states().expect("serialize state");

    let mut writer = SessionWriter::create(&path, header).unwrap();
    writer.append(&user_entry("msg-1", None, "root")).unwrap();
    writer
        .append(&SessionEntry::ExtensionState(ExtensionStateEntry {
            id: "state-1".to_string(),
            parent_id: Some("msg-1".to_string()),
            timestamp: "2026-06-09T00:00:01Z".to_string(),
            state,
        }))
        .unwrap();
    drop(writer);

    let (_, entries) = SessionReader::read_all(&path).unwrap();
    let state = opi_coding_agent::session_coordinator::latest_extension_state(&entries)
        .expect("latest state");

    let (_restored_host, restored) = start_todo_registry().await;
    restored.restore_states(state).expect("restore state");
    let list = ExtensionCommand::new("todo/list", serde_json::json!({}));
    let data = restored
        .dispatch_command(&list)
        .await
        .expect("list todos")
        .expect("todo list result");

    let items = data["items"].as_array().expect("items array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["title"], "resume me");
}

// ---------------------------------------------------------------------------
// Phase 6 (task 6.5): extension-state lifecycle through the production harness.
//
// The DoD requires focused tests that extension state is restored before
// adapter-backed runtime behavior and persisted after turns that mutate
// adapter state. Both are exercised through the real CodingHarness resume +
// prompt path. Each harness resumes from a temp JSONL file so appends land in
// the temp dir (no OPI_SESSIONS_DIR mutation). The `todo` example adapter
// advertises commands only (tools: []), so adapter state mutates via
// dispatch_command (todo/add) and is captured by persist_extension_state after
// the next turn.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn extension_state_persists_to_session_jsonl_after_mutating_turn() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("session.jsonl");
    let header = make_header("sess-persist", &dir.path().display().to_string());

    let user = user_entry("msg-1", None, "seed turn");
    let mut writer = SessionWriter::create(&path, header).unwrap();
    writer.append(&user).unwrap();
    drop(writer);
    let entries = vec![user];

    let (_host, registry) = start_todo_registry().await;
    let provider = MockProvider::new("mock", vec![test_support::text_response("ok")]);
    let resume = ResumeInfo {
        path: path.clone(),
        session_id: "sess-persist".into(),
        entries,
        original_cwd: dir.path().to_path_buf(),
        diagnostics: Vec::new(),
        recorded_model: None,
        recorded_thinking: None,
    };
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        dir.path().to_path_buf(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .resume(resume)
    .extension_registry(registry)
    .build();

    // Mutate adapter state through the production command path, then run a
    // turn. persist_turn -> persist_extension_state must serialize the mutated
    // registry state and append an ExtensionState entry to the session JSONL.
    harness
        .dispatch_extension_command(
            "todo/add",
            None,
            serde_json::json!({"title": "persisted me", "description": "state"}),
        )
        .await
        .expect("todo/add dispatch");
    harness.prompt("run the turn").await.expect("prompt turn");

    let (_header, readback) = SessionReader::read_all(&path).unwrap();
    let state_entry = readback
        .iter()
        .find(|entry| matches!(entry, SessionEntry::ExtensionState(_)))
        .expect("session JSONL should gain an ExtensionState entry after a mutating turn");
    let SessionEntry::ExtensionState(ext_state) = state_entry else {
        unreachable!()
    };
    let items = ext_state.state["todo"]["items"]
        .as_array()
        .expect("serialized todo items");
    assert!(
        items.iter().any(|item| item["title"] == "persisted me"),
        "persisted extension state must contain the mutated todo item: {items:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn command_only_extension_state_persists_to_session_jsonl() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("session.jsonl");
    let header = make_header("sess-command-persist", &dir.path().display().to_string());

    let user = user_entry("msg-1", None, "seed turn");
    let mut writer = SessionWriter::create(&path, header).unwrap();
    writer.append(&user).unwrap();
    drop(writer);
    let entries = vec![user];

    let (_host, registry) = start_todo_registry().await;
    let provider = MockProvider::new("mock", vec![test_support::text_response("unused")]);
    let resume = ResumeInfo {
        path: path.clone(),
        session_id: "sess-command-persist".into(),
        entries,
        original_cwd: dir.path().to_path_buf(),
        diagnostics: Vec::new(),
        recorded_model: None,
        recorded_thinking: None,
    };
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        dir.path().to_path_buf(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .resume(resume)
    .extension_registry(registry)
    .build();

    harness
        .dispatch_extension_command(
            "todo/add",
            None,
            serde_json::json!({"title": "command persisted", "description": "state"}),
        )
        .await
        .expect("todo/add dispatch");

    let (_header, readback) = SessionReader::read_all(&path).unwrap();
    let state_entry = readback
        .iter()
        .find_map(|entry| match entry {
            SessionEntry::ExtensionState(ext_state) => Some(ext_state),
            _ => None,
        })
        .expect("command-only mutation should persist an ExtensionState entry");
    let items = state_entry.state["todo"]["items"]
        .as_array()
        .expect("serialized todo items");
    assert!(
        items
            .iter()
            .any(|item| item["title"] == "command persisted"),
        "command-only persisted state must contain the mutated todo item: {items:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn command_only_extension_command_restores_pending_session_state() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("session.jsonl");
    let header = make_header("sess-command-restore", &dir.path().display().to_string());

    let user = user_entry("msg-1", None, "seed turn");
    let state = SessionEntry::ExtensionState(ExtensionStateEntry {
        id: "state-1".to_string(),
        parent_id: Some("msg-1".to_string()),
        timestamp: "2026-06-09T00:00:01Z".to_string(),
        state: serde_json::json!({
            "todo": {
                "items": [{
                    "id": "todo-1",
                    "title": "command restored",
                    "description": "state",
                    "status": "pending"
                }],
                "next_id": 2
            }
        }),
    });
    let mut writer = SessionWriter::create(&path, header).unwrap();
    writer.append(&user).unwrap();
    writer.append(&state).unwrap();
    drop(writer);
    let entries = vec![user, state];

    let (_host, registry) = start_todo_registry().await;
    let provider = MockProvider::new("mock", vec![test_support::text_response("unused")]);
    let resume = ResumeInfo {
        path,
        session_id: "sess-command-restore".into(),
        entries,
        original_cwd: dir.path().to_path_buf(),
        diagnostics: Vec::new(),
        recorded_model: None,
        recorded_thinking: None,
    };
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        dir.path().to_path_buf(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .resume(resume)
    .extension_registry(registry)
    .build();

    let list = harness
        .dispatch_extension_command("todo/list", None, serde_json::json!({}))
        .await
        .expect("todo/list dispatch")
        .expect("todo list result");
    let items = list["items"].as_array().expect("items array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["title"], "command restored");
}

#[tokio::test(flavor = "multi_thread")]
async fn resumed_extension_state_is_restored_before_adapter_command() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("session.jsonl");
    let header = make_header("sess-restore", &dir.path().display().to_string());

    let user = user_entry("msg-1", None, "seed turn");
    let state = SessionEntry::ExtensionState(ExtensionStateEntry {
        id: "state-1".to_string(),
        parent_id: Some("msg-1".to_string()),
        timestamp: "2026-06-09T00:00:01Z".to_string(),
        state: serde_json::json!({
            "todo": {
                "items": [{
                    "id": "todo-1",
                    "title": "resume me",
                    "description": "state",
                    "status": "pending"
                }],
                "next_id": 2
            }
        }),
    });
    let mut writer = SessionWriter::create(&path, header).unwrap();
    writer.append(&user).unwrap();
    writer.append(&state).unwrap();
    drop(writer);
    let entries = vec![user, state];

    // Fresh adapter registry (empty todo state). On the first prompt the
    // harness restores the persisted extension state BEFORE the agent turn
    // runs (restore_pending_extension_state precedes agent.prompt), so a
    // subsequent adapter-backed command observes the restored state.
    let (_host, registry) = start_todo_registry().await;
    let provider = MockProvider::new("mock", vec![test_support::text_response("ok")]);
    let resume = ResumeInfo {
        path: path.clone(),
        session_id: "sess-restore".into(),
        entries,
        original_cwd: dir.path().to_path_buf(),
        diagnostics: Vec::new(),
        recorded_model: None,
        recorded_thinking: None,
    };
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        dir.path().to_path_buf(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .resume(resume)
    .extension_registry(registry)
    .build();

    harness.prompt("trigger restore").await.expect("prompt");

    let list = harness
        .dispatch_extension_command("todo/list", None, serde_json::json!({}))
        .await
        .expect("todo/list dispatch")
        .expect("todo list result");
    let items = list["items"].as_array().expect("items array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["title"], "resume me");
}
