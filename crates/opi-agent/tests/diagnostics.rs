//! Phase 7 task 7.1 — shared diagnostic model and redaction core.
//!
//! These tests pin the observable contract of `opi_agent::diagnostic`:
//! severity ordering and serde stability, deterministic serialization, stable
//! snake_case codes/sources, and redaction that scrubs known secrets and
//! sensitive content by default (summary mode) while preserving it in verbose
//! mode minus secrets.

use opi_agent::diagnostic::{
    Diagnostic, RedactionMode, SOURCE_ADAPTER, SOURCE_CONFIG, SOURCE_PROVIDER, SOURCE_RPC,
    SOURCE_SESSION, SOURCE_TOOL, SOURCE_TUI, Severity, redact,
};
use opi_ai::provider::{ProviderError, ProviderErrorSummary};
use serde_json::{Value, json};

// ---------------------------------------------------------------------------
// Severity: ordering + serde stability
// ---------------------------------------------------------------------------

#[test]
fn severity_orders_error_above_warning_above_info() {
    assert!(Severity::Error > Severity::Warning);
    assert!(Severity::Warning > Severity::Info);
    assert!(Severity::Error > Severity::Info);
}

#[test]
fn severity_serializes_to_stable_lowercase_strings() {
    assert_eq!(serde_json::to_string(&Severity::Info).unwrap(), "\"info\"");
    assert_eq!(
        serde_json::to_string(&Severity::Warning).unwrap(),
        "\"warning\""
    );
    assert_eq!(
        serde_json::to_string(&Severity::Error).unwrap(),
        "\"error\""
    );
}

// ---------------------------------------------------------------------------
// Diagnostic: deterministic serialization + stable shape
// ---------------------------------------------------------------------------

#[test]
fn diagnostic_serializes_deterministically() {
    let diag = Diagnostic::new(
        Severity::Error,
        "provider_auth_failed",
        "provider",
        "authentication failed",
    );
    let first = serde_json::to_string(&diag).unwrap();
    let second = serde_json::to_string(&diag).unwrap();
    assert_eq!(first, second, "serialization must be deterministic");
}

#[test]
fn diagnostic_serializes_stable_field_names_and_snake_case() {
    let diag = Diagnostic::new(
        Severity::Warning,
        "config_invalid_key",
        "config",
        "unknown key `foo`",
    );
    let value: Value = serde_json::to_value(&diag).unwrap();
    assert_eq!(value["severity"], "warning");
    assert_eq!(value["code"], "config_invalid_key");
    assert_eq!(value["source"], "config");
    assert_eq!(value["message"], "unknown key `foo`");
    // Optionals are absent when unset, not null, keeping the wire shape minimal.
    assert!(value.get("details").is_none());
    assert!(value.get("action").is_none());
}

#[test]
fn diagnostic_serializes_optionals_when_present() {
    let diag = Diagnostic::new(
        Severity::Info,
        "session_compacted",
        "session",
        "compacted 3 turns",
    )
    .details(json!({ "removed_turns": 3 }))
    .action("no action needed");
    let value: Value = serde_json::to_value(&diag).unwrap();
    assert_eq!(value["details"]["removed_turns"], 3);
    assert_eq!(value["action"], "no action needed");
}

// ---------------------------------------------------------------------------
// Shared source vocabulary constants
// ---------------------------------------------------------------------------

#[test]
fn source_constants_are_stable_snake_case_strings() {
    assert_eq!(SOURCE_PROVIDER, "provider");
    assert_eq!(SOURCE_TOOL, "tool");
    assert_eq!(SOURCE_CONFIG, "config");
    assert_eq!(SOURCE_SESSION, "session");
    assert_eq!(SOURCE_ADAPTER, "adapter");
    assert_eq!(SOURCE_RPC, "rpc");
    assert_eq!(SOURCE_TUI, "tui");
}

// ---------------------------------------------------------------------------
// Redaction: summary mode scrubs secrets and sensitive content by default
// ---------------------------------------------------------------------------

fn sensitive_details() -> Value {
    json!({
        "api_key": "sk-ant-1234567890abcdefghijklmnopqrstuv",
        "bearer": "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIg.abc123def456",
        "prompt": "You are a hidden system prompt.",
        "tool_output": "contents of /etc/passwd style leak",
        "env": { "HOME": "/Users/secret", "TOKEN": "leak" },
        "command": "curl -H 'Authorization: Bearer x' https://example",
        "cwd": "/Users/secret/proj",
        "kept": "benign metadata",
        "count": 42
    })
}

