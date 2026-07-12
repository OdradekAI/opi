//! Behavioral tests for the top-level `opi doctor` command (Phase 7 task 7.4).
//!
//! Two layers:
//! - **Library API** tests exercise the pure `doctor` module (`DoctorScope`,
//!   `run_doctor`, `DoctorReport`, formatters) directly. These pin scope
//!   parsing, per-scope diagnostics, the exit-code policy, the NDJSON shape,
//!   and the credential-value non-leak guarantee without spawning anything.
//! - **Binary** tests spawn the real `opi` binary to prove the top-level CLI
//!   dispatch, exit codes, scope selection, network-free behavior, and that
//!   `opi package doctor` remains a distinct, intact subcommand.
//!
//! No test makes a network call or requires real credentials. Provider scope
//! checks credential *presence* only; the credential *value* is never emitted.

use std::collections::HashMap;
use std::path::Path;

use opi_agent::{Diagnostic, Severity};
use opi_coding_agent::config::{ConfigError, OpiConfig};
use opi_coding_agent::diagnostic_bridge::{diagnostic_from_config, diagnostic_from_package};
use opi_coding_agent::doctor::{
    DoctorContext, DoctorEntry, DoctorReport, DoctorScope, format_json, format_text, run_doctor,
};
use opi_coding_agent::package_resolver::{
    InstalledPackageScope, PackageDiagnostic, PackageDiagnosticSeverity,
};

const ANTHROPIC_ENV: &str = "ANTHROPIC_API_KEY";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn test_config(model: &str) -> OpiConfig {
    let mut config = OpiConfig::default();
    config.defaults.model = model.to_string();
    config
}

#[allow(clippy::type_complexity)]
fn ctx<'a>(
    config: &'a OpiConfig,
    sessions_dir: &'a Path,
    env_var: &'a dyn Fn(&str) -> Option<String>,
) -> DoctorContext<'a> {
    DoctorContext {
        config,
        config_error: None,
        workspace_root: Path::new("."),
        user_config_dir: Path::new("."),
        sessions_dir,
        term: None,
        term_program: None,
        term_features: None,
        no_color: false,
        colorterm: None,
        env_var,
        store_probe: &EMPTY_STORE_PROBE,
    }
}

fn no_env(_: &str) -> Option<String> {
    None
}

/// Shared empty probe map for tests that do not exercise StoreCredential.
static EMPTY_STORE_PROBE: std::sync::LazyLock<
    std::collections::HashMap<String, opi_ai::CredentialSource>,
> = std::sync::LazyLock::new(std::collections::HashMap::new);

/// Collect the distinct scope strings present in a report's NDJSON output.
fn scope_strings(report: &DoctorReport) -> Vec<String> {
    let mut scopes: Vec<String> = report
        .entries
        .iter()
        .map(|e| DoctorScope::as_str(&e.scope).to_string())
        .collect();
    scopes.sort();
    scopes.dedup();
    scopes
}

// ---------------------------------------------------------------------------
// Scope parsing
// ---------------------------------------------------------------------------

#[test]
fn parse_list_empty_is_ok_empty() {
    // Empty/blank input means "all scopes" at the call site (caller treats
    // empty as ALL), so parsing itself succeeds with an empty selection.
    assert!(DoctorScope::parse_list("").unwrap().is_empty());
    assert!(DoctorScope::parse_list("   ").unwrap().is_empty());
}

#[test]
fn parse_list_subset() {
    let scopes = DoctorScope::parse_list("config,tui").unwrap();
    assert_eq!(scopes, vec![DoctorScope::Config, DoctorScope::Tui]);
}

#[test]
fn parse_list_trims_whitespace() {
    let scopes = DoctorScope::parse_list(" config , rpc ,tui").unwrap();
    assert_eq!(
        scopes,
        vec![DoctorScope::Config, DoctorScope::Rpc, DoctorScope::Tui]
    );
}

#[test]
fn parse_list_unknown_token_errors() {
    assert!(DoctorScope::parse_list("bogus").is_err());
    assert!(DoctorScope::parse_list("config,notascope").is_err());
}

#[test]
fn all_six_scopes_listed() {
    // The doctor surface must cover exactly the six design scopes.
    assert_eq!(
        DoctorScope::ALL.len(),
        6,
        "expected exactly six doctor scopes"
    );
}

// ---------------------------------------------------------------------------
// Config scope
// ---------------------------------------------------------------------------

#[test]
fn config_scope_reports_resolved_model() {
    let config = test_config("anthropic:claude-test-model");
    let dir = tempfile::tempdir().unwrap();
    let report = run_doctor(&[DoctorScope::Config], &ctx(&config, dir.path(), &no_env));
    assert!(
        !report.entries.is_empty(),
        "config scope must emit >=1 entry"
    );
    let has_model = format_text(&report).contains("claude-test-model");
    assert!(
        has_model,
        "config scope should mention the resolved model, got: {}",
        format_text(&report)
    );
    assert_eq!(report.entries[0].diagnostic.source, "config");
}

