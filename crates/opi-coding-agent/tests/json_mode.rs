//! JSON mode contract tests (task 2.14).
//!
//! DoD: "one AgentSessionEvent JSON object per line to stdout,
//!       schema version field, contract tests for framing"
//!
//! The `session_summary` line is the `AgentSessionEvent::SessionSummary`
//! variant (renamed for wire compatibility); all stdout lines after the
//! header round-trip through `AgentSessionEvent`.

mod common;

use opi_agent::session::{SessionHeader, SessionWriter};
use opi_agent::session_event::AgentSessionEvent;
use opi_ai::provider::{Provider, ProviderError};
use opi_ai::test_support::{self, MockProvider, MockResponse};
use opi_coding_agent::config::{ExecutionStrategy, OpiConfig, PermissionDecision};
use opi_coding_agent::harness::ResumeInfo;
use opi_coding_agent::policy::ToolSelection;
use opi_coding_agent::runner::{ExitCode, NDJSON_SCHEMA_VERSION, NonInteractiveRunner};
use opi_coding_agent::runtime_packages::RuntimePackageStartup;

/// Parse NDJSON output into individual JSON values, one per line.
fn parse_ndjson(output: &str) -> Vec<serde_json::Value> {
    output
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| serde_json::from_str(line).unwrap_or_else(|_| panic!("invalid JSON: {line}")))
        .collect()
}

fn runner_with_isolated_session(
    provider: Box<dyn Provider>,
    model: &str,
    session_dir: &tempfile::TempDir,
    runtime_startup: Option<RuntimePackageStartup>,
) -> NonInteractiveRunner {
    let workspace = std::env::current_dir().expect("workspace cwd");
    let session_id = "json-mode-summary".to_owned();
    let session_path = session_dir.path().join(format!("{session_id}.jsonl"));
    let header = SessionHeader::new(
        session_id.clone(),
        "2026-07-14T00:00:00Z".into(),
        workspace.display().to_string(),
        None,
    );
    SessionWriter::create(&session_path, header).expect("create isolated session");
    let resume = ResumeInfo {
        path: session_path,
        session_id,
        entries: Vec::new(),
        original_cwd: workspace.clone(),
        diagnostics: Vec::new(),
        recorded_model: None,
        recorded_thinking: None,
    };

    match runtime_startup {
        Some(startup) => NonInteractiveRunner::new_with_resume_and_runtime_packages(
            provider,
            model.into(),
            OpiConfig::default(),
            workspace,
            false,
            None,
            Vec::new(),
            Some(resume),
            ToolSelection::Default,
            startup,
            None,
        ),
        None => NonInteractiveRunner::new_with_resume(
            provider,
            model.into(),
            OpiConfig::default(),
            workspace,
            false,
            None,
            Vec::new(),
            Some(resume),
            ToolSelection::Default,
            opi_coding_agent::project_trust::TrustDecision::Trusted,
        ),
    }
    .expect("isolated non-interactive runner")
}

/// Phase 11.8 D2 wire-compat: a `ToolExecutionEnd` with empty diagnostics omits
/// the `diagnostics` key (`skip_serializing_if`), an old payload without the
/// field round-trips back via `#[serde(default)]`, and populated diagnostics
/// serialize as `{code,message,context}` entries.
#[test]
fn tool_execution_end_diagnostics_field_is_wire_compat() {
    use opi_agent::event::AgentEvent;
    use opi_agent::tool::ToolDiagnostic;

    // Empty diagnostics: the key is omitted on the wire.
    let with_empty = AgentEvent::ToolExecutionEnd {
        tool_call_id: "c1".into(),
        tool_name: "read".into(),
        result: serde_json::json!([]),
        details: None,
        is_error: false,
        truncated: false,
        diagnostics: Vec::new(),
    };
    let json = serde_json::to_string(&with_empty).expect("serializes");
    assert!(
        !json.contains("\"diagnostics\""),
        "empty diagnostics must be omitted (skip_serializing_if): {json}"
    );

    // Old payload (no diagnostics field) deserializes via #[serde(default)].
    let old = r#"{"type":"ToolExecutionEnd","tool_call_id":"c2","tool_name":"read","result":[],"details":null,"is_error":false,"truncated":false}"#;
    let back: AgentEvent = serde_json::from_str(old).expect("old payload round-trips");
    match back {
        AgentEvent::ToolExecutionEnd { diagnostics, .. } => {
            assert!(diagnostics.is_empty(), "defaults empty for old payload");
        }
        other => panic!("expected ToolExecutionEnd, got {other:?}"),
    }

    // Populated diagnostics serialize as an array of {code,message,context}.
    let with_diag = AgentEvent::ToolExecutionEnd {
        tool_call_id: "c3".into(),
        tool_name: "bash".into(),
        result: serde_json::json!([]),
        details: None,
        is_error: true,
        truncated: false,
        diagnostics: vec![ToolDiagnostic {
            code: "tool_execution_failed".into(),
            message: "command exited non-zero".into(),
            context: serde_json::json!({ "exit_code": 1 }),
        }],
    };
    let v: serde_json::Value = serde_json::to_value(&with_diag).expect("serializes");
    assert_eq!(v["diagnostics"][0]["code"], "tool_execution_failed");
    assert_eq!(v["diagnostics"][0]["context"]["exit_code"], 1);
}

// ---------------------------------------------------------------------------
// SC16-14 cross-surface: a stable execution-failure code reaches the NDJSON wire
// ---------------------------------------------------------------------------

