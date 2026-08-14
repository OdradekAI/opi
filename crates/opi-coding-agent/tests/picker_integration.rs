//! Picker integration tests (task 3.11).
//!
//! Tests the bridge between provider registry / session listing and the
//! SelectList widget, verifying model picker and session picker data
//! collection and result handling.

use std::path::Path;
use std::sync::Mutex;

use opi_agent::session::{LeafEntry, MessageEntry, SessionEntry};
use opi_agent::session_branch::SessionTree;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{EventStream, ModelInfo, Provider, ProviderError, Request};
use opi_ai::registry::ModelCapabilities;
use opi_ai::stream::AssistantStreamEvent;
use opi_coding_agent::picker;
use opi_tui::select_list::SelectListState;

const KEY_CANARY: &str = "sk-ant-FAKE1234567890abcdefghijklmnop";
static ENV_MUTEX: Mutex<()> = Mutex::new(());

/// Minimal provider with configurable models for picker tests.
struct TestProvider {
    id: String,
    models: Vec<ModelInfo>,
}

impl TestProvider {
    fn new(id: &str, models: Vec<ModelInfo>) -> Self {
        Self {
            id: id.into(),
            models,
        }
    }
}

impl Provider for TestProvider {
    fn id(&self) -> &str {
        &self.id
    }
    fn models(&self) -> &[ModelInfo] {
        &self.models
    }
    fn stream_prepared(&self, _request: Request, _auth: opi_ai::auth::ResolvedAuth) -> EventStream {
        // Picker tests never call stream(); return empty stream.
        let stream: Vec<Result<AssistantStreamEvent, ProviderError>> = vec![];
        Box::pin(futures_util::stream::iter(stream))
    }
}

// ---------------------------------------------------------------------------
// Model picker
// ---------------------------------------------------------------------------

fn sample_registry() -> opi_ai::registry::ProviderRegistry {
    let models = vec![
        ModelInfo::new(
            "claude-sonnet-4-5-20250514",
            "Claude Sonnet 4.5",
            opi_ai::WireApi::AnthropicMessages,
            ModelCapabilities::new(200000, 8192)
                .with_images(true)
                .with_streaming(true)
                .with_thinking(true),
        ),
        ModelInfo::new(
            "claude-opus-4-20250514",
            "Claude Opus 4",
            opi_ai::WireApi::AnthropicMessages,
            ModelCapabilities::new(200000, 8192)
                .with_images(true)
                .with_streaming(true)
                .with_thinking(true),
        ),
    ];
    let provider = TestProvider::new("anthropic", models);
    let mut registry = opi_ai::registry::ProviderRegistry::new();
    registry.register_provider(Box::new(provider)).unwrap();
    registry
}

#[test]
fn model_picker_items_from_registry() {
    let registry = sample_registry();
    let items = picker::model_picker_items(&registry);
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].id, "anthropic:claude-sonnet-4-5-20250514");
    assert_eq!(items[0].display, "Claude Sonnet 4.5");
    assert_eq!(items[0].metadata, "anthropic");
}

#[test]
fn model_picker_filter_and_select() {
    let registry = sample_registry();
    let items = picker::model_picker_items(&registry);
    let mut state = SelectListState::new(items);
    state.set_filter("opus");
    assert_eq!(state.visible_count(), 1);
    let selected = state.confirm().unwrap();
    assert_eq!(selected.id, "anthropic:claude-opus-4-20250514");
}