#[test]
fn config_scope_surfaces_config_error_as_error_severity() {
    // A config read failure must surface as an Error-severity shared diagnostic
    // (exit code 2), not an internal doctor failure (exit code 1).
    let config = test_config("anthropic:claude-test-model");
    let err = ConfigError::Read {
        path: std::path::PathBuf::from("/nonexistent/config.toml"),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "config file not found"),
    };
    let expected = diagnostic_from_config(&err);
    assert_eq!(expected.severity, Severity::Error);

    let dir = tempfile::tempdir().unwrap();
    let ctx = DoctorContext {
        config_error: Some(&err),
        ..ctx(&config, dir.path(), &no_env)
    };
    let report = run_doctor(&[DoctorScope::Config], &ctx);
    assert!(
        report.has_errors(),
        "config error should produce an error-severity diagnostic"
    );
    assert_eq!(report.exit_code(), 2);
    assert!(
        report
            .entries
            .iter()
            .any(|e| e.diagnostic.severity == Severity::Error)
    );
}

#[test]
fn doctor_json_redacts_absolute_path_in_config_details() {
    // A config error carries the config file path in `details`; the public
    // --json boundary must redact it (Phase 7 design: details are redacted
    // structured metadata, absolute paths are not emitted by default).
    let config = test_config("anthropic:claude-test-model");
    let leak_path = "/tmp/opi-secret-leak/config.toml";
    let err = ConfigError::Read {
        path: std::path::PathBuf::from(leak_path),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "config file not found"),
    };
    let dir = tempfile::tempdir().unwrap();
    let ctx = DoctorContext {
        config_error: Some(&err),
        ..ctx(&config, dir.path(), &no_env)
    };
    let report = run_doctor(&[DoctorScope::Config], &ctx);
    let json = format_json(&report);
    assert!(
        !json.contains(leak_path),
        "absolute config path leaked into doctor --json details: {json}",
    );
    assert!(
        json.contains("[REDACTED]"),
        "expected a redaction marker in details, got: {json}",
    );
}

#[test]
fn doctor_outputs_redact_diagnostic_message_and_action() {
    let secret = "sk-proj-1234567890abcdefghijklmnopqrstuv";
    let report = DoctorReport {
        entries: vec![DoctorEntry {
            scope: DoctorScope::Package,
            diagnostic: Diagnostic::new(
                Severity::Warning,
                "package_diagnostic",
                "package",
                format!("read C:\\Users\\alice\\.config\\opi\\packages\\p\\package.toml: {secret}"),
            )
            .action("open C:\\Users\\alice\\.config\\opi\\config.toml"),
        }],
    };

    let json = format_json(&report);
    let text = format_text(&report);

    for output in [&json, &text] {
        assert!(!output.contains("alice"), "OS username leaked: {output}");
        assert!(!output.contains(secret), "provider key leaked: {output}");
        assert!(
            output.contains("[REDACTED]"),
            "expected redaction marker in output: {output}"
        );
    }
}

// ---------------------------------------------------------------------------
// Provider scope (network-free; credential presence only)
// ---------------------------------------------------------------------------

#[test]
fn provider_scope_credential_present_is_info() {
    let config = test_config("anthropic:claude-test-model");
    let dir = tempfile::tempdir().unwrap();
    let map: HashMap<&str, String> = [(ANTHROPIC_ENV, "sk-present".into())].into_iter().collect();
    let env = |n: &str| map.get(n).cloned();
    let report = run_doctor(&[DoctorScope::Provider], &ctx(&config, dir.path(), &env));
    assert!(
        report
            .entries
            .iter()
            .any(|e| e.diagnostic.severity == Severity::Info),
        "present credentials should be Info, got: {:?}",
        report.entries
    );
    assert!(!report.has_errors());
}

#[test]
fn provider_scope_credential_absent_is_warning() {
    let config = test_config("anthropic:claude-test-model");
    let dir = tempfile::tempdir().unwrap();
    let report = run_doctor(&[DoctorScope::Provider], &ctx(&config, dir.path(), &no_env));
    assert!(
        report
            .entries
            .iter()
            .any(|e| e.diagnostic.severity == Severity::Warning),
        "absent credentials should be Warning, got: {:?}",
        report.entries
    );
    // Missing credentials is a warning, not an error -> still exit 0.
    assert!(!report.has_errors());
    assert_eq!(report.exit_code(), 0);
}

#[test]
fn provider_scope_never_emits_credential_value() {
    // The credential *value* must never appear in any diagnostic field, even
    // though doctor inspects credential presence for the selected provider.
    let sentinel = "sk-test-DO-NOT-LEAK-1234567890";
    let config = test_config("anthropic:claude-test-model");
    let dir = tempfile::tempdir().unwrap();
    let map: HashMap<&str, String> = [(ANTHROPIC_ENV, sentinel.into())].into_iter().collect();
    let env = |n: &str| map.get(n).cloned();
    let report = run_doctor(&[DoctorScope::Provider], &ctx(&config, dir.path(), &env));
    let json = format_json(&report);
    let text = format_text(&report);
    assert!(
        !json.contains(sentinel),
        "credential value leaked into JSON output: {json}"
    );
    assert!(
        !text.contains(sentinel),
        "credential value leaked into text output: {text}"
    );
}

