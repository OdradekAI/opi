//! AGENTS.md / CLAUDE.md context loading tests for task 3.7.
//!
//! Validates file discovery, precedence, ordering, error handling,
//! concatenation format, resume behavior, and E2E system prompt injection.

use std::fs;

use opi_ai::test_support::{MockProvider, text_response};
use opi_coding_agent::config::OpiConfig;
use opi_coding_agent::context_files;
use opi_coding_agent::harness::{CodingHarness, ResumeInfo};

// --- Helpers ---

fn create_temp_workspace() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    // Initialize a git boundary so walking stops at the temp workspace root
    // instead of finding real context files in parent directories.
    init_git_repo(dir.path());
    dir
}

fn write_file(dir: &std::path::Path, name: &str, content: &str) {
    fs::write(dir.join(name), content).expect("failed to write file");
}

fn init_git_repo(dir: &std::path::Path) {
    let git_dir = dir.join(".git");
    fs::create_dir_all(&git_dir).expect("failed to create .git dir");
}

// --- File discovery: basic ---

#[test]
fn discover_agents_md_from_cwd() {
    let workspace = create_temp_workspace();
    let cwd = workspace.path();
    write_file(cwd, "AGENTS.md", "Project agents instructions");

    let result = context_files::discover_context_files(cwd, None);
    assert!(
        result.content.contains("Project agents instructions"),
        "Should contain AGENTS.md content"
    );
    assert_eq!(result.files_loaded, 1);
}

#[test]
fn discover_claude_md_from_cwd() {
    let workspace = create_temp_workspace();
    let cwd = workspace.path();
    write_file(cwd, "CLAUDE.md", "Project claude instructions");

    let result = context_files::discover_context_files(cwd, None);
    assert!(
        result.content.contains("Project claude instructions"),
        "Should contain CLAUDE.md content"
    );
    assert_eq!(result.files_loaded, 1);
}

#[test]
fn discover_both_from_cwd() {
    let workspace = create_temp_workspace();
    let cwd = workspace.path();
    write_file(cwd, "AGENTS.md", "Agents content");
    write_file(cwd, "CLAUDE.md", "Claude content");

    let result = context_files::discover_context_files(cwd, None);
    assert_eq!(result.files_loaded, 2);
    let agents_pos = result
        .content
        .find("Agents content")
        .expect("AGENTS.md content should be present");
    let claude_pos = result
        .content
        .find("Claude content")
        .expect("CLAUDE.md content should be present");
    assert!(
        agents_pos < claude_pos,
        "AGENTS.md should appear before CLAUDE.md within a directory"
    );
}

#[test]
fn no_context_files_returns_empty() {
    let workspace = create_temp_workspace();
    let cwd = workspace.path();

    let result = context_files::discover_context_files(cwd, None);
    assert!(
        result.content.is_empty(),
        "Should be empty when no files found"
    );
    assert_eq!(result.files_loaded, 0);
}

// --- Precedence: nearest directory first, then ancestors ---

#[test]
fn precedence_nearest_first() {
    let workspace = create_temp_workspace();
    let root = workspace.path();
    let subdir = root.join("sub");
    fs::create_dir_all(&subdir).unwrap();

    init_git_repo(root);

    write_file(root, "AGENTS.md", "Root agents");
    write_file(&subdir, "AGENTS.md", "Sub agents");

    let result = context_files::discover_context_files(&subdir, None);
    let sub_pos = result
        .content
        .find("Sub agents")
        .expect("subdir content should be present");
    let root_pos = result
        .content
        .find("Root agents")
        .expect("root content should be present");
    assert!(
        sub_pos < root_pos,
        "Nearest directory (subdir) should appear before ancestors"
    );
}

#[test]
fn precedence_includes_global_config_last() {
    let workspace = create_temp_workspace();
    let cwd = workspace.path();
    let global_dir = tempfile::tempdir().expect("failed to create global dir");

    init_git_repo(cwd);
    write_file(cwd, "AGENTS.md", "Local agents");
    write_file(global_dir.path(), "CLAUDE.md", "Global claude");

    let result = context_files::discover_context_files(cwd, Some(global_dir.path()));
    let local_pos = result
        .content
        .find("Local agents")
        .expect("local content should be present");
    let global_pos = result
        .content
        .find("Global claude")
        .expect("global content should be present");
    assert!(
        local_pos < global_pos,
        "Local files should appear before global config"
    );
    assert_eq!(result.files_loaded, 2);
}

// --- Git root boundary ---