#[test]
fn summary_redaction_scrubs_known_secrets() {
    let redacted = redact(&sensitive_details(), RedactionMode::Summary);
    assert_eq!(redacted["api_key"], "[REDACTED]");
    assert_eq!(redacted["bearer"], "[REDACTED]");
}

#[test]
fn summary_redaction_scrubs_sensitive_content_fields() {
    let redacted = redact(&sensitive_details(), RedactionMode::Summary);
    assert_eq!(redacted["prompt"], "[REDACTED]");
    assert_eq!(redacted["tool_output"], "[REDACTED]");
    // env/command/cwd carry full environment, commands, and absolute working dirs.
    assert_eq!(redacted["env"], "[REDACTED]");
    assert_eq!(redacted["command"], "[REDACTED]");
    assert_eq!(redacted["cwd"], "[REDACTED]");
}

#[test]
fn summary_redaction_scrubs_command_args_and_tool_result_keys() {
    let details = json!({
        "command": "rm -rf /",
        "args": ["--token", "sk-leak1234567890abcdefghijklmnopqr"],
        "tool_result": "tool stdout containing secrets"
    });
    let redacted = redact(&details, RedactionMode::Summary);
    assert_eq!(redacted["command"], "[REDACTED]");
    assert_eq!(redacted["args"], "[REDACTED]");
    assert_eq!(redacted["tool_result"], "[REDACTED]");
}

#[test]
fn summary_redaction_preserves_benign_metadata() {
    let redacted = redact(&sensitive_details(), RedactionMode::Summary);
    assert_eq!(redacted["kept"], "benign metadata");
    assert_eq!(redacted["count"], 42);
}

#[test]
fn summary_redaction_scrubs_absolute_paths_but_keeps_relative() {
    let details = json!({
        "unix_path": "/Users/secret/proj/src/main.rs",
        "win_path": "C:\\Users\\secret\\proj",
        "rel_path": "src/main.rs",
        "mentions_path": "see /Users/secret/proj for more"
    });
    let redacted = redact(&details, RedactionMode::Summary);
    assert_eq!(redacted["unix_path"], "[REDACTED]");
    assert_eq!(redacted["win_path"], "[REDACTED]");
    assert_eq!(redacted["rel_path"], "src/main.rs");
    // A value that merely mentions an absolute path is scrubbed too: the path
    // must not leak out of the workspace by default.
    assert_eq!(redacted["mentions_path"], "[REDACTED]");
}

#[test]
fn summary_redaction_is_recursive() {
    let details = json!({
        "nested": {
            "deep": {
                "api_key": "sk-ant-1234567890abcdefghijklmnopqrstuv",
                "prompt": "hidden"
            }
        },
        "list": [
            { "token": "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIg.abc123def456" },
            "plain"
        ]
    });
    let redacted = redact(&details, RedactionMode::Summary);
    assert_eq!(redacted["nested"]["deep"]["api_key"], "[REDACTED]");
    assert_eq!(redacted["nested"]["deep"]["prompt"], "[REDACTED]");
    assert_eq!(redacted["list"][0]["token"], "[REDACTED]");
    assert_eq!(redacted["list"][1], "plain");
}

// ---------------------------------------------------------------------------
// Redaction: verbose mode keeps metadata but scrubs structural content and secrets
// ---------------------------------------------------------------------------

#[test]
fn verbose_redaction_keeps_metadata_but_scrubs_structural_content_and_secrets() {
    let details = json!({
        "prompt": "benign prompt text without secrets",
        "tool_output": "ordinary tool output",
        "api_key": "sk-ant-1234567890abcdefghijklmnopqrstuv",
        "token": "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIg.abc123def456",
        "kept": "ok"
    });
    let redacted = redact(&details, RedactionMode::Verbose);
    // Structural content fields are scrubbed in verbose mode.
    assert_eq!(redacted["prompt"], "[REDACTED]");
    assert_eq!(redacted["tool_output"], "[REDACTED]");
    assert_eq!(redacted["kept"], "ok");
    // Secrets are still scrubbed.
    assert_eq!(redacted["api_key"], "[REDACTED]");
    assert_eq!(redacted["token"], "[REDACTED]");
}

// ---------------------------------------------------------------------------
// Diagnostic.redacted_details: redaction does not touch core fields
// ---------------------------------------------------------------------------