#[test]
fn provider_scope_bedrock_requires_access_key_and_secret() {
    let config = test_config("bedrock:anthropic.claude-test");
    let dir = tempfile::tempdir().unwrap();

    let only_access_key: HashMap<&str, String> =
        [("AWS_ACCESS_KEY_ID", "akid".into())].into_iter().collect();
    let only_access_key_env = |n: &str| only_access_key.get(n).cloned();
    let missing_secret = run_doctor(
        &[DoctorScope::Provider],
        &ctx(&config, dir.path(), &only_access_key_env),
    );
    assert!(
        missing_secret
            .entries
            .iter()
            .any(|e| e.diagnostic.severity == Severity::Warning
                && e.diagnostic
                    .details
                    .as_ref()
                    .is_some_and(|details| details["credentials_present"] == false)),
        "bedrock should warn when only AWS_ACCESS_KEY_ID is present: {:?}",
        missing_secret.entries
    );

    let complete: HashMap<&str, String> = [
        ("AWS_ACCESS_KEY_ID", "akid".into()),
        ("AWS_SECRET_ACCESS_KEY", "secret".into()),
    ]
    .into_iter()
    .collect();
    let complete_env = |n: &str| complete.get(n).cloned();
    let present = run_doctor(
        &[DoctorScope::Provider],
        &ctx(&config, dir.path(), &complete_env),
    );
    assert!(
        present
            .entries
            .iter()
            .any(|e| e.diagnostic.severity == Severity::Info
                && e.diagnostic
                    .details
                    .as_ref()
                    .is_some_and(|details| details["credentials_present"] == true)),
        "bedrock should report credentials present when access key and secret are present: {:?}",
        present.entries
    );
}

#[test]
fn provider_scope_bedrock_accepts_config_access_key_with_custom_secret_env() {
    let mut config = test_config("bedrock:anthropic.claude-test");
    config.providers.bedrock.access_key_id = Some("CONFIG_AKID".into());
    config.providers.bedrock.secret_access_key_env = Some("BEDROCK_SECRET".into());
    let dir = tempfile::tempdir().unwrap();
    let env_values: HashMap<&str, String> =
        [("BEDROCK_SECRET", "secret".into())].into_iter().collect();
    let env = |n: &str| env_values.get(n).cloned();

    let report = run_doctor(&[DoctorScope::Provider], &ctx(&config, dir.path(), &env));

    assert!(
        report.entries.iter().any(|e| {
            e.diagnostic.severity == Severity::Info
                && e.diagnostic.details.as_ref().is_some_and(|details| {
                    details["credentials_present"] == true
                        && details["credential_probe"]
                            == "config access_key_id + env BEDROCK_SECRET"
                })
        }),
        "bedrock should report config access_key_id plus custom secret env as present: {:?}",
        report.entries
    );
}

#[test]
fn provider_scope_bedrock_accepts_configured_profile() {
    let mut config = test_config("bedrock:anthropic.claude-test");
    config.providers.bedrock.profile = Some("dev".into());
    let dir = tempfile::tempdir().unwrap();

    let report = run_doctor(&[DoctorScope::Provider], &ctx(&config, dir.path(), &no_env));

    assert!(
        report.entries.iter().any(|e| {
            e.diagnostic.severity == Severity::Info
                && e.diagnostic.details.as_ref().is_some_and(|details| {
                    details["credentials_present"] == true
                        && details["credential_probe"] == "profile dev"
                })
        }),
        "bedrock should report configured AWS profile as present: {:?}",
        report.entries
    );
}

#[test]
fn provider_scope_bedrock_accepts_aws_profile_env() {
    let config = test_config("bedrock:anthropic.claude-test");
    let dir = tempfile::tempdir().unwrap();
    let env_values: HashMap<&str, String> = [("AWS_PROFILE", "dev".into())].into_iter().collect();
    let env = |n: &str| env_values.get(n).cloned();

    let report = run_doctor(&[DoctorScope::Provider], &ctx(&config, dir.path(), &env));

    assert!(
        report.entries.iter().any(|e| {
            e.diagnostic.severity == Severity::Info
                && e.diagnostic.details.as_ref().is_some_and(|details| {
                    details["credentials_present"] == true
                        && details["credential_probe"] == "env AWS_PROFILE dev"
                })
        }),
        "bedrock should report AWS_PROFILE as present: {:?}",
        report.entries
    );
}

// ---------------------------------------------------------------------------
// Session scope
// ---------------------------------------------------------------------------

#[test]
fn session_scope_reports_session_count() {
    let config = test_config("anthropic:claude-test-model");
    let dir = tempfile::tempdir().unwrap();
    // Drop two fake session JSONL files into the sessions dir.
    std::fs::write(dir.path().join("aaa.jsonl"), "{}\n").unwrap();
    std::fs::write(dir.path().join("bbb.jsonl"), "{}\n").unwrap();
    let report = run_doctor(&[DoctorScope::Session], &ctx(&config, dir.path(), &no_env));
    let text = format_text(&report);
    assert!(
        text.contains('2') || text.contains("two"),
        "session scope should report the session count, got: {text}"
    );
    assert_eq!(report.entries[0].diagnostic.source, "session");
    assert!(!report.has_errors());
}