#[test]
fn model_picker_multiple_providers() {
    let p1 = TestProvider::new(
        "anthropic",
        vec![ModelInfo::new(
            "claude-sonnet-4-5-20250514",
            "Claude Sonnet 4.5",
            opi_ai::WireApi::AnthropicMessages,
            ModelCapabilities::new(200000, 8192)
                .with_images(true)
                .with_streaming(true)
                .with_thinking(true),
        )],
    );
    let p2 = TestProvider::new(
        "openai",
        vec![ModelInfo::new(
            "gpt-4o",
            "GPT-4o",
            opi_ai::WireApi::OpenAiCompletions,
            ModelCapabilities::new(128000, 4096)
                .with_images(true)
                .with_streaming(true),
        )],
    );
    let mut registry = opi_ai::registry::ProviderRegistry::new();
    registry.register_provider(Box::new(p1)).unwrap();
    registry.register_provider(Box::new(p2)).unwrap();

    let items = picker::model_picker_items(&registry);
    assert_eq!(items.len(), 2);

    let mut state = SelectListState::new(items);
    state.set_filter("gpt");
    assert_eq!(state.visible_count(), 1);
    assert_eq!(state.confirm().unwrap().id, "openai:gpt-4o");
}

#[test]
fn model_picker_items_include_registry_model_overrides() {
    let mut registry = sample_registry();
    registry
        .register_model(
            "anthropic",
            ModelInfo::new(
                "custom-sonnet",
                "Custom Sonnet",
                opi_ai::WireApi::AnthropicMessages,
                ModelCapabilities::new(200000, 8192)
                    .with_images(true)
                    .with_streaming(true)
                    .with_thinking(true),
            ),
        )
        .unwrap();

    let items = picker::model_picker_items(&registry);

    assert!(
        items
            .iter()
            .any(|item| item.id == "anthropic:custom-sonnet" && item.display == "Custom Sonnet")
    );
}