#[test]
fn redacted_details_scrubs_only_details() {
    let diag = Diagnostic::new(
        Severity::Error,
        "provider_request_failed",
        "provider",
        "request failed",
    )
    .details(json!({ "api_key": "sk-ant-1234567890abcdefghijklmnopqrstuv", "status": 503 }))
    .action("retry with backoff");

    let redacted = diag.redacted_details(RedactionMode::Summary).unwrap();
    assert_eq!(redacted["api_key"], "[REDACTED]");
    assert_eq!(redacted["status"], 503);

    // Core diagnostic fields are untouched by redaction.
    assert_eq!(diag.code, "provider_request_failed");
    assert_eq!(diag.source, "provider");
    assert_eq!(diag.severity, Severity::Error);
    assert_eq!(diag.action.as_deref(), Some("retry with backoff"));
}

#[test]
fn redacted_details_is_none_when_no_details() {
    let diag = Diagnostic::new(Severity::Info, "noop", "config", "nothing happened");
    assert!(diag.redacted_details(RedactionMode::Summary).is_none());
}

// ---------------------------------------------------------------------------
// Phase 7 task 7.6 — DoD SC6 end-to-end redaction guard
//
// Consolidates every sensitive class the shared redaction core must scrub at
// the diagnostic boundary: API keys, bearer tokens, environment values, prompt
// content, and tool output, plus the 7.4-evaluator-deferred gaps (GitHub PATs,
// credentialed-URL userinfo, and Authorization-header-by-name). These flow
// through `redact()` (shared by doctor --json, JSON/RPC, and trace sinks), so a
// single test pins the contract for every Phase 7 surface.
// ---------------------------------------------------------------------------

#[test]
fn phase7_redacts_sensitive_values() {
    let details = json!({
        "api_key": "sk-ant-1234567890abcdefghijklmnopqrstuv",
        "bearer": "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIg.abc123def456",
        "github_pat": "ghp_01234567890123456789012345678901234567",
        // A credentialed git URL (the doctor package_source leak path).
        "package_source": "https://ghp_01234567890123456789012345678901234567@github.com/owner/repo.git",
        "userpass_url": "https://alice:s3cr3t@gitlab.example.com/owner/repo.git",
        "authorization": "Bearer opaqueTokenValueNotMatchingOtherPatterns",
        "prompt": "system prompt text",
        "tool_output": "stdout embedding sk-ant-1234567890abcdefghijklmnopqrstuv",
        "env": { "OPENAI_API_KEY": "sk-leak1234567890abcdefghijklmnopqr" },
        "cwd": "/Users/secret/proj",
        "benign": "ordinary value kept",
        "count": 7
    });

    let redacted = redact(&details, RedactionMode::Summary);

    // API keys + bearer (SecretRedactor value patterns).
    assert_eq!(redacted["api_key"], "[REDACTED]");
    assert_eq!(redacted["bearer"], "[REDACTED]");
    // Content-sensitive whole-field redaction (Summary mode).
    assert_eq!(redacted["prompt"], "[REDACTED]");
    assert_eq!(redacted["tool_output"], "[REDACTED]");
    assert_eq!(redacted["env"], "[REDACTED]");
    assert_eq!(redacted["cwd"], "[REDACTED]");
    // 7.6 gaps: GitHub PAT, credentialed URLs, Authorization header.
    assert_eq!(redacted["github_pat"], "[REDACTED]");
    assert_eq!(redacted["package_source"], "[REDACTED]");
    assert_eq!(redacted["userpass_url"], "[REDACTED]");
    assert_eq!(redacted["authorization"], "[REDACTED]");
    // Benign metadata survives.
    assert_eq!(redacted["benign"], "ordinary value kept");
    assert_eq!(redacted["count"], 7);

    // Verbose mode still scrubs the shared secret patterns.
    let verbose = redact(&details, RedactionMode::Verbose);
    assert_eq!(verbose["github_pat"], "[REDACTED]");
    assert_eq!(verbose["package_source"], "[REDACTED]");
    assert_eq!(verbose["authorization"], "[REDACTED]");
    // ...and structural content fields are scrubbed.
    assert_eq!(verbose["prompt"], "[REDACTED]");
}

#[test]
fn diagnostic_redacts_hyphenated_provider_keys_in_details() {
    let details = json!({
        "anthropic": "sk-ant-api03-1234567890abcdefghijklmnopqrstuv",
        "openai": "sk-proj-1234567890abcdefghijklmnopqrstuv",
        "ordinary": "kept"
    });

    let redacted = redact(&details, RedactionMode::Summary);

    assert_eq!(redacted["anthropic"], "[REDACTED]");
    assert_eq!(redacted["openai"], "[REDACTED]");
    assert_eq!(redacted["ordinary"], "kept");
}