#[test]
fn session_scope_missing_createable_dir_is_info() {
    // A not-yet-created sessions dir under an existing parent is normal on a
    // fresh install and must not be an error.
    let config = test_config("anthropic:claude-test-model");
    let parent = tempfile::tempdir().unwrap();
    let missing = parent.path().join("sessions");
    let report = run_doctor(&[DoctorScope::Session], &ctx(&config, &missing, &no_env));
    assert!(
        !report.has_errors(),
        "missing-but-createable sessions dir should not error, got: {:?}",
        report.entries
    );
}

// ---------------------------------------------------------------------------
// TUI scope
// ---------------------------------------------------------------------------

#[test]
fn tui_scope_detects_iterm_protocol_and_no_color() {
    let config = test_config("anthropic:claude-test-model");
    let dir = tempfile::tempdir().unwrap();
    let ctx = DoctorContext {
        term: Some("xterm-256color"),
        term_program: Some("iTerm.app"),
        term_features: None,
        no_color: true,
        colorterm: None,
        ..ctx(&config, dir.path(), &no_env)
    };
    let report = run_doctor(&[DoctorScope::Tui], &ctx);
    let text = format_text(&report).to_lowercase();
    assert!(
        text.contains("iterm"),
        "tui scope should report the iTerm2 protocol, got: {text}"
    );
    assert!(
        text.contains("no color") || text.contains("no_color"),
        "tui scope should report no-color state, got: {text}"
    );
    assert_eq!(report.entries[0].diagnostic.source, "tui");
}

#[test]
fn tui_scope_fallback_when_no_graphics_protocol() {
    let config = test_config("anthropic:claude-test-model");
    let dir = tempfile::tempdir().unwrap();
    let report = run_doctor(&[DoctorScope::Tui], &ctx(&config, dir.path(), &no_env));
    assert!(!report.has_errors());
    assert_eq!(report.entries[0].diagnostic.source, "tui");
}

// ---------------------------------------------------------------------------
// RPC scope
// ---------------------------------------------------------------------------

#[test]
fn rpc_scope_reports_schema_version() {
    let config = test_config("anthropic:claude-test-model");
    let dir = tempfile::tempdir().unwrap();
    let report = run_doctor(&[DoctorScope::Rpc], &ctx(&config, dir.path(), &no_env));
    let text = format_text(&report);
    let version = opi_coding_agent::rpc::RPC_SCHEMA_VERSION;
    assert!(
        text.contains(&version.to_string()),
        "rpc scope should report the schema version {version}, got: {text}"
    );
    assert_eq!(report.entries[0].diagnostic.source, "rpc");
    assert!(!report.has_errors());
}

// ---------------------------------------------------------------------------
// Package scope (delegates to the installed-package resolver)
// ---------------------------------------------------------------------------

#[test]
fn package_scope_empty_workspace_is_info_no_errors() {
    let config = test_config("anthropic:claude-test-model");
    let dir = tempfile::tempdir().unwrap();
    let report = run_doctor(&[DoctorScope::Package], &ctx(&config, dir.path(), &no_env));
    assert!(
        !report.entries.is_empty(),
        "package scope must emit >=1 entry even with no packages"
    );
    assert!(
        !report.has_errors(),
        "empty workspace should not produce package errors, got: {:?}",
        report.entries
    );
    assert!(
        report
            .entries
            .iter()
            .all(|e| e.diagnostic.source == "package")
    );
}

// ---------------------------------------------------------------------------
// Whole-report behavior
// ---------------------------------------------------------------------------

#[test]
fn run_doctor_all_scopes_covers_every_scope() {
    let config = test_config("anthropic:claude-test-model");
    let dir = tempfile::tempdir().unwrap();
    let report = run_doctor(DoctorScope::ALL, &ctx(&config, dir.path(), &no_env));
    let scopes = scope_strings(&report);
    assert_eq!(
        scopes.len(),
        6,
        "default doctor run must cover all six scopes, got: {scopes:?}"
    );
}

#[test]
fn exit_code_no_errors_is_zero() {
    let report = DoctorReport::default();
    assert_eq!(report.exit_code(), 0);
}

#[test]
fn exit_code_with_error_is_two() {
    let config = test_config("anthropic:claude-test-model");
    let err = ConfigError::Read {
        path: std::path::PathBuf::from("/nonexistent/config.toml"),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "missing"),
    };
    let dir = tempfile::tempdir().unwrap();
    let ctx = DoctorContext {
        config_error: Some(&err),
        ..ctx(&config, dir.path(), &no_env)
    };
    let report = run_doctor(DoctorScope::ALL, &ctx);
    assert_eq!(report.exit_code(), 2);
}

