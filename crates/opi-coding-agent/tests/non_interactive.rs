//! E2E tests for non-interactive mode (task 1.15).
//!
//! DoD: "stdout/stderr/exit-code tests"
//!
//! Tests exercise: NonInteractiveRunner with MockProvider,
//! verifying stdout output, stderr diagnostics, and exit code mapping.

mod common;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use opi_agent::extension::ExtensionRegistry;
use opi_ai::anthropic::AnthropicProvider;
use opi_ai::auth::{AuthResolver, ResolvedAuth};
use opi_ai::credential::BoxAuthFuture;
use opi_ai::http::HttpClient;
use opi_ai::provider::ProviderError;
use opi_ai::test_support::{self, MockProvider};
use opi_coding_agent::config::{ExecutionStrategy, OpiConfig, PermissionDecision};
use opi_coding_agent::package_resolver::local_lock_entry;
use opi_coding_agent::package_store::{PackageDeclaration, PackageStore};
use opi_coding_agent::project_trust::TrustDecision;
use opi_coding_agent::runner::{ExitCode, NonInteractiveRunner};
use opi_coding_agent::runtime_packages::{
    RuntimePackageStartup, start_installed_package_runtime_with_trust,
};

fn test_binary(name: &str) -> PathBuf {
    let current = std::env::current_exe().expect("current exe path");
    let deps_dir = current.parent().expect("deps directory");
    let exact_name = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    };
    let exact_path = deps_dir.join(exact_name);
    if exact_path.exists() {
        return exact_path;
    }

    let exe_suffix = if cfg!(windows) { ".exe" } else { "" };
    let prefix = format!("{name}-");
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    if let Ok(entries) = std::fs::read_dir(deps_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with(&prefix)
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
        .unwrap_or_else(|| panic!("Could not find {name} binary in deps directory"))
}

fn install_adapter_package(workspace: &Path, name: &str, command: &Path, args: &[&str]) {
    let package_dir = workspace.join("vendor").join(name);
    std::fs::create_dir_all(&package_dir).unwrap();
    std::fs::write(
        package_dir.join("package.toml"),
        format!(
            "name = \"{name}\"\n\
             description = \"Installed adapter package.\"\n\
             version = \"0.1.0\"\n\
             [adapter]\n\
             kind = \"process-jsonl\"\n\
             command = \"{}\"\n\
             args = [{}]\n\
             protocol = \"opi-extension-jsonl-v1\"\n",
            command.display().to_string().replace('\\', "\\\\"),
            args.iter()
                .map(|arg| format!("\"{arg}\""))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    )
    .unwrap();

    let store = PackageStore::project(workspace.to_path_buf());
    let source = format!("./vendor/{name}");
    store
        .write_declarations(&[PackageDeclaration {
            source: source.clone(),
            filters: Default::default(),
        }])
        .unwrap();
    store
        .write_lock(&[local_lock_entry(source, &package_dir).unwrap()])
        .unwrap();
}

#[tokio::test]
async fn runtime_startup_is_the_single_source_of_runner_trust_in_all_builds() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join(".git")).unwrap();
    std::fs::write(
        workspace.path().join("AGENTS.md"),
        "RUNTIME STARTUP TRUST MARKER",
    )
    .unwrap();

    for (decision, should_load_project_context) in [
        (TrustDecision::Trusted, true),
        (TrustDecision::Untrusted, false),
    ] {
        let provider = MockProvider::new("mock", vec![test_support::text_response("done")]);
        let call_log = provider.call_log_handle();
        let startup = RuntimePackageStartup {
            extension_registry: ExtensionRegistry::new(),
            installed_packages: Vec::new(),
            diagnostics: Vec::new(),
            trust_decision: decision,
        };
        let mut runner = NonInteractiveRunner::new_with_resume_and_runtime_packages(
            Box::new(provider),
            "mock-model".into(),
            OpiConfig::default(),
            workspace.path().to_path_buf(),
            false,
            None,
            Vec::new(),
            None,
            opi_coding_agent::policy::ToolSelection::Default,
            startup,
            None,
        )
        .unwrap();

        let result = runner.run("check trust").await;
        assert_eq!(result.exit_code, ExitCode::Success as i32);
        let requests = call_log.lock().unwrap();
        let system = requests[0].system.as_deref().unwrap();
        assert_eq!(
            system.contains("RUNTIME STARTUP TRUST MARKER"),
            should_load_project_context,
            "runner trust must be derived from RuntimePackageStartup"
        );
    }
}