#[test]
fn stops_at_git_root() {
    let workspace = create_temp_workspace();
    let root = workspace.path();
    let outside = root.parent().unwrap().to_path_buf();

    let subdir = root.join("deep");
    fs::create_dir_all(&subdir).unwrap();

    init_git_repo(root);
    write_file(&outside, "AGENTS.md", "Outside git root");
    write_file(root, "AGENTS.md", "Inside git root");

    let result = context_files::discover_context_files(&subdir, None);
    assert!(
        !result.content.contains("Outside git root"),
        "Should not discover files outside git root"
    );
    assert!(
        result.content.contains("Inside git root"),
        "Should discover files inside git root"
    );
}

// --- Per-directory ordering: AGENTS.md before CLAUDE.md ---

#[test]
fn per_dir_agents_before_claude() {
    let workspace = create_temp_workspace();
    let cwd = workspace.path();
    write_file(cwd, "AGENTS.md", "A-content");
    write_file(cwd, "CLAUDE.md", "C-content");

    let result = context_files::discover_context_files(cwd, None);
    let a_pos = result.content.find("A-content").unwrap();
    let c_pos = result.content.find("C-content").unwrap();
    assert!(
        a_pos < c_pos,
        "AGENTS.md must come before CLAUDE.md in same dir"
    );
}

// --- Error handling ---

#[test]
fn missing_files_are_skipped_silently() {
    let workspace = create_temp_workspace();
    let cwd = workspace.path();
    // No files at all
    let result = context_files::discover_context_files(cwd, None);
    assert!(result.content.is_empty());
}

#[test]
fn non_utf8_file_is_skipped() {
    let workspace = create_temp_workspace();
    let cwd = workspace.path();
    fs::write(cwd.join("AGENTS.md"), b"\xff\xfeinvalid utf8").unwrap();

    let result = context_files::discover_context_files(cwd, None);
    assert_eq!(result.files_loaded, 0, "Non-UTF-8 file should be skipped");
}

#[test]
fn oversized_file_is_skipped() {
    let workspace = create_temp_workspace();
    let cwd = workspace.path();
    let big_content = "x".repeat(200_000); // > 128KB
    fs::write(cwd.join("AGENTS.md"), big_content).unwrap();

    let result = context_files::discover_context_files(cwd, None);
    assert_eq!(result.files_loaded, 0, "Oversized file should be skipped");
}

#[test]
fn symlink_is_followed() {
    let workspace = create_temp_workspace();
    let cwd = workspace.path();
    let real = cwd.join("real_instructions.md");
    fs::write(&real, "Symlinked content").unwrap();

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&real, cwd.join("AGENTS.md")).unwrap();
    }
    #[cfg(windows)]
    {
        // Windows symlinks require elevated privileges; use a junction/shortcut
        // as a best-effort fallback, or skip the test.
        std::os::windows::fs::symlink_file(&real, cwd.join("AGENTS.md")).unwrap_or_else(|_| {
            // If symlink creation fails (no admin), write directly as a proxy
            fs::write(cwd.join("AGENTS.md"), "Symlinked content").unwrap();
        });
    }

    let result = context_files::discover_context_files(cwd, None);
    assert!(
        result.content.contains("Symlinked content"),
        "Symlinked file should be followed"
    );
}

// --- OPI.md is NOT loaded ---

#[test]
fn opi_md_not_loaded() {
    let workspace = create_temp_workspace();
    let cwd = workspace.path();
    write_file(cwd, "OPI.md", "Legacy opi content");
    write_file(cwd, "AGENTS.md", "Agents content");

    let result = context_files::discover_context_files(cwd, None);
    assert!(
        !result.content.contains("Legacy opi content"),
        "OPI.md should NOT be loaded"
    );
    assert!(
        result.content.contains("Agents content"),
        "AGENTS.md should still be loaded"
    );
}

// --- Concatenation format ---

#[test]
fn concatenation_has_heading_per_file() {
    let workspace = create_temp_workspace();
    let cwd = workspace.path();
    write_file(cwd, "AGENTS.md", "Agents text");
    write_file(cwd, "CLAUDE.md", "Claude text");

    let result = context_files::discover_context_files(cwd, None);
    assert!(
        result.content.contains("AGENTS.md"),
        "Should have AGENTS.md heading"
    );
    assert!(
        result.content.contains("CLAUDE.md"),
        "Should have CLAUDE.md heading"
    );
}

// --- Resume: uses workspace root, not current dir ---