#[test]
fn format_json_is_ndjson_with_required_fields() {
    let config = test_config("anthropic:claude-test-model");
    let dir = tempfile::tempdir().unwrap();
    let report = run_doctor(DoctorScope::ALL, &ctx(&config, dir.path(), &no_env));
    let json = format_json(&report);
    assert!(!json.trim().is_empty(), "json output must not be empty");
    for line in json.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("line is not valid JSON: {line}\nerror: {e}"));
        assert!(value.get("scope").is_some(), "missing scope: {line}");
        assert!(value.get("severity").is_some(), "missing severity: {line}");
        assert!(value.get("code").is_some(), "missing code: {line}");
        assert!(value.get("source").is_some(), "missing source: {line}");
        assert!(value.get("message").is_some(), "missing message: {line}");
    }
}

// ===========================================================================
// Phase 7 task 7.6 — redaction + shared-diagnostic-shape guards
//
// `phase7_doctor_redacts_sensitive_values` closes the credentialed-URL leak
// path the 7.4 evaluator deferred (a package source URL carrying a GitHub PAT
// or user:password userinfo must not survive the `--json` boundary). It drives
// the real production bridge (`diagnostic_from_package`) so the assertion
// covers the path that produces `details.package_source`, not a synthetic
// shortcut. `phase7_shared_diagnostics_used_by_doctor` proves every doctor
// entry is the shared `opi_agent::Diagnostic` at the public boundary (SC 1).
// ===========================================================================

#[test]
fn phase7_doctor_redacts_sensitive_values() {
    let credentialed =
        "https://ghp_01234567890123456789012345678901234567@github.com/owner/repo.git";
    let pd = PackageDiagnostic {
        scope: InstalledPackageScope::Project,
        source: credentialed.to_string(),
        severity: PackageDiagnosticSeverity::Warning,
        code: "source_unavailable".to_string(),
        message: "package source unreachable".to_string(),
    };
    let diagnostic = diagnostic_from_package(&pd);
    // The shared bridge places the raw source into details.package_source.
    assert_eq!(
        diagnostic.details.as_ref().unwrap()["package_source"],
        credentialed,
        "precondition: the raw credentialed URL must reach details before redaction"
    );

    let report = DoctorReport {
        entries: vec![DoctorEntry {
            scope: DoctorScope::Package,
            diagnostic,
        }],
    };

    let json = format_json(&report);
    assert!(
        !json.contains("ghp_01234567890123456789012345678901234567"),
        "GitHub PAT leaked through doctor --json details.package_source: {json}",
    );
    assert!(
        !json.contains(":s3cr3t@") && !json.contains("ghp_"),
        "credentialed-URL credential leaked through doctor --json: {json}",
    );
    assert!(
        json.contains("[REDACTED]"),
        "expected a redaction marker in doctor --json, got: {json}",
    );

    // A user:password userinfo URL must also be scrubbed at the boundary.
    let pd2 = PackageDiagnostic {
        scope: InstalledPackageScope::Project,
        source: "https://alice:s3cr3t@gitlab.example.com/o/r.git".to_string(),
        ..pd
    };
    let report2 = DoctorReport {
        entries: vec![DoctorEntry {
            scope: DoctorScope::Package,
            diagnostic: diagnostic_from_package(&pd2),
        }],
    };
    let json2 = format_json(&report2);
    assert!(
        !json2.contains("s3cr3t") && !json2.contains("alice:s3cr3t@"),
        "userinfo credentials leaked through doctor --json: {json2}",
    );
    assert!(
        json2.contains("[REDACTED]"),
        "expected redaction, got: {json2}"
    );
}

#[test]
fn phase7_shared_diagnostics_used_by_doctor() {
    // SC 1: doctor emits the shared `opi_agent::Diagnostic` shape (stable
    // severity/code/source/message) at its public boundary, not ad-hoc strings.
    let config = test_config("anthropic:claude-test-model");
    let dir = tempfile::tempdir().unwrap();
    let report = run_doctor(DoctorScope::ALL, &ctx(&config, dir.path(), &no_env));

    // Every entry's diagnostic IS the shared Diagnostic (source is a stable
    // shared SOURCE_* vocabulary token; code is stable snake_case).
    let valid_sources = ["config", "provider", "package", "session", "tui", "rpc"];
    assert!(!report.entries.is_empty());
    for entry in &report.entries {
        assert!(
            valid_sources.contains(&entry.diagnostic.source),
            "doctor diagnostic source {:?} is not in the shared SOURCE_* vocabulary",
            entry.diagnostic.source,
        );
        assert!(
            !entry.diagnostic.code.is_empty()
                && entry
                    .diagnostic
                    .code
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
            "doctor diagnostic code {:?} is not stable snake_case",
            entry.diagnostic.code,
        );
    }

    // The NDJSON output flattens the shared Diagnostic fields (not a custom
    // shape): every line carries severity/code/source/message.
    let json = format_json(&report);
    for line in json.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("doctor --json line is not valid JSON: {line}\nerror: {e}"));
        for field in ["severity", "code", "source", "message"] {
            assert!(
                value.get(field).is_some(),
                "doctor --json line does not carry shared Diagnostic field {field}: {line}"
            );
        }
    }

    // A package diagnostic built through the shared bridge carries the stable
    // shared code `package_diagnostic` (CODE_PACKAGE_DIAGNOSTIC) and source
    // `package`, proving the shared shape crosses the doctor boundary.
    let pd = PackageDiagnostic {
        scope: InstalledPackageScope::Project,
        source: "https://example.com/o/r.git".to_string(),
        severity: PackageDiagnosticSeverity::Warning,
        code: "source_unavailable".to_string(),
        message: "unreachable".to_string(),
    };
    let bridged = diagnostic_from_package(&pd);
    assert_eq!(bridged.code, "package_diagnostic");
    assert_eq!(bridged.source, "package");
}