#[test]
fn redacted_payload_scrubs_message_action_and_details() {
    let diag = Diagnostic::new(
        Severity::Warning,
        "provider_request_failed",
        "provider",
        "HTTP 500: token sk-proj-1234567890abcdefghijklmnopqrstuv at /Users/alice/body",
    )
    .action("inspect /Users/alice/.config/opi/config.toml")
    .details(json!({
        "provider_error": "raw body with sk-ant-api03-1234567890abcdefghijklmnopqrstuv",
        "ordinary": "kept"
    }));

    let payload = diag.redacted_payload(RedactionMode::Summary);

    assert_eq!(payload.message, "[REDACTED]");
    assert_eq!(payload.action.as_deref(), Some("[REDACTED]"));
    assert_eq!(
        payload.details.as_ref().unwrap()["provider_error"],
        "[REDACTED]"
    );
    assert_eq!(payload.details.as_ref().unwrap()["ordinary"], "kept");
}

// ---------------------------------------------------------------------------
// Phase 8 task 8.6 — real provider key formats survive the secret scrubbers.
//
// Pins every real-format secret class the redaction core must scrub, plus the
// Summary-only absolute-path redaction (Verbose deliberately keeps paths). The
// provider key suffixes are chosen to clear the `{20,}` quantifier floor so a
// regression that widens or narrows the floor is caught here.
// ---------------------------------------------------------------------------

#[test]
fn phase8_real_format_redaction_contract() {
    let details = json!({
        "anthropic_api03": "sk-ant-api03-1234567890abcdefghijklmnopqrstuv",
        "anthropic_api06": "sk-ant-api06-1234567890abcdefghijklmnopqrstuv",
        "openai_proj": "sk-proj-1234567890abcdefghijklmnopqrstuv",
        "openai_live": "sk-live-1234567890abcdefghijklmnopqrstuv",
        "openai_svcacct": "sk-svcacct-1234567890abcdefghijklmnopqrstuv",
        "credentialed_url": "https://alice:s3cr3t-pw@gitlab.example.com/owner/repo.git",
        "win_drive_path": "C:\\Users\\alice\\.config\\opi\\config.toml",
        "unix_abs_path": "/Users/alice/.config/opi/config.toml",
        "benign": "ordinary value"
    });

    let summary = redact(&details, RedactionMode::Summary);
    // Every real-format provider key is scrubbed by the SecretRedactor patterns.
    assert_eq!(summary["anthropic_api03"], "[REDACTED]");
    assert_eq!(summary["anthropic_api06"], "[REDACTED]");
    assert_eq!(summary["openai_proj"], "[REDACTED]");
    assert_eq!(summary["openai_live"], "[REDACTED]");
    assert_eq!(summary["openai_svcacct"], "[REDACTED]");
    // Credentialed URL userinfo is scrubbed in both modes.
    assert_eq!(summary["credentialed_url"], "[REDACTED]");
    // Summary mode additionally redacts absolute paths.
    assert_eq!(summary["win_drive_path"], "[REDACTED]");
    assert_eq!(summary["unix_abs_path"], "[REDACTED]");
    // Benign content survives.
    assert_eq!(summary["benign"], "ordinary value");

    // Verbose keeps non-sensitive metadata but still scrubs shared secret patterns.
    // Absolute-path value redaction remains Summary-only by design, so do not
    // assert path redaction here.
    let verbose = redact(&details, RedactionMode::Verbose);
    assert_eq!(verbose["anthropic_api03"], "[REDACTED]");
    assert_eq!(verbose["anthropic_api06"], "[REDACTED]");
    assert_eq!(verbose["openai_proj"], "[REDACTED]");
    assert_eq!(verbose["openai_live"], "[REDACTED]");
    assert_eq!(verbose["openai_svcacct"], "[REDACTED]");
    assert_eq!(verbose["credentialed_url"], "[REDACTED]");
}

// ---------------------------------------------------------------------------
// Display: stable one-line form, no CLI/color formatting in the model
// ---------------------------------------------------------------------------