#[test]
fn resume_discovers_from_original_workspace() {
    let original_workspace = create_temp_workspace();
    let original_cwd = original_workspace.path();
    write_file(original_cwd, "AGENTS.md", "Original workspace context");

    let other_dir = create_temp_workspace();

    // Simulate resume: pass original workspace root, not current dir
    let result = context_files::discover_context_files(original_cwd, None);
    assert!(
        result.content.contains("Original workspace context"),
        "Resume should discover from original workspace root"
    );

    // Verify it doesn't pick up from other dir
    let result2 = context_files::discover_context_files(other_dir.path(), None);
    assert!(
        result2.content.is_empty(),
        "Other dir should have no context"
    );
}

// --- E2E: system prompt includes context ---

#[tokio::test]
async fn e2e_context_in_system_prompt() {
    let workspace = create_temp_workspace();
    let cwd = workspace.path();
    write_file(cwd, "AGENTS.md", "E2E test agents instructions");
    write_file(cwd, "CLAUDE.md", "E2E test claude instructions");

    let mock = MockProvider::new("mock", vec![text_response("done")]);
    let call_log = mock.call_log_handle();

    let mut harness = CodingHarness::new(
        Box::new(mock),
        "mock:mock-model".into(),
        OpiConfig::default(),
        cwd.to_path_buf(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    let _ = harness.prompt("test prompt").await;

    let requests = call_log.lock().unwrap();
    let system = requests[0]
        .system
        .as_deref()
        .expect("system prompt should be set");
    assert!(
        system.contains("E2E test agents instructions"),
        "System prompt should contain AGENTS.md content: {system}"
    );
    assert!(
        system.contains("E2E test claude instructions"),
        "System prompt should contain CLAUDE.md content: {system}"
    );
}

// --- E2E: resume preserves context from original workspace ---

#[tokio::test]
async fn e2e_resume_context_from_original_workspace() {
    let original_workspace = create_temp_workspace();
    let original_cwd = original_workspace.path();
    write_file(original_cwd, "AGENTS.md", "Resume test original context");

    // Create a session header and entries for resume
    let session_dir = tempfile::tempdir().unwrap();
    let session_path = session_dir.path().join("test_session.jsonl");
    let header = opi_agent::session::SessionHeader::new_for_test(
        "test-session-id".into(),
        "2026-01-01T00:00:00Z".into(),
        original_cwd.to_string_lossy().into_owned(),
        None,
    );
    fs::write(
        &session_path,
        serde_json::to_string(&header).unwrap() + "\n",
    )
    .unwrap();

    let entries = vec![];
    let resume_info = ResumeInfo {
        path: session_path,
        session_id: "test-session-id".into(),
        entries,
        original_cwd: original_cwd.to_path_buf(),
        diagnostics: Vec::new(),
        recorded_model: None,
        recorded_thinking: None,
    };

    let mock = MockProvider::new("mock", vec![text_response("done")]);

    let harness = CodingHarness::new_with_hooks_and_resume(
        Box::new(mock),
        "mock:mock-model".into(),
        OpiConfig::default(),
        original_cwd.to_path_buf(),
        Box::new(opi_coding_agent::harness::CodingAgentHooks),
        None,
        vec![],
        Some(resume_info),
        opi_coding_agent::policy::ToolSelection::Default,
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    let system = harness.system_prompt();
    assert!(
        system.contains("Resume test original context"),
        "System prompt on resume should contain context from original workspace"
    );
}

// --- E2E: global context file appears in system prompt ---

#[tokio::test]
async fn e2e_global_context_in_system_prompt() {
    let workspace = create_temp_workspace();
    let cwd = workspace.path();

    let global_dir = tempfile::tempdir().expect("failed to create global dir");
    write_file(
        global_dir.path(),
        "AGENTS.md",
        "Global agents instructions from config dir",
    );

    let mock = MockProvider::new("mock", vec![text_response("done")]);
    let call_log = mock.call_log_handle();

    let mut harness = CodingHarness::new_with_global_config_dir(
        Box::new(mock),
        "mock:mock-model".into(),
        OpiConfig::default(),
        cwd.to_path_buf(),
        Box::new(opi_coding_agent::harness::CodingAgentHooks),
        None,
        vec![],
        None,
        opi_coding_agent::policy::ToolSelection::Default,
        Some(global_dir.path().to_path_buf()),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    let _ = harness.prompt("test prompt").await;

    let requests = call_log.lock().unwrap();
    let system = requests[0]
        .system
        .as_deref()
        .expect("system prompt should be set");
    assert!(
        system.contains("Global agents instructions from config dir"),
        "System prompt should contain global AGENTS.md content: {system}"
    );
}