// ===========================================================================
// Phase 8 task 8.6 — public diagnostic message/action redaction contract.
//
// Pins that the public doctor formatters scrub real-format Anthropic keys,
// credentialed-URL userinfo, and host absolute paths from BOTH `message` and
// `action`, regardless of the JSON or text rendering. Both fields are routed
// through the shared `redacted_payload(Summary)` boundary, so the same
// redaction contract must hold for either formatter.
// ===========================================================================

#[test]
fn phase8_public_diagnostic_message_redaction() {
    let key = "sk-ant-api03-1234567890abcdefghijklmnopqrstuv";
    let credentialed = "https://alice:s3cr3t@host/repo.git";
    let posix_path = "/Users/alice/.config/opi/config.toml";
    let win_path = "C:\\Users\\alice\\.config\\opi\\packages\\p\\package.toml";
    let report = DoctorReport {
        entries: vec![DoctorEntry {
            scope: DoctorScope::Package,
            diagnostic: Diagnostic::new(
                Severity::Warning,
                "package_diagnostic",
                "package",
                format!("read {posix_path} then {win_path}: key {key} src {credentialed}"),
            )
            .action(format!("open {win_path} and check {key} at {credentialed}")),
        }],
    };

    let json = format_json(&report);
    let text = format_text(&report);

    for output in [&json, &text] {
        assert!(!output.contains(key), "Anthropic key leaked: {output}");
        assert!(
            !output.contains("s3cr3t"),
            "userinfo password leaked: {output}"
        );
        assert!(!output.contains("alice"), "OS username leaked: {output}");
        assert!(
            output.contains("[REDACTED]"),
            "expected a redaction marker in output: {output}"
        );
    }
}

// ===========================================================================
// Binary (integration) tests — spawn the real `opi` binary.
// ===========================================================================

fn opi_bin() -> String {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest.parent().unwrap().parent().unwrap();
    for profile in &["debug", "release"] {
        let mut path = workspace_root.join("target").join(profile).join("opi");
        if cfg!(windows) {
            path.set_extension("exe");
        }
        if path.exists() {
            return path.to_string_lossy().into_owned();
        }
    }
    let mut path = workspace_root.join("target/debug/opi");
    if cfg!(windows) {
        path.set_extension("exe");
    }
    path.to_string_lossy().into_owned()
}

fn run_opi(args: &[&str], envs: &[(&str, &str)]) -> std::process::Output {
    let bin = opi_bin();
    let tmp = tempfile::tempdir().unwrap();
    let mut cmd = std::process::Command::new(&bin);
    cmd.args(args).current_dir(tmp.path()).env_clear();
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.output()
        .unwrap_or_else(|e| panic!("failed to run {bin}: {e}"))
}

#[test]
fn doctor_clean_env_exits_zero() {
    // No credentials, no config files, from a clean tempdir: doctor should run
    // with only warnings/info (missing provider credentials is a warning) and
    // exit 0.
    let output = run_opi(&["doctor"], &[]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected exit 0 for clean doctor run\nstdout: {stdout}\nstderr: {stderr}",
    );
    assert!(
        !stdout.trim().is_empty(),
        "doctor should print a report to stdout, got empty stdout\nstderr: {stderr}"
    );
}

#[test]
fn doctor_json_reports_all_scopes_without_network() {
    // Acceptance scenario `phase7-doctor-all-scopes`: every scope is reported
    // as NDJSON with no network and no credentials.
    let output = run_opi(&["doctor", "--json"], &[]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected exit 0\nstdout: {stdout}\nstderr: {stderr}",
    );

    let mut scopes: Vec<String> = Vec::new();
    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("doctor --json line not valid JSON: {line}\n{e}"));
        if let Some(scope) = value.get("scope").and_then(|v| v.as_str()) {
            scopes.push(scope.to_string());
        }
    }
    scopes.sort();
    scopes.dedup();
    assert_eq!(
        scopes,
        vec!["config", "package", "provider", "rpc", "session", "tui"],
        "doctor --json must report all six scopes, got: {scopes:?}\nstdout: {stdout}",
    );
}