/// Headless fixed-local `ask` is refused while the harness is built, without a
/// prompt. NDJSON carries the stable `permission_required` startup diagnostic
/// with its headless-specific remediation intact (bash omitted, no fallback).
#[tokio::test]
async fn ndjson_refuses_local_ask_at_startup_without_prompt() {
    let response = test_support::text_response("hi");
    let provider = MockProvider::new("mock", vec![response]);
    let call_log = provider.call_log_handle();
    let mut config = OpiConfig::default();
    config.execution.strategy = ExecutionStrategy::Fixed;
    config.execution.backend = "local".into();
    config
        .execution
        .permissions
        .insert("local".into(), PermissionDecision::Ask);
    // The runner resolves this refusal synchronously during construction. The
    // isolated config dir makes the result independent of host package state.
    let _env_guard = common::empty_user_config_dir();
    let mut runner = NonInteractiveRunner::new(
        Box::new(provider),
        "mock-model".into(),
        config,
        std::env::current_dir().unwrap(),
        true,
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );
    drop(_env_guard);

    let result = tokio::time::timeout(std::time::Duration::from_secs(2), runner.run_json("hello"))
        .await
        .expect("headless local ask must not wait for a permission prompt");
    assert_eq!(result.exit_code, ExitCode::Success as i32);
    let calls = call_log.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert!(
        calls[0].tools.iter().all(|tool| tool.name != "bash"),
        "headless ask must omit bash instead of falling back to local: {:?}",
        calls[0]
            .tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>()
    );

    let lines = parse_ndjson(&result.stdout);
    let startup = lines
        .iter()
        .find(|line| line["type"] == "StartupDiagnostics")
        .expect("a StartupDiagnostics line must be emitted");
    let diags = startup["diagnostics"]
        .as_array()
        .expect("startup diagnostics array");
    let permission_required = diags
        .iter()
        .find(|d| d["details"]["code"] == "permission_required");
    assert!(
        permission_required.is_some(),
        "the stable permission_required code must surface in NDJSON startup diagnostics: {startup}"
    );
    let diagnostic = permission_required.expect("found");
    let remediation = diagnostic["details"]["remediation"]
        .as_str()
        .unwrap_or_default();
    assert!(
        !remediation.is_empty(),
        "remediation text must ride along and be actionable: {diagnostic}"
    );
    assert!(
        remediation.contains("cannot be granted non-interactively")
            && remediation.contains("run interactively"),
        "remediation must carry the headless-specific actionable fragment: {remediation}"
    );
}

// ---------------------------------------------------------------------------
// Schema version header
// ---------------------------------------------------------------------------

#[tokio::test]
async fn json_mode_schema_version_header() {
    let response = test_support::text_response("hi");
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

    let result = runner.run_json("hello").await;
    assert_eq!(result.exit_code, ExitCode::Success as i32);

    let lines = parse_ndjson(&result.stdout);
    assert!(!lines.is_empty(), "should have at least a header line");

    let header = &lines[0];
    assert_eq!(header["type"], "session_header");
    assert_eq!(header["schema_version"], NDJSON_SCHEMA_VERSION);
}

// ---------------------------------------------------------------------------
// Each line is valid JSON with a type field
// ---------------------------------------------------------------------------

#[tokio::test]
async fn json_mode_each_line_valid_json_with_type() {
    let response = test_support::text_response("hello world");
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

    let result = runner.run_json("test").await;
    assert_eq!(result.exit_code, ExitCode::Success as i32);

    let lines = parse_ndjson(&result.stdout);
    assert!(lines.len() > 1, "should have header + at least one event");

    for (i, line) in lines.iter().enumerate() {
        assert!(
            line.get("type").is_some(),
            "line {i} missing 'type' field: {line}"
        );
    }
}

// ---------------------------------------------------------------------------
// Agent events are wrapped in AgentSessionEvent::Agent
// ---------------------------------------------------------------------------

#[tokio::test]
async fn json_mode_agent_events_emitted() {
    let response = test_support::text_response("response text");
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

    let result = runner.run_json("prompt").await;
    assert_eq!(result.exit_code, ExitCode::Success as i32);

    let lines = parse_ndjson(&result.stdout);

    // After header, all lines should be valid AgentSessionEvent::Agent
    let agent_events: Vec<_> = lines[1..].iter().filter(|v| v["type"] == "Agent").collect();
    assert!(
        !agent_events.is_empty(),
        "should have at least one Agent event"
    );
}

// ---------------------------------------------------------------------------
// AgentSessionEvent round-trip deserialization
// ---------------------------------------------------------------------------

#[tokio::test]
async fn json_mode_events_deserialize_as_session_events() {
    let response = test_support::text_response("hello");
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

    let result = runner.run_json("test").await;
    assert_eq!(result.exit_code, ExitCode::Success as i32);

    // Every line after the schema header should deserialize as an
    // AgentSessionEvent. The wire contract is "one AgentSessionEvent per line",
    // so the session_summary line is now part of the union — no special-casing.
    for line in result.stdout.lines().skip(1) {
        if line.is_empty() {
            continue;
        }
        let parsed: Result<AgentSessionEvent, _> = serde_json::from_str(line);
        assert!(
            parsed.is_ok(),
            "failed to deserialize as AgentSessionEvent: {line}: {:?}",
            parsed.err(),
        );
    }
}

// ---------------------------------------------------------------------------
// NDJSON framing: no blank lines between events
// ---------------------------------------------------------------------------

#[tokio::test]
async fn json_mode_no_blank_lines() {
    let response = test_support::text_response("ok");
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

    let result = runner.run_json("test").await;

    for (i, line) in result.stdout.lines().enumerate() {
        assert!(
            !line.trim().is_empty(),
            "line {i} is blank — NDJSON framing violation"
        );
    }
}

// ---------------------------------------------------------------------------
// Provider error still emits events with proper exit code
// ---------------------------------------------------------------------------

#[tokio::test]
async fn json_mode_provider_error_exit_code() {
    let response = test_support::error_response("rate limited");
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

    let result = runner.run_json("test").await;

    assert_eq!(
        result.exit_code,
        ExitCode::ProviderFailure as i32,
        "should exit 4 on provider error"
    );
    // Error info goes to stderr, not stdout
    assert!(
        result.stderr.contains("provider error"),
        "stderr should contain a redacted provider error class: {:?}",
        result.stderr
    );
    // Still should have header even on error
    let lines = parse_ndjson(&result.stdout);
    assert_eq!(lines[0]["type"], "session_header");
}