#[test]
fn diagnostic_display_is_stable_one_line() {
    let diag = Diagnostic::new(
        Severity::Warning,
        "config_invalid_key",
        "config",
        "unknown key `foo`",
    )
    .action("remove the key");
    let rendered = diag.to_string();
    assert!(rendered.contains("[warning]"), "rendered: {rendered}");
    assert!(
        rendered.contains("config::config_invalid_key"),
        "rendered: {rendered}"
    );
    assert!(
        rendered.contains("unknown key `foo`"),
        "rendered: {rendered}"
    );
    // No ANSI color escapes leak into the runtime-layer formatting.
    assert!(!rendered.contains('\u{1b}'), "rendered: {rendered}");
}

// ---------------------------------------------------------------------------
// Phase 12 task 12.2 — provider error taxonomy bridge (From<&ProviderError>).
//
// Each of the nine Phase 12 classes must reach a distinct, stable diagnostic
// code with provider_error detail redaction (the detail value is registered in
// CONTENT_SENSITIVE_KEYS, so it is scrubbed at the public surface). The Timeout
// variant keeps its existing provider_timeout code (it classifies as Network at
// the category layer but preserves its diagnostic code for back-compat).
// ---------------------------------------------------------------------------

#[test]
fn provider_error_bridge_classifies_network_variant() {
    let diag: Diagnostic = (&ProviderError::Network(ProviderErrorSummary::redacted())).into();
    assert_eq!(diag.source, SOURCE_PROVIDER);
    assert_eq!(diag.severity, Severity::Warning);
    assert_eq!(diag.code, "provider_network_error");
    assert_eq!(
        diag.details.as_ref().unwrap()["provider_error"],
        "[REDACTED]"
    );
}

#[test]
fn provider_error_bridge_classifies_config_variant() {
    let diag: Diagnostic = (&ProviderError::Config(ProviderErrorSummary::redacted())).into();
    assert_eq!(diag.source, SOURCE_PROVIDER);
    assert_eq!(diag.severity, Severity::Error);
    assert_eq!(diag.code, "provider_config_error");
    assert_eq!(
        diag.details.as_ref().unwrap()["provider_error"],
        "[REDACTED]"
    );
}

#[test]
fn provider_error_bridge_classifies_provider_side_variant() {
    let diag: Diagnostic = (&ProviderError::ProviderSide(ProviderErrorSummary::redacted())).into();
    assert_eq!(diag.source, SOURCE_PROVIDER);
    assert_eq!(diag.severity, Severity::Error);
    assert_eq!(diag.code, "provider_side_error");
    assert_eq!(
        diag.details.as_ref().unwrap()["provider_error"],
        "[REDACTED]"
    );
}

#[test]
fn provider_error_bridge_classifies_unsupported_capability_variant() {
    let diag: Diagnostic =
        (&ProviderError::UnsupportedCapability(ProviderErrorSummary::redacted())).into();
    assert_eq!(diag.source, SOURCE_PROVIDER);
    assert_eq!(diag.severity, Severity::Error);
    assert_eq!(diag.code, "provider_capability_invalid");
    assert_eq!(
        diag.details.as_ref().unwrap()["provider_error"],
        "[REDACTED]"
    );
}

#[test]
fn provider_error_bridge_classifies_cancelled_variant() {
    let diag: Diagnostic = (&ProviderError::Cancelled).into();
    assert_eq!(diag.source, SOURCE_PROVIDER);
    assert_eq!(diag.severity, Severity::Info);
    assert_eq!(diag.code, "provider_cancelled");
    assert!(
        diag.details.is_none(),
        "cancelled carries no provider body/details"
    );
}

#[test]
fn provider_error_bridge_gives_each_class_a_distinct_code() {
    // One-to-one category -> diagnostic code coverage for the nine classes.
    let codes: std::collections::HashSet<&str> = [
        (&ProviderError::AuthFailed(ProviderErrorSummary::authentication_rejected())
            as &ProviderError),
        &ProviderError::Config(ProviderErrorSummary::redacted()),
        &ProviderError::RequestFailed(ProviderErrorSummary::redacted()),
        &ProviderError::Timeout,
        &ProviderError::Network(ProviderErrorSummary::redacted()),
        &ProviderError::RateLimited {
            retry_after_ms: None,
        },
        &ProviderError::ProviderSide(ProviderErrorSummary::redacted()),
        &ProviderError::StreamError(ProviderErrorSummary::redacted()),
        &ProviderError::UnsupportedCapability(ProviderErrorSummary::redacted()),
        &ProviderError::Cancelled,
    ]
    .iter()
    .map(|e| {
        let diag: Diagnostic = (*e).into();
        diag.code
    })
    .collect();
    assert_eq!(
        codes.len(),
        10,
        "each provider error variant needs a distinct diagnostic code"
    );
}