#[test]
fn doctor_scope_subset_reports_only_requested_scopes() {
    let output = run_opi(&["doctor", "--json", "--scope", "config,rpc"], &[]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected exit 0; stdout: {stdout}",
    );
    let mut scopes: Vec<String> = Vec::new();
    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value =
            serde_json::from_str(line).expect("doctor --json line must be valid JSON");
        if let Some(scope) = value.get("scope").and_then(|v| v.as_str()) {
            scopes.push(scope.to_string());
        }
    }
    scopes.sort();
    scopes.dedup();
    assert_eq!(
        scopes,
        vec!["config", "rpc"],
        "only requested scopes, got: {scopes:?}"
    );
}

#[test]
fn doctor_unknown_scope_exits_one() {
    // An unknown scope token is an internal doctor command failure -> exit 1.
    let output = run_opi(&["doctor", "--scope", "bogus"], &[]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit 1 for unknown scope\nstderr: {stderr}",
    );
}

#[test]
fn doctor_config_error_exits_two() {
    // A malformed config must surface as an error-severity diagnostic (exit 2),
    // reported through the shared diagnostic shape, not exit 1.
    let tmp = tempfile::tempdir().unwrap();
    let config_path = tmp.path().join("broken.toml");
    std::fs::write(&config_path, "this is = = not valid toml [[[\n").unwrap();
    let bin = opi_bin();
    let output = std::process::Command::new(&bin)
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "doctor",
            "--json",
        ])
        .current_dir(tmp.path())
        .env_clear()
        .output()
        .unwrap_or_else(|e| panic!("failed to run {bin}: {e}"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(2),
        "expected exit 2 for config error\nstdout: {stdout}\nstderr: {stderr}",
    );
    // The error must be carried as a structured diagnostic in --json output.
    let saw_config_error = stdout.lines().any(|line| {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            return false;
        };
        value.get("source").and_then(|v| v.as_str()) == Some("config")
            && value.get("severity").and_then(|v| v.as_str()) == Some("error")
    });
    assert!(
        saw_config_error,
        "expected a config error diagnostic in --json, got stdout: {stdout}",
    );
}

#[test]
fn doctor_does_not_leak_credential_value_end_to_end() {
    // Set a real-looking credential; doctor --json --scope provider must report
    // presence but never the value itself.
    let sentinel = "sk-test-SECRET-VALUE-xyz";
    let output = run_opi(
        &["doctor", "--json", "--scope", "provider"],
        &[(ANTHROPIC_ENV, sentinel)],
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected exit 0\nstdout: {stdout}\nstderr: {stderr}",
    );
    assert!(
        !stdout.contains(sentinel),
        "credential value leaked into doctor output: {stdout}",
    );
}