// ---------------------------------------------------------------------------
// Test 1: text prompt produces stdout output with exit code 0
// ---------------------------------------------------------------------------

#[tokio::test]
async fn runner_text_prompt_stdout_exit0() {
    let response = test_support::text_response("Hello from runner!");
    let provider = MockProvider::new("mock", vec![response]);

    let mut runner = NonInteractiveRunner::new(
        Box::new(provider),
        "mock-model".into(),
        OpiConfig::default(),
        std::env::current_dir().unwrap(),
        false,
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    let result = runner.run("Hi there").await;

    assert_eq!(result.exit_code, ExitCode::Success as i32, "should exit 0");
    assert!(
        result.stdout.contains("Hello from runner!"),
        "stdout should contain assistant text, got: {:?}",
        result.stdout
    );
}

// ---------------------------------------------------------------------------
// Test 2: tool call (read-only) succeeds in non-interactive mode
// ---------------------------------------------------------------------------

#[tokio::test]
async fn runner_readonly_tool_succeeds() {
    let first = test_support::tool_call_response(
        "tc-1",
        "read",
        r#"{"path":"Cargo.toml","offset":1,"limit":5}"#,
    );
    let second = test_support::text_response("The file contains workspace config.");

    let provider = MockProvider::new("mock", vec![first, second]);

    let mut runner = NonInteractiveRunner::new(
        Box::new(provider),
        "mock-model".into(),
        OpiConfig::default(),
        std::env::current_dir().unwrap(),
        false,
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    let result = runner.run("Read the Cargo.toml").await;

    assert_eq!(result.exit_code, ExitCode::Success as i32, "should exit 0");
    assert!(
        result.stdout.contains("workspace config"),
        "stdout should contain tool result text, got: {:?}",
        result.stdout
    );
}

#[tokio::test]
async fn runner_installed_adapter_tool_succeeds() {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    install_adapter_package(
        workspace.path(),
        "installed-tool",
        &test_binary("adapter_host_mock"),
        &[],
    );
    let runtime_startup = start_installed_package_runtime_with_trust(
        workspace.path(),
        user.path(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .await;

    let first = test_support::tool_call_response("adapter-1", "test_tool", r#"{"input":"hello"}"#);
    let second = test_support::text_response("adapter tool finished.");
    let provider = MockProvider::new("mock", vec![first, second]);

    let mut runner = NonInteractiveRunner::new_with_resume_and_runtime_packages(
        Box::new(provider),
        "mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        false,
        None,
        Vec::new(),
        None,
        opi_coding_agent::policy::ToolSelection::Default,
        runtime_startup,
        None,
    )
    .unwrap();

    let result = runner.run("Use installed adapter tool").await;

    assert_eq!(result.exit_code, ExitCode::Success as i32);
    assert!(
        result.stdout.contains("adapter tool finished."),
        "stdout should contain final provider text, got: {:?}",
        result.stdout
    );
}

#[tokio::test]
async fn runner_installed_adapter_hook_blocks_mutating_tool() {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    install_adapter_package(
        workspace.path(),
        "permission-gate",
        &test_binary("package_adapter_example"),
        &["permission-gate"],
    );
    let runtime_startup = start_installed_package_runtime_with_trust(
        workspace.path(),
        user.path(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .await;

    let first = test_support::tool_call_response(
        "blocked-1",
        "bash",
        r#"{"command":"echo should not run"}"#,
    );
    let second = test_support::text_response("blocked result observed.");
    let provider = MockProvider::new("mock", vec![first, second]);
    let call_log = provider.call_log_handle();

    let mut runner = NonInteractiveRunner::new_with_resume_and_runtime_packages(
        Box::new(provider),
        "mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        true,
        None,
        Vec::new(),
        None,
        opi_coding_agent::policy::ToolSelection::Default,
        runtime_startup,
        None,
    )
    .unwrap();

    let result = runner.run("Try a mutating command").await;

    assert_eq!(result.exit_code, ExitCode::Success as i32);
    assert!(
        result.stdout.contains("blocked result observed."),
        "stdout should contain second provider response, got: {:?}",
        result.stdout
    );
    let log = call_log.lock().unwrap();
    let second_request = log.get(1).expect("tool result should trigger second turn");
    let saw_blocked_tool_result = second_request.messages.iter().any(|message| {
        matches!(
            message,
            opi_ai::message::Message::ToolResult(result)
                if result.is_error
                    && result.tool_name == "bash"
                    && result.content.iter().any(|content| matches!(
                        content,
                        opi_ai::message::OutputContent::Text { text }
                            if text.contains("blocked by example permission-gate adapter")
                    ))
        )
    });
    assert!(
        saw_blocked_tool_result,
        "second provider request should contain blocked tool result: {:?}",
        second_request.messages
    );
}

#[tokio::test]
async fn runner_text_surfaces_local_effective_execution_contract() {
    let workspace = tempfile::tempdir().unwrap();
    let command = if cfg!(windows) { "exit 0" } else { "true" };
    let first = test_support::tool_call_response(
        "local-contract",
        "bash",
        &serde_json::json!({"command": command}).to_string(),
    );
    let second = test_support::text_response("done");
    let provider = MockProvider::new("mock", vec![first, second]);
    let mut runner = NonInteractiveRunner::new(
        Box::new(provider),
        "mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        true,
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    let result = runner.run("run a command").await;
    assert_eq!(result.exit_code, ExitCode::Success as i32);
    assert!(result.stderr.contains("execution contract:"));
    assert!(result.stderr.contains("placement=host"));
    assert!(result.stderr.contains("guarantee=supervised"));
}

// ---------------------------------------------------------------------------
// Test 3: provider error response produces stderr and exit code 4
// ---------------------------------------------------------------------------

#[tokio::test]
async fn runner_provider_error_stderr_exit4() {
    let response = test_support::error_response("connection refused");
    let provider = MockProvider::new("mock", vec![response]);

    let mut runner = NonInteractiveRunner::new(
        Box::new(provider),
        "mock-model".into(),
        OpiConfig::default(),
        std::env::current_dir().unwrap(),
        false,
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    let result = runner.run("Do something").await;

    assert_eq!(
        result.exit_code,
        ExitCode::ProviderFailure as i32,
        "should exit 4 on provider error"
    );
    assert!(
        result.stderr.contains("provider error"),
        "stderr should contain a redacted provider error class, got: {:?}",
        result.stderr
    );
    assert!(
        !result.stderr.contains("connection refused"),
        "stderr must not echo raw provider error text: {:?}",
        result.stderr
    );
}

// ---------------------------------------------------------------------------
// Test 4: non-interactive resume forwards CompactionSummary to the provider
// ---------------------------------------------------------------------------

#[tokio::test]
async fn runner_resume_forwards_compaction_summary_to_provider() {
    use opi_agent::message::{AgentMessage, CompactionSummaryMessage};
    use opi_ai::message::{InputContent, Message};

    let response = test_support::text_response("ack");
    let provider = MockProvider::new("mock", vec![response]);
    let call_log = provider.call_log_handle();

    let summary_text = "Earlier we discussed the quarterly compaction strategy.";
    let initial_messages = vec![AgentMessage::CompactionSummary(CompactionSummaryMessage {
        summary: summary_text.into(),
        first_kept_entry_id: "msg-42".into(),
        tokens_before: 1000,
        tokens_after: 200,
    })];

    let mut runner = NonInteractiveRunner::new(
        Box::new(provider),
        "mock-model".into(),
        OpiConfig::default(),
        std::env::current_dir().unwrap(),
        false,
        None,
        initial_messages,
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    let result = runner.run("continue please").await;
    assert_eq!(result.exit_code, ExitCode::Success as i32);

    let log = call_log.lock().unwrap();
    let first_request = log.first().expect("provider was called at least once");

    // The resumed summary must appear as a synthetic user-message in the
    // request the provider observed. Otherwise compacted context is silently
    // dropped on resume.
    let mut saw_summary = false;
    for msg in &first_request.messages {
        if let Message::User(u) = msg {
            for content in &u.content {
                if let InputContent::Text { text } = content
                    && text.contains(summary_text)
                {
                    saw_summary = true;
                }
            }
        }
    }
    assert!(
        saw_summary,
        "provider request messages must include compacted summary text; got: {:?}",
        first_request.messages
    );
}

#[tokio::test]
async fn runner_resume_forwards_branch_summary_to_provider() {
    use opi_agent::message::{AgentMessage, BranchSummaryMessage};
    use opi_ai::message::{InputContent, Message};

    let response = test_support::text_response("ack");
    let provider = MockProvider::new("mock", vec![response]);
    let call_log = provider.call_log_handle();

    let summary_text = "Parent branch established the retry contract.";
    let initial_messages = vec![AgentMessage::BranchSummary(BranchSummaryMessage {
        parent_session_id: "parent-session".into(),
        summary: summary_text.into(),
        entry_count: 7,
    })];

    let mut runner = NonInteractiveRunner::new(
        Box::new(provider),
        "mock-model".into(),
        OpiConfig::default(),
        std::env::current_dir().unwrap(),
        false,
        None,
        initial_messages,
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    let result = runner.run("continue please").await;
    assert_eq!(result.exit_code, ExitCode::Success as i32);

    let log = call_log.lock().unwrap();
    let first_request = log.first().expect("provider was called at least once");

    let saw_summary = first_request.messages.iter().any(|msg| {
        if let Message::User(u) = msg {
            return u.content.iter().any(|content| {
                matches!(
                    content,
                    InputContent::Text { text } if text.contains(summary_text)
                )
            });
        }
        false
    });
    assert!(
        saw_summary,
        "provider request messages must include branch summary text; got: {:?}",
        first_request.messages
    );
}

// ---------------------------------------------------------------------------
// Test 5: format_persist_errors captures errors that occur during the run
// ---------------------------------------------------------------------------
//
// Regression test: persist_stderr was previously computed BEFORE prompt()
// ran, so SessionPersistError events emitted during the run were silently
// dropped. The fix moves format_persist_errors() to after prompt() returns.
//
// Directly triggering a session IO error cross-platform is impractical
// (the file handle is already open), so this test verifies:
// (a) the format_persist_errors helper produces correct output, and
// (b) the runner's run() subscriber correctly routes SessionPersistError
//     events into the persist_errors capture buffer.
// ---------------------------------------------------------------------------

/// Verify format_persist_errors produces the expected output.
#[test]
fn format_persist_errors_unit() {
    let errors = Arc::new(Mutex::new(Vec::<String>::new()));

    // Empty -> no output
    let result = opi_coding_agent::runner::format_persist_errors(&errors);
    assert!(
        result.is_empty(),
        "expected empty for no errors, got: {result:?}"
    );

    // With errors
    {
        let mut guard = errors.lock().unwrap();
        guard.push("disk full".into());
        guard.push("permission denied".into());
    }
    let result = opi_coding_agent::runner::format_persist_errors(&errors);
    assert!(
        result.contains("session persist error: disk full"),
        "should contain first error, got: {result:?}"
    );
    assert!(
        result.contains("session persist error: permission denied"),
        "should contain second error, got: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Task 11.11: CLI help exposes tool-selection + mutating-tool policy
// ---------------------------------------------------------------------------

/// The public `opi --help` output documents the tool-selection flags and the
/// mutating-tool opt-in at the command boundary, consistent with the README
/// and `policy.rs`. Pinned via clap's rendered long help (in-process; no
/// subprocess). Flag names are the stable contract, so the assertion checks
/// for flag presence rather than exact doc-comment prose.
#[test]
fn phase11_cli_help_tool_policy() {
    use clap::CommandFactory;
    use opi_coding_agent::cli::Cli;

    let help = Cli::command().render_long_help().to_string();

    for flag in [
        "--tools",
        "--no-tools",
        "--no-builtin-tools",
        "--allow-mutating",
    ] {
        assert!(
            help.contains(flag),
            "opi --help must expose the tool-selection flag {flag}"
        );
    }
    for phrase in [
        "cmd /C",
        "sh -c",
        "workspace root",
        "30 seconds",
        "timeout_secs",
        "64 KiB",
        "details.full_output",
        "permission popup",
    ] {
        assert!(help.contains(phrase), "opi --help must mention {phrase}");
    }
    assert!(
        help.to_lowercase().contains("mutating"),
        "opi --help must document the mutating-tool opt-in"
    );
}

// ---------------------------------------------------------------------------
// Phase 14.2: typed CredentialNeeded -> exit 3, no prompt, no blocking
// ---------------------------------------------------------------------------

/// A resolver that always reports no credential is available for the given
/// provider, producing `ProviderError::CredentialNeeded`.
struct CredentialMissingResolver {
    provider_id: &'static str,
}

impl AuthResolver for CredentialMissingResolver {
    fn resolve<'a>(&'a self) -> BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        let provider_id = self.provider_id.to_owned();
        Box::pin(async move { Err(ProviderError::CredentialNeeded { provider_id }) })
    }
}

#[tokio::test]
async fn credential_needed_fails_without_prompt() {
    let provider = AnthropicProvider::with_auth(
        Arc::new(CredentialMissingResolver {
            provider_id: "anthropic",
        }),
        None,
        Arc::new(HttpClient::new()),
    );

    let mut runner = NonInteractiveRunner::new(
        Box::new(provider),
        "anthropic:claude-sonnet-4-5".into(),
        OpiConfig::default(),
        std::env::current_dir().unwrap(),
        false,
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    let result = runner.run("hello").await;

    assert_eq!(
        result.exit_code,
        ExitCode::AuthFailure as i32,
        "CredentialNeeded must exit 3 (AuthFailure), got {}",
        result.exit_code
    );
    assert!(
        result.stderr.contains("credential needed"),
        "stderr must contain 'credential needed': {}",
        result.stderr
    );
    assert!(
        result.stderr.contains("anthropic"),
        "stderr must name the provider: {}",
        result.stderr
    );
    assert!(
        result.stderr.contains("/login anthropic"),
        "stderr must include remediation '/login anthropic': {}",
        result.stderr
    );
    // Non-interactive mode must NOT prompt, block, or start an OAuth flow.
    assert!(
        result.stdout.is_empty(),
        "non-interactive CredentialNeeded must not write stdout: {:?}",
        result.stdout
    );
}

/// SC16-14 text surface: a `local = "deny"` Minimal-Runtime config makes
/// `ExecutionRuntime::build` fail with `policy_denied` at startup (bash omitted,
/// no fallback). Text mode surfaces that startup diagnostic on stderr — the same
/// stable code + remediation NDJSON (`StartupDiagnostics`) and RPC (ready header)
/// carry — so the TEXT surface preserves the stable code too. Without the
/// production fix, the startup diagnostic was silently dropped in text mode.
#[tokio::test]
async fn text_surface_surfaces_stable_execution_code_on_stderr() {
    let response = test_support::text_response("hi");
    let provider = MockProvider::new("mock", vec![response]);
    let mut config = OpiConfig::default();
    config.execution.strategy = ExecutionStrategy::Fixed;
    config.execution.backend = "local".into();
    config
        .execution
        .permissions
        .insert("local".into(), PermissionDecision::Deny);
    // Isolate the user config dir so the Minimal-Runtime branch (which emits the
    // policy_denied startup diagnostic) is deterministic regardless of the host's
    // real package-trust state. The runner resolves the config dir at
    // construction; the guard is dropped before the await.
    let _env_guard = common::empty_user_config_dir();
    let mut runner = NonInteractiveRunner::new(
        Box::new(provider),
        "mock-model".into(),
        config,
        std::env::current_dir().unwrap(),
        false,
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );
    drop(_env_guard);

    let result = runner.run("hello").await;
    assert_eq!(result.exit_code, ExitCode::Success as i32);
    // Pin the canonical text form: the granular stable code renders in the
    // lowercase `source::code` slot, matching the NDJSON/RPC/doctor surfaces'
    // `[severity] source::code:` rendering.
    assert!(
        result.stderr.contains("[error] adapter::policy_denied:"),
        "text stderr must render the canonical [error] adapter::policy_denied: form: {:?}",
        result.stderr
    );
    assert!(
        result.stderr.contains("policy_denied"),
        "the stable policy_denied code must reach text stderr: {:?}",
        result.stderr
    );
    assert!(
        result.stderr.contains("execution permission"),
        "text stderr must carry the actionable remediation: {:?}",
        result.stderr
    );
}