#[test]
fn custom_mapped_provider_picker_uses_one_production_identity() {
    use opi_coding_agent::config::load_config_file;
    use opi_coding_agent::harness::CodingHarness;
    use opi_coding_agent::provider_factory::build_provider;

    let _env_guard = ENV_MUTEX.lock().expect("env lock");
    let env_name = "OPI_TEST_CUSTOM_PICKER_ONE_IDENTITY_1416";
    let original = std::env::var_os(env_name);
    // SAFETY: the test uses a task-unique variable and restores it below.
    unsafe { std::env::set_var(env_name, "test-key") };
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("config.toml");
    std::fs::write(
        &path,
        format!(
            r#"
[defaults]
model = "acme:claude"

[providers.custom.acme]
base_url = "https://api.acme.example"
api_key_env = "{env_name}"
auth_scheme = "bearer"

[[providers.custom.acme.models]]
id = "claude"
display_name = "Claude"
api = "anthropic-messages"
context_window = 200000
max_output_tokens = 8192

[[providers.custom.acme.models]]
id = "chat"
display_name = "Chat"
api = "openai-completions"
context_window = 128000
max_output_tokens = 8192

[[providers.custom.acme.models]]
id = "responses"
display_name = "Responses"
api = "openai-responses"
context_window = 128000
max_output_tokens = 8192
"#
        ),
    )
    .unwrap();
    let config = load_config_file(&path).unwrap();
    let provider = build_provider(&config).expect("build mapped runtime provider");
    let harness = CodingHarness::builder(
        provider,
        config.defaults.model.clone(),
        config,
        root.path().to_path_buf(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .global_config_dir(root.path().join("global"))
    .build();
    let items = harness.model_picker_items();

    let custom: Vec<_> = items
        .iter()
        .filter(|item| item.metadata == "acme")
        .collect();
    assert_eq!(custom.len(), 3);
    assert_eq!(
        custom
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        vec!["acme:claude", "acme:chat", "acme:responses"]
    );
    assert!(items.iter().all(|item| {
        !item.metadata.contains("route")
            && !item.metadata.contains("anthropic-messages")
            && !item.metadata.contains("openai-completions")
            && !item.metadata.contains("openai-responses")
    }));

    match original {
        Some(value) => {
            // SAFETY: restoring the task-unique variable.
            unsafe { std::env::set_var(env_name, value) };
        }
        None => {
            // SAFETY: restoring the task-unique variable.
            unsafe { std::env::remove_var(env_name) };
        }
    }
}

#[test]
fn branch_picker_items_from_session_tree() {
    let tree = SessionTree::from_entries(&[
        test_user_entry("e1", None, "Root summary"),
        test_user_entry("e2a", Some("e1"), "Branch A"),
        test_user_entry("e2b", Some("e1"), "Branch B"),
        SessionEntry::Leaf(LeafEntry {
            id: "leaf-1".into(),
            parent_id: None,
            timestamp: "2026-06-01T12:03:00Z".into(),
            entry_id: "e2a".into(),
        }),
    ]);

    let items = picker::branch_picker_items(&tree);

    assert_eq!(items.len(), 3);
    assert_eq!(items[0].id, "e1");
    assert!(items[0].display.contains("Root summary"));
    assert_eq!(items[1].id, "e2a");
    assert!(items[1].display.contains("Branch A"));
    assert!(items[1].metadata.contains("active"));
}

#[test]
fn branch_picker_items_redact_branch_summaries() {
    let tree = SessionTree::from_entries(&[test_user_entry(
        "e1",
        None,
        &format!("Root contains {KEY_CANARY}"),
    )]);

    let items = picker::branch_picker_items(&tree);

    assert_eq!(items.len(), 1);
    assert!(
        !items[0].display.contains(KEY_CANARY),
        "branch picker summary must be redacted: {}",
        items[0].display
    );
    assert!(items[0].display.contains("[REDACTED]"));
}

fn test_user_entry(id: &str, parent_id: Option<&str>, text: &str) -> SessionEntry {
    SessionEntry::Message(MessageEntry {
        id: id.into(),
        parent_id: parent_id.map(str::to_owned),
        timestamp: "2026-06-01T12:00:00Z".into(),
        message: Message::User(UserMessage {
            content: vec![InputContent::Text { text: text.into() }],
            timestamp_ms: 0,
        }),
    })
}

// ---------------------------------------------------------------------------
// Session picker
// ---------------------------------------------------------------------------

#[test]
fn session_picker_items_from_directory() {
    let dir = tempfile::tempdir().unwrap();
    create_test_session(
        dir.path(),
        "sess-001",
        "2026-05-26T10:00:00Z",
        "/home/user/project",
    );
    create_test_session(
        dir.path(),
        "sess-002",
        "2026-05-26T11:00:00Z",
        "/home/user/other",
    );

    let items = picker::session_picker_items(dir.path()).unwrap();
    assert_eq!(items.len(), 2);
    // Sorted newest-first by timestamp
    assert_eq!(items[0].id, "sess-002");
    assert!(items[1].display.contains("project"));
    assert_eq!(items[1].id, "sess-001");
}

#[test]
fn session_picker_empty_directory() {
    let dir = tempfile::tempdir().unwrap();
    let items = picker::session_picker_items(dir.path()).unwrap();
    assert!(items.is_empty());
}

#[test]
fn session_picker_filter_and_select() {
    let dir = tempfile::tempdir().unwrap();
    create_test_session(
        dir.path(),
        "sess-001",
        "2026-05-26T10:00:00Z",
        "/home/user/project",
    );
    create_test_session(
        dir.path(),
        "sess-002",
        "2026-05-26T11:00:00Z",
        "/home/user/other",
    );

    let items = picker::session_picker_items(dir.path()).unwrap();
    let mut state = SelectListState::new(items);
    state.set_filter("other");
    assert_eq!(state.visible_count(), 1);
    let selected = state.confirm().unwrap();
    assert_eq!(selected.id, "sess-002");
}

#[test]
fn session_picker_corrupt_file_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    create_test_session(
        dir.path(),
        "sess-001",
        "2026-05-26T10:00:00Z",
        "/home/user/project",
    );

    // Write a corrupt file.
    std::fs::write(dir.path().join("corrupt.jsonl"), "not valid json\n").unwrap();

    let items = picker::session_picker_items(dir.path()).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, "sess-001");
}