#[test]
fn package_doctor_remains_a_distinct_intact_subcommand() {
    // `opi package doctor` is a separate subcommand from the top-level
    // `opi doctor` and must keep working unchanged.
    let output = run_opi(&["package", "doctor"], &[]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let _ = String::from_utf8_lossy(&output.stderr); // package doctor prints to stdout/stderr
    // A clean environment (no installed packages, env_clear) yields no package
    // diagnostics, so package doctor must exit 0. Pin the exact code so an
    // exit-semantic regression cannot hide behind a 0-or-2 disjunction.
    assert!(
        output.status.code() == Some(0),
        "package doctor should exit 0 in a clean environment\nstdout: {stdout}",
    );
}

// ---------------------------------------------------------------------------
// Phase 14.1: stored-credential probe surfaces (acceptance scenario)
// ---------------------------------------------------------------------------

/// Acceptance scenario `phase14-store-probe-surfaces` (doctor half): with a
/// keychain-backend config, doctor consults the injected redacted probe and
/// distinguishes Present / Absent / BackendUnavailable with distinct
/// severities+messages, and never emits the credential value. Exercises the
/// production `run_doctor` path with a hand-built store_probe map (the async
/// outer orchestration in `run_doctor_cli` produces exactly this map).
/// Acceptance scenario `phase14-store-probe-surfaces` (doctor half): with a
/// keychain-backend config, a REAL secret-bearing credential is seeded in the
/// store; doctor probes it (Present, label-only — never reading the secret) and
/// emits only the redacted label. The seeded access/refresh secrets must NOT
/// appear in either output mode. This is non-vacuous: the secrets ARE in the
/// store, so a regression that read+emitted the secret would fail this test.
/// (The Absent-vs-BackendUnavailable distinction is covered by
/// `stored_credential_backend_unavailable_is_distinct_from_absent`.)
#[tokio::test]
async fn stored_credential_probe_is_redacted() {
    use opi_ai::credential::CredentialStore;
    use opi_ai::{AuthDescriptor, Credential, CredentialSource};
    use opi_coding_agent::config::CredentialBackendSource;
    use opi_coding_agent::credential_store::{FakeKeyringBackend, KeychainCredentialStore};
    use secrecy::SecretString;

    let mut config = OpiConfig::default();
    config.defaults.model = "anthropic:claude-store-probe".into();
    config.defaults.credential_backend = Some(CredentialBackendSource::Keychain);

    // Keychain backend -> StoreCredential descriptor for the API-key provider.
    let descriptor =
        opi_coding_agent::provider_factory::auth_descriptor_for(&config, "anthropic").unwrap();
    assert!(
        matches!(descriptor, AuthDescriptor::StoreCredential { .. }),
        "expected StoreCredential descriptor, got {descriptor:?}"
    );

    let secret_access = "atk-doctor-probe-DO-NOT-LEAK";
    let secret_refresh = "rtk-doctor-probe-DO-NOT-LEAK";

    let dir = tempfile::tempdir().unwrap();
    let store = KeychainCredentialStore::new(
        Box::new(FakeKeyringBackend::new()),
        dir.path().to_path_buf(),
    );
    // Seed a real OAuth credential carrying access + refresh secrets.
    store
        .write(
            "anthropic",
            &Credential::OAuthToken {
                access: SecretString::new(secret_access.to_owned().into_boxed_str()),
                refresh: SecretString::new(secret_refresh.to_owned().into_boxed_str()),
                expires_at: None,
                base_url: Some("https://copilot.example/api".to_owned()),
            },
        )
        .await
        .unwrap();

    // Probe -> Present{label}, carrying NO secret. The async outer command path
    // (run_doctor_cli) builds exactly this map from store.probe.
    let probed = store.probe("anthropic").await;
    assert!(
        matches!(probed, CredentialSource::Present { .. }),
        "expected Present probe, got {probed:?}"
    );
    let mut store_probe = HashMap::new();
    store_probe.insert("anthropic".to_string(), probed);

    let env_probe = |_: &str| None;
    let report = run_doctor(
        &[DoctorScope::Provider],
        &DoctorContext {
            config: &config,
            config_error: None,
            workspace_root: Path::new("."),
            user_config_dir: dir.path(),
            sessions_dir: dir.path(),
            term: None,
            term_program: None,
            term_features: None,
            no_color: false,
            colorterm: None,
            env_var: &env_probe,
            store_probe: &store_probe,
        },
    );

    let entry = report
        .entries
        .iter()
        .find(|e| e.diagnostic.source == opi_agent::diagnostic::SOURCE_PROVIDER)
        .expect("provider diagnostic present");
    assert_eq!(entry.diagnostic.severity, Severity::Info);

    // NON-VACUOUS redaction: access + refresh ARE in the store; doctor only
    // probed, so neither may appear in text or JSON output.
    let text = format_text(&report);
    let json = format_json(&report);
    assert!(!text.contains(secret_access), "text leaked access: {text}");
    assert!(
        !text.contains(secret_refresh),
        "text leaked refresh: {text}"
    );
    assert!(!json.contains(secret_access), "json leaked access: {json}");
    assert!(
        !json.contains(secret_refresh),
        "json leaked refresh: {json}"
    );
}

/// BackendUnavailable must be a *distinct* diagnostic from Absent (spec SC1:
/// doctor distinguishes "missing entry" from "no keychain daemon").
#[test]
fn stored_credential_backend_unavailable_is_distinct_from_absent() {
    use opi_ai::CredentialSource;
    use opi_coding_agent::config::CredentialBackendSource;

    let mut config = OpiConfig::default();
    config.defaults.model = "anthropic:claude-distinct".into();
    config.defaults.credential_backend = Some(CredentialBackendSource::Keychain);

    let dir = tempfile::tempdir().unwrap();
    let env_probe = |_: &str| None;

    let mut absent = HashMap::new();
    absent.insert("anthropic".to_string(), CredentialSource::Absent);
    let absent_report = run_doctor(
        &[DoctorScope::Provider],
        &DoctorContext {
            config: &config,
            config_error: None,
            workspace_root: Path::new("."),
            user_config_dir: dir.path(),
            sessions_dir: dir.path(),
            term: None,
            term_program: None,
            term_features: None,
            no_color: false,
            colorterm: None,
            env_var: &env_probe,
            store_probe: &absent,
        },
    );

    let mut unavail = HashMap::new();
    unavail.insert(
        "anthropic".to_string(),
        CredentialSource::BackendUnavailable {
            reason: "no keychain daemon".to_owned(),
        },
    );
    let unavail_report = run_doctor(
        &[DoctorScope::Provider],
        &DoctorContext {
            config: &config,
            config_error: None,
            workspace_root: Path::new("."),
            user_config_dir: dir.path(),
            sessions_dir: dir.path(),
            term: None,
            term_program: None,
            term_features: None,
            no_color: false,
            colorterm: None,
            env_var: &env_probe,
            store_probe: &unavail,
        },
    );

    // Different diagnostic codes: Absent -> doctor_provider_credentials,
    // BackendUnavailable -> doctor_provider_credential_backend.
    let absent_code = absent_report
        .entries
        .iter()
        .find(|e| e.diagnostic.source == opi_agent::diagnostic::SOURCE_PROVIDER)
        .map(|e| e.diagnostic.code)
        .expect("absent provider diagnostic");
    let unavail_code = unavail_report
        .entries
        .iter()
        .find(|e| e.diagnostic.source == opi_agent::diagnostic::SOURCE_PROVIDER)
        .map(|e| e.diagnostic.code)
        .expect("unavailable provider diagnostic");
    assert_ne!(absent_code, unavail_code);
    assert_eq!(unavail_code, "doctor_provider_credential_backend");
}