#[tokio::test]
async fn json_mode_provider_error_stderr_is_redacted() {
    let secret = "sk-proj-1234567890abcdefghijklmnopqrstuv";
    let provider = MockProvider::new_with_errors(
        "mock",
        vec![MockResponse::Error(ProviderError::RequestFailed(format!(
            "HTTP 500: body contained {secret} at C:\\Users\\alice\\.config\\opi\\config.toml"
        )))],
    );
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

    let result = runner.run_json("test").await;

    assert_eq!(result.exit_code, ExitCode::ProviderFailure as i32);
    assert!(
        !result.stderr.contains(secret),
        "stderr leaked provider secret: {}",
        result.stderr
    );
    assert!(
        !result.stderr.contains("alice"),
        "stderr leaked absolute path user component: {}",
        result.stderr
    );
    assert!(
        result.stderr.contains("provider error"),
        "stderr should retain a useful static error class: {}",
        result.stderr
    );
}

#[tokio::test]
async fn json_mode_credential_needed_emits_typed_remediation_without_prompt() {
    let provider = MockProvider::new_with_errors(
        "anthropic",
        vec![MockResponse::Error(ProviderError::CredentialNeeded {
            provider_id: "anthropic".into(),
        })],
    );
    let call_log = provider.call_log_handle();
    let mut runner = NonInteractiveRunner::new(
        Box::new(provider),
        "anthropic:mock-model".into(),
        OpiConfig::default(),
        std::env::current_dir().unwrap(),
        false,
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    let result = tokio::time::timeout(std::time::Duration::from_secs(2), runner.run_json("hello"))
        .await
        .expect("JSON credential remediation must not prompt or block");

    assert_eq!(result.exit_code, ExitCode::AuthFailure as i32);
    let lines = parse_ndjson(&result.stdout);
    assert_eq!(lines[0]["schema_version"], NDJSON_SCHEMA_VERSION);
    let raw_remediations: Vec<_> = result
        .stdout
        .lines()
        .filter(|line| line.contains("\"type\":\"CredentialNeeded\""))
        .collect();
    assert_eq!(
        raw_remediations.len(),
        1,
        "expected exactly one raw CredentialNeeded remediation line"
    );
    println!("{}", raw_remediations[0]);
    let remediation_events: Vec<_> = lines
        .iter()
        .filter(|line| line["type"] == "CredentialNeeded")
        .collect();
    assert_eq!(
        remediation_events.len(),
        1,
        "expected exactly one typed CredentialNeeded remediation record"
    );
    let remediation = remediation_events[0];
    let typed: AgentSessionEvent = serde_json::from_value(remediation.clone())
        .expect("credential remediation remains inside AgentSessionEvent");
    match typed {
        AgentSessionEvent::CredentialNeeded {
            provider_id,
            remediation,
            diagnostic,
        } => {
            assert_eq!(provider_id, "anthropic");
            assert_eq!(remediation, "/login anthropic");
            assert_eq!(diagnostic.code, "provider_credential_needed");
        }
        other => panic!("unexpected remediation event: {other:?}"),
    }
    assert_eq!(call_log.lock().unwrap().len(), 1, "no automatic retry");
}

#[tokio::test]
async fn json_mode_account_id_missing_emits_typed_remediation_without_prompt() {
    // C-3.2: an AccountIdMissing auth failure (e.g. a Codex token lacking
    // chatgpt_account_id) must surface in JSON mode as a typed /login
    // remediation with an AuthFailure exit code, not a generic provider error.
    let provider = MockProvider::new_with_errors(
        "openai-codex",
        vec![MockResponse::Error(ProviderError::AccountIdMissing {
            provider_id: "openai-codex".into(),
        })],
    );
    let call_log = provider.call_log_handle();
    let mut runner = NonInteractiveRunner::new(
        Box::new(provider),
        "openai-codex:mock-model".into(),
        OpiConfig::default(),
        std::env::current_dir().unwrap(),
        false,
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    let result = tokio::time::timeout(std::time::Duration::from_secs(2), runner.run_json("hello"))
        .await
        .expect("JSON account-id-missing remediation must not prompt or block");

    assert_eq!(result.exit_code, ExitCode::AuthFailure as i32);
    let lines = parse_ndjson(&result.stdout);
    assert_eq!(lines[0]["schema_version"], NDJSON_SCHEMA_VERSION);
    let raw_remediations: Vec<_> = result
        .stdout
        .lines()
        .filter(|line| line.contains("\"type\":\"CredentialNeeded\""))
        .collect();
    assert_eq!(
        raw_remediations.len(),
        1,
        "expected exactly one raw CredentialNeeded remediation line"
    );
    let remediation_events: Vec<_> = lines
        .iter()
        .filter(|line| line["type"] == "CredentialNeeded")
        .collect();
    assert_eq!(
        remediation_events.len(),
        1,
        "expected exactly one typed CredentialNeeded remediation record"
    );
    let remediation = remediation_events[0];
    let typed: AgentSessionEvent = serde_json::from_value(remediation.clone())
        .expect("credential remediation remains inside AgentSessionEvent");
    match typed {
        AgentSessionEvent::CredentialNeeded {
            provider_id,
            remediation,
            diagnostic,
        } => {
            assert_eq!(provider_id, "openai-codex");
            assert_eq!(remediation, "/login openai-codex");
            assert_eq!(diagnostic.code, "provider_auth_failed");
        }
        other => panic!("unexpected remediation event: {other:?}"),
    }
    assert_eq!(call_log.lock().unwrap().len(), 1, "no automatic retry");
}

/// Phase 12 task 12.2 — provider errors visible through the non-interactive
/// NDJSON public surface must not leak credentials. A retryable `Network`
/// error carrying a secret triggers an `AutoRetryStart` event whose
/// `error_message` is the provider-error string; the public emit boundary
/// (`AgentEvent::redacted_for_public`) must scrub it before it reaches stdout.
#[tokio::test]
async fn provider_errors_are_redacted() {
    let secret = "sk-proj-1234567890abcdefghijklmnopqrstuv";
    // Retryable Network error carrying a secret -> AutoRetryStart before the
    // final failure. Default RetryConfig retries, so an AutoRetry event fires.
    let make_err = || {
        MockResponse::Error(ProviderError::Network(format!(
            "conn reset; echoed key {secret}"
        )))
    };
    let provider = MockProvider::new_with_errors(
        "mock",
        vec![
            make_err(),
            make_err(),
            make_err(),
            make_err(),
            make_err(),
            make_err(),
        ],
    );
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

    let result = runner.run_json("test").await;

    assert_eq!(result.exit_code, ExitCode::ProviderFailure as i32);
    assert!(
        !result.stdout.contains(secret),
        "stdout NDJSON leaked provider secret via an AutoRetry event: {}",
        result.stdout
    );
    assert!(
        !result.stderr.contains(secret),
        "stderr leaked provider secret: {}",
        result.stderr
    );
}

// ---------------------------------------------------------------------------
// Tool call events emitted in JSON mode
// ---------------------------------------------------------------------------

#[tokio::test]
async fn json_mode_tool_call_events() {
    let first = test_support::tool_call_response(
        "tc-1",
        "read",
        r#"{"path":"Cargo.toml","offset":1,"limit":5}"#,
    );
    let second = test_support::text_response("file contents here");
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

    let result = runner.run_json("Read Cargo.toml").await;
    assert_eq!(result.exit_code, ExitCode::Success as i32);

    let lines = parse_ndjson(&result.stdout);
    // Should have tool execution events
    let tool_events: Vec<_> = lines[1..]
        .iter()
        .filter(|v| {
            let evt = &v["event"];
            evt.get("type")
                .map(|t| t.as_str().unwrap_or("").starts_with("ToolExecution"))
                .unwrap_or(false)
        })
        .collect();
    assert!(!tool_events.is_empty(), "should have tool execution events");
}

// ---------------------------------------------------------------------------
// Phase 11.4: write audit details cross the NDJSON output boundary.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn write_tool_result_carries_write_audit_details() {
    let first = test_support::tool_call_response(
        "tc-write",
        "write",
        r#"{"path":"out.txt","content":"payload"}"#,
    );
    let second = test_support::text_response("done");
    let provider = MockProvider::new("mock", vec![first, second]);

    let workspace = std::env::temp_dir().join(format!("opi-write-json-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&workspace);
    let mut runner = NonInteractiveRunner::new(
        Box::new(provider),
        "mock-model".into(),
        OpiConfig::default(),
        workspace.clone(),
        true, // allow_mutating: write must be executable
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    let result = runner.run_json("Write out.txt").await;
    assert_eq!(result.exit_code, ExitCode::Success as i32);

    let lines = parse_ndjson(&result.stdout);
    let end = lines
        .iter()
        .find(|v| v["event"]["type"] == "ToolExecutionEnd" && v["event"]["tool_name"] == "write")
        .expect("expected a write ToolExecutionEnd event in the NDJSON stream");

    assert_eq!(end["event"]["is_error"], false);
    assert_eq!(end["event"]["truncated"], false);
    let details = &end["event"]["details"];
    assert!(
        details.is_object(),
        "details must be an object, got: {details}"
    );
    assert_eq!(details["action"], "created");
    assert_eq!(details["bytes_written"], 7); // "payload" == 7 bytes

    let _ = std::fs::remove_file(workspace.join("out.txt"));
    let _ = std::fs::remove_dir_all(&workspace);
}

// ---------------------------------------------------------------------------
// run_json does not duplicate stdout text (no plain text mixed in)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn json_mode_stdout_is_only_ndjson() {
    let response = test_support::text_response("plain text response");
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

    let result = runner.run_json("test").await;

    // Every line should be valid JSON (not raw text)
    for (i, line) in result.stdout.lines().enumerate() {
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(line);
        assert!(parsed.is_ok(), "line {i} is not valid JSON: {line}");
    }
}

#[tokio::test]
async fn json_mode_emits_session_summary_with_token_totals() {
    let response = test_support::text_response("hi");
    // Phase 17.5: the harness dispatches via ProviderCollection::prepare_call,
    // which strictly resolves the `provider:model` spec. Register the asserted
    // model under provider id "anthropic" so "anthropic:claude-sonnet-4" resolves
    // and the session summary records exactly that canonical spec.
    let provider = MockProvider::new_with_models(
        "anthropic",
        vec![opi_ai::provider::ModelInfo::new(
            "claude-sonnet-4",
            "Claude Sonnet 4",
            opi_ai::model_info::WireApi::OpenAiCompletions,
            opi_ai::registry::ModelCapabilities::new(100_000, 4_096)
                .with_images(true)
                .with_streaming(true),
        )],
        vec![response],
    );
    let session_dir = tempfile::tempdir().expect("session dir");
    let mut runner = runner_with_isolated_session(
        Box::new(provider),
        "anthropic:claude-sonnet-4",
        &session_dir,
        None,
    );

    let result = runner.run_json("test").await;
    assert_eq!(result.exit_code, ExitCode::Success as i32);

    let parsed = parse_ndjson(&result.stdout);
    let summary = parsed
        .iter()
        .find(|v| v.get("type").and_then(|t| t.as_str()) == Some("session_summary"))
        .expect("session_summary line should be emitted");

    assert!(
        summary.get("session_id").is_some(),
        "summary has session_id"
    );
    assert!(summary.get("turns").is_some(), "summary has turn count");
    assert!(summary.get("tokens").is_some(), "summary has token totals");
    assert_eq!(
        summary
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or_default(),
        "anthropic:claude-sonnet-4"
    );
}

#[tokio::test]
async fn json_mode_session_summary_roundtrips_through_agent_session_event() {
    // The session_summary line must be the AgentSessionEvent::SessionSummary
    // variant — not an ad-hoc JSON shape. Consumers parsing the NDJSON stream
    // as a sequence of AgentSessionEvent values rely on this.
    let response = test_support::text_response("hi");
    // Phase 17.5: register the asserted model under provider id "anthropic" so
    // prepare_call strictly resolves "anthropic:claude-sonnet-4" and the summary
    // records that canonical spec.
    let provider = MockProvider::new_with_models(
        "anthropic",
        vec![opi_ai::provider::ModelInfo::new(
            "claude-sonnet-4",
            "Claude Sonnet 4",
            opi_ai::model_info::WireApi::OpenAiCompletions,
            opi_ai::registry::ModelCapabilities::new(100_000, 4_096)
                .with_images(true)
                .with_streaming(true),
        )],
        vec![response],
    );
    let session_dir = tempfile::tempdir().expect("session dir");
    let mut runner = runner_with_isolated_session(
        Box::new(provider),
        "anthropic:claude-sonnet-4",
        &session_dir,
        None,
    );

    let result = runner.run_json("test").await;
    assert_eq!(result.exit_code, ExitCode::Success as i32);

    let summary_line = result
        .stdout
        .lines()
        .find(|l| l.contains(r#""type":"session_summary""#))
        .expect("session_summary line emitted");

    let parsed: AgentSessionEvent = serde_json::from_str(summary_line)
        .unwrap_or_else(|e| panic!("session_summary line must round-trip: {e}: {summary_line}"));
    match parsed {
        AgentSessionEvent::SessionSummary {
            ref model, turns, ..
        } => {
            assert_eq!(model, "anthropic:claude-sonnet-4");
            assert!(turns >= 1, "turns should advance after a successful run");
        }
        other => panic!("expected SessionSummary, got {other:?}"),
    }
}

// ===========================================================================
// Phase 7 task 7.5 — JSON exposure: startup diagnostics, run-summary
// diagnostic counts, and the versioned redacted trace envelope.
// ===========================================================================

mod phase7 {
    use super::parse_ndjson;
    use opi_agent::TRACE_SCHEMA_VERSION;
    use opi_agent::diagnostic::{Diagnostic, SOURCE_PACKAGE, SOURCE_SESSION, Severity, code};
    use opi_agent::extension::ExtensionRegistry;
    use opi_agent::session_event::AgentSessionEvent;
    use opi_ai::provider::{Provider, ProviderError};
    use opi_ai::test_support::{self, MockProvider, MockResponse};
    use opi_coding_agent::config::OpiConfig;
    use opi_coding_agent::harness::ResumeInfo;
    use opi_coding_agent::policy::ToolSelection;
    use opi_coding_agent::runner::{ExitCode, NonInteractiveRunner};
    use opi_coding_agent::runtime_packages::RuntimePackageStartup;

    fn workspace_root() -> std::path::PathBuf {
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
    }

    fn runner_with_startup(
        provider: Box<dyn Provider>,
        diagnostics: Vec<Diagnostic>,
        trace_path: Option<std::path::PathBuf>,
    ) -> NonInteractiveRunner {
        NonInteractiveRunner::new_with_resume_and_runtime_packages(
            provider,
            "mock-model".into(),
            OpiConfig::default(),
            workspace_root(),
            false,
            None,
            Vec::new(),
            None,
            ToolSelection::Default,
            RuntimePackageStartup {
                extension_registry: ExtensionRegistry::new(),
                installed_packages: Vec::new(),
                diagnostics,
                trust_decision: opi_coding_agent::project_trust::TrustDecision::Trusted,
            },
            trace_path,
        )
        .expect("non-interactive tool policy should be valid")
    }

    /// Startup diagnostics appear before accepted prompt output (clause 1).
    #[tokio::test]
    async fn phase7_startup_diagnostics_and_counts() {
        let provider = MockProvider::new("mock", vec![test_support::text_response("hi")]);
        let startup_diag = Diagnostic::new(
            Severity::Warning,
            code::CODE_PACKAGE_DIAGNOSTIC,
            SOURCE_PACKAGE,
            "phase7 startup warn",
        );
        let mut runner = runner_with_startup(Box::new(provider), vec![startup_diag], None);
        let result = runner.run_json("hello").await;
        assert_eq!(result.exit_code, ExitCode::Success as i32);

        let lines = parse_ndjson(&result.stdout);
        // lines[0] is the session_header; the startup diagnostics line must
        // follow it and precede the first Agent event.
        assert_eq!(lines[0]["type"], "session_header", "first line is header");
        assert_eq!(
            lines[1]["type"], "StartupDiagnostics",
            "second line must be startup diagnostics"
        );
        assert_eq!(
            lines[1]["diagnostics"][0]["message"], "phase7 startup warn",
            "startup diagnostic carried as a structured payload"
        );
        assert_eq!(
            lines[1]["diagnostics"][0]["code"],
            code::CODE_PACKAGE_DIAGNOSTIC
        );
        let agent_idx = lines
            .iter()
            .position(|l| l["type"] == "Agent")
            .expect("at least one Agent event");
        assert!(
            agent_idx > 1,
            "startup diagnostics must precede the first Agent event"
        );
    }

    #[tokio::test]
    async fn phase7_resume_diagnostics_are_startup_diagnostics() {
        let provider = MockProvider::new("mock", vec![test_support::text_response("hi")]);
        let workspace = tempfile::tempdir().expect("workspace tempdir");
        let resume_info = ResumeInfo {
            path: workspace.path().join("resume.jsonl"),
            session_id: "resume".into(),
            entries: Vec::new(),
            original_cwd: workspace.path().to_path_buf(),
            diagnostics: vec![Diagnostic::new(
                Severity::Warning,
                code::CODE_SESSION_TRUNCATED_LINE,
                SOURCE_SESSION,
                "session file ended with a truncated line",
            )],
            recorded_model: None,
            recorded_thinking: None,
        };
        let mut runner = NonInteractiveRunner::new_with_resume_and_runtime_packages(
            Box::new(provider),
            "mock-model".into(),
            OpiConfig::default(),
            workspace.path().to_path_buf(),
            false,
            None,
            Vec::new(),
            Some(resume_info),
            ToolSelection::Default,
            RuntimePackageStartup {
                extension_registry: ExtensionRegistry::new(),
                installed_packages: Vec::new(),
                diagnostics: Vec::new(),
                trust_decision: opi_coding_agent::project_trust::TrustDecision::Trusted,
            },
            None,
        )
        .expect("non-interactive runner");

        let result = runner.run_json("hello").await;
        assert_eq!(result.exit_code, ExitCode::Success as i32);
        let lines = parse_ndjson(&result.stdout);
        let diagnostics = lines[1]["diagnostics"]
            .as_array()
            .expect("startup diagnostics array");
        assert!(
            diagnostics
                .iter()
                .any(|d| d["code"] == code::CODE_SESSION_TRUNCATED_LINE
                    && d["source"] == SOURCE_SESSION),
            "resume recovery diagnostic should be emitted as startup diagnostics: {diagnostics:?}"
        );
    }

    /// Phase 11.8 S4: JSON/NDJSON output exposes tool result details,
    /// diagnostics, is_error, and truncated for a failing tool result. bash
    /// nonzero-exit carries operation details AND a tool-owned diagnostic, so
    /// all four fields are observable in one result.
    #[tokio::test]
    async fn tool_result_details_diagnostics_and_truncated_shape() {
        let cmd = if cfg!(windows) {
            "cmd /C exit 1"
        } else {
            "exit 1"
        };
        let args =
            serde_json::to_string(&serde_json::json!({ "command": cmd })).expect("args serialize");
        let provider = MockProvider::new(
            "mock",
            vec![
                test_support::tool_call_response("c1", "bash", &args),
                test_support::text_response("done"),
            ],
        );
        let mut runner = NonInteractiveRunner::new(
            Box::new(provider),
            "mock-model".into(),
            OpiConfig::default(),
            workspace_root(),
            true, // allow_mutating so bash is an active built-in
            None,
            Vec::new(),
            opi_coding_agent::project_trust::TrustDecision::Trusted,
        );
        let result = runner.run_json("run it").await;
        assert_eq!(
            result.exit_code,
            ExitCode::Success as i32,
            "stderr: {}",
            result.stderr
        );

        let lines = parse_ndjson(&result.stdout);
        let tee = lines
            .iter()
            .filter_map(|l| {
                if l["type"] == "Agent" && l["event"]["type"] == "ToolExecutionEnd" {
                    Some(&l["event"])
                } else {
                    None
                }
            })
            .next()
            .expect("a ToolExecutionEnd event on the NDJSON stream");
        assert_eq!(tee["is_error"], true, "nonzero exit is_error: {tee}");
        assert!(
            tee["details"].is_object(),
            "operation details present (command/exit_code/...): {tee}"
        );
        assert_eq!(tee["truncated"], false, "no truncation: {tee}");
        let diags = tee["diagnostics"]
            .as_array()
            .expect("diagnostics array present on ToolExecutionEnd (Phase 11.8 D2)");
        assert!(
            diags
                .iter()
                .any(|d| d["code"] == "tool_execution_failed" && d["context"]["exit_code"] == 1),
            "bash operation diagnostic (exit_code=1) surfaces in NDJSON: {diags:?}"
        );
    }

    /// Run summary carries structured diagnostic counts (clause 2).
    #[tokio::test]
    async fn phase7_run_summary_carries_diagnostic_counts() {
        // One retryable error then success: emits a Warning retry-attempt and
        // an Info retry-succeeded diagnostic, which must aggregate into counts.
        let provider = MockProvider::new_with_errors(
            "mock",
            vec![
                MockResponse::Error(ProviderError::RateLimited {
                    retry_after_ms: Some(1),
                }),
                MockResponse::Events(test_support::text_response("ok")),
            ],
        );
        let session_dir = tempfile::tempdir().expect("session dir");
        let mut runner = super::runner_with_isolated_session(
            Box::new(provider),
            "mock-model",
            &session_dir,
            Some(RuntimePackageStartup {
                extension_registry: ExtensionRegistry::new(),
                installed_packages: Vec::new(),
                diagnostics: Vec::new(),
                trust_decision: opi_coding_agent::project_trust::TrustDecision::Trusted,
            }),
        );
        let result = runner.run_json("hello").await;
        assert_eq!(result.exit_code, ExitCode::Success as i32);

        let summary_line = result
            .stdout
            .lines()
            .find(|l| l.contains(r#""type":"session_summary""#))
            .expect("session_summary line emitted");
        let parsed: AgentSessionEvent = serde_json::from_str(summary_line).unwrap();
        match parsed {
            AgentSessionEvent::SessionSummary {
                diagnostics: Some(counts),
                ..
            } => {
                assert!(
                    counts.warning >= 1,
                    "expected >=1 warning (retry attempt), got {}",
                    counts.warning
                );
                assert!(
                    counts.info >= 1,
                    "expected >=1 info (retry succeeded), got {}",
                    counts.info
                );
            }
            AgentSessionEvent::SessionSummary {
                diagnostics: None, ..
            } => {
                panic!("run summary must carry diagnostic counts")
            }
            other => panic!("expected SessionSummary, got {other:?}"),
        }
    }

    /// The requested trace envelope is versioned and does not leak the prompt
    /// (clause 6; redaction applied at the trace emit boundary).
    #[tokio::test]
    async fn phase7_trace_envelope_versioned_and_no_prompt_leak() {
        let dir = tempfile::tempdir().expect("tempdir");
        let trace_path = dir.path().join("trace.jsonl");
        let secret = "sk-ant-AAAAAAAAAAAAAAAAAAAAleak";
        let prompt = format!("my secret plan {secret}");

        let provider = MockProvider::new("mock", vec![test_support::text_response("hi")]);
        let mut runner =
            runner_with_startup(Box::new(provider), Vec::new(), Some(trace_path.clone()));
        let result = runner.run_json(&prompt).await;
        assert_eq!(result.exit_code, ExitCode::Success as i32);

        let contents =
            std::fs::read_to_string(&trace_path).expect("trace file written for the run");
        assert!(!contents.is_empty(), "trace envelope must be produced");
        let records: Vec<serde_json::Value> = contents
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| serde_json::from_str(l).expect("each line is a JSON record"))
            .collect();
        assert!(!records.is_empty(), "at least one trace record");
        for record in &records {
            assert_eq!(
                record["schema_version"],
                serde_json::json!(TRACE_SCHEMA_VERSION),
                "every trace record carries the unstable schema version"
            );
        }
        // No prompt leak: the prompt text and the secret-like token must not
        // appear anywhere in the trace envelope.
        assert!(
            !contents.contains(&prompt),
            "trace must not leak the prompt text"
        );
        assert!(
            !contents.contains(secret),
            "trace must not leak secret-like content"
        );
    }

    /// DoD SC6 (JSON trace surface): every sensitive class the shared redaction
    /// core must scrub — API keys, bearer/JWT, GitHub tokens, and credentialed
    /// URLs embedded in the prompt — is absent from the requested trace
    /// envelope. The envelope carries only structural metadata by design, so
    /// this also guards against any future regression that attaches prompt
    /// content to a trace record.
    #[tokio::test]
    async fn phase7_json_trace_redacts_sensitive_values() {
        let dir = tempfile::tempdir().expect("tempdir");
        let trace_path = dir.path().join("trace.jsonl");
        let secrets = [
            "sk-ant-1234567890abcdefghijklmnopqrstuv",
            "ghp_01234567890123456789012345678901234567",
            "https://alice:s3cr3t@gitlab.example.com/o/r.git",
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIg.abc123def456",
        ];
        let prompt = format!(
            "rotate these now: {} {} {} {}",
            secrets[0], secrets[1], secrets[2], secrets[3]
        );

        let provider = MockProvider::new("mock", vec![test_support::text_response("done")]);
        let mut runner =
            runner_with_startup(Box::new(provider), Vec::new(), Some(trace_path.clone()));
        let result = runner.run_json(&prompt).await;
        assert_eq!(result.exit_code, ExitCode::Success as i32);

        let contents = std::fs::read_to_string(&trace_path).expect("trace file written");
        assert!(!contents.is_empty(), "trace envelope must be produced");
        for secret in secrets {
            assert!(
                !contents.contains(secret),
                "trace envelope leaked a sensitive value: {secret}\n--- trace ---\n{contents}",
            );
        }
        // The secrets must not appear in any diagnostic/details payload in the
        // run's NDJSON output either. (The prompt text itself is the user's own
        // input and is legitimately echoed in the conversation event stream, so
        // it is intentionally excluded from this redaction assertion.)
        for line in result.stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            // Only inspect structured diagnostic-bearing events, not the
            // conversation message stream.
            let ty = value.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if !matches!(
                ty,
                "StartupDiagnostics" | "session_summary" | "run_summary" | "RuntimeFailure"
            ) {
                continue;
            }
            let serialized = serde_json::to_string(&value).unwrap_or_default();
            for secret in secrets {
                assert!(
                    !serialized.contains(secret),
                    "JSON diagnostic event {ty:?} leaked a sensitive value: {secret}\n{serialized}",
                );
            }
        }
    }

    /// Phase 8 task 8.6 — closes the vacuous H5 guard on
    /// `StartupDiagnostics` redaction. The phase 7 redaction test seeded its
    /// secrets into the user prompt (not into the startup diagnostic), so it
    /// could not prove that a real `RuntimePackageStartup.diagnostics` payload
    /// is scrubbed before it reaches NDJSON. This test seeds every supported
    /// sensitive class — real-format API keys, a GitHub token, a credentialed
    /// URL, and a Windows absolute path — directly into a structured
    /// `Diagnostic` (across `message`, `details`, and `action`), then asserts
    /// the emitted `StartupDiagnostics` line is both redacted AND still a
    /// structured `Diagnostic` (carrying `code` / `source` / `severity`),
    /// proving the runtime did not collapse it to a free-text string.
    #[tokio::test]
    async fn phase8_startup_diagnostics_are_structured_and_redacted() {
        let provider = MockProvider::new("mock", vec![test_support::text_response("hi")]);
        let diagnostic = Diagnostic::new(
            Severity::Error,
            code::CODE_PACKAGE_RESOLUTION_FAILED,
            SOURCE_PACKAGE,
            "failed to read config at C:\\Users\\alice\\.config\\opi\\config.toml with key sk-proj-1234567890abcdefghijklmnopqrstuv",
        )
        .details(serde_json::json!({
            "upstream": "https://alice:s3cr3t@github.example.com/o/r.git",
            "token": "sk-ant-api03-1234567890abcdefghijklmnopqrstuv",
        }))
        .action("rotate ghp_1234567890abcdefghijklmnopqrstuvwxyz and retry");
        let mut runner = runner_with_startup(Box::new(provider), vec![diagnostic], None);
        let result = runner.run_json("hi").await;
        assert_eq!(result.exit_code, ExitCode::Success as i32);

        let lines = parse_ndjson(&result.stdout);
        assert_eq!(lines[0]["type"], "session_header", "first line is header");
        assert_eq!(
            lines[1]["type"], "StartupDiagnostics",
            "second line must be startup diagnostics"
        );

        let serialized = serde_json::to_string(&lines[1]).unwrap_or_default();

        // Redaction: none of the seeded secrets survive the startup boundary.
        let leaked_secrets = [
            "sk-proj-1234567890abcdefghijklmnopqrstuv",
            "sk-ant-api03-1234567890abcdefghijklmnopqrstuv",
            "ghp_1234567890abcdefghijklmnopqrstuvwxyz",
            "s3cr3t",
            "alice",
        ];
        for secret in leaked_secrets {
            assert!(
                !serialized.contains(secret),
                "StartupDiagnostics leaked a sensitive value: {secret}\n{serialized}"
            );
        }
        // A redaction marker must be present, proving the scrubber ran rather
        // than dropping the content silently.
        assert!(
            serialized.contains("[REDACTED]"),
            "StartupDiagnostics should carry at least one redaction marker\n{serialized}"
        );

        // Structure: the payload is still a Diagnostic, not a flattened string.
        // Asserting code/source/severity here is the non-vacuous core of H5.
        let diag = &lines[1]["diagnostics"][0];
        assert_eq!(
            diag["code"],
            code::CODE_PACKAGE_RESOLUTION_FAILED,
            "structured code field survives redaction"
        );
        assert_eq!(
            diag["source"], SOURCE_PACKAGE,
            "structured source field survives redaction"
        );
        assert_eq!(
            diag["severity"], "error",
            "structured severity field survives redaction"
        );
        // message is present and redacted (not absent), proving the field was
        // scrubbed in place rather than dropped.
        assert!(
            diag.get("message").and_then(|v| v.as_str()).is_some(),
            "message field must remain after redaction: {diag}"
        );
    }
}

#[tokio::test]
async fn json_mode_read_tool_result_does_not_leak_workspace_root() {
    let workspace = tempfile::tempdir().expect("workspace");
    std::fs::write(workspace.path().join("safe.txt"), "secret body").unwrap();

    let first = test_support::tool_call_response("tc-read", "read", r#"{"path":"safe.txt"}"#);
    let second = test_support::text_response("done");
    let provider = MockProvider::new("mock", vec![first, second]);
    let mut runner = NonInteractiveRunner::new(
        Box::new(provider),
        "mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        false,
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    let result = runner.run_json("read safe.txt").await;
    assert_eq!(result.exit_code, ExitCode::Success as i32);
    let root = workspace.path().display().to_string();
    assert!(
        !result.stdout.contains(&root),
        "NDJSON leaked workspace root {root}: {}",
        result.stdout
    );
    assert!(
        result.stdout.contains("safe.txt"),
        "NDJSON should retain useful relative path context"
    );
}

#[tokio::test]
async fn json_mode_runtime_messages_have_nonzero_timestamps() {
    let provider = MockProvider::new("mock", vec![test_support::text_response("hi")]);
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

    let result = runner.run_json("hello").await;
    assert_eq!(result.exit_code, ExitCode::Success as i32);
    let lines = parse_ndjson(&result.stdout);
    let mut timestamps = Vec::new();
    for value in lines {
        if value["type"].as_str() != Some("Agent") {
            continue;
        }
        if let Some(ts) = value["event"]["message"]["timestamp_ms"].as_i64() {
            timestamps.push(ts);
        }
    }
    assert!(
        !timestamps.is_empty(),
        "expected streamed message timestamps"
    );
    assert!(
        timestamps.iter().all(|ts| *ts > 0),
        "all runtime timestamps must be nonzero: {timestamps:?}"
    );
}

#[tokio::test]
async fn json_mode_session_summary_reports_provider_turns() {
    let first = test_support::tool_call_response(
        "tc-read",
        "read",
        r#"{"path":"Cargo.toml","offset":1,"limit":1}"#,
    );
    let second = test_support::text_response("done");
    let provider = MockProvider::new("mock", vec![first, second]);
    let session_dir = tempfile::tempdir().expect("session dir");
    let mut runner =
        runner_with_isolated_session(Box::new(provider), "mock-model", &session_dir, None);

    let result = runner.run_json("read Cargo.toml").await;
    assert_eq!(result.exit_code, ExitCode::Success as i32);
    let lines = parse_ndjson(&result.stdout);
    let provider_turns_seen = lines
        .iter()
        .filter(|v| {
            v["type"].as_str() == Some("Agent") && v["event"]["type"].as_str() == Some("TurnStart")
        })
        .count() as u64;
    let summary = lines
        .iter()
        .find(|v| v["type"].as_str() == Some("session_summary"))
        .expect("summary");

    assert_eq!(
        summary["turns"], 1,
        "legacy turns remains user prompt count"
    );
    assert_eq!(
        summary["provider_turns"].as_u64(),
        Some(provider_turns_seen),
        "provider_turns must match TurnStart events"
    );
    assert!(
        provider_turns_seen > 1,
        "tool loop should require more than one provider turn"
    );
}

#[tokio::test]
async fn json_mode_compact_text_deltas_are_constant_size() {
    use opi_ai::message::AssistantContent;
    use opi_ai::stream::AssistantStreamEvent;

    let mut events = vec![AssistantStreamEvent::Start {
        partial: test_support::base_assistant(),
    }];
    for i in 1u32..=200 {
        let mut partial = test_support::base_assistant();
        partial.content = vec![AssistantContent::Text {
            text: "x".repeat(i as usize),
        }];
        events.push(AssistantStreamEvent::TextDelta {
            content_index: 0,
            delta: "x".into(),
            partial,
        });
    }
    let mut done = test_support::base_assistant();
    done.content = vec![AssistantContent::Text {
        text: "x".repeat(200),
    }];
    events.push(AssistantStreamEvent::Done {
        reason: opi_ai::stream::StopReason::Stop,
        message: done,
    });

    let provider = MockProvider::new("mock", vec![events]);
    let mut runner = NonInteractiveRunner::new(
        Box::new(provider),
        "mock-model".into(),
        OpiConfig::default(),
        std::env::current_dir().unwrap(),
        false,
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .with_compact_ndjson(true);

    let result = runner.run_json("stream").await;
    assert_eq!(result.exit_code, ExitCode::Success as i32);

    let text_delta_lines: Vec<&str> = result
        .stdout
        .lines()
        .filter(|line| line.contains(r#""text_delta""#))
        .collect();
    assert_eq!(text_delta_lines.len(), 200, "expected 200 text_delta lines");
    // Each compact line must be constant-size (proves ~linear, not quadratic).
    assert!(
        text_delta_lines.iter().all(|line| line.len() < 700),
        "each compact text_delta line must be constant-size; got a line > 700 bytes"
    );
    assert!(
        !text_delta_lines
            .iter()
            .any(|line| line.contains(r#""partial""#)),
        "compact text_delta lines must omit assistant_event.partial"
    );
    // Cumulative text (a long run of 'x') must not appear in any delta line.
    assert!(
        !text_delta_lines
            .iter()
            .any(|line| line.contains("xxxxxxxxxx")),
        "compact text_delta lines must omit cumulative event.message text"
    );
}

#[tokio::test]
async fn json_mode_default_output_still_carries_partial() {
    use opi_ai::message::AssistantContent;
    use opi_ai::stream::AssistantStreamEvent;

    let mut partial = test_support::base_assistant();
    partial.content = vec![AssistantContent::Text { text: "ab".into() }];
    let events = vec![
        AssistantStreamEvent::Start {
            partial: test_support::base_assistant(),
        },
        AssistantStreamEvent::TextDelta {
            content_index: 0,
            delta: "ab".into(),
            partial: partial.clone(),
        },
        AssistantStreamEvent::Done {
            reason: opi_ai::stream::StopReason::Stop,
            message: partial,
        },
    ];
    let provider = MockProvider::new("mock", vec![events]);
    let mut runner = NonInteractiveRunner::new(
        Box::new(provider),
        "mock-model".into(),
        OpiConfig::default(),
        std::env::current_dir().unwrap(),
        false,
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    ); // no with_compact_ndjson -> default mode

    let result = runner.run_json("stream").await;
    assert_eq!(result.exit_code, ExitCode::Success as i32);
    assert!(
        result
            .stdout
            .lines()
            .any(|l| l.contains(r#""text_delta""#) && l.contains(r#""partial""#)),
        "default mode must still carry assistant_event.partial (non-breaking)"
    );
}