#[test]
fn session_picker_long_cwd_is_truncated() {
    let dir = tempfile::tempdir().unwrap();
    let long_cwd =
        "/home/user/very/deeply/nested/directory/structure/that/exceeds/forty/characters";
    create_test_session(dir.path(), "sess-001", "2026-05-26T10:00:00Z", long_cwd);

    let items = picker::session_picker_items(dir.path()).unwrap();
    assert_eq!(items.len(), 1);
    assert!(items[0].display.starts_with("..."));
    assert!(items[0].display.len() <= 40);
}

#[test]
fn session_picker_multibyte_cwd_does_not_panic() {
    let dir = tempfile::tempdir().unwrap();
    // CJK characters: each is 3 bytes in UTF-8. 20 chars = 60 bytes.
    let cjk_cwd = "/home/\u{4f60}\u{597d}\u{4e16}\u{754c}\u{6587}\u{4ef6}\u{76ee}\u{5f55}\u{8def}\u{5f84}\u{6d4b}\u{8bd5}\u{6570}\u{636e}\u{9879}\u{76ee}\u{5de5}\u{7a0b}\u{4ee3}\u{7801}/end";
    create_test_session(dir.path(), "sess-cjk", "2026-05-26T10:00:00Z", cjk_cwd);

    let items = picker::session_picker_items(dir.path()).unwrap();
    assert_eq!(items.len(), 1);
    // Should not panic, and display should be valid UTF-8.
    assert!(items[0].display.starts_with("...") || items[0].display.len() <= 40);
}

// ---------------------------------------------------------------------------
// Phase 13.6: stable TUI/RPC handoff metadata for branch + session pickers
// ---------------------------------------------------------------------------

#[test]
fn phase13_branch_picker_session_metadata() {
    //! Phase 13.6 acceptance (picker half): `branch_picker_items` exposes a
    //! stable, documented metadata contract (entry count, depth, tip id, active
    //! marker) in stable tree order, and `session_picker_items` surfaces the
    //! typed session name and label set that RPC `session_info` and
    //! `--list-sessions --json` already expose. The picker renders in the
    //! user's own terminal, so summaries are not redacted here; cross-process
    //! handoff redaction is pinned by `phase13_rpc_session_metadata_shape`.

    use opi_agent::session::{
        LabelAction, LabelEntry, SessionEntry, SessionHeader, SessionInfoEntry, SessionWriter,
    };

    // --- Part A: branch_picker_items stability + metadata contract -------
    let tree = SessionTree::from_entries(&[
        test_user_entry("e1", None, "Root summary"),
        test_user_entry("e2a", Some("e1"), "Branch A"),
        test_user_entry("e2b", Some("e1"), "Branch B"),
        SessionEntry::Leaf(LeafEntry {
            id: "leaf-1".into(),
            parent_id: None,
            timestamp: "2026-06-01T12:03:00Z".into(),
            entry_id: "e2a".into(),
        }),
    ]);
    let items = picker::branch_picker_items(&tree);
    assert_eq!(items.len(), 3, "three branch segments: {items:?}");
    let tips: Vec<&str> = items.iter().map(|i| i.id.as_str()).collect();
    assert_eq!(
        tips,
        vec!["e1", "e2a", "e2b"],
        "stable tree-order tip ids: {items:?}"
    );
    for item in &items {
        assert!(
            item.metadata.contains("entries"),
            "metadata exposes entry_count: {item:?}"
        );
        assert!(
            item.metadata.contains("depth"),
            "metadata exposes depth: {item:?}"
        );
        assert!(
            item.metadata.contains("tip"),
            "metadata exposes tip id: {item:?}"
        );
    }
    let active_items: Vec<&opi_tui::select_list::SelectItem> = items
        .iter()
        .filter(|i| i.metadata.contains("active"))
        .collect();
    assert_eq!(
        active_items.len(),
        1,
        "exactly one active branch: {items:?}"
    );
    assert_eq!(active_items[0].id, "e2a", "active branch is e2a");

    // --- Part B: session_picker_items surfaces typed name + labels -------
    let dir = tempfile::tempdir().unwrap();
    create_named_labelled_session(
        dir.path(),
        "sess-meta",
        "2026-07-06T10:00:00Z",
        "/home/user/project",
        "demo-session",
        &["bug", "draft"],
    );
    // A second plain session with no name/labels, to confirm ordering is
    // stable (newest-first) and plain rows are unaffected.
    create_test_session(
        dir.path(),
        "sess-plain",
        "2026-07-06T11:00:00Z",
        "/home/user/other",
    );

    let items = picker::session_picker_items(dir.path()).unwrap();
    assert_eq!(items.len(), 2, "two sessions listed: {items:?}");
    // Sorted newest-first: sess-plain (11:00) then sess-meta (10:00).
    assert_eq!(items[0].id, "sess-plain");
    let meta_item = items
        .iter()
        .find(|i| i.id == "sess-meta")
        .expect("sess-meta row present");
    assert!(
        meta_item.display.contains("demo-session"),
        "session name surfaced in display: {meta_item:?}"
    );
    assert!(
        meta_item.metadata.contains("bug"),
        "label 'bug' surfaced in metadata: {meta_item:?}"
    );
    assert!(
        meta_item.metadata.contains("draft"),
        "label 'draft' surfaced in metadata: {meta_item:?}"
    );

    // --- Part C: forbidden Phase 13 non-goal markers absent --------------
    for item in &items {
        for forbidden in ["auth_token", "share_url", "publish", "sync_url", "cloud"] {
            assert!(
                !item.id.contains(forbidden)
                    && !item.display.contains(forbidden)
                    && !item.metadata.contains(forbidden),
                "forbidden non-goal marker '{forbidden}' in picker item: {item:?}"
            );
        }
    }

    /// Write a session file with a content message plus typed `session_info`
    /// (name) and `label` entries parented to the active tip, so
    /// `list_sessions` -> `reconstruct_context` surfaces them.
    fn create_named_labelled_session(
        dir: &Path,
        id: &str,
        timestamp: &str,
        cwd: &str,
        name: &str,
        labels: &[&str],
    ) {
        let header = SessionHeader::new(id.into(), timestamp.into(), cwd.into(), None);
        let path = dir.join(format!("{id}.jsonl"));
        let mut writer = SessionWriter::create(&path, header).expect("create session");
        let tip_id = "msg-1";
        writer
            .append(&SessionEntry::Message(MessageEntry {
                id: tip_id.into(),
                parent_id: None,
                timestamp: timestamp.into(),
                message: Message::User(UserMessage {
                    content: vec![InputContent::Text {
                        text: "seed prompt".into(),
                    }],
                    timestamp_ms: 0,
                }),
            }))
            .expect("append message");
        writer
            .append(&SessionEntry::SessionInfo(SessionInfoEntry {
                id: "info-1".into(),
                parent_id: Some(tip_id.into()),
                timestamp: timestamp.into(),
                name: name.into(),
            }))
            .expect("append session_info");
        for (i, label) in labels.iter().enumerate() {
            writer
                .append(&SessionEntry::Label(LabelEntry {
                    id: format!("label-{i}"),
                    parent_id: Some(tip_id.into()),
                    timestamp: timestamp.into(),
                    label: (*label).into(),
                    action: LabelAction::Add,
                }))
                .expect("append label");
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn create_test_session(dir: &Path, id: &str, timestamp: &str, cwd: &str) {
    use std::io::Write;
    let header = serde_json::json!({
        "type": "session",
        "version": 1,
        "id": id,
        "timestamp": timestamp,
        "cwd": cwd,
    });
    let path = dir.join(format!("{id}.jsonl"));
    let mut f = std::fs::File::create(path).unwrap();
    writeln!(f, "{header}").unwrap();
}
